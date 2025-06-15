use axum::{
    Router,
    extract::Request,
    http::{Method, StatusCode},
};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

type RoutesMap = Lazy<Arc<RwLock<HashMap<(Method, String), String>>>>;

static ROUTES: RoutesMap = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub fn add_route(method: Method, path: &str, response: &str) {
    let mut routes = ROUTES.write().unwrap();
    routes.insert((method, path.to_string()), response.to_string());
}

pub async fn start_server(port: u16) {
    let app = Router::new().fallback(fallback_handler);

    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

pub async fn fallback_handler(req: Request) -> (StatusCode, String) {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    let routes = ROUTES.read().unwrap();
    match routes.get(&(method.clone(), path)) {
        Some(resp) => (StatusCode::OK, resp.clone()),
        None => match method {
            Method::GET => (StatusCode::NOT_FOUND, "GET not found".to_string()),
            Method::POST => (StatusCode::NOT_FOUND, "POST not found".to_string()),
            _ => (
                StatusCode::METHOD_NOT_ALLOWED,
                "Method not allowed".to_string(),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request};

    #[tokio::test]
    async fn test_get_route_found() {
        add_route(Method::GET, "/hello", "Hello GET!");

        let req = Request::builder()
            .method(Method::GET)
            .uri("/hello")
            .body(Body::empty())
            .unwrap();

        let (status, body) = fallback_handler(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "Hello GET!");
    }

    #[tokio::test]
    async fn test_get_route_not_found() {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/nope")
            .body(Body::empty())
            .unwrap();

        let (status, body) = fallback_handler(req).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, "GET not found");
    }

    #[tokio::test]
    async fn test_post_route_found() {
        add_route(Method::POST, "/submit", "Hello POST!");

        let req = Request::builder()
            .method(Method::POST)
            .uri("/submit")
            .body(Body::empty())
            .unwrap();

        let (status, body) = fallback_handler(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "Hello POST!");
    }

    #[tokio::test]
    async fn test_post_route_not_found() {
        let req = Request::builder()
            .method(Method::POST)
            .uri("/nope")
            .body(Body::empty())
            .unwrap();

        let (status, body) = fallback_handler(req).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, "POST not found");
    }

    #[tokio::test]
    async fn test_method_not_allowed() {
        let req = Request::builder()
            .method(Method::PUT)
            .uri("/hello")
            .body(Body::empty())
            .unwrap();

        let (status, body) = fallback_handler(req).await;

        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(body, "Method not allowed");
    }
}
