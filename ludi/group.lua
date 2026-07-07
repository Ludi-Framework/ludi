--- Route groups: a shared path prefix plus middlewares for a set of
--- routes, in the spirit of an express Router mounted on a path or a
--- fastify plugin registered with a prefix. A group owns no routes —
--- every registration lands on the root application, with the prefix
--- prepended and the group middlewares merged in.

---@class ludi.Group
---@field app Ludi
---@field prefix string full prefix, parents included
---@field middlewares ludi.Middleware[] accumulated group middlewares
---@field get ludi.GroupRouteMethod
---@field post ludi.GroupRouteMethod
---@field put ludi.GroupRouteMethod
---@field delete ludi.GroupRouteMethod
---@field patch ludi.GroupRouteMethod
---@field options ludi.GroupRouteMethod
---@field head ludi.GroupRouteMethod
local Group = {}
Group.__index = Group

--- Route registration method: (path, handler) or (path, middleware(s), handler).
---@alias ludi.GroupRouteMethod fun(self: ludi.Group, path: string, handler: ludi.Handler)|fun(self: ludi.Group, path: string, middlewares: ludi.Middleware|ludi.Middleware[], handler: ludi.Handler)

--- The body callback style: app:group("/api", function(api) ... end).
---@alias ludi.GroupBody fun(group: ludi.Group)

local function copy(list)
    local out = {}
    for i, item in ipairs(list) do out[i] = item end
    return out
end

local function to_list(middlewares)
    if middlewares == nil then return {} end
    if type(middlewares) == "function" then return {middlewares} end
    if type(middlewares) == "table" then return copy(middlewares) end
    error("Group middlewares must be function or table of functions")
end

-- "/api" + "/" must expose the group root as "/api", not "/api/".
-- Everything else is plain concatenation; the router ignores empty
-- segments, so a double slash in the joined path is harmless.
local function join(prefix, path)
    if path == "/" or path == "" then return prefix end
    return prefix .. path
end

--- Builds a group under a parent prefix, resolving the overloads:
--- (prefix), (prefix, body), (prefix, middlewares) and
--- (prefix, middlewares, body). A lone function argument is always the
--- body — a single middleware without a body needs `{mw}` or `group:use`.
--- The parent middlewares are copied, so later changes to the parent do
--- not leak into an already-created group.
---@param app Ludi
---@param parent_prefix string
---@param parent_middlewares ludi.Middleware[]
---@param prefix string
---@param a? ludi.Middleware|ludi.Middleware[]|ludi.GroupBody
---@param b? ludi.GroupBody
---@return ludi.Group
function Group.new(app, parent_prefix, parent_middlewares, prefix, a, b)
    assert(type(prefix) == "string" and prefix:sub(1, 1) == "/",
           'Group prefix must be a string starting with "/"')

    local middlewares, body
    if b ~= nil then
        middlewares, body = a, b
    elseif type(a) == "function" then
        body = a
    else
        middlewares = a
    end
    assert(body == nil or type(body) == "function",
           "Group body must be a function")

    local group = setmetatable({
        app = app,
        prefix = join(parent_prefix, prefix),
        middlewares = copy(parent_middlewares)
    }, Group)
    for _, middleware in ipairs(to_list(middlewares)) do
        table.insert(group.middlewares, middleware)
    end

    if body then body(group) end
    return group
end

--- Adds a middleware to the group. Applies to routes registered after
--- this call (and to nested groups created after it), express-style.
---@param middleware ludi.Middleware
function Group:use(middleware)
    assert(type(middleware) == "function", "Middleware must be a function")
    table.insert(self.middlewares, middleware)
end

--- Registers a route on the root application, with the group prefix
--- prepended and the group middlewares before the route's own.
---@param method string uppercase HTTP verb, e.g. "GET"
---@param path string path pattern, relative to the group prefix
---@param options? ludi.Middleware|ludi.Middleware[] route-level middleware(s)
---@param handler ludi.Handler
function Group:addRoute(method, path, options, handler)
    local merged = copy(self.middlewares)
    for _, middleware in ipairs(to_list(options)) do
        table.insert(merged, middleware)
    end
    self.app:addRoute(method, join(self.prefix, path), merged, handler)
end

--- Creates a nested group; prefix and middlewares accumulate.
---@param prefix string
---@param middlewares? ludi.Middleware|ludi.Middleware[]
---@param body? ludi.GroupBody
---@return ludi.Group
---@overload fun(self: ludi.Group, prefix: string, body: ludi.GroupBody): ludi.Group
---@overload fun(self: ludi.Group, prefix: string): ludi.Group
function Group:group(prefix, middlewares, body)
    return Group.new(self.app, self.prefix, self.middlewares, prefix,
                     middlewares, body)
end

local function makeMethod(method)
    return function(self, path, ...)
        local args = {...}
        if #args == 1 then
            self:addRoute(method, path, nil, args[1])
        elseif #args == 2 then
            self:addRoute(method, path, args[1], args[2])
        else
            error(
                ("Expected: %s(path, handler) or %s(path, middleware(s), handler)"):format(
                    method:lower(), method:lower()))
        end
    end
end

Group.get = makeMethod("GET")
Group.post = makeMethod("POST")
Group.put = makeMethod("PUT")
Group.delete = makeMethod("DELETE")
Group.patch = makeMethod("PATCH")
Group.options = makeMethod("OPTIONS")
Group.head = makeMethod("HEAD")

return Group
