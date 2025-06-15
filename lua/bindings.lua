local ffi = require("ffi")

ffi.cdef([[
  void add_route(const char* method, const char* path, const char* response);
  void start_server(const int* port);
]])

local lib = ffi.load("../rust/target/release/librust.dylib")

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

	local c_port = ffi.new("int[1]", port or 3000)

	lib.start_server(c_port)
end

return Ludi
