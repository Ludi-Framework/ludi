--- Dev-mode hot reload (`LUDI_WATCH=1 lua app.lua`).
---
--- The native core watches `*.lua` files under the working directory and
--- calls the handler built here between requests, on the Lua thread. The
--- handler clears application modules from `package.loaded` and re-runs the
--- entrypoint; the `app:listen()` call inside it is intercepted, so the new
--- app replaces the serving one without rebinding the port. A reload that
--- fails (syntax error, runtime error) keeps the previous version serving.

local reload = {}

-- Modules loaded before the application's own code: the Lua stdlib plus
-- whatever ludi itself pulled in. Snapshot taken when ludi is required,
-- which is before the app requires its own modules. Never cleared.
local baseline = {}
for name in pairs(package.loaded) do
    baseline[name] = true
end

local function is_protected(name)
    return baseline[name]
        or name == "ludi"
        or name == "ludi_core"
        or name:find("^ludi%.") ~= nil
end

--- Builds the `on_reload` callback handed to ludi_core.
---@param entrypoint string path of the script that started the server (arg[0])
---@param capture fun(interceptor: (fun(app: table))|nil) installs (or removes, with nil) the listen interceptor
---@param swap fun(app: table) replaces the app currently serving requests
---@return fun(changed: string[])
function reload.handler(entrypoint, capture, swap)
    return function(changed)
        io.stderr:write(("ludi: reloading (%s changed)\n"):format(
                            table.concat(changed, ", ")))

        for name in pairs(package.loaded) do
            if not is_protected(name) then
                package.loaded[name] = nil
            end
        end

        local new_app
        capture(function(app) new_app = app end)
        local ok, err = pcall(dofile, entrypoint)
        capture(nil)

        if not ok then
            io.stderr:write(("ludi: reload failed, keeping previous version: %s\n"):format(
                                tostring(err)))
        elseif new_app then
            swap(new_app)
        else
            io.stderr:write("ludi: reload never reached app:listen(); keeping previous version\n")
        end
    end
end

return reload
