---@meta ludi_core
-- Type definitions for the Rust core (src/lib.rs). Never loaded at runtime;
-- this file only feeds the Lua language server.

local core = {}

--- True when running inside a binary produced by `ludi build`; absent when
--- loaded as a regular native module.
---@type boolean?
core.bundled = nil

--- WebSocket callbacks, all invoked on the Lua thread like `on_request`.
--- `upgrade` decides a handshake: return `{ accept = true }` to upgrade, or
--- a raw response table to reject. `open` delivers the native handle once
--- the upgrade completes; `event` delivers "message" (data, binary),
--- "close" (code?, reason) and "error" (message) for open connections.
---@class ludi.WsCallbacks
---@field upgrade fun(id: integer, raw: ludi.RawRequest): table
---@field open fun(id: integer, handle: ludi.WsNativeHandle)
---@field event fun(id: integer, kind: string, a?: any, b?: any)

--- Starts the HTTP server and blocks. Calls `on_request` once per request
--- with a raw request table; expects a raw response table back.
--- `on_listen` runs once after the port is bound, before any request is
--- served. Raises an error when the port cannot be bound.
--- When `on_reload` is given, `*.lua` files under the working directory are
--- watched and the callback runs on the Lua thread, between requests, with
--- the list of changed paths.
--- Without `ws`, every WebSocket handshake is rejected with a 404.
---@param port integer
---@param on_request fun(raw: ludi.RawRequest): ludi.RawResponse
---@param on_listen? fun()
---@param on_reload? fun(changed: string[])
---@param ws? ludi.WsCallbacks
function core.start_server(port, on_request, on_listen, on_reload, ws) end

return core
