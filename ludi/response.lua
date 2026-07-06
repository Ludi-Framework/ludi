local json = require("ludi.json")

local Response = {}
Response.__index = Response

function Response.new()
    return setmetatable({status_code = 200, body = "", headers = {}}, Response)
end

function Response:status(code)
    self.status_code = code
    return self
end

function Response:header(name, value)
    self.headers[name] = value
    return self
end

function Response:send(data)
    if type(data) == "table" then
        self.body = json.encode(data)
        self.headers["Content-Type"] = self.headers["Content-Type"] or
                                           "application/json"
    elseif type(data) == "string" then
        self.body = data
        self.headers["Content-Type"] = self.headers["Content-Type"] or
                                           "text/plain"
    end

    return self
end

function Response:json(data) return self:send(data) end

--- Plain table handed back to ludi_core.
function Response:build()
    return {
        status = self.status_code,
        headers = self.headers,
        body = self.body
    }
end

return Response
