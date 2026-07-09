local json = require("ludi.json")

describe("json", function()
    it("roundtrips tables", function()
        local decoded = json.decode(json.encode({
            name = "ludi",
            n = 3,
            list = { 1, 2, 3 },
            flag = true,
        }))

        assert.are.equal("ludi", decoded.name)
        assert.are.equal(3, decoded.n)
        assert.are.same({ 1, 2, 3 }, decoded.list)
        assert.is_true(decoded.flag)
    end)

    it("parses numbers followed by a comma", function()
        local decoded = json.decode('{"a":1,"b":2}')

        assert.are.equal(1, decoded.a)
        assert.are.equal(2, decoded.b)
    end)

    it("parses whitespace before object keys", function()
        local decoded = json.decode('{\n    "name": "Marcos",\n    "email": "marcos@email.com"\n}')

        assert.are.equal("Marcos", decoded.name)
        assert.are.equal("marcos@email.com", decoded.email)
    end)

    it("roundtrips escaped strings", function()
        local decoded = json.decode(json.encode({ s = 'quote " and \n newline' }))

        assert.are.equal('quote " and \n newline', decoded.s)
    end)
end)
