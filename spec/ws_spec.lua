local json = require("ludi.json")
local ludi = require("ludi")
local ws = require("ludi.ws")

-- Unique per test: the ws registry is module state shared by the suite,
-- exactly like it is shared across hot reloads at runtime.
local next_id = 0
local function newId()
    next_id = next_id + 1
    return next_id
end

local function upgrade(app, raw)
    raw.method = "GET"
    raw.query = raw.query or ""
    raw.headers = raw.headers or {}
    raw.body = ""
    return app:_ws_upgrade(newId(), raw), next_id
end

local function fakeHandle()
    local handle = {sent = {}, closed = nil}
    function handle:send(data, binary)
        table.insert(self.sent, {data = data, binary = binary})
        return true
    end
    function handle:close(code, reason)
        self.closed = {code = code, reason = reason}
        return true
    end
    return handle
end

-- Runs the full accept path: handshake, then the upgrade completing.
local function connect(app, path)
    local handle = fakeHandle()
    local decision, id = upgrade(app, {path = path})
    assert.is_true(decision.accept)
    ws.open(id, handle)
    return handle, id
end

describe("websocket", function()
    it("accepts the handshake for a registered route", function()
        local app = ludi.new()
        app:ws("/chat", function() end)

        local decision = upgrade(app, {path = "/chat"})

        assert.is_true(decision.accept)
    end)

    it("rejects an unknown path with 404", function()
        local app = ludi.new()
        app:ws("/chat", function() end)

        local decision = upgrade(app, {path = "/nope"})

        assert.is_nil(decision.accept)
        assert.are.equal(404, decision.status)
    end)

    it("runs the handler with the connection and the request", function()
        local app = ludi.new()
        local got
        app:ws("/room/:id", function(conn, req)
            got = {conn = conn, room = req.params.id}
        end)

        local handle = connect(app, "/room/42")

        assert.are.equal("42", got.room)
        got.conn:send("hi")
        assert.are.same({data = "hi", binary = false}, handle.sent[1])
    end)

    it("lets middlewares reject the handshake", function()
        local app = ludi.new()
        local function auth(req, res, next)
            if req.headers["authorization"] then
                next()
            else
                res:status(401):json({error = "Unauthorized"})
            end
        end
        app:ws("/private", {auth}, function() end)

        local denied = upgrade(app, {path = "/private"})
        local allowed = upgrade(app, {
            path = "/private",
            headers = {authorization = "token"}
        })

        assert.are.equal(401, denied.status)
        assert.are.equal("Unauthorized", json.decode(denied.body).error)
        assert.is_true(allowed.accept)
    end)

    it("runs global middlewares at handshake time", function()
        local app = ludi.new()
        local seen = {}
        app:use(function(_, _, next)
            table.insert(seen, "global")
            next()
        end)
        app:ws("/chat", function() end)

        local decision = upgrade(app, {path = "/chat"})

        assert.is_true(decision.accept)
        assert.are.same({"global"}, seen)
    end)

    it("rejects with 500 when a middleware errors", function()
        local app = ludi.new()
        app:use(function() error("kaboom") end)
        app:ws("/chat", function() end)

        local decision = upgrade(app, {path = "/chat"})

        assert.are.equal(500, decision.status)
    end)

    it("dispatches message events to the listeners", function()
        local app = ludi.new()
        local got = {}
        app:ws("/chat", function(conn)
            conn:on("message", function(data, binary)
                table.insert(got, {data = data, binary = binary})
            end)
        end)

        local _, id = connect(app, "/chat")
        ws.event(id, "message", "hello", false)
        ws.event(id, "message", "\1\2", true)

        assert.are.same({
            {data = "hello", binary = false},
            {data = "\1\2", binary = true}
        }, got)
    end)

    it("echoes through the native handle", function()
        local app = ludi.new()
        app:ws("/echo", function(conn)
            conn:on("message", function(data) conn:send("echo: " .. data) end)
        end)

        local handle, id = connect(app, "/echo")
        ws.event(id, "message", "hi", false)

        assert.are.same({data = "echo: hi", binary = false}, handle.sent[1])
    end)

    it("fires close listeners once and drops the connection", function()
        local app = ludi.new()
        local closes, messages = {}, 0
        app:ws("/chat", function(conn)
            conn:on("message", function() messages = messages + 1 end)
            conn:on("close", function(code, reason)
                table.insert(closes, {code = code, reason = reason})
            end)
        end)

        local _, id = connect(app, "/chat")
        ws.event(id, "close", 1000, "bye")
        ws.event(id, "message", "late", false)
        ws.event(id, "close", 1000, "again")

        assert.are.same({{code = 1000, reason = "bye"}}, closes)
        assert.are.equal(0, messages)
    end)

    it("delivers protocol errors as a table", function()
        local app = ludi.new()
        local got
        app:ws("/chat", function(conn)
            conn:on("error", function(err) got = err end)
        end)

        local _, id = connect(app, "/chat")
        ws.event(id, "error", "Space in headers")

        assert.are.same({message = "Space in headers"}, got)
    end)

    it("exposes closed as a queryable property", function()
        local app = ludi.new()
        local conns = {}
        app:ws("/chat", function(conn) table.insert(conns, conn) end)

        local _, id = connect(app, "/chat")

        assert.is_false(conns[1].closed)
        ws.event(id, "close", 1000, "")
        assert.is_true(conns[1].closed)
    end)

    it("hands the handler the same req the middlewares saw", function()
        local app = ludi.new()
        local got
        local function auth(req, _, next)
            req.user = {name = "ana"}
            next()
        end
        app:ws("/private", {auth}, function(_, req) got = req.user end)

        connect(app, "/private")

        assert.are.same({name = "ana"}, got)
    end)

    it("closes with 1011 when the handler errors", function()
        local app = ludi.new()
        app:ws("/boom", function() error("kaboom") end)

        local handle = connect(app, "/boom")

        assert.are.equal(1011, handle.closed.code)
    end)

    it("closes an upgrade the application never accepted", function()
        local handle = fakeHandle()

        ws.open(newId(), handle)

        assert.are.equal(1011, handle.closed.code)
    end)

    it("registers ws routes through groups with their middlewares", function()
        local app = ludi.new()
        local seen = {}
        local function tag(name)
            return function(_, _, next)
                table.insert(seen, name)
                next()
            end
        end
        app:group("/api", {tag("group")}, function(api)
            api:ws("/chat", tag("route"), function() end)
        end)

        local decision = upgrade(app, {path = "/api/chat"})

        assert.is_true(decision.accept)
        assert.are.same({"group", "route"}, seen)
    end)

    it("hands the ws callbacks to the native core", function()
        local core_stub = require("spec.core_stub")
        ludi.new():listen(8080)

        local callbacks = core_stub.started.ws
        assert.are.equal("function", type(callbacks.upgrade))
        assert.are.equal("function", type(callbacks.open))
        assert.are.equal("function", type(callbacks.event))
    end)
end)
