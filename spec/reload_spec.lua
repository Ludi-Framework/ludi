local ludi = require("ludi")
local reload = require("ludi.reload")

-- Builds a handler wired the same way init.lua wires it: the interceptor
-- lands on Ludi._capture_listen so the entrypoint's app:listen() is captured
-- instead of starting a server.
local function make_handler(entrypoint, on_swap)
    return reload.handler(entrypoint,
                          function(fn) ludi._capture_listen = fn end,
                          on_swap)
end

local function write_entrypoint(contents)
    local path = os.tmpname() .. ".lua"
    local file = assert(io.open(path, "w"))
    file:write(contents)
    file:close()
    return path
end

describe("hot reload", function()
    local entrypoint

    after_each(function()
        if entrypoint then os.remove(entrypoint) end
        entrypoint = nil
        ludi._capture_listen = nil
    end)

    it("re-runs the entrypoint and swaps in the app it listens with", function()
        entrypoint = write_entrypoint([[
            local ludi = require("ludi")
            local app = ludi.new()
            app:get("/fresh", function(_, res) res:send("ok") end)
            app:listen(3000)
        ]])

        local swapped
        make_handler(entrypoint, function(app) swapped = app end)({"app.lua"})

        assert.is_not_nil(swapped)
        assert.are.equal("/fresh", swapped.routes[1].path)
        assert.is_nil(ludi._capture_listen)
    end)

    it("clears application modules from package.loaded, keeping ludi and the stdlib", function()
        entrypoint = write_entrypoint([[require("ludi").new():listen(3000)]])
        package.loaded["myapp.routes"] = {version = 1}

        make_handler(entrypoint, function() end)({"myapp/routes.lua"})

        assert.is_nil(package.loaded["myapp.routes"])
        assert.are.equal(ludi, package.loaded["ludi"])
        assert.is_not_nil(package.loaded["string"])
    end)

    it("keeps the previous app when the entrypoint has a syntax error", function()
        entrypoint = write_entrypoint([[local x = = 1]])

        local swapped
        make_handler(entrypoint, function(app) swapped = app end)({"app.lua"})

        assert.is_nil(swapped)
        assert.is_nil(ludi._capture_listen)
    end)

    it("keeps the previous app when the reload errors", function()
        entrypoint = write_entrypoint([[error("boom at load time")]])

        local swapped
        make_handler(entrypoint, function(app) swapped = app end)({"app.lua"})

        assert.is_nil(swapped)
        assert.is_nil(ludi._capture_listen)
    end)

    it("keeps the previous app when the entrypoint never calls listen", function()
        entrypoint = write_entrypoint([[local _ = require("ludi").new()]])

        local swapped
        make_handler(entrypoint, function(app) swapped = app end)({"app.lua"})

        assert.is_nil(swapped)
    end)
end)
