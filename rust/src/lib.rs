mod server;

use std::ffi::c_char;

use libc::c_int;
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

pub type HandlerFn = extern "C" fn(*const c_char, *const c_char) -> *const c_char;
static TOKIO_RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

/// Adds an HTTP route that responds to the given method and path with a predefined response.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw C pointers (*const c_char) received via FFI.
///
/// The caller must ensure:
/// - The method, path, and response pointers are valid and non-null.
/// - Each pointer refers to a null-terminated C string.
/// - The memory they point to is readable and remains valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_route_handler(
    method: *const c_char,
    path: *const c_char,
    handler: HandlerFn,
) {
    if method.is_null() || path.is_null() {
        eprintln!("method or path is null");
        return;
    }

    TOKIO_RUNTIME.block_on(async {
        server::add_route_handler(method, path, handler);
    });
}

/// Starts the Axum server on the given port.
///
/// # Safety
///
/// This function is marked as unsafe because it is intended to be called
/// from foreign code (FFI), such as C or Lua. The caller must ensure:
///
/// - The port value must be a valid TCP port number: between 0 and 65535 (inclusive).
///   Any value outside this range will be rejected internally, but it is still the
///   caller's responsibility to pass valid input.
/// - This function should be called only once unless the runtime supports multiple
///   invocations. Calling it multiple times or concurrently without proper handling
///   may lead to panics or undefined behavior.
/// - The calling environment must be properly initialized and must ensure this call
///   does not conflict with other async runtimes or I/O systems.
///
/// Failure to meet these requirements may result in crashes, panics, or undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn start_server(port: c_int) {
    if port < 0 || port > u16::MAX as c_int {
        eprintln!("Invalid port number: {}", port);
        return;
    }
    let port = port as u16;

    TOKIO_RUNTIME.block_on(async move {
        server::start_server(port).await;
    });
}
