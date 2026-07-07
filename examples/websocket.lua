local ludi = require("ludi")

local app = ludi.new()

-- Echo: every message comes straight back.
app:ws("/echo", function(conn)
    conn:on("message", function(data, binary)
        conn:send(data, binary)
    end)
end)

-- Chat room: naive broadcast to everyone connected to the same room.
local rooms = {}

app:ws("/chat/:room", function(conn, req)
    local room = req.params.room
    rooms[room] = rooms[room] or {}
    table.insert(rooms[room], conn)

    conn:on("message", function(data)
        for _, peer in ipairs(rooms[room]) do
            peer:send(data)
        end
    end)

    conn:on("close", function()
        for i, peer in ipairs(rooms[room]) do
            if peer == conn then
                table.remove(rooms[room], i)
                break
            end
        end
    end)
end)

-- WebSocket routes take middlewares too; they run at handshake time.
local function auth(req, res, next)
    if req.headers["authorization"] then
        next()
    else
        res:status(401):json({ error = "Unauthorized" })
    end
end

app:ws("/private", { auth }, function(conn)
    conn:send("welcome")
end)

app:listen(3000, function()
    print("Server listening on http://localhost:3000")
end)
