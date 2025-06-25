local Ludi = require("bindings")

local server = Ludi.new()

server:get("/hello", function(req, res)
    res:status(202):json({message = "Hello from lua"})
end)

server:post("/echo", function(req, res)
    res:status(200)
    res:send(req.body or "Sem corpo")
end)

server:listen(3000)
