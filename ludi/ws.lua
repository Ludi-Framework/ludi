--- WebSocket connections: the Lua side of the native transport.
--- ludi_core completes the handshake and pumps the frames; this module
--- keeps the registry of connections and dispatches events to the
--- listeners registered with `conn:on()`.
---
--- The registry is module state, not application state, so it survives a
--- hot reload: connections opened before the reload keep working — with
--- the handlers they were opened with — until the peer disconnects.

local errfmt = require("ludi.errfmt")

---@alias ludi.WsHandler fun(ws: ludi.WsConn, req: ludi.Request)

--- Native handle exposed by ludi_core for one open connection.
---@class ludi.WsNativeHandle
---@field send fun(self: ludi.WsNativeHandle, data: string, binary?: boolean): boolean
---@field close fun(self: ludi.WsNativeHandle, code?: integer, reason?: string): boolean

-- Dev mode mirrors init.lua: framed errors on stderr under LUDI_WATCH.
local watch_env = os.getenv("LUDI_WATCH")
local DEV = watch_env ~= nil and watch_env ~= "" and watch_env ~= "0"

--- One open WebSocket connection, handed to the route handler.
---@class ludi.WsConn
---@field handle ludi.WsNativeHandle
---@field listeners table<string, function[]>
local Conn = {}
Conn.__index = Conn

local EVENTS = {message = true, close = true, ["error"] = true}

function Conn.new(handle)
    return setmetatable({
        handle = handle,
        listeners = {message = {}, close = {}, ["error"] = {}}
    }, Conn)
end

--- Registers a listener. Events:
---   "message" (data: string, binary: boolean) — a frame arrived
---   "close"   (code: integer?, reason: string) — connection is gone; last event
---   "error"   (message: string) — protocol error; a close follows
---@param event "message"|"close"|"error"
---@param fn function
function Conn:on(event, fn)
    assert(EVENTS[event],
           'Unknown WebSocket event: expected "message", "close" or "error"')
    assert(type(fn) == "function", "Listener must be a function")
    table.insert(self.listeners[event], fn)
end

--- Sends a text frame — or a binary one when `binary` is true. Queues on
--- the native side, never blocks. Returns false once the connection is
--- closed.
---@param data string
---@param binary? boolean
---@return boolean
function Conn:send(data, binary)
    return self.handle:send(data, binary == true)
end

--- Starts closing the connection. `code` defaults to 1000 (normal
--- closure); the "close" event still fires when the shutdown completes.
---@param code? integer
---@param reason? string
---@return boolean
function Conn:close(code, reason)
    return self.handle:close(code, reason)
end

local Ws = {}

local pending = {} -- id -> {handler, req}: accepted, upgrade in flight
local open = {}    -- id -> Conn

local function report(err, context)
    if DEV then
        io.stderr:write(errfmt.render(err, "handler", context))
    else
        io.stderr:write(("ludi: %s: %s\n"):format(context, tostring(err)))
    end
end

-- Every callback runs inside a coroutine (ADR 0003), like _dispatch.
local function protected(fn, context)
    local co = coroutine.create(fn)
    local ok, err = coroutine.resume(co)
    if ok and coroutine.status(co) == "suspended" then
        ok = false
        err = "coroutine yielded outside an async context"
    end
    if coroutine.close then coroutine.close(co) end
    if not ok then report(err, context) end
    return ok
end

--- Remembers an accepted handshake until ludi_core reports the upgrade
--- complete. Called by Ludi:_ws_upgrade.
function Ws.register(id, handler, req)
    pending[id] = {handler = handler, req = req}
end

--- Called by ludi_core when the upgrade completes: promotes the pending
--- entry to an open connection and runs the route handler. A handler
--- error closes the connection with 1011 (internal error).
function Ws.open(id, handle)
    local accepted = pending[id]
    pending[id] = nil

    if not accepted then
        handle:close(1011, "unknown connection")
        return
    end

    local conn = Conn.new(handle)
    open[id] = conn

    local ok = protected(function() accepted.handler(conn, accepted.req) end,
                         ("WS %s handler failed"):format(accepted.req.path))
    if not ok then conn:close(1011, "") end
end

--- Called by ludi_core for every event on a connection. "close" is the
--- last one: it clears the registry entry (and any pending entry, for an
--- accepted upgrade that never completed).
function Ws.event(id, kind, a, b)
    if kind == "close" then
        pending[id] = nil
        local conn = open[id]
        open[id] = nil
        if conn then
            for _, fn in ipairs(conn.listeners.close) do
                protected(function() fn(a, b) end, "WS close listener failed")
            end
        end
        return
    end

    local conn = open[id]
    if not conn then return end

    if kind == "message" then
        for _, fn in ipairs(conn.listeners.message) do
            protected(function() fn(a, b) end, "WS message listener failed")
        end
    elseif kind == "error" then
        for _, fn in ipairs(conn.listeners["error"]) do
            protected(function() fn(a) end, "WS error listener failed")
        end
    end
end

return Ws
