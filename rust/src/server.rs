use axum::body::to_bytes;
use axum::{
    Router,
    body::{Body, Bytes},
    http::{HeaderMap, Method, StatusCode},
    response::Response,
    routing::any,
};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_char},
    sync::RwLock,
};

pub type HandlerFn = extern "C" fn(*const c_char, *const c_char, *const c_char) -> *const c_char;

static ROUTES: Lazy<RwLock<HashMap<(Method, String), HandlerFn>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Adds an HTTP route handler for a specific method and path.
///
/// # Parameters
///
/// - `method`: a pointer to a C string (`*const c_char`) representing the HTTP method (e.g., "GET", "POST").
/// - `path`: a pointer to a C string representing the route path (e.g., "/hello").
/// - `handler`: an external function (`extern "C"`) that will be called when the route is accessed.
///
/// # Details
///
/// This function converts the C string pointers into Rust strings, converts the HTTP method
/// to uppercase, and maps it to the `Method` enum from Axum. Unknown methods default to `GET`.
/// The (method, path) pair is inserted into a global `HashMap` protected by a `RwLock`.
///
/// # Safety
///
/// - This function uses `unsafe` code to convert raw C pointers into Rust strings.
/// - The `method` and `path` pointers must be valid and point to null-terminated C strings.
///
/// # Examples
///
/// ```ignore
/// let method = std::ffi::CString::new("GET").unwrap();
/// let path = std::ffi::CString::new("/hello").unwrap();
/// add_route_handler(method.as_ptr(), path.as_ptr(), my_handler_fn);
/// ```
pub fn add_route_handler(method: *const c_char, path: *const c_char, handler: HandlerFn) {
    let method_str = unsafe { CStr::from_ptr(method) }
        .to_string_lossy()
        .to_uppercase();
    let path_str = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .to_string();

    let method = match method_str.as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        _ => Method::GET,
    };

    ROUTES.write().unwrap().insert((method, path_str), handler);
}

/// Serializes the headers into a string format suitable for HTTP responses.
///   /// # Parameters
/// /// - `headers`: a reference to the `HeaderMap` containing the HTTP headers.
///  /// # Returns
/// /// - A `String` containing the serialized headers in the format "Header-Name:
///  Header-Value\r\n".
async fn serialize_headers(headers: &HeaderMap) -> String {
    let mut s = String::new();
    for (name, value) in headers.iter() {
        s.push_str(&format!("{}: {}\r\n", name, value.to_str().unwrap_or("")));
    }
    s
}

/// Handles incoming HTTP requests by dispatching them to the appropriate handler.
/// # Parameters
/// - `method`: The HTTP method of the request (e.g., GET, POST).
///  - `uri`: The URI of the request.
/// - `headers`: The headers of the request as a `HeaderMap`.
///  - `body`: The body of the request as an `axum::body::Body`.
///  # Returns
///  - A `Response<Body>` containing the response to the request.
/// /// # Details
///  This function reads the request body and headers, looks up the appropriate handler
///  based on the method and path, and calls the handler with the body and headers.
///  If a handler is found, it returns the response from the handler; otherwise, it returns a 404 Not Found response.
async fn handler_dispatch(
    method: Method,
    uri: axum::http::Uri,
    headers: HeaderMap,
    body: Body,
) -> Response<Body> {
    let path = uri.path().to_string();

    let query_str = uri.query().unwrap_or("").to_string();
    let body_bytes: Bytes = to_bytes(body, 65536).await.unwrap_or_default();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    let headers_str = serialize_headers(&headers).await;

    let routes = ROUTES.read().unwrap();

    if let Some(handler) = routes.get(&(method.clone(), path.clone())) {
        let c_body = CString::new(body_str).unwrap_or_default();
        let c_query = CString::new(query_str).unwrap_or_default();
        let c_headers = CString::new(headers_str).unwrap_or_default();
        let res_ptr = handler(c_body.as_ptr(), c_headers.as_ptr(), c_query.as_ptr());

        let res_str = unsafe {
            if res_ptr.is_null() {
                CString::new("").unwrap()
            } else {
                CStr::from_ptr(res_ptr).to_owned()
            }
        };
        let res_bytes = res_str.into_bytes();

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(res_bytes))
            .unwrap()
    } else {
        println!("No handler found for {} {}", method, path);
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Rota não encontrada"))
            .unwrap()
    }
}

///// Starts the Axum server on the specified port.
/// # Parameters
///  - `port`: The port number on which the server will listen for incoming requests.
/// /// # Details
///  This function creates an Axum router with a catch-all route that dispatches requests to the `handler_dispatch` function.
///  It binds the server to the specified port and starts listening for incoming requests.
pub async fn start_server(port: u16) {
    let app = Router::new().route("/{*path}", any(handler_dispatch));

    println!("Server running in http://localhost:{}", port);
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{HeaderMap, Method, Uri};
    use std::ffi::CString;

    extern "C" fn test_handler(
      body: *const c_char,
      headers: *const c_char,
      query: *const c_char,
    )  -> *const c_char {
    unsafe {
        let b = if body.is_null() {
            ""
        } else {
            CStr::from_ptr(body).to_str().unwrap_or("")
        };
        let h = if headers.is_null() {
            ""
        } else {
            CStr::from_ptr(headers).to_str().unwrap_or("")
        };
        let q = if query.is_null() {
            ""
        } else {
            CStr::from_ptr(query).to_str().unwrap_or("")
        };

        let response = CString::new(format!(
            r#"{{"echo_body": "{}","echo_headers": "{}","echo_query": "{}"}}"#,
            b, h, q
        ))
        .unwrap();

        response.into_raw()
    }
}


    #[test]
    fn test_add_route_handler() {
        let method = CString::new("GET").unwrap();
        let path = CString::new("/test").unwrap();
        add_route_handler(method.as_ptr(), path.as_ptr(), test_handler);

        let routes = ROUTES.read().unwrap();
        assert!(routes.contains_key(&(Method::GET, "/test".to_string())));
    }

    #[tokio::test]
    async fn test_handler_dispatch_found() {
        let method = CString::new("GET").unwrap();
        let path = CString::new("/hello").unwrap();
        add_route_handler(method.as_ptr(), path.as_ptr(), test_handler);

        let uri: Uri = "/hello".parse().unwrap();
        let headers = HeaderMap::new();
        let body = Body::from("");

        let response = handler_dispatch(Method::GET, uri, headers, body).await;
        assert_eq!(response.status(), 200);

        let bytes = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap();
        let body_str = std::str::from_utf8(&bytes).unwrap();

        assert!(body_str.contains(r#""echo_body":"""#));
        assert!(body_str.contains(r#""echo_headers":"""#));
    }

    #[tokio::test]
    async fn test_handler_dispatch_not_found() {
        let uri: Uri = "/notfound".parse().unwrap();
        let headers = HeaderMap::new();
        let body = Body::from("");

        let response = handler_dispatch(Method::GET, uri, headers, body).await;
        assert_eq!(response.status(), 404);

        let bytes = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap();
        let body_str = std::str::from_utf8(&bytes).unwrap();

        assert_eq!(body_str, "Route not found");
    }
}
