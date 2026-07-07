local core = require("ludi_core")
local reload = require("ludi.reload")
local Router = require("ludi.router")
local Request = require("ludi.request")
local Response = require("ludi.response")
local run_chain = require("ludi.middleware")

---@alias ludi.Handler fun(req: ludi.Request, res: ludi.Response)
---@alias ludi.Middleware fun(req: ludi.Request, res: ludi.Response, next: fun())

--- Route registration method: (path, handler) or (path, middleware(s), handler).
---@alias ludi.RouteMethod fun(self: Ludi, path: string, handler: ludi.Handler)|fun(self: Ludi, path: string, middlewares: ludi.Middleware|ludi.Middleware[], handler: ludi.Handler)

---@class ludi.Route
---@field method string
---@field path string
---@field segments ludi.RouteSegment[]
---@field middlewares ludi.Middleware[]
---@field handler ludi.Handler

--- Raw request table handed over by ludi_core.
---@class ludi.RawRequest
---@field method string
---@field path string
---@field query? string
---@field headers? table<string, string>
---@field body? string

--- Raw response table handed back to ludi_core.
---@class ludi.RawResponse
---@field status integer
---@field headers table<string, string>
---@field body string

---@class Ludi
---@field routes ludi.Route[]
---@field middlewares ludi.Middleware[]
---@field get ludi.RouteMethod
---@field post ludi.RouteMethod
---@field put ludi.RouteMethod
---@field delete ludi.RouteMethod
---@field patch ludi.RouteMethod
---@field options ludi.RouteMethod
---@field head ludi.RouteMethod
local Ludi = {}
Ludi.__index = Ludi

---@return Ludi
function Ludi.new()
    return setmetatable({routes = {}, middlewares = {}}, Ludi)
end

--- Registers a global middleware, run before every route.
---@param middleware ludi.Middleware
function Ludi:use(middleware)
    assert(type(middleware) == "function", "Middleware must be a function")
    table.insert(self.middlewares, middleware)
end

--- Registers a route. Prefer the verb helpers (get, post, ...).
---@param method string uppercase HTTP verb, e.g. "GET"
---@param path string path pattern, e.g. "/users/:id"
---@param options? ludi.Middleware|ludi.Middleware[] route-level middleware(s)
---@param handler ludi.Handler
function Ludi:addRoute(method, path, options, handler)
    assert(type(handler) == "function", "Handler must be a function")
    local route_middlewares = {}

    if options then
        if type(options) == "function" then
            route_middlewares = {options}
        elseif type(options) == "table" then
            route_middlewares = options
        else
            error("Route middlewares must be function or table of functions")
        end
    end

    table.insert(self.routes, {
        method = method,
        path = path,
        segments = Router.compile(path),
        middlewares = route_middlewares,
        handler = handler
    })
end

local function makeMethod(method)
    return function(self, path, ...)
        local args = {...}
        if #args == 1 then
            self:addRoute(method, path, nil, args[1])
        elseif #args == 2 then
            self:addRoute(method, path, args[1], args[2])
        else
            error(
                ("Expected: %s(path, handler) or %s(path, middleware(s), handler)"):format(
                    method:lower(), method:lower()))
        end
    end
end

Ludi.get = makeMethod("GET")
Ludi.post = makeMethod("POST")
Ludi.put = makeMethod("PUT")
Ludi.delete = makeMethod("DELETE")
Ludi.patch = makeMethod("PATCH")
Ludi.options = makeMethod("OPTIONS")
Ludi.head = makeMethod("HEAD")

--- Entry point called by ludi_core for every request. Receives a plain
--- table (method, path, query, headers, body) and must return a plain
--- table (status, headers, body).
---@param raw ludi.RawRequest
---@return ludi.RawResponse
function Ludi:_dispatch(raw)
    local route, params = Router.match(self.routes, raw.method, raw.path)

    if not route then
        return {
            status = 404,
            headers = {["Content-Type"] = "application/json"},
            body = '{"error":"Not Found"}'
        }
    end

    local req = Request.new(raw, params)
    local res = Response.new()

    -- Every request runs inside a coroutine (ADR 0003). Nothing yields
    -- yet; this is what lets the future async stdlib suspend handlers
    -- without breaking existing applications.
    local co = coroutine.create(function()
        run_chain(req, res, self.middlewares, route.middlewares,
                  function() route.handler(req, res) end)
    end)

    local ok, err = coroutine.resume(co)

    if ok and coroutine.status(co) == "suspended" then
        -- yielded with no async runtime to resume it (v1 has none)
        ok = false
        err = "coroutine yielded outside an async context"
    end

    if coroutine.close then coroutine.close(co) end

    if not ok then
        io.stderr:write(("ludi: handler error on %s %s: %s\n"):format(
                            raw.method, raw.path, tostring(err)))
        return {
            status = 500,
            headers = {["Content-Type"] = "application/json"},
            body = '{"error":"Internal Server Error"}'
        }
    end

    return res:build()
end

--- Starts the HTTP server and blocks serving requests.
--- The callback runs once, right after the port is bound and before any
--- request is served — the place for the application's own startup log.
--- Raises an error when the port cannot be bound.
---
--- With `LUDI_WATCH=1` in the environment, `*.lua` files under the working
--- directory are watched and the application is hot-reloaded on change: the
--- entrypoint is re-executed and its `app:listen()` call swaps the app in
--- place, without rebinding the port (see ludi/reload.lua).
---@param port? integer defaults to 3000
---@param callback? fun()
function Ludi:listen(port, callback)
    -- During a hot reload the re-executed entrypoint lands here: hand the
    -- new app to the interceptor instead of starting a second server.
    if Ludi._capture_listen then
        Ludi._capture_listen(self)
        return
    end

    local current = self
    local on_reload

    local watch = os.getenv("LUDI_WATCH")
    if watch and watch ~= "" and watch ~= "0" then
        local entrypoint = arg and arg[0]
        if entrypoint then
            on_reload = reload.handler(
                entrypoint,
                function(interceptor) Ludi._capture_listen = interceptor end,
                function(app) current = app end)
        else
            io.stderr:write("ludi: LUDI_WATCH is set but the entrypoint is " ..
                                "unknown (no arg[0]); hot reload disabled\n")
        end
    end

    core.start_server(port or 3000,
                      function(raw) return current:_dispatch(raw) end,
                      callback, on_reload)
end

return Ludi
