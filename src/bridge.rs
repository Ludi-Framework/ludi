//! Lua ↔ Rust boundary: starts the server and converts requests and
//! responses between plain Lua tables and the native types.

use mlua::prelude::*;
use tokio::net::TcpListener;

use crate::server;
use crate::types::{Msg, Request, Response, WsDecision, WsEvent};
use crate::watch;

/// Blocks the calling (Lua) thread serving HTTP until the process exits.
///
/// Hyper accepts connections on tokio worker threads and pushes each request
/// through a channel; this thread pops them and invokes the Lua `dispatch`
/// function. All Lua execution therefore stays on the thread that owns the
/// Lua state — workers never touch the interpreter.
///
/// `on_listen` runs on the Lua thread right after the port is bound, before
/// any request is served. A failed bind raises a Lua error instead.
///
/// `on_reload` (dev mode) enables the file watcher: `*.lua` changes under
/// the working directory arrive as `Msg::Reload` on the same channel, so
/// the callback also runs on the Lua thread, never during a request.
///
/// `ws` holds the WebSocket callbacks — `upgrade(id, raw) -> decision`,
/// `open(id, handle)` and `event(id, kind, ...)` — all invoked on the Lua
/// thread like `dispatch`. Without it every handshake is answered 404.
pub fn start_server(
    lua: &Lua,
    (port, dispatch, on_listen, on_reload, ws): (
        u16,
        LuaFunction,
        Option<LuaFunction>,
        Option<LuaFunction>,
        Option<LuaTable>,
    ),
) -> LuaResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .into_lua_err()?;

    let listener = runtime
        .block_on(TcpListener::bind(("0.0.0.0", port)))
        .map_err(|err| LuaError::runtime(format!("ludi: failed to bind port {port}: {err}")))?;

    let (msgs_tx, mut msgs_rx) = tokio::sync::mpsc::unbounded_channel::<Msg>();

    if on_reload.is_some() {
        let root = std::env::current_dir().into_lua_err()?;
        watch::spawn(root, msgs_tx.clone());
    }

    runtime.spawn(server::serve(listener, msgs_tx));

    if let Some(callback) = on_listen {
        callback.call::<()>(())?;
    }

    while let Some(msg) = msgs_rx.blocking_recv() {
        match msg {
            Msg::Job(job) => {
                let response = match dispatch.call::<LuaTable>(request_to_lua(lua, &job.request)?) {
                    Ok(table) => response_from_lua(&table)?,
                    Err(err) => {
                        eprintln!("ludi: unhandled error in Lua dispatch: {err}");
                        internal_error()
                    }
                };

                let _ = job.respond.send(response);
            }
            Msg::WsUpgrade(upgrade) => {
                let decision = match &ws {
                    None => WsDecision::Reject(not_found()),
                    Some(callbacks) => {
                        ws_decision(lua, callbacks, &upgrade).unwrap_or_else(|err| {
                            eprintln!("ludi: unhandled error in Lua websocket upgrade: {err}");
                            WsDecision::Reject(internal_error())
                        })
                    }
                };
                let _ = upgrade.respond.send(decision);
            }
            Msg::WsOpen { id, handle } => {
                if let Some(callbacks) = &ws {
                    let result: LuaResult<()> = callbacks
                        .get::<LuaFunction>("open")
                        .and_then(|open| open.call((id, handle)));
                    if let Err(err) = result {
                        eprintln!("ludi: unhandled error in Lua websocket open: {err}");
                    }
                }
            }
            Msg::WsEvent { id, event } => {
                if let Some(callbacks) = &ws {
                    if let Err(err) = ws_event(lua, callbacks, id, event) {
                        eprintln!("ludi: unhandled error in Lua websocket event: {err}");
                    }
                }
            }
            Msg::Reload(changed) => {
                if let Some(callback) = &on_reload {
                    if let Err(err) = callback.call::<()>(changed) {
                        eprintln!("ludi: unhandled error in Lua reload: {err}");
                    }
                }
            }
        }
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

    Ok(Response {
        status,
        headers,
        body,
    })
}

/// Asks Lua to decide a handshake: `{ accept = true }` upgrades, any
/// other table is sent back as the HTTP response.
fn ws_decision(
    lua: &Lua,
    callbacks: &LuaTable,
    upgrade: &crate::types::WsUpgrade,
) -> LuaResult<WsDecision> {
    let decide = callbacks.get::<LuaFunction>("upgrade")?;
    let decision = decide.call::<LuaTable>((upgrade.id, request_to_lua(lua, &upgrade.request)?))?;

    if decision.get::<Option<bool>>("accept")?.unwrap_or(false) {
        Ok(WsDecision::Accept)
    } else {
        Ok(WsDecision::Reject(response_from_lua(&decision)?))
    }
}

fn ws_event(lua: &Lua, callbacks: &LuaTable, id: u64, event: WsEvent) -> LuaResult<()> {
    let notify = callbacks.get::<LuaFunction>("event")?;
    match event {
        WsEvent::Message { data, binary } => {
            notify.call((id, "message", lua.create_string(&data)?, binary))
        }
        WsEvent::Close { code, reason } => notify.call((id, "close", code, reason)),
        WsEvent::Error { message } => notify.call((id, "error", message)),
    }
}

fn internal_error() -> Response {
    Response {
        status: 500,
        headers: vec![("Content-Type".into(), "text/plain".into())],
        body: b"Internal Server Error".to_vec(),
    }
}

fn not_found() -> Response {
    Response {
        status: 404,
        headers: vec![("Content-Type".into(), "application/json".into())],
        body: br#"{"error":"Not Found"}"#.to_vec(),
    }
}
