mod server;

use std::ffi::{CStr, c_char};

use axum::http::Method;
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

/// Adds an HTTP route that responds to the given method and path with a predefined response.
///
/// # Safety
///
/// This function is `unsafe` because it dereferences raw C pointers (`*const c_char`) received via FFI.
///
/// The caller must ensure:
/// - The `method`, `path`, and `response` pointers are valid and non-null.
/// - Each pointer refers to a null-terminated C string.
/// - The memory they point to is readable and remains valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_route(
    method: *const c_char,
    path: *const c_char,
    response: *const c_char,
) {
    let method = unsafe { CStr::from_ptr(method).to_str().unwrap_or("GET") };
    let path = unsafe { CStr::from_ptr(path).to_str().unwrap_or_default() };
    let resp = unsafe { CStr::from_ptr(response).to_str().unwrap_or_default() };

    let method = match method.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        _ => Method::GET,
    };

    TOKIO_RUNTIME.block_on(async {
        server::add_route(method, path, resp);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn lua_start_server() {
    TOKIO_RUNTIME.block_on(async {
        server::start_server().await;
    });
}
