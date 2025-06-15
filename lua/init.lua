local Ludi = require("bindings")

local app = Ludi.new()

app:get("/hello", "Hello, World!")

app:listen()
