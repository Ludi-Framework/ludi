local ffi = require("ffi")
local request = require("request")
local response = require("response")

ffi.cdef [[
  typedef const char* (*HandlerFn)(const char* body, const char* headers, const char* queries);
  void add_route_handler(const char* method, const char* path, HandlerFn handler);
  void start_server(const int port);
]]

local os_name = jit.os:lower()

local lib_map = {
    windows = "../rust/target/release/rust.dll",
    linux = "../rust/target/release/librust.so",
    osx = "../rust/target/release/librust.dylib"
}

local lib_path = lib_map[os_name]
local lib = ffi.load(lib_path)

local Ludi = {}
Ludi.__index = Ludi
Ludi.middlewares = {}

function Ludi.new() return setmetatable({routes = {}, middlewares = {}}, Ludi) end

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
        middlewares = route_middlewares,
        handler = handler
    })
end

Ludi.get = makeMethod("GET")
Ludi.post = makeMethod("POST")
Ludi.put = makeMethod("PUT")
Ludi.delete = makeMethod("DELETE")
Ludi.patch = makeMethod("PATCH")
Ludi.options = makeMethod("OPTIONS")
Ludi.head = makeMethod("HEAD")

local function run_chain(req, res, global_middlewares, route_middlewares,
                         handler)
    local all_middlewares = {}

    for _, m in ipairs(global_middlewares) do
        table.insert(all_middlewares, m)
    end
    for _, m in ipairs(route_middlewares) do table.insert(all_middlewares, m) end

    local function next_middleware(i)
        if i > #all_middlewares then
            handler()
            return
        end
        all_middlewares[i](req, res, function() next_middleware(i + 1) end)
    end

    next_middleware(1)
end

function Ludi:listen(port)
    self._handlers = {}

    for _, route in ipairs(self.routes) do
        local lua_handler = ffi.cast("HandlerFn",
                                     function(body_cstr, headers_cstr,
                                              queries_cstr)
            local req = request.new(ffi.string(body_cstr),
                                    ffi.string(headers_cstr),
                                    ffi.string(queries_cstr))
            local res = response.new()

            run_chain(req, res, self.middlewares, route.middlewares,
                      function() route.handler(req, res) end)

            return res:build()
        end)

        table.insert(self._handlers, lua_handler)

        lib.add_route_handler(route.method, route.path, lua_handler)
    end

    lib.start_server(port or 3000)
end

function Ludi:use(middleware)
    assert(type(middleware) == "function", "Middleware must be a function")
    table.insert(self.middlewares, middleware)
end

return Ludi
