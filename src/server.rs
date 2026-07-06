//! HTTP transport: accepts connections with hyper and forwards every
//! request as a `Job` through the channel consumed by the Lua thread.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

use crate::types::{Job, Request};

const MAX_BODY_BYTES: usize = 1024 * 1024;

pub async fn serve(port: u16, jobs: mpsc::UnboundedSender<Job>) {
    let listener = match TcpListener::bind(("0.0.0.0", port)).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("ludi: failed to bind port {port}: {err}");
            std::process::exit(1);
        }
    };

    println!("Ludi listening on http://localhost:{port}");
    serve_on(listener, jobs).await
}

async fn serve_on(listener: TcpListener, jobs: mpsc::UnboundedSender<Job>) {
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
                .await
            {
                eprintln!("ludi: connection error: {err}");
            }
        });
    }
}

async fn handle(
    req: HyperRequest<Incoming>,
    jobs: mpsc::UnboundedSender<Job>,
) -> Result<HyperResponse<Full<Bytes>>, hyper::Error> {
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
        headers: parts
            .headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect(),
        body: body.to_vec(),
    };

    let (respond, response_rx) = oneshot::channel();

    if jobs.send(Job { request, respond }).is_err() {
        return Ok(plain(StatusCode::INTERNAL_SERVER_ERROR, "Server shutting down"));
    }

    let response = match response_rx.await {
        Ok(response) => response,
        Err(_) => return Ok(plain(StatusCode::INTERNAL_SERVER_ERROR, "Handler dropped request")),
    };

    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = HyperResponse::builder().status(status);
    for (name, value) in response.headers {
        builder = builder.header(name, value);
    }

    Ok(builder
        .body(Full::new(Bytes::from(response.body)))
        .unwrap_or_else(|_| plain(StatusCode::INTERNAL_SERVER_ERROR, "Invalid response headers")))
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

    async fn spawn_test_server() -> (u16, mpsc::UnboundedReceiver<Job>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (jobs_tx, jobs_rx) = mpsc::unbounded_channel();
        tokio::spawn(serve_on(listener, jobs_tx));
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
            while let Some(job) = jobs_rx.recv().await {
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
        assert!(response.to_lowercase().contains("x-test: 1"), "got: {response}");
        assert!(
            response.contains("method=POST;path=/echo;query=a=1;body=hello"),
            "got: {response}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dropped_handler_returns_500() {
        let (port, mut jobs_rx) = spawn_test_server().await;

        tokio::spawn(async move {
            while let Some(job) = jobs_rx.recv().await {
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
