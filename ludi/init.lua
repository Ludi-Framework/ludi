local core = require("ludi_core")
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
---@param port? integer defaults to 3000
function Ludi:listen(port)
    core.start_server(port or 3000, function(raw) return self:_dispatch(raw) end)
end

return Ludi
