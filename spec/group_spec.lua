local json = require("ludi.json")
local ludi = require("ludi")

local function dispatch(app, raw)
    raw.query = raw.query or ""
    raw.headers = raw.headers or {}
    raw.body = raw.body or ""
    return app:_dispatch(raw)
end

describe("group", function()
    it("prefixes every route registered through it", function()
        local app = ludi.new()
        app:group("/api", function(api)
            api:get("/users", function(_, res)
                res:send("list")
            end)
            api:post("/users", function(_, res)
                res:status(201):send("made")
            end)
        end)

        assert.are.equal(200, dispatch(app, { method = "GET", path = "/api/users" }).status)
        assert.are.equal(201, dispatch(app, { method = "POST", path = "/api/users" }).status)
        assert.are.equal(404, dispatch(app, { method = "GET", path = "/users" }).status)
    end)

    it("also works without a body callback", function()
        local app = ludi.new()
        local api = app:group("/api")
        api:get("/ping", function(_, res)
            res:send("pong")
        end)

        local out = dispatch(app, { method = "GET", path = "/api/ping" })

        assert.are.equal(200, out.status)
        assert.are.equal("pong", out.body)
    end)

    it("maps the '/' path to the group root", function()
        local app = ludi.new()
        app:group("/api", function(api)
            api:get("/", function(_, res)
                res:send("root")
            end)
        end)

        assert.are.equal(200, dispatch(app, { method = "GET", path = "/api" }).status)
        assert.are.equal(200, dispatch(app, { method = "GET", path = "/api/" }).status)
    end)

    it("captures path params under the prefix", function()
        local app = ludi.new()
        app:group("/api", function(api)
            api:get("/users/:id", function(req, res)
                res:json({ id = req.params.id })
            end)
        end)

        local out = dispatch(app, { method = "GET", path = "/api/users/42" })

        assert.are.equal("42", json.decode(out.body).id)
    end)

    it("nests groups, accumulating prefixes", function()
        local app = ludi.new()
        app:group("/api", function(api)
            api:group("/v1", function(v1)
                v1:get("/users", function(_, res)
                    res:send("v1")
                end)
            end)
        end)

        local out = dispatch(app, { method = "GET", path = "/api/v1/users" })

        assert.are.equal(200, out.status)
        assert.are.equal("v1", out.body)
    end)

    it("runs group middlewares after global and before route ones", function()
        local app = ludi.new()
        local seen = {}
        local function tag(name)
            return function(_, _, next)
                table.insert(seen, name)
                next()
            end
        end

        app:use(tag("global"))
        app:group("/api", { tag("group") }, function(api)
            api:get("/x", tag("route"), function(_, res)
                table.insert(seen, "handler")
                res:send("ok")
            end)
        end)

        dispatch(app, { method = "GET", path = "/api/x" })

        assert.are.same({ "global", "group", "route", "handler" }, seen)
    end)

    it("halts the request when a group middleware does not call next", function()
        local app = ludi.new()
        local function auth(req, res, next)
            if req.headers["authorization"] then
                next()
            else
                res:status(401):json({ error = "Unauthorized" })
            end
        end

        app:group("/admin", { auth }, function(admin)
            admin:get("/stats", function(_, res)
                res:send("secret")
            end)
        end)

        local denied = dispatch(app, { method = "GET", path = "/admin/stats" })
        local allowed = dispatch(app, {
            method = "GET",
            path = "/admin/stats",
            headers = { authorization = "token" },
        })

        assert.are.equal(401, denied.status)
        assert.are.equal(200, allowed.status)
    end)

    it("applies use() middlewares to routes registered afterwards", function()
        local app = ludi.new()
        local seen = {}

        app:group("/api", function(api)
            api:get("/before", function(_, res)
                res:send("ok")
            end)
            api:use(function(_, _, next)
                table.insert(seen, "mw")
                next()
            end)
            api:get("/after", function(_, res)
                res:send("ok")
            end)
        end)

        dispatch(app, { method = "GET", path = "/api/before" })
        assert.are.same({}, seen)

        dispatch(app, { method = "GET", path = "/api/after" })
        assert.are.same({ "mw" }, seen)
    end)

    it("passes parent middlewares down to nested groups", function()
        local app = ludi.new()
        local seen = {}
        local function tag(name)
            return function(_, _, next)
                table.insert(seen, name)
                next()
            end
        end

        local api = app:group("/api", { tag("outer") })
        api:group("/v1", { tag("inner") }, function(v1)
            v1:get("/users", function(_, res)
                res:send("ok")
            end)
        end)

        dispatch(app, { method = "GET", path = "/api/v1/users" })

        assert.are.same({ "outer", "inner" }, seen)
    end)

    it("rejects a prefix that does not start with a slash", function()
        assert.has_error(function()
            ludi.new():group("api")
        end)
    end)
end)
