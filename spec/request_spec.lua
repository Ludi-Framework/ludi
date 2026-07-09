local Request = require("ludi.request")

describe("request", function()
    it("parses the query string", function()
        local req = Request.new({
            method = "GET",
            path = "/",
            query = "a=1&b=two",
        })

        assert.are.equal("1", req.query.a)
        assert.are.equal("two", req.query.b)
    end)

    it("keeps the raw body and decodes JSON on demand", function()
        local req = Request.new({
            method = "POST",
            path = "/",
            query = "",
            body = '{"name":"ludi"}',
        })

        assert.are.equal('{"name":"ludi"}', req.body)
        assert.are.equal("ludi", req:json().name)
    end)

    it("exposes path params passed by the router", function()
        local req = Request.new({ method = "GET", path = "/users/42", query = "" }, { id = "42" })

        assert.are.equal("42", req.params.id)
    end)
end)
