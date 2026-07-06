local json = {}

local function escape_str(s)
    s = s:gsub("\\", "\\\\")
    s = s:gsub("\"", "\\\"")
    s = s:gsub("\b", "\\b")
    s = s:gsub("\f", "\\f")
    s = s:gsub("\n", "\\n")
    s = s:gsub("\r", "\\r")
    s = s:gsub("\t", "\\t")
    return s
end

-- utf8 is only available on Lua 5.3+; on LuaJIT/5.1 fall back to a plain
-- codepoint-to-utf8 encoder.
local function codepoint_to_utf8(code)
    if utf8 and utf8.char then return utf8.char(code) end
    if code < 0x80 then
        return string.char(code)
    elseif code < 0x800 then
        return string.char(0xC0 + math.floor(code / 0x40), 0x80 + code % 0x40)
    else
        return string.char(0xE0 + math.floor(code / 0x1000),
                           0x80 + math.floor(code / 0x40) % 0x40,
                           0x80 + code % 0x40)
    end
end

local function encode_value(v)
    local t = type(v)
    if t == "nil" then
        return "null"
    elseif t == "boolean" or t == "number" then
        return tostring(v)
    elseif t == "string" then
        return '"' .. escape_str(v) .. '"'
    elseif t == "table" then
        local is_array = (#v > 0)
        local res = {}
        if is_array then
            for _, item in ipairs(v) do
                table.insert(res, encode_value(item))
            end
            return "[" .. table.concat(res, ",") .. "]"
        else
            for k, val in pairs(v) do
                table.insert(res,
                             '"' .. escape_str(k) .. '":' .. encode_value(val))
            end
            return "{" .. table.concat(res, ",") .. "}"
        end
    end
    error("unsupported value: " .. t)
end

--- Encodes a Lua value as a JSON string.
---@param tbl any
---@return string
function json.encode(tbl) return encode_value(tbl) end

--- Decodes a JSON string into a Lua value. Raises on invalid JSON.
---@param str string
---@return any
function json.decode(str)
    if type(str) ~= "string" or str == "" then return {} end

    local idx = 1

    local parse_object, parse_array, parse_value, parse_string, parse_number

    local function skip_ws()
        while str:sub(idx, idx):match("%s") do idx = idx + 1 end
    end

    parse_value = function()
        skip_ws()
        local c = str:sub(idx, idx)
        if c == "{" then
            return parse_object()
        elseif c == "[" then
            return parse_array()
        elseif c == '"' then
            return parse_string()
        elseif str:sub(idx, idx + 3) == "true" then
            idx = idx + 4
            return true
        elseif str:sub(idx, idx + 4) == "false" then
            idx = idx + 5
            return false
        elseif str:sub(idx, idx + 3) == "null" then
            idx = idx + 4
            return nil
        else
            return parse_number()
        end
    end

    parse_string = function()
        idx = idx + 1
        local res = ""
        while true do
            local c = str:sub(idx, idx)
            if c == '"' then
                idx = idx + 1
                break
            end
            if c == '\\' then
                local next_c = str:sub(idx + 1, idx + 1)
                if next_c == 'u' then
                    local hex = str:sub(idx + 2, idx + 5)
                    res = res .. codepoint_to_utf8(tonumber(hex, 16))
                    idx = idx + 6
                else
                    local escapes = {
                        ['"'] = '"',
                        ['\\'] = '\\',
                        ['/'] = '/',
                        b = '\b',
                        f = '\f',
                        n = '\n',
                        r = '\r',
                        t = '\t'
                    }
                    res = res .. (escapes[next_c] or next_c)
                    idx = idx + 2
                end
            else
                res = res .. c
                idx = idx + 1
            end
        end
        return res
    end

    parse_number = function()
        local start = idx
        -- '-' must be last inside the set, otherwise '+-.' reads as the
        -- ASCII range 43-46 which accidentally includes ','
        while str:sub(idx, idx):match("[0-9eE.+-]") do idx = idx + 1 end
        return tonumber(str:sub(start, idx - 1))
    end

    parse_array = function()
        idx = idx + 1
        local res = {}
        skip_ws()
        if str:sub(idx, idx) == "]" then
            idx = idx + 1
            return res
        end
        while true do
            table.insert(res, parse_value())
            skip_ws()
            local c = str:sub(idx, idx)
            if c == "]" then
                idx = idx + 1
                break
            end
            assert(c == ",", "Expected ',' in array")
            idx = idx + 1
        end
        return res
    end

    parse_object = function()
        idx = idx + 1
        local res = {}
        skip_ws()
        if str:sub(idx, idx) == "}" then
            idx = idx + 1
            return res
        end
        while true do
            skip_ws()
            local key = parse_string()
            skip_ws()
            assert(str:sub(idx, idx) == ":", "Expected ':' in object")
            idx = idx + 1
            res[key] = parse_value()
            skip_ws()
            local c = str:sub(idx, idx)
            if c == "}" then
                idx = idx + 1
                break
            end
            assert(c == ",", "Expected ',' in object")
            idx = idx + 1
        end
        return res
    end

    local val = parse_value()
    skip_ws()
    return val
end

return json
