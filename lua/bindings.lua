local ffi = require("ffi")
local request = require("request")
local response = require("response")

ffi.cdef [[
  typedef const char* (*HandlerFn)(const char* body, const char* headers);
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

function Ludi.new() return setmetatable({routes = {}}, Ludi) end

local function makeMethod(method)
    return function(self, path, ...)
        local args = {...}
        if #args == 1 then
            self:addRoute(method, path, nil, args[1])
        elseif #args == 2 then
            self:addRoute(method, path, args[1], args[2])
        else
            error(
                ("Expected: %s(path, response) or %s(path, options, response)"):format(
                    method:lower(), method:lower()))
        end
    end
end

function Ludi:addRoute(method, path, options, handler)
    assert(type(handler) == "function", "Handler must be a function")
    table.insert(self.routes, {
        method = method,
        path = path,
        options = options,
        handler = handler
    })
end

Ludi.get = makeMethod("GET")
Ludi.post = makeMethod("POST")
Ludi.put = makeMethod("PUT")
Ludi.delete = makeMethod("DELETE")
-- Ludi.patch = makeMethod("PATCH")

function Ludi:listen(port)
    self._handlers = {}

    for _, route in ipairs(self.routes) do
        local lua_handler = ffi.cast("HandlerFn",
                                     function(body_cstr, headers_cstr)
            local req = request.new(ffi.string(body_cstr),
                                    ffi.string(headers_cstr))

            local res = response.new()
            route.handler(req, res)

            return res:build()
        end)

        table.insert(self._handlers, lua_handler)

        lib.add_route_handler(route.method, route.path, lua_handler)
    end

    lib.start_server(port or 3000)
end

return Ludi
