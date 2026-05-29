use crate::error::AppError;
use crate::llm::RouterResponse;
use crate::types::Result;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval, sleep, timeout};

// retry delay in seconds for TCP connection with router service 
static CONNECTION_RETRY_DELAY: Duration = Duration::from_secs(4);

/// The public message format used for communication.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
enum TcpMessage {
    Ping,
    Pong,
    Request {
        payload: Value,
    },
    Response {
        text: String,
        confidence: f32,
        duration: f32,
    },
}

impl Into<Result<RouterResponse>> for TcpMessage {
    fn into(self) -> Result<RouterResponse> {
        match self {
            TcpMessage::Response {
                text, confidence, ..
            } => Ok(RouterResponse { text, confidence }),
            _ => Err(AppError::Fatal(
                "Only TCP response message can be converted into router response.".to_string(),
            )),
        }
    }
}

/// Internal command type used to send instructions to the background worker.
#[derive(Debug)]
enum ControlMessage {
    Send {
        request: TcpMessage,
        // sender end of the oneshot response channel between TCP connection manager thread and client thread
        // response channel is used to convey server response message back to client
        response_channel_sender: oneshot::Sender<TcpMessage>,
    },
    _Shutdown,
}

/// The public handle used by the rest of the application.
/// It is cheap to clone and can be passed around easily.
#[derive(Clone, Debug)]
pub(crate) struct RouterClient {
    control_channel_sender: mpsc::Sender<ControlMessage>,
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
                        tcp_connection.run(&mut control_receiver).await;
                        warn!("Routing server connection lost. Reconnecting in 5 seconds...");
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
        })
    }

    /// Shuts down the background connection manager.
    pub async fn _shutdown(&self) -> Result<()> {
        self.control_channel_sender
            .send(ControlMessage::_Shutdown)
            .await
            .map_err(|e| AppError::Fatal(format!("Background worker died: {}", e)))
    }

    /// Sends a request to the router and waits for a response.
    /// This method handles the high-level logic of the request-response lifecycle.
    pub async fn get_routing(&self, prompt: &str) -> Result<RouterResponse> {
        trace!("get_routing(&self, prompt: &str) -> Result<RouterResponse>");
        let (response_channel_sender, response_channel_receiver) = oneshot::channel();
        let start = Instant::now();

        #[derive(Serialize)]
        struct Payload<'a> {
            prompt: &'a str,
        }
        let request = TcpMessage::Request {
            payload: serde_json::to_value(Payload { prompt })?,
        };

        // Send the instruction to the background worker
        self.control_channel_sender
            .send(ControlMessage::Send {
                request,
                response_channel_sender,
            })
            .await
            .map_err(|e| AppError::Fatal(format!("Background worker died: {}", e)))?;

        // Wait for the response with a timeout
        let response = timeout(Duration::from_secs(10), response_channel_receiver)
            .await?
            .map_err(|e| AppError::Fatal(format!("Response channel closed: {}", e)))?;

        debug!(
            "Routing processing time: {} ms",
            start.elapsed().as_millis()
        );
        response.into()
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
    response_channel_sender: Option<oneshot::Sender<TcpMessage>>,
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
    async fn run(&mut self, control_channel_receiver: &mut mpsc::Receiver<ControlMessage>) {
        trace!("run(&mut self, control_channel_receiver: &mut mpsc::Receiver<ControlMessage>)");
        let mut heartbeat_timer = interval(Duration::from_secs(10));

        loop {
            let result = tokio::select! {
                _ = heartbeat_timer.tick() => self.on_heartbeat().await,
                Some(control_message) = control_channel_receiver.recv() => self.on_control_message(control_message).await,
                Ok(tcp_message) = self.read_tcp_message() => self.on_tcp_message(tcp_message).await,
            };
            if let Err(error) = result {
                error!("Fail on router processing: {}", error);
                break;
            }
        }
    }

    async fn on_heartbeat(&mut self) -> Result<()> {
        trace!("on_heartbeat(&mut self) -> Result<()>");
        self.write_tcp_message(&TcpMessage::Ping).await
    }

    async fn on_control_message(&mut self, control_message: ControlMessage) -> Result<()> {
        trace!("on_control_message(&mut self, control_message: ControlMessage) -> Result<()>");
        debug!("control_message: {:?}", control_message);

        match control_message {
            ControlMessage::Send {
                request,
                response_channel_sender,
            } => {
                // if a previous request is still pending, notify it that we are busy
                if let Some(old_response_channel_sender) = self.response_channel_sender.take() {
                    let _ = old_response_channel_sender.send(TcpMessage::Response {
                        text: "Error: Busy".into(),
                        confidence: 0.0,
                        duration: 0.0,
                    });
                }

                self.write_tcp_message(&request).await?;
                self.response_channel_sender = Some(response_channel_sender);
            }

            ControlMessage::_Shutdown => {
                info!("Shutdown command received. Closing connection...");
            }
        }
        Ok(())
    }

    async fn on_tcp_message(&mut self, tcp_message: TcpMessage) -> Result<()> {
        trace!("on_tcp_message(&mut self, tcp_message: TcpMessage) -> Result<()>");
        debug!("tcp_message: {:?}", tcp_message);

        match tcp_message {
            TcpMessage::Pong => (), // heartbeat acknowledged, do nothing

            TcpMessage::Response { .. } => {
                if let Some(response_channel_sender) = self.response_channel_sender.take() {
                    let _ = response_channel_sender.send(tcp_message);
                }
            }

            _ => {
                return Err(AppError::Fatal(
                    "Unexpected TCP response message.".to_string(),
                ));
            }
        };
        Ok(())
    }

    /// Writes a length-prefixed JSON message to the stream.
    async fn write_tcp_message(&mut self, tcp_message: &TcpMessage) -> Result<()> {
        trace!("write_tcp_message(&mut self, tcp_message: &TcpMessage) -> Result<()>");
        debug!("tcp_message: {:?}", tcp_message);

        let json = serde_json::to_vec(tcp_message)?;
        let json_length = json.len() as u32;

        self.tcp_writer
            .write_all(&json_length.to_be_bytes())
            .await?;
        self.tcp_writer.write_all(&json).await?;
        self.tcp_writer.flush().await?;
        Ok(())
    }

    /// Reads a length-prefixed JSON message from the stream.
    async fn read_tcp_message(&mut self) -> Result<TcpMessage> {
        trace!("read_tcp_message(&mut self) -> Result<TcpMessage>");

        let mut json_length_buffer = [0u8; 4];
        self.tcp_reader.read_exact(&mut json_length_buffer).await?;
        let json_length = u32::from_be_bytes(json_length_buffer) as usize;

        let mut json_buffer = vec![0u8; json_length];
        self.tcp_reader.read_exact(&mut json_buffer).await?;

        let message = serde_json::from_slice(&json_buffer)?;
        Ok(message)
    }
}
