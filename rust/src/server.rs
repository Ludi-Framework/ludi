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

static GET_ROUTES: Lazy<Arc<RwLock<HashMap<String, String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

static POST_ROUTES: Lazy<Arc<RwLock<HashMap<String, String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub fn add_get_route(path: &str, response: &str) {
    let mut routes = GET_ROUTES.write().unwrap();
    routes.insert(path.to_string(), response.to_string());
}

pub fn add_post_route(path: &str, response: &str) {
    let mut routes = POST_ROUTES.write().unwrap();
    routes.insert(path.to_string(), response.to_string());
}

pub async fn start_server() {
    let app = Router::new().fallback(fallback_handler);

    let addr = format!("0.0.0.0:{}", 3000);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn fallback_handler(req: Request) -> (StatusCode, String) {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    match method {
        Method::GET => {
            let routes = GET_ROUTES.read().unwrap();
            match routes.get(&path) {
                Some(resp) => (StatusCode::OK, resp.clone()),
                None => (StatusCode::NOT_FOUND, "GET not found".to_string()),
            }
        }
        Method::POST => {
            let routes = POST_ROUTES.read().unwrap();
            match routes.get(&path) {
                Some(resp) => (StatusCode::OK, resp.clone()),
                None => (StatusCode::NOT_FOUND, "POST not found".to_string()),
            }
        }
        _ => (
            StatusCode::METHOD_NOT_ALLOWED,
            "Method not allowed".to_string(),
        ),
    }
}
