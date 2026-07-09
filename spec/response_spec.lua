local Response = require("ludi.response")

describe("response", function()
    it("chains status and json", function()
        local built = Response.new():status(202):json({ ok = true }):build()

        assert.are.equal(202, built.status)
        assert.are.equal("application/json", built.headers["Content-Type"])
        assert.are.equal('{"ok":true}', built.body)
    end)

    it("send with a string sets text/plain", function()
        local built = Response.new():send("hi"):build()

        assert.are.equal("text/plain", built.headers["Content-Type"])
        assert.are.equal("hi", built.body)
    end)

    it("keeps a custom Content-Type over the send default", function()
        local built = Response.new():header("Content-Type", "text/html"):send("<p>hi</p>"):build()

        assert.are.equal("text/html", built.headers["Content-Type"])
    end)
end)
