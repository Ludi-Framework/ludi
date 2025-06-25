local json = require("utils.json")
local parse_query_string = require("utils.parser")
local Request = {}
Request.__index = Request

function Request.new(raw_body, raw_headers, raw_url)
    local self = setmetatable({}, Request)

    local ok, decoded = pcall(json.decode, raw_body)
    if not ok then
        print("JSON decode error:", decoded)
        self.body = {}
    else
        self.body = decoded
    end

    local parsed_query = {}
    if raw_url then
        local query_string = raw_url:match("%?(.*)")
        if query_string then
            parsed_query = parse_query_string(query_string)
        end
    end
    self.query = parsed_query or ""

    self.headers = raw_headers or ""
    return self
end

return Request
