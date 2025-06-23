local ffi = require("ffi")

ffi.cdef([[
  void add_route(const char* method, const char* path, const char* response);
  void start_server(const int port);
]])

local os_name = jit.os:lower()

local lib_map = {
    windows = "../rust/target/release/rust.dll",
    linux   = "../rust/target/release/librust.so",
    osx     = "../rust/target/release/librust.dylib",
}

local lib_path = lib_map[os_name]

local lib = ffi.load(lib_path)

local Ludi = {}
Ludi.__index = Ludi

function Ludi.new()
	return setmetatable({ routes = {} }, Ludi)
end

function Ludi:get(path, response)
	table.insert(self.routes, { method = "GET", path = path, response = response })
end

function Ludi:post(path, response)
	table.insert(self.routes, { method = "POST", path = path, response = response })
end

function Ludi:listen(port)
	for _, route in ipairs(self.routes) do
		lib.add_route(route.method, route.path, route.response)
	end

	lib.start_server(port or 3000)
end

return Ludi
