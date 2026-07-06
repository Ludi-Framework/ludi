---@meta ludi_core
-- Type definitions for the Rust core (src/lib.rs). Never loaded at runtime;
-- this file only feeds the Lua language server.

local core = {}

--- Starts the HTTP server and blocks. Calls `on_request` once per request
--- with a raw request table; expects a raw response table back.
---@param port integer
---@param on_request fun(raw: ludi.RawRequest): ludi.RawResponse
function core.start_server(port, on_request) end

return core
