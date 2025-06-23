local ffi = require("ffi")

ffi.cdef([[
  void add_route(const char* method, const char* path, const char* response);
  void start_server(const int port);
]])

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

function Ludi:addRoute(method, path, options, response)
    if type(response) == "function" then
        local result = response()
        assert(type(result) == "string",
               "Response function must return a string")
        response = result
    end

    table.insert(self.routes, {
        method = method,
        path = path,
        options = options,
        response = response
    })
end

Ludi.get = makeMethod("GET")
Ludi.post = makeMethod("POST")
Ludi.put = makeMethod("PUT")
Ludi.delete = makeMethod("DELETE")
-- Ludi.patch = makeMethod("PATCH")

function Ludi:listen(port)
    for _, route in ipairs(self.routes) do
        lib.add_route(route.method, route.path, route.response)
    end

    lib.start_server(port or 3000)
end

return Ludi
