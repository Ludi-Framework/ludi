local ludi = require("ludi")

local app = ludi.new()

local function auth(req, res, next)
    if req.headers["authorization"] then
        next()
    else
        res:status(401):json({error = "Unauthorized"})
    end
end

app:group("/api", function(api)
    api:get("/users", function(_, res)          -- GET /api/users
        res:json({users = {"ana", "bruno"}})
    end)

    api:group("/v2", function(v2)
        v2:get("/users/:id", function(req, res) -- GET /api/v2/users/42
            res:json({id = req.params.id, version = 2})
        end)
    end)
end)

app:group("/admin", {auth}, function(admin)
    admin:get("/stats", function(_, res)        -- 401 without Authorization
        res:json({uptime = os.clock()})
    end)
end)

app:listen(3000, function()
    print("Server listening on http://localhost:3000")
end)
