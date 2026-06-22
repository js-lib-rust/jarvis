use crate::error::AppError;
use crate::llm::RouterResponse;
use crate::types::Result;
use bson::DateTime;
use log::{debug, error, info, trace, warn};
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{OnceCell, Semaphore, mpsc, oneshot};
use tokio::time::{interval, sleep, timeout};

// retry delay for TCP connection with router service
const CONNECTION_RETRY_DELAY: Duration = Duration::from_secs(4);

// timeout for a single routing request (covers full TCP round-trip)
const ROUTING_TIMEOUT: Duration = Duration::from_secs(2);

const MAX_PAYLOAD_LEN: usize = 1000;

/// Messages sent from client to server.
#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
enum TcpRequest {
    Ping,
    Request { payload: Value },
}

/// Messages received from server to client.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
enum TcpResponse {
    Pong,
    Response {
        estimated_confidence: f32,
        confidence: f32,
        text: String,
        processing_time: f32,
    },
}

impl TryFrom<TcpResponse> for RouterResponse {
    type Error = AppError;

    fn try_from(tcp_response: TcpResponse) -> Result<Self> {
        match tcp_response {
            TcpResponse::Response {
                estimated_confidence,
                confidence,
                text,
                processing_time,
            } => {
                debug!("estimated_confidence: {}", estimated_confidence);
                debug!("confidence: {}", confidence);
                debug!("text: {}", text);
                debug!("processing_time: {}", processing_time);
                Ok(RouterResponse {
                    estimated_confidence,
                    confidence,
                    text,
                    processing_time,
                })
            }

            TcpResponse::Pong => Err(AppError::Fatal(
                "Unexpected Pong on response channel".to_string(),
            )),
        }
    }
}

/// Internal command type used to send instructions to the background worker.
#[derive(Debug)]
enum ControlMessage {
    Send {
        tcp_request: TcpRequest,
        // sender end of the oneshot response channel between TCP connection manager thread and client thread
        // response channel is used to convey server response message back to client
        response_channel_sender: oneshot::Sender<TcpResponse>,
    },
    Shutdown,
}

/// The public handle used by the rest of the application.
/// It is cheap to clone and can be passed around easily.
#[derive(Clone, Debug)]
pub(crate) struct RouterClient {
    control_channel_sender: mpsc::Sender<ControlMessage>,
    get_routing_semaphore: Arc<Semaphore>,
}

