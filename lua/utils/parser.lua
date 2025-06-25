local function parse_query_string(query)
    local result = {}
    for key, value in string.gmatch(query, "([^&=?]+)=([^&=?]+)") do
        if value ~= "nil" then
            result[key] = value
        else
            result[key] = nil
        end
    end
    return result
end

return parse_query_string
