local core = require("ludi_core")
local Router = require("ludi.router")
local Request = require("ludi.request")
local Response = require("ludi.response")
local run_chain = require("ludi.middleware")

local Ludi = {}
Ludi.__index = Ludi

function Ludi.new()
    return setmetatable({routes = {}, middlewares = {}}, Ludi)
end

function Ludi:use(middleware)
    assert(type(middleware) == "function", "Middleware must be a function")
    table.insert(self.middlewares, middleware)
end

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

function Ludi:listen(port)
    core.start_server(port or 3000, function(raw) return self:_dispatch(raw) end)
end

return Ludi
