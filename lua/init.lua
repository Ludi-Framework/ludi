local Ludi = require("bindings")

local app = Ludi.new()

app:get("/hello", "Hello World")
app:get("/secure", {auth = true, cache = false}, "Secure Content")
app:post("/submit", function() return "Form submitted" end)
app:put("/item/1", {auth = true}, "Item updated")
app:delete("/item/1", "Item deleted")
-- app:patch("/item/1", {partial = true}, "Item partially updated")

app:listen(4000)
