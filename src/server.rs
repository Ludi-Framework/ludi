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

