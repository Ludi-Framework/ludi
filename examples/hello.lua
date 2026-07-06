local ludi = require("ludi")

local app = ludi.new()

app:use(function(req, res, next)
    print(("%s %s"):format(req.method, req.path))
    next()
end)

app:get("/hello", function(req, res)
    print(req.query["teste"])
    res:status(202):json({message = "Hello from lua"})
end)

app:get("/users/:id", function(req, res)
    res:json({id = req.params.id})
end)

app:post("/echo", function(req, res)
    res:status(200):send(req.body ~= "" and req.body or "Sem corpo")
end)

app:listen(3000)
