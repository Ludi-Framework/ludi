--- Runs global + route middlewares in order, then the handler.
--- Each middleware receives (req, res, next); the chain only advances
--- when next() is called, express-style.
---@param req ludi.Request
---@param res ludi.Response
---@param global_middlewares ludi.Middleware[]
---@param route_middlewares ludi.Middleware[]
---@param handler fun()
local function run_chain(req, res, global_middlewares, route_middlewares, handler)
    local chain = {}

    for _, middleware in ipairs(global_middlewares) do
        table.insert(chain, middleware)
    end
    for _, middleware in ipairs(route_middlewares) do
        table.insert(chain, middleware)
    end

    local function step(i)
        if i > #chain then
            handler()
            return
        end
        chain[i](req, res, function() step(i + 1) end)
    end

    step(1)
end

return run_chain
