-- Busted helper (loaded before every spec, see .busted).
-- Stubs the native ludi_core module so framework logic is tested in
-- isolation; specs can inspect what reached the core via spec.core_stub.

local stub = {started = nil}
package.loaded["spec.core_stub"] = stub

package.preload["ludi_core"] = function()
    return {
        start_server = function(port, dispatch, on_listen, on_reload)
            stub.started = {
                port = port,
                dispatch = dispatch,
                on_listen = on_listen,
                on_reload = on_reload
            }
            if on_listen then on_listen() end
        end
    }
end
