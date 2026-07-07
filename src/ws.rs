//! WebSocket transport: runs the protocol over a connection the Lua
//! application accepted, pumping frames between the socket and the Lua
//! thread. The handshake decision itself is made in Lua (see server.rs);
//! here the upgrade is already done.

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use mlua::prelude::*;
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Role, WebSocketConfig};

use crate::types::{Msg, WsEvent};

/// Same ceiling as HTTP bodies: a frame or message larger than this
/// closes the connection with a protocol error.
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

/// The Lua-facing side of one open connection. Exposed to Lua as a
/// userdata with `send` and `close`; both just queue on the outbound
/// channel consumed by the connection task, so they never block the Lua
/// thread. They return false once the connection is gone.
pub struct WsHandle {
    tx: mpsc::UnboundedSender<Message>,
}

impl WsHandle {
    pub(crate) fn send_text(&self, text: String) -> bool {
        self.tx.send(Message::Text(text.into())).is_ok()
    }

    pub(crate) fn close(&self, code: u16, reason: String) -> bool {
        let frame = CloseFrame { code: CloseCode::from(code), reason: reason.into() };
        self.tx.send(Message::Close(Some(frame))).is_ok()
    }
}

impl LuaUserData for WsHandle {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("send", |_, this, (data, binary): (LuaString, Option<bool>)| {
            if binary.unwrap_or(false) {
                let bytes = Bytes::from(data.as_bytes().to_vec());
                Ok(this.tx.send(Message::Binary(bytes)).is_ok())
            } else {
                let text = std::str::from_utf8(&data.as_bytes())
                    .map_err(|_| {
                        LuaError::runtime(
                            "ludi: a text frame must be valid UTF-8 (send(data, true) for binary)",
                        )
                    })?
                    .to_owned();
                Ok(this.send_text(text))
            }
        });

        methods.add_method("close", |_, this, (code, reason): (Option<u16>, Option<String>)| {
            Ok(this.close(code.unwrap_or(1000), reason.unwrap_or_default()))
        });
    }
}

/// Serves one upgraded connection until either side closes: inbound
/// frames become `Msg::WsEvent`s for the Lua thread, outbound messages
/// queued by the `WsHandle` are written to the socket. Always emits a
/// final `Close` event so Lua can drop the connection from its registry.
pub async fn run(upgraded: Upgraded, id: u64, msgs: mpsc::UnboundedSender<Msg>) {
    let mut config = WebSocketConfig::default();
    config.max_message_size = Some(MAX_MESSAGE_BYTES);
    config.max_frame_size = Some(MAX_MESSAGE_BYTES);

    let stream =
        WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, Some(config)).await;
    let (mut write, mut read) = stream.split();

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();
    if msgs.send(Msg::WsOpen { id, handle: WsHandle { tx: out_tx } }).is_err() {
        return;
    }

    let mut close = WsEvent::Close { code: None, reason: String::new() };

    loop {
        tokio::select! {
            frame = read.next() => match frame {
                Some(Ok(Message::Text(text))) => {
                    let event = WsEvent::Message { data: text.as_bytes().to_vec(), binary: false };
                    let _ = msgs.send(Msg::WsEvent { id, event });
                }
                Some(Ok(Message::Binary(data))) => {
                    let event = WsEvent::Message { data: data.to_vec(), binary: true };
                    let _ = msgs.send(Msg::WsEvent { id, event });
                }
                Some(Ok(Message::Ping(payload))) => {
                    if write.send(Message::Pong(payload)).await.is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {}
                Some(Ok(Message::Close(frame))) => {
                    if let Some(frame) = frame {
                        close = WsEvent::Close {
                            code: Some(frame.code.into()),
                            reason: frame.reason.to_string(),
                        };
                    }
                    break;
                }
                Some(Err(err)) => {
                    let event = WsEvent::Error { message: err.to_string() };
                    let _ = msgs.send(Msg::WsEvent { id, event });
                    break;
                }
                None => break,
            },
            queued = out_rx.recv() => match queued {
                Some(message) => {
                    let closing = matches!(message, Message::Close(_));
                    if write.send(message).await.is_err() || closing {
                        break;
                    }
                }
                // Every WsHandle clone is gone (Lua GC'd the connection
                // without closing it): shut down politely.
                None => break,
            },
        }
    }

    let _ = write.close().await;
    let _ = msgs.send(Msg::WsEvent { id, event: close });
}
