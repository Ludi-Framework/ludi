---@meta ludi_core
-- Type definitions for the Rust core (src/lib.rs). Never loaded at runtime;
-- this file only feeds the Lua language server.

local core = {}

--- True when running inside a binary produced by `ludi build`; absent when
--- loaded as a regular native module.
---@type boolean?
core.bundled = nil

--- Starts the HTTP server and blocks. Calls `on_request` once per request
--- with a raw request table; expects a raw response table back.
--- `on_listen` runs once after the port is bound, before any request is
--- served. Raises an error when the port cannot be bound.
--- When `on_reload` is given, `*.lua` files under the working directory are
--- watched and the callback runs on the Lua thread, between requests, with
--- the list of changed paths.
---@param port integer
---@param on_request fun(raw: ludi.RawRequest): ludi.RawResponse
---@param on_listen? fun()
---@param on_reload? fun(changed: string[])
function core.start_server(port, on_request, on_listen, on_reload) end

return core
