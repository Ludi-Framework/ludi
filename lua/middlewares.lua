local function exec_middlewares(req, res, middlewares, index, on_done)
    index = index or 1

    local middleware = middlewares[index]

    if not middleware then
        if on_done then on_done() end
        return
    end

    middleware(req, res, function()
        exec_middlewares(req, res, middlewares, index + 1, on_done)
    end)
end

return exec_middlewares
