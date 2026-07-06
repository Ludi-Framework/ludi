-- Busted helper (loaded before every spec, see .busted).
-- Stubs the native ludi_core module so framework logic is tested in
-- isolation; specs can inspect what reached the core via spec.core_stub.

local stub = {started = nil}
package.loaded["spec.core_stub"] = stub

package.preload["ludi_core"] = function()
    return {
        start_server = function(port, dispatch)
            stub.started = {port = port, dispatch = dispatch}
        end
    }
end
