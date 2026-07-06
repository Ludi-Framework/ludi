local run_chain = require("ludi.middleware")

describe("middleware chain", function()
    local function tracker(order, name)
        return function(_, _, next)
            table.insert(order, name)
            next()
        end
    end

    it("runs globals, then route middlewares, then the handler", function()
        local order = {}

        run_chain({}, {}, {tracker(order, "g1"), tracker(order, "g2")},
                  {tracker(order, "r1")},
                  function() table.insert(order, "handler") end)

        assert.are.same({"g1", "g2", "r1", "handler"}, order)
    end)

    it("halts when a middleware does not call next()", function()
        local reached = false

        run_chain({}, {}, {function() end}, {},
                  function() reached = true end)

        assert.is_false(reached)
    end)
end)
