local Ludi = require("bindings")

local app = Ludi.new()

app:get("/hello", "hello")

app:listen(4000)
