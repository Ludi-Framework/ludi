local Router = {}

--- Compiles a path pattern like "/users/:id/posts/:post_id" into segments.
function Router.compile(path)
    local segments = {}
    for segment in path:gmatch("[^/]+") do
        if segment:sub(1, 1) == ":" then
            table.insert(segments, { param = segment:sub(2) })
        else
            table.insert(segments, { literal = segment })
        end
    end
    return segments
end

local function split(path)
    local parts = {}
    for segment in path:gmatch("[^/]+") do table.insert(parts, segment) end
    return parts
end

--- Matches a request against a list of routes.
--- Returns the route and a table of extracted path params, or nil.
function Router.match(routes, method, path)
    local parts = split(path)

    for _, route in ipairs(routes) do
        if route.method == method and #route.segments == #parts then
            local params = {}
            local matched = true

            for i, segment in ipairs(route.segments) do
                if segment.param then
                    params[segment.param] = parts[i]
                elseif segment.literal ~= parts[i] then
                    matched = false
                    break
                end
            end

            if matched then return route, params end
        end
    end

    return nil
end

return Router
