use crate::error::AppError;
use crate::types::Result;
use log::debug;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval, timeout};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RouterMessage {
    Ping,
    Pong,
    Request {
        payload: serde_json::Value,
    },
    Response {
        text: String,
        confidence: f32,
        duration: f32,
    },
}

#[derive(Clone, Debug)]
pub struct RouterClient {
    cmd_tx: mpsc::Sender<ClientCommand>,
}

#[derive(Debug)]
enum ClientCommand {
    Send {
        request: RouterMessage,
        resp_tx: oneshot::Sender<RouterMessage>,
    },
}

impl RouterClient {
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (mut reader, mut writer) = tokio::io::split(stream);
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<ClientCommand>(32);

        tokio::spawn(async move {
            let mut heartbeat_timer = interval(Duration::from_secs(10));
            // use a single slot for the pending response because it's a dedicated connection
            let mut pending_responder: Option<oneshot::Sender<RouterMessage>> = None;

            loop {
                tokio::select! {
                    // 1. Handle Heartbeats
                    _ = heartbeat_timer.tick() => {
                        let p = serde_json::to_vec(&RouterMessage::Ping).unwrap();
                        let _ = writer.write_all(&(p.len() as u32).to_be_bytes()).await;
                        let _ = writer.write_all(&p).await;
                    }

                    // 2. Handle Outgoing Requests
                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            ClientCommand::Send { request, resp_tx } => {
                                // If there was a previous request still waiting, drop it (or error)
                                if let Some(old) = pending_responder.take() {
                                    let _ = old.send(RouterMessage::Response { text: "Error: Busy".into(), confidence: 1.0, duration: 0.0 });
                                }

                                let p = serde_json::to_vec(&request).unwrap();
                                if writer.write_all(&(p.len() as u32).to_be_bytes()).await.is_ok() {
                                    if writer.write_all(&p).await.is_ok() {
                                        pending_responder = Some(resp_tx);
                                    }
                                }
                            }
                        }
                    }

                    // 3. Handle Incoming Messages
                    res = read_next_msg(&mut reader) => {
                        match res {
                            Ok(msg) => {
                                match msg {
                                    RouterMessage::Pong => { /* Heartbeat acknowledged, do nothing */ }
                                    RouterMessage::Request { .. } | RouterMessage::Response { .. } => {
                                        // This is the response to our request!
                                        if let Some(tx) = pending_responder.take() {
                                            let _ = tx.send(msg);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Err(_) => break, // Connection closed
                        }
                    }
                }
            }
        });

        Ok(RouterClient { cmd_tx })
    }

    pub async fn request(&self, prompt: &str) -> Result<RouterMessage> {
        let (tx, rx) = oneshot::channel();

        let start = Instant::now();
        let payload = serde_json::json!({"prompt": prompt});
        let request = RouterMessage::Request { payload };

        self.cmd_tx
            .send(ClientCommand::Send {
                request,
                resp_tx: tx,
            })
            .await
            .map_err(|e| AppError::Fatal(e.to_string()))?;

        let result = timeout(Duration::from_secs(10), rx)
            .await?
            .map_err(|e| AppError::Fatal(e.to_string()));
        debug!(
            "Routing processing time: {} msec.",
            start.elapsed().as_millis()
        );
        result
    }
}

// Helper function to handle the framing logic
async fn read_next_msg(reader: &mut tokio::io::ReadHalf<TcpStream>) -> Result<RouterMessage> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;

    let msg = serde_json::from_slice(&payload)?;
    Ok(msg)
}
