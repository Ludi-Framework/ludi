local Router = require("ludi.router")

describe("router", function()
    it("matches a literal path", function()
        local routes = { { method = "GET", segments = Router.compile("/hello") } }
        local route, params = Router.match(routes, "GET", "/hello")

        assert.is_not_nil(route)
        assert.are.same({}, params)
    end)

    it("extracts path params", function()
        local routes = {
            {
                method = "GET",
                segments = Router.compile("/users/:id/posts/:post_id"),
            },
        }
        local route, params = Router.match(routes, "GET", "/users/42/posts/7")

        assert.is_not_nil(route)
        assert.are.equal("42", params.id)
        assert.are.equal("7", params.post_id)
    end)

    it("respects the HTTP method", function()
        local routes = { { method = "GET", segments = Router.compile("/hello") } }

        assert.is_nil(Router.match(routes, "POST", "/hello"))
    end)

    it("rejects paths with a different segment count", function()
        local routes = { { method = "GET", segments = Router.compile("/a/b") } }

        assert.is_nil(Router.match(routes, "GET", "/a"))
        assert.is_nil(Router.match(routes, "GET", "/a/b/c"))
    end)
end)