impl RouterClient {
    /// Establishes a connection and starts a background manager that handles
    /// automatic reconnections if the socket fails.
    pub async fn connect(router_address: &str) -> Result<Self> {
        trace!("connect(router_address: &str) -> Result<Self>");
        let (control_sender, mut control_receiver) = mpsc::channel::<ControlMessage>(32);

        // the connection manager loop: lives for the entire duration of the application
        let router_address = router_address.to_string();
        tokio::spawn(async move {
            loop {
                info!("Attempting to connect to {}...", router_address);

                match TcpStream::connect(&router_address).await {
                    Ok(stream) => {
                        info!("Successfully connected to {}", router_address);
                        let mut tcp_connection = TcpConnection::new(stream);
                        match tcp_connection.run(&mut control_receiver).await {
                            Err(AppError::Shutdown) => {
                                info!("Shutdown complete.");
                                break;
                            }
                            _ => {
                                warn!(
                                    "Routing server connection lost. Reconnecting in {}...",
                                    CONNECTION_RETRY_DELAY.as_secs()
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("Connection failed: {}. Retrying...", e);
                    }
                }

                sleep(CONNECTION_RETRY_DELAY).await;
            }
        });

        Ok(Self {
            control_channel_sender: control_sender,
            get_routing_semaphore: Arc::new(Semaphore::new(1)),
        })
    }

    /// Shuts down the background connection manager.
    pub async fn shutdown(&self) -> Result<()> {
        self.control_channel_sender
            .send(ControlMessage::Shutdown)
            .await
            .map_err(|e| AppError::Fatal(format!("Background worker died: {}", e)))
    }

    /// Sends a request to the router and waits for a response.
    /// Rejects immediately if another request is already in flight.
    pub async fn get_routing(&self, prompt: &str) -> Result<RouterResponse> {
        trace!("get_routing(&self, prompt: &str) -> Result<RouterResponse>");
        let start = Instant::now();

        let _permit = self
            .get_routing_semaphore
            .try_acquire()
            .map_err(|_| AppError::Fatal("Router is busy".into()))?;

        let response = match timeout(ROUTING_TIMEOUT, self.do_routing(prompt)).await {
            Ok(result) => result?,
            Err(error) => {
                error!("Router error: {}", error);
                return Err(AppError::from(error));
            }
        };

        RouterMetrics::new(prompt, &response, start.elapsed().as_secs_f32())
            .save()
            .await;
        Ok(response)
    }

    async fn do_routing(&self, prompt: &str) -> Result<RouterResponse> {
        trace!("do_routing(&self, prompt: &str) -> Result<RouterResponse>");
        let (response_channel_sender, response_channel_receiver) = oneshot::channel();
        let start = Instant::now();

        #[derive(Serialize)]
        struct Payload<'a> {
            prompt: &'a str,
        }
        let tcp_request = TcpRequest::Request {
            payload: serde_json::to_value(Payload { prompt })?,
        };

        // Send the instruction to the background worker
        self.control_channel_sender
            .send(ControlMessage::Send {
                tcp_request,
                response_channel_sender,
            })
            .await
            .map_err(|e| AppError::Fatal(format!("Background worker died: {}", e)))?;

        let response = response_channel_receiver
            .await
            .map_err(|e| AppError::Fatal(format!("Response channel closed: {}", e)))?;

        debug!(
            "Routing processing time: {} ms",
            start.elapsed().as_millis()
        );
        RouterResponse::try_from(response)
    }
}

/// Manages the active TCP connection and the protocol logic.
struct TcpConnection {
    // read half of the TCP stream with the router server
    tcp_reader: ReadHalf<TcpStream>,
    // write half of the TCP stream with the router server
    tcp_writer: WriteHalf<TcpStream>,
    // sender end of the oneshot response channel between TCP connection manager thread and client thread
    // response channel is used to convey server response message back to client
    response_channel_sender: Option<oneshot::Sender<TcpResponse>>,
}

impl TcpConnection {
    fn new(tcp_stream: TcpStream) -> Self {
        trace!("new(tcp_stream: TcpStream) -> Self");
        let (tcp_reader, tcp_writer) = io::split(tcp_stream);
        Self {
            tcp_reader,
            tcp_writer,
            response_channel_sender: None,
        }
    }

    /// The main event loop for the active connection.
    async fn run(
        &mut self,
        control_channel_receiver: &mut mpsc::Receiver<ControlMessage>,
    ) -> Result<()> {
        trace!("run(&mut self, control_channel_receiver: &mut mpsc::Receiver<ControlMessage>)");
        let mut heartbeat_timer = interval(Duration::from_secs(10));

        loop {
            let result = tokio::select! {
                _ = heartbeat_timer.tick() => self.on_heartbeat().await,
                Some(control_message) = control_channel_receiver.recv() => self.on_control_message(control_message).await,
                Ok(tcp_response) = self.read_tcp_message() => self.on_tcp_message(tcp_response).await,
            };
            if let Err(e) = result {
                if matches!(e, AppError::Shutdown) {
                    return Err(e);
                }
                error!("Fail on router processing: {}", e);
                break;
            }
        }
        Ok(())
    }

    async fn on_heartbeat(&mut self) -> Result<()> {
        trace!("on_heartbeat(&mut self) -> Result<()>");
        self.write_tcp_message(&TcpRequest::Ping).await
    }

    async fn on_control_message(&mut self, control_message: ControlMessage) -> Result<()> {
        trace!("on_control_message(&mut self, control_message: ControlMessage) -> Result<()>");
        debug!("control_message: {:?}", control_message);

        match control_message {
            ControlMessage::Send {
                tcp_request: request,
                response_channel_sender,
            } => {
                self.write_tcp_message(&request).await?;
                self.response_channel_sender = Some(response_channel_sender);
            }

            ControlMessage::Shutdown => {
                info!("Shutdown command received. Closing connection...");
                return Err(AppError::Shutdown);
            }
        }
        Ok(())
    }

    async fn on_tcp_message(&mut self, tcp_response: TcpResponse) -> Result<()> {
        trace!("on_tcp_message(&mut self, tcp_response: TcpResponse) -> Result<()>");
        debug!("tcp_response: {:?}", tcp_response);

        match tcp_response {
            TcpResponse::Pong => (), // heartbeat acknowledged, do nothing
            TcpResponse::Response { .. } => {
                if let Some(response_channel_sender) = self.response_channel_sender.take() {
                    let _ = response_channel_sender.send(tcp_response);
                }
            }
        };
        Ok(())
    }

    /// Writes a length-prefixed JSON message to the stream.
    async fn write_tcp_message(&mut self, tcp_request: &TcpRequest) -> Result<()> {
        trace!("write_tcp_message(&mut self, tcp_request: &TcpRequest) -> Result<()>");
        debug!("tcp_request: {:?}", tcp_request);

        let json = serde_json::to_vec(tcp_request)?;
        let json_length = json.len() as u32;

        self.tcp_writer
            .write_all(&json_length.to_be_bytes())
            .await?;
        self.tcp_writer.write_all(&json).await?;
        self.tcp_writer.flush().await?;
        Ok(())
    }

    /// Reads a length-prefixed JSON message from the stream.
    async fn read_tcp_message(&mut self) -> Result<TcpResponse> {
        trace!("read_tcp_message(&mut self) -> Result<TcpResponse>");

        let mut length_buffer = [0u8; 4];
        self.tcp_reader.read_exact(&mut length_buffer).await?;
        let payload_len = u32::from_be_bytes(length_buffer) as usize;
        debug!("payload_len: {}", payload_len);

        if payload_len > MAX_PAYLOAD_LEN {
            let error = format!("router response too large: {}", payload_len);
            error!("{}", error);
            return Err(AppError::Fatal(error));
        }

        let mut payload = vec![0u8; payload_len];
        self.tcp_reader.read_exact(&mut payload).await?;

        let response = serde_json::from_slice(&payload)?;
        debug!("response: {:?}", response);
        Ok(response)
    }
}

// --------------------------------------------------------
// Router Metrics Database

const SERVER: &str = "mongodb://localhost:27017";
const DATABASE: &str = "jarvis";
const COLLECTION: &str = "router_metrics";

static MONGO_CLIENT: OnceCell<Client> = OnceCell::const_new();
static MONGO_COLLECTION: OnceCell<Collection<RouterMetrics>> = OnceCell::const_new();

async fn collection() -> Result<&'static Collection<RouterMetrics<'static>>> {
    let client = MONGO_CLIENT
        .get_or_try_init(|| async { Client::with_uri_str(SERVER).await })
        .await?;

    let collection = MONGO_COLLECTION
        .get_or_init(|| async {
            client
                .database(DATABASE)
                .collection::<RouterMetrics>(COLLECTION)
        })
        .await;

    Ok(collection)
}

#[derive(Serialize)]
struct RouterMetrics<'a> {
    timestamp: DateTime,
    prompt: &'a str,
    text: &'a str,
    estimated_confidence: f32,
    confidence: f32,
    processing_time: f32,
    request_time: f32,
}

impl<'a> RouterMetrics<'a> {
    fn new(prompt: &'a str, response: &'a RouterResponse, request_time: f32) -> Self {
        Self {
            timestamp: DateTime::now(),
            prompt: prompt,
            text: &response.text,
            estimated_confidence: response.estimated_confidence,
            confidence: response.confidence,
            processing_time: response.processing_time,
            request_time: request_time,
        }
    }

    async fn save(&self) {
        // best effort; is not critic if metrics are lost since they are used for statistics
        if let Ok(collection) = collection().await {
            let _ = collection.insert_one(self, None).await;
        }
    }
}
