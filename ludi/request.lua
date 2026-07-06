local json = require("ludi.json")

local Request = {}
Request.__index = Request

local function parse_query_string(query)
    local result = {}
    for key, value in string.gmatch(query, "([^&=?]+)=([^&=?]+)") do
        result[key] = value
    end
    return result
end

--- raw: the plain table handed over by ludi_core
--- params: path params extracted by the router (e.g. { id = "42" })
function Request.new(raw, params)
    local self = setmetatable({}, Request)

    self.method = raw.method
    self.path = raw.path
    self.headers = raw.headers or {}
    self.params = params or {}
    self.query = parse_query_string(raw.query or "")
    self.body = raw.body or ""

    return self
end

--- Decodes the body as JSON. Returns a table, or nil plus an error message.
function Request:json()
    local ok, decoded = pcall(json.decode, self.body)
    if not ok then return nil, decoded end
    return decoded
end

return Request
