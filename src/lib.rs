mod bridge;
mod server;
mod types;
mod watch;

#[cfg(feature = "cli")]
pub mod bundle;
#[cfg(feature = "cli")]
pub mod embedded;

use mlua::prelude::*;

/// The table `require("ludi_core")` returns. Shared by the native module
/// entrypoint below and by the `ludi` CLI runtime, which preloads it into
/// its embedded interpreter.
pub fn exports(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("start_server", lua.create_function(bridge::start_server)?)?;
    Ok(exports)
}

#[cfg(feature = "module")]
#[mlua::lua_module]
fn ludi_core(lua: &Lua) -> LuaResult<LuaTable> {
    exports(lua)
}
