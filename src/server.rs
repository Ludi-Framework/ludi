//! HTTP transport: accepts connections with hyper and forwards every
//! request as a `Job` through the channel consumed by the Lua thread.
//! WebSocket handshakes take a detour: the request is forwarded as a
//! `WsUpgrade`, and when Lua accepts it the connection is upgraded and
//! handed to the ws module.

use std::sync::atomic::{AtomicU64, Ordering};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, UPGRADE};
use hyper::service::service_fn;
use hyper::{Method, Request as HyperRequest, Response as HyperResponse, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;

use crate::types::{Job, Msg, Request, Response, WsDecision, WsUpgrade};
use crate::ws;

const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Connection ids are process-global: the Lua registry keys on them, and
/// they must survive hot reloads without colliding.
static NEXT_WS_ID: AtomicU64 = AtomicU64::new(1);

pub async fn serve(listener: TcpListener, jobs: mpsc::UnboundedSender<Msg>) {
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(_) => continue,
        };

        let jobs = jobs.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| handle(req, jobs.clone()));

            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                eprintln!("ludi: connection error: {err}");
            }
        });
    }
}

async fn handle(
    req: HyperRequest<Incoming>,
    jobs: mpsc::UnboundedSender<Msg>,
) -> Result<HyperResponse<Full<Bytes>>, hyper::Error> {
    if is_ws_handshake(&req) {
        return handle_ws(req, jobs).await;
    }

    let (parts, body) = req.into_parts();

    let body = match http_body_util::Limited::new(body, MAX_BODY_BYTES)
        .collect()
        .await
    {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return Ok(plain(StatusCode::PAYLOAD_TOO_LARGE, "Payload too large")),
    };

    let request = Request {
        method: parts.method.to_string(),
        path: parts.uri.path().to_string(),
        query: parts.uri.query().unwrap_or("").to_string(),
        headers: convert_headers(&parts.headers),
        body: body.to_vec(),
    };

    let (respond, response_rx) = oneshot::channel();

    if jobs.send(Msg::Job(Job { request, respond })).is_err() {
        return Ok(plain(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server shutting down",
        ));
    }

    let response = match response_rx.await {
        Ok(response) => response,
        Err(_) => {
            return Ok(plain(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Handler dropped request",
            ));
        }
    };

    Ok(build_response(response))
}

/// A `GET` carrying the RFC 6455 upgrade headers. Anything half-formed
/// (wrong method, missing key) is served as plain HTTP instead, which
/// ends in the router's 404.
fn is_ws_handshake(req: &HyperRequest<Incoming>) -> bool {
    let headers = req.headers();
    let upgrade_is_websocket = headers
        .get(UPGRADE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("websocket"));
    let connection_upgrades = headers
        .get(CONNECTION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().contains("upgrade"));

    req.method() == Method::GET
        && upgrade_is_websocket
        && connection_upgrades
        && headers.contains_key(SEC_WEBSOCKET_KEY)
}

