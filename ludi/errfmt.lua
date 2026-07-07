--- Pretty terminal rendering for reload errors.
---
--- Takes the raw error string from a failed reload (`file.lua:12: '='
--- expected near 'x'`), locates the offending source line and renders a
--- framed excerpt with the guilty line highlighted and carets under the
--- token Lua reported `near`. Colors are ANSI, written for stderr; they
--- are disabled under NO_COLOR or a dumb terminal, and can be forced off
--- via `errfmt.color = false` (tests do this).

local errfmt = {}

local CONTEXT = 2 -- source lines shown above and below the offending one

local function colors_enabled()
    if os.getenv("NO_COLOR") then return false end
    local term = os.getenv("TERM")
    return term ~= nil and term ~= "" and term ~= "dumb"
end

errfmt.color = colors_enabled()

local function paint(code, text)
    if not errfmt.color then return text end
    return "\27[" .. code .. "m" .. text .. "\27[0m"
end

local RED, BOLD, DIM, CYAN, GREEN, YELLOW = "1;31", "1", "2", "36", "32", "33"

--- Extracts `file`, `line` and the trailing message from a Lua error
--- string. Handles both direct errors (`./app.lua:3: ...`) and errors
--- wrapped by require (`error loading module 'x' from file ...`), where
--- the useful location sits mid-string.
---@param msg string
---@return {file: string, line: integer, message: string}|nil
function errfmt.parse(msg)
    local file, line, rest = msg:match("([^%s'\"]+%.lua):(%d+): ([^\n]*)")
    if not file then
        file, line, rest = msg:match("^([^\n:]+):(%d+): ([^\n]*)")
    end
    if not file then return nil end
    return {file = file, line = tonumber(line), message = rest}
end

--- The token the parser choked on, from `near 'x'` / `near <eof>`.
---@param message string
---@return string|nil
function errfmt.near_token(message)
    return message:match("near '([^']+)'")
        or (message:find("near <eof>", 1, true) and "<eof>" or nil)
end

-- Last occurrence of the token in the line: for repeated tokens (e.g. a
-- stray second `=`) the parser is complaining about the later one. Bare
-- words match on word boundaries so `end` never highlights `append`.
local function find_token(text, token)
    local pattern = token:gsub("(%W)", "%%%1")
    if token:match("^[%a_][%w_]*$") then
        pattern = "%f[%w_]" .. pattern .. "%f[^%w_]"
    end
    local last_s, last_e, init = nil, nil, 1
    while true do
        local s, e = text:find(pattern, init)
        if not s then return last_s, last_e end
        last_s, last_e, init = s, e, e + 1
    end
end

local function read_lines(path, from, to)
    local file = io.open(path, "r")
    if not file then return nil end
    local lines, n = {}, 0
    for text in file:lines() do
        n = n + 1
        if n > to then break end
        if n >= from then lines[n] = text end
    end
    file:close()
    return lines
end

--- Framed source excerpt around `loc.line`, or nil when the file cannot
--- be read (or is shorter than the reported line).
---@param loc {file: string, line: integer, message: string}
---@return string|nil
function errfmt.frame(loc)
    local first = math.max(1, loc.line - CONTEXT)
    local last = loc.line + CONTEXT
    local lines = read_lines(loc.file, first, last)
    if not lines or lines[loc.line] == nil then return nil end

    while lines[last] == nil do last = last - 1 end
    local width = #tostring(last)

    local out = {}
    for n = first, last do
        local expanded = lines[n]:gsub("\t", "    ")
        local num = ("%" .. width .. "d"):format(n)
        if n == loc.line then
            out[#out + 1] = ("  %s %s %s %s\n"):format(
                paint(RED, "▶"), paint(RED, num), paint(DIM, "│"),
                paint(BOLD, expanded))

            local token = errfmt.near_token(loc.message)
            if token then
                local s, e
                if token == "<eof>" then
                    s, e = #expanded + 1, #expanded + 1
                else
                    s, e = find_token(expanded, token)
                end
                if s then
                    out[#out + 1] = ("    %s %s %s%s\n"):format(
                        (" "):rep(width), paint(DIM, "│"),
                        (" "):rep(s - 1), paint(RED, ("^"):rep(e - s + 1)))
                end
            end
        else
            out[#out + 1] = ("    %s %s %s\n"):format(
                paint(DIM, num), paint(DIM, "│"), paint(DIM, expanded))
        end
    end
    return table.concat(out)
end

--- Full multi-line report for a failed reload, ready for stderr.
---@param err any the error value caught during the reload
---@param kind "syntax"|"runtime"
---@return string
function errfmt.render(err, kind)
    local msg = tostring(err)
    local loc = errfmt.parse(msg)
    local out = {"\n"}

    out[#out + 1] = ("  %s %s %s\n\n"):format(
        paint(RED, "✗"),
        paint(BOLD, kind == "syntax" and "Syntax error" or "Reload failed"),
        paint(DIM, "— keeping previous version"))

    if loc then
        out[#out + 1] = ("  %s %s\n"):format(
            paint(CYAN, ("%s:%d"):format(loc.file, loc.line)), loc.message)
        local frame = errfmt.frame(loc)
        if frame then
            out[#out + 1] = "\n" .. frame
        end
    else
        out[#out + 1] = "  " .. msg:gsub("\n", "\n  ") .. "\n"
    end

    out[#out + 1] = "\n  " ..
        paint(DIM, "Fix the file and save to reload.") .. "\n\n"
    return table.concat(out)
end

--- One-line status messages, matching the report's visual language.
function errfmt.ok(text)
    return ("  %s %s\n"):format(paint(GREEN, "✓"), text)
end

function errfmt.info(text)
    return paint(DIM, ("  %s\n"):format(text))
end

function errfmt.warn(text)
    return ("  %s %s\n"):format(paint(YELLOW, "⚠"), text)
end

return errfmt
