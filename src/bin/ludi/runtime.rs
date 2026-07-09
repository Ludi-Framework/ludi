//! Embedded-app mode: executes the bundle carried by this binary.

use std::process::ExitCode;

use ludi_core::bundle::Bundle;
use ludi_core::embedded;
use mlua::prelude::*;

pub fn run(bundle: Bundle) -> ExitCode {
    // unsafe_new: mlua's default state blocks the C loader ("safe mode"),
    // which would break native rocks installed on the host (fredy_core,
    // lsqlite3, ...). The bundle carries only Lua sources; native
    // dependencies keep resolving from package.cpath like under a plain
    // interpreter.
    let lua = unsafe { Lua::unsafe_new() };
    match execute(&lua, bundle) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("ludi: {err}");
            ExitCode::FAILURE
        }
    }
}

fn execute(lua: &Lua, bundle: Bundle) -> LuaResult<()> {
    let entry_source =
        bundle.files.get(&bundle.entry).cloned().ok_or_else(|| {
            LuaError::runtime(format!("bundle has no entrypoint {:?}", bundle.entry))
        })?;

    let package: LuaTable = lua.globals().get("package")?;

    // require("ludi_core") resolves to the Rust core compiled into this
    // binary. `bundled = true` tells the Lua side it runs from a bundle
    // (used to refuse LUDI_WATCH).
    let preload: LuaTable = package.get("preload")?;
    preload.set(
        "ludi_core",
        lua.create_function(|lua, ()| {
            let exports = ludi_core::exports(lua)?;
            exports.set("bundled", true)?;
            Ok(exports)
        })?,
    )?;

    // Searcher for everything else: the framework's embedded sources, then
    // the application files packed by `ludi build`. Sits right after the
    // preload searcher so bundled modules win over any host installation.
    let files = bundle.files;
    let searcher = lua.create_function(move |lua, name: String| {
        if let Some((_, source)) = embedded::LUA_MODULES.iter().find(|(m, _)| *m == name) {
            let loader = lua
                .load(*source)
                .set_name(format!("@ludi/{name}"))
                .into_function()?;
            return Ok(LuaValue::Function(loader));
        }

        let base = name.replace('.', "/");
        for candidate in [format!("{base}.lua"), format!("{base}/init.lua")] {
            if let Some(content) = files.get(&candidate) {
                let loader = lua
                    .load(content.as_slice())
                    .set_name(format!("@{candidate}"))
                    .into_function()?;
                return Ok(LuaValue::Function(loader));
            }
        }

        Ok(LuaValue::String(lua.create_string(format!(
            "\n\tno module '{name}' in the bundle"
        ))?))
    })?;
    let searchers: LuaTable = package.get("searchers")?;
    searchers.raw_insert(2, searcher)?;

    // Same `arg` shape as `lua entry.lua ...`.
    let arg: LuaTable = lua.create_table()?;
    arg.set(0, bundle.entry.as_str())?;
    for (i, value) in std::env::args().skip(1).enumerate() {
        arg.set(i + 1, value)?;
    }
    lua.globals().set("arg", arg)?;

    lua.load(entry_source.as_slice())
        .set_name(format!("@{}", bundle.entry))
        .exec()
}
