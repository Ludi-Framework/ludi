//! Lua ↔ Rust boundary: starts the server and converts requests and
//! responses between plain Lua tables and the native types.

use mlua::prelude::*;
use tokio::net::TcpListener;

use crate::server;
use crate::types::{Job, Request, Response};

/// Blocks the calling (Lua) thread serving HTTP until the process exits.
///
/// Hyper accepts connections on tokio worker threads and pushes each request
/// through a channel; this thread pops them and invokes the Lua `dispatch`
/// function. All Lua execution therefore stays on the thread that owns the
/// Lua state — workers never touch the interpreter.
///
/// `on_listen` runs on the Lua thread right after the port is bound, before
/// any request is served. A failed bind raises a Lua error instead.
pub fn start_server(
    lua: &Lua,
    (port, dispatch, on_listen): (u16, LuaFunction, Option<LuaFunction>),
) -> LuaResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .into_lua_err()?;

    let listener = runtime
        .block_on(TcpListener::bind(("0.0.0.0", port)))
        .map_err(|err| LuaError::runtime(format!("ludi: failed to bind port {port}: {err}")))?;

    let (jobs_tx, mut jobs_rx) = tokio::sync::mpsc::unbounded_channel::<Job>();
    runtime.spawn(server::serve(listener, jobs_tx));

    if let Some(callback) = on_listen {
        callback.call::<()>(())?;
    }

    while let Some(job) = jobs_rx.blocking_recv() {
        let response = match dispatch.call::<LuaTable>(request_to_lua(lua, &job.request)?) {
            Ok(table) => response_from_lua(&table)?,
            Err(err) => {
                eprintln!("ludi: unhandled error in Lua dispatch: {err}");
                internal_error()
            }
        };

        let _ = job.respond.send(response);
    }

    Ok(())
}

fn request_to_lua(lua: &Lua, request: &Request) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    table.set("method", request.method.as_str())?;
    table.set("path", request.path.as_str())?;
    table.set("query", request.query.as_str())?;
    table.set("body", lua.create_string(&request.body)?)?;

    let headers = lua.create_table()?;
    for (name, value) in &request.headers {
        headers.set(name.to_lowercase(), value.as_str())?;
    }
    table.set("headers", headers)?;

    Ok(table)
}

fn response_from_lua(table: &LuaTable) -> LuaResult<Response> {
    let status = table.get::<Option<u16>>("status")?.unwrap_or(200);

    let body = table
        .get::<Option<LuaString>>("body")?
        .map(|s| s.as_bytes().to_vec())
        .unwrap_or_default();

    let mut headers = Vec::new();
    if let Some(header_table) = table.get::<Option<LuaTable>>("headers")? {
        for pair in header_table.pairs::<String, String>() {
            let (name, value) = pair?;
            headers.push((name, value));
        }
    }

    Ok(Response { status, headers, body })
}

fn internal_error() -> Response {
    Response {
        status: 500,
        headers: vec![("Content-Type".into(), "text/plain".into())],
        body: b"Internal Server Error".to_vec(),
    }
}