/// Forwards the handshake to the Lua thread and acts on its decision:
/// a rejection is answered like any HTTP response; an acceptance answers
/// `101` and hands the upgraded connection to the ws module.
async fn handle_ws(
    req: HyperRequest<Incoming>,
    jobs: mpsc::UnboundedSender<Msg>,
) -> Result<HyperResponse<Full<Bytes>>, hyper::Error> {
    if req
        .headers()
        .get("sec-websocket-version")
        .is_none_or(|version| version.as_bytes() != b"13")
    {
        return Ok(plain(
            StatusCode::UPGRADE_REQUIRED,
            "Unsupported WebSocket version",
        ));
    }

    let key = req.headers()[SEC_WEBSOCKET_KEY].as_bytes().to_vec();
    let request = Request {
        method: req.method().to_string(),
        path: req.uri().path().to_string(),
        query: req.uri().query().unwrap_or("").to_string(),
        headers: convert_headers(req.headers()),
        body: Vec::new(),
    };

    let id = NEXT_WS_ID.fetch_add(1, Ordering::Relaxed);
    let (respond, decision_rx) = oneshot::channel();

    if jobs
        .send(Msg::WsUpgrade(WsUpgrade {
            id,
            request,
            respond,
        }))
        .is_err()
    {
        return Ok(plain(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server shutting down",
        ));
    }

    match decision_rx.await {
        Err(_) => Ok(plain(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Handler dropped request",
        )),
        Ok(WsDecision::Reject(response)) => Ok(build_response(response)),
        Ok(WsDecision::Accept) => {
            tokio::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => ws::run(upgraded, id, jobs).await,
                    Err(err) => {
                        // The accepted connection never opened; a Close
                        // event lets Lua discard its pending handler.
                        eprintln!("ludi: websocket upgrade failed: {err}");
                        let event = crate::types::WsEvent::Close {
                            code: None,
                            reason: String::new(),
                        };
                        let _ = jobs.send(Msg::WsEvent { id, event });
                    }
                }
            });

            Ok(HyperResponse::builder()
                .status(StatusCode::SWITCHING_PROTOCOLS)
                .header(UPGRADE, "websocket")
                .header(CONNECTION, "Upgrade")
                .header(SEC_WEBSOCKET_ACCEPT, derive_accept_key(&key))
                .body(Full::new(Bytes::new()))
                .unwrap())
        }
    }
}

fn convert_headers(headers: &hyper::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect()
}

fn build_response(response: Response) -> HyperResponse<Full<Bytes>> {
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = HyperResponse::builder().status(status);
    for (name, value) in response.headers {
        builder = builder.header(name, value);
    }

    builder
        .body(Full::new(Bytes::from(response.body)))
        .unwrap_or_else(|_| {
            plain(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid response headers",
            )
        })
}

