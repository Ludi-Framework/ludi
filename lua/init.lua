local Ludi = require("bindings")

local server = Ludi.new()

server:get("/hello", function(req, res)
    res:status(202):json({message = "Hello from lua"})
end)

server:post("/echo", function(req, res) res:status(200):send(req.body) end)

server:listen(3000)
