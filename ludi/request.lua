local json = require("ludi.json")

---@class ludi.Request
---@field method string HTTP verb, e.g. "GET"
---@field path string request path, e.g. "/users/42"
---@field headers table<string, string>
---@field params table<string, string> path params, e.g. { id = "42" }
---@field query table<string, string> parsed query string
---@field body string raw request body
local Request = {}
Request.__index = Request

local function parse_query_string(query)
    local result = {}
    for key, value in string.gmatch(query, "([^&=?]+)=([^&=?]+)") do
        result[key] = value
    end
    return result
end

---@param raw ludi.RawRequest the plain table handed over by ludi_core
---@param params? table<string, string> path params extracted by the router
---@return ludi.Request
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

--- Decodes the body as JSON.
---@return any? decoded decoded value, or nil on parse failure
---@return string? err error message when decoding fails
function Request:json()
    local ok, decoded = pcall(json.decode, self.body)
    if not ok then return nil, decoded end
    return decoded
end

return Request