fn plain(status: StatusCode, message: &str) -> HyperResponse<Full<Bytes>> {
    HyperResponse::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from(message.to_string())))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Response;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn spawn_test_server() -> (u16, mpsc::UnboundedReceiver<Msg>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (jobs_tx, jobs_rx) = mpsc::unbounded_channel();
        tokio::spawn(serve(listener, jobs_tx));
        (port, jobs_rx)
    }

    async fn raw_request(port: u16, request: &str) -> String {
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn request_reaches_handler_and_response_comes_back() {
        let (port, mut jobs_rx) = spawn_test_server().await;

        // Emulates the Lua thread: echoes the parsed request back.
        tokio::spawn(async move {
            while let Some(Msg::Job(job)) = jobs_rx.recv().await {
                let req = &job.request;
                let body = format!(
                    "method={};path={};query={};body={}",
                    req.method,
                    req.path,
                    req.query,
                    String::from_utf8_lossy(&req.body)
                );
                let _ = job.respond.send(Response {
                    status: 201,
                    headers: vec![("X-Test".into(), "1".into())],
                    body: body.into_bytes(),
                });
            }
        });

        let response = raw_request(
            port,
            "POST /echo?a=1 HTTP/1.1\r\nHost: t\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 201"), "got: {response}");
        assert!(
            response.to_lowercase().contains("x-test: 1"),
            "got: {response}"
        );
        assert!(
            response.contains("method=POST;path=/echo;query=a=1;body=hello"),
            "got: {response}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dropped_handler_returns_500() {
        let (port, mut jobs_rx) = spawn_test_server().await;

        tokio::spawn(async move {
            while let Some(Msg::Job(job)) = jobs_rx.recv().await {
                drop(job.respond);
            }
        });

        let response = raw_request(
            port,
            "GET / HTTP/1.1\r\nHost: t\r\nConnection: close\r\n\r\n",
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 500"), "got: {response}");
    }

    /// Emulates the Lua thread for the ws tests: accepts every handshake
    /// and echoes text messages back with an "echo: " prefix.
    fn spawn_echo_app(mut jobs_rx: mpsc::UnboundedReceiver<Msg>) {
        tokio::spawn(async move {
            let mut handles = std::collections::HashMap::new();
            while let Some(msg) = jobs_rx.recv().await {
                match msg {
                    Msg::WsUpgrade(upgrade) => {
                        let _ = upgrade.respond.send(WsDecision::Accept);
                    }
                    Msg::WsOpen { id, handle } => {
                        handles.insert(id, handle);
                    }
                    Msg::WsEvent {
                        id,
                        event:
                            crate::types::WsEvent::Message {
                                data,
                                binary: false,
                            },
                    } => {
                        let text = String::from_utf8(data).unwrap();
                        handles[&id].send_text(format!("echo: {text}"));
                    }
                    _ => {}
                }
            }
        });
    }

    async fn ws_client(
        port: u16,
        path: &str,
    ) -> Result<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        tokio_tungstenite::tungstenite::Error,
    > {
        let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        tokio_tungstenite::client_async(format!("ws://127.0.0.1:{port}{path}"), stream)
            .await
            .map(|(ws, _)| ws)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_handshake_upgrades_and_echoes() {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message;

        let (port, jobs_rx) = spawn_test_server().await;
        spawn_echo_app(jobs_rx);

        let mut client = ws_client(port, "/chat").await.unwrap();
        client.send(Message::Text("hi".into())).await.unwrap();

        let reply = client.next().await.unwrap().unwrap();
        assert_eq!(reply.to_text().unwrap(), "echo: hi");

        client.close(None).await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejected_handshake_gets_the_http_response() {
        let (port, mut jobs_rx) = spawn_test_server().await;

        tokio::spawn(async move {
            while let Some(msg) = jobs_rx.recv().await {
                if let Msg::WsUpgrade(upgrade) = msg {
                    let _ = upgrade.respond.send(WsDecision::Reject(Response {
                        status: 404,
                        headers: vec![("Content-Type".into(), "application/json".into())],
                        body: br#"{"error":"Not Found"}"#.to_vec(),
                    }));
                }
            }
        });

        let err = ws_client(port, "/nope").await.unwrap_err();
        match err {
            tokio_tungstenite::tungstenite::Error::Http(response) => {
                assert_eq!(response.status(), 404);
            }
            other => panic!("expected an HTTP error, got: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn client_close_emits_a_close_event() {
        use futures_util::SinkExt;

        let (port, mut jobs_rx) = spawn_test_server().await;
        let (events_tx, mut events_rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            // Handles must stay alive: dropping one shuts its connection down.
            let mut handles = std::collections::HashMap::new();
            while let Some(msg) = jobs_rx.recv().await {
                match msg {
                    Msg::WsUpgrade(upgrade) => {
                        let _ = upgrade.respond.send(WsDecision::Accept);
                    }
                    Msg::WsOpen { id, handle } => {
                        handles.insert(id, handle);
                    }
                    Msg::WsEvent {
                        id,
                        event: crate::types::WsEvent::Close { code, .. },
                    } => {
                        handles.remove(&id);
                        let _ = events_tx.send(code);
                    }
                    _ => {}
                }
            }
        });

        use tokio_tungstenite::tungstenite::protocol::CloseFrame;
        use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

        let mut client = ws_client(port, "/chat").await.unwrap();
        client
            .close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "".into(),
            }))
            .await
            .unwrap();

        let code = events_rx.recv().await.unwrap();
        assert_eq!(code, Some(1000));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn oversized_body_returns_413() {
        let (port, _jobs_rx) = spawn_test_server().await;

        let body = "x".repeat(MAX_BODY_BYTES + 1);
        let request = format!(
            "POST / HTTP/1.1\r\nHost: t\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let response = raw_request(port, &request).await;

        assert!(response.starts_with("HTTP/1.1 413"), "got: {response}");
    }
}
