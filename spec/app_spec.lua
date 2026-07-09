local json = require("ludi.json")
local ludi = require("ludi")
local core_stub = require("spec.core_stub")

local function dispatch(app, raw)
    raw.query = raw.query or ""
    raw.headers = raw.headers or {}
    raw.body = raw.body or ""
    return app:_dispatch(raw)
end

describe("app", function()
    it("routes a request and builds the response", function()
        local app = ludi.new()
        app:get("/users/:id", function(req, res)
            res:status(200):json({ id = req.params.id, q = req.query.x })
        end)

        local out = dispatch(app, {
            method = "GET",
            path = "/users/42",
            query = "x=9",
        })

        assert.are.equal(200, out.status)
        local decoded = json.decode(out.body)
        assert.are.equal("42", decoded.id)
        assert.are.equal("9", decoded.q)
    end)

    it("returns 404 for an unknown route", function()
        local out = dispatch(ludi.new(), { method = "GET", path = "/nope" })

        assert.are.equal(404, out.status)
    end)

    it("returns 500 when the handler errors", function()
        local app = ludi.new()
        app:get("/boom", function()
            error("kaboom")
        end)

        local out = dispatch(app, { method = "GET", path = "/boom" })

        assert.are.equal(500, out.status)
    end)

    it("runs global and route middlewares before the handler", function()
        local app = ludi.new()
        local seen = {}

        app:use(function(_, _, next)
            table.insert(seen, "global")
            next()
        end)
        app:get("/mw", function(_, _, next)
            table.insert(seen, "route")
            next()
        end, function(_, res)
            table.insert(seen, "handler")
            res:send("ok")
        end)

        dispatch(app, { method = "GET", path = "/mw" })

        assert.are.same({ "global", "route", "handler" }, seen)
    end)

    it("runs the handler inside a coroutine", function()
        local app = ludi.new()
        local yieldable
        app:get("/co", function(_, res)
            yieldable = coroutine.isyieldable()
            res:send("ok")
        end)

        local out = dispatch(app, { method = "GET", path = "/co" })

        assert.are.equal(200, out.status)
        assert.is_true(yieldable)
    end)

    it("returns 500 when a handler yields without an async runtime", function()
        local app = ludi.new()
        app:get("/yield", function(_, res)
            coroutine.yield()
            res:send("never reached")
        end)

        local out = dispatch(app, { method = "GET", path = "/yield" })

        assert.are.equal(500, out.status)
    end)

    it("hands port and dispatcher to the native core", function()
        ludi.new():listen(8080)

        assert.is_not_nil(core_stub.started)
        assert.are.equal(8080, core_stub.started.port)
        assert.are.equal("function", type(core_stub.started.dispatch))
    end)

    it("runs the listen callback once the server is bound", function()
        local called = false
        ludi.new():listen(8080, function()
            called = true
        end)

        assert.is_true(called)
        assert.are.equal("function", type(core_stub.started.on_listen))
    end)
end)
