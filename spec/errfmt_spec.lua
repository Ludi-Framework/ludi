local errfmt = require("ludi.errfmt")

local function write_source(contents)
    local path = os.tmpname() .. ".lua"
    local file = assert(io.open(path, "w"))
    file:write(contents)
    file:close()
    return path
end

describe("errfmt", function()
    local source

    setup(function() errfmt.color = false end)
    teardown(function() errfmt.color = true end)

    after_each(function()
        if source then os.remove(source) end
        source = nil
    end)

    describe("parse", function()
        it("extracts file, line and message from a plain error", function()
            local loc = errfmt.parse("./app.lua:12: '=' expected near 'x'")
            assert.are.equal("./app.lua", loc.file)
            assert.are.equal(12, loc.line)
            assert.are.equal("'=' expected near 'x'", loc.message)
        end)

        it("finds the location inside a require() wrapper", function()
            local loc = errfmt.parse(
                "error loading module 'routes' from file './routes.lua':\n" ..
                    "\t./routes.lua:3: unexpected symbol near ')'")
            assert.are.equal("./routes.lua", loc.file)
            assert.are.equal(3, loc.line)
        end)

        it("returns nil when there is no location", function()
            assert.is_nil(errfmt.parse("boom"))
        end)
    end)

    describe("near_token", function()
        it("reads quoted tokens", function()
            assert.are.equal("end", errfmt.near_token("'=' expected near 'end'"))
        end)

        it("reads <eof>", function()
            assert.are.equal("<eof>", errfmt.near_token("'end' expected near <eof>"))
        end)

        it("returns nil otherwise", function()
            assert.is_nil(errfmt.near_token("attempt to index a nil value"))
        end)
    end)

    describe("render", function()
        it("frames the offending line with context and carets", function()
            source = write_source("local a = 1\nlocal b = 2\nlocal c = = 3\nlocal d = 4\n")
            local report = errfmt.render(
                source .. ":3: unexpected symbol near '='", "syntax")

            assert.truthy(report:find("Syntax error", 1, true))
            assert.truthy(report:find("keeping previous version", 1, true))
            assert.truthy(report:find(source .. ":3", 1, true))
            assert.truthy(report:find("▶ 3 │ local c = = 3", 1, true))
            assert.truthy(report:find("2 │ local b = 2", 1, true))
            assert.truthy(report:find("4 │ local d = 4", 1, true))
        end)

        it("points the caret at the last occurrence of the token", function()
            source = write_source("local c = = 3\n")
            local report = errfmt.render(
                source .. ":1: unexpected symbol near '='", "syntax")

            local caret_line = report:match("│ ( *%^+)\n")
            assert.are.equal((" "):rep(10) .. "^", caret_line)
        end)

        it("matches bare words on word boundaries", function()
            source = write_source("local append end\n")
            local report = errfmt.render(
                source .. ":1: unexpected symbol near 'end'", "syntax")

            local caret_line = report:match("│ ( *%^+)\n")
            assert.are.equal((" "):rep(13) .. "^^^", caret_line)
        end)

        it("falls back to the raw message when the file is unreadable", function()
            local report = errfmt.render("/nope/missing.lua:9: boom", "runtime")
            assert.truthy(report:find("Reload failed", 1, true))
            assert.truthy(report:find("/nope/missing.lua:9", 1, true))
            assert.is_nil(report:find("▶", 1, true))
        end)

        it("prints unparseable errors verbatim", function()
            local report = errfmt.render("something exploded", "runtime")
            assert.truthy(report:find("something exploded", 1, true))
        end)
    end)
end)
