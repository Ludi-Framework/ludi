local json = require("utils.json")
local ffi = require("ffi")

local Response = {}
Response.__index = Response

function Response.new()
    local self = setmetatable({status_code = 200, body = {}, headers = {}},
                              Response)
    return self
end

function Response:status(code)
    self.status_code = code
    return self
end

function Response:send(data)
    if type(data) == "table" then
        self.body = json.encode(data)
        self.headers["Content-Type"] = "application/json"
    elseif type(data) == "string" then
        self.body = data
        self.headers["Content-Type"] = "text/plain"
    else
        return self
    end

    return self
end

function Response:json(table) return self:send(table) end

function Response:build()
    local encoded = json.encode({
        status_code = self.status_code,
        body = self.body,
        headers = self.headers
    })
    local cstr = ffi.new("char[?]", #encoded + 1)
    ffi.copy(cstr, encoded)
    cstr[#encoded] = 0
    return cstr
end

return Response
