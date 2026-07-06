mod bridge;
mod server;
mod types;

use mlua::prelude::*;

#[mlua::lua_module]
fn ludi_core(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("start_server", lua.create_function(bridge::start_server)?)?;
    Ok(exports)
}
