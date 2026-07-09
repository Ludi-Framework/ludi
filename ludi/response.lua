local json = require("ludi.json")

---@class ludi.Response
---@field status_code integer
---@field body string
---@field headers table<string, string>
local Response = {}
Response.__index = Response

---@return ludi.Response
function Response.new()
    return setmetatable({ status_code = 200, body = "", headers = {} }, Response)
end

--- Sets the HTTP status code.
---@param code integer
---@return ludi.Response self
function Response:status(code)
    self.status_code = code
    return self
end

--- Sets a response header.
---@param name string
---@param value string
---@return ludi.Response self
function Response:header(name, value)
    self.headers[name] = value
    return self
end

--- Sets the body. Tables are JSON-encoded (Content-Type application/json),
--- strings sent as-is (Content-Type text/plain), unless already set.
---@param data table|string
---@return ludi.Response self
function Response:send(data)
    if type(data) == "table" then
        self.body = json.encode(data)
        self.headers["Content-Type"] = self.headers["Content-Type"] or "application/json"
    elseif type(data) == "string" then
        self.body = data
        self.headers["Content-Type"] = self.headers["Content-Type"] or "text/plain"
    end

    return self
end

--- Sends data as JSON (alias for send).
---@param data table|string
---@return ludi.Response self
function Response:json(data)
    return self:send(data)
end

--- Plain table handed back to ludi_core.
---@return ludi.RawResponse
function Response:build()
    return {
        status = self.status_code,
        headers = self.headers,
        body = self.body,
    }
end

return Response
