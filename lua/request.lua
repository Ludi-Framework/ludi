local json = require("utils.json")

local Request = {}
Request.__index = Request

function Request.new(raw_body, raw_headers)
    local self = setmetatable({}, Request)

    local ok, decoded = pcall(json.decode, raw_body)
    if not ok then
        print("JSON decode error:", decoded)
        self.body = {}
    else
        self.body = decoded
    end

    self.headers = raw_headers or ""
    return self
end

return Request
