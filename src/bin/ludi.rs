//! The `ludi` binary. Two personalities, decided by its own trailing bytes:
//!
//! - **CLI** (plain executable): `ludi build` packs the current directory's
//!   `*.lua` files into a copy of this same executable — a self-contained
//!   binary with the interpreter (static Lua 5.5), the framework and the
//!   application inside, Go-style.
//! - **Runtime** (bundled executable): runs the embedded application. All
//!   `require`s resolve from the bundle; nothing is read from disk.

use std::path::Path;
use std::process::ExitCode;

use ludi_core::bundle::{self, Bundle};
use ludi_core::embedded;
use mlua::prelude::*;

fn main() -> ExitCode {
    match bundle::read_own_bundle() {
        Ok(Some(bundle)) => run(bundle),
        Ok(None) => cli(),
        Err(err) => {
            eprintln!("ludi: cannot read own executable: {err}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------- runtime

fn run(bundle: Bundle) -> ExitCode {
    let lua = Lua::new();
    match execute(&lua, bundle) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("ludi: {err}");
            ExitCode::FAILURE
        }
    }
}

fn execute(lua: &Lua, bundle: Bundle) -> LuaResult<()> {
    let entry_source = bundle
        .files
        .get(&bundle.entry)
        .cloned()
        .ok_or_else(|| LuaError::runtime(format!("bundle has no entrypoint {:?}", bundle.entry)))?;

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

        Ok(LuaValue::String(
            lua.create_string(format!("\n\tno module '{name}' in the bundle"))?,
        ))
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

// -------------------------------------------------------------------- cli

const USAGE: &str = "\
ludi - build self-contained binaries from Ludi applications

Usage:
  ludi build [entry.lua] [-o <output>]   pack the current directory into a binary
  ludi version                           print the version

The build embeds a static Lua 5.5, the framework and every *.lua file
under the current directory (hidden dirs, target/, lua_modules/ and
node_modules/ excluded). The output runs anywhere: no Lua, no LuaRocks.

Defaults: entry is the first of app.lua, server.lua or main.lua that
exists; output is the current directory's name.";

fn cli() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("build") => build(&args[1..]),
        Some("version" | "--version" | "-V") => {
            println!("ludi {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("help" | "--help" | "-h") | None => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("ludi: unknown command {other:?}\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn build(args: &[String]) -> ExitCode {
    match try_build(args) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("ludi build: {err}");
            ExitCode::FAILURE
        }
    }
}

fn try_build(args: &[String]) -> Result<String, String> {
    let mut entry: Option<String> = None;
    let mut output: Option<String> = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                output = Some(
                    iter.next()
                        .ok_or_else(|| format!("{arg} expects a file name"))?
                        .clone(),
                );
            }
            flag if flag.starts_with('-') => return Err(format!("unknown flag {flag:?}")),
            positional if entry.is_none() => entry = Some(positional.to_string()),
            extra => return Err(format!("unexpected argument {extra:?}")),
        }
    }

    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let files = bundle::collect_files(&cwd).map_err(|e| e.to_string())?;

    let entry = match entry {
        Some(given) => {
            let normalized = given.strip_prefix("./").unwrap_or(&given).to_string();
            if !files.iter().any(|(path, _)| *path == normalized) {
                return Err(format!("entrypoint {given:?} not found under {}", cwd.display()));
            }
            normalized
        }
        None => ["app.lua", "server.lua", "main.lua"]
            .iter()
            .find(|name| files.iter().any(|(path, _)| path == *name))
            .map(|name| name.to_string())
            .ok_or("no app.lua, server.lua or main.lua here; pass the entrypoint")?,
    };

    let output = output.unwrap_or_else(|| {
        cwd.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "app".to_string())
    });

    let own_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let runtime = std::fs::read(&own_exe).map_err(|e| e.to_string())?;
    if bundle::extract_payload(&runtime).is_some() {
        return Err("this executable already contains an app; build with the plain ludi CLI".into());
    }

    let payload = bundle::encode(&entry, &files);
    let binary = bundle::assemble(&runtime, &payload);
    let size = binary.len();

    write_executable(Path::new(&output), &binary).map_err(|e| e.to_string())?;

    Ok(format!(
        "built {output} ({} files, entry {entry}, {:.1} MB)",
        files.len(),
        size as f64 / (1024.0 * 1024.0)
    ))
}

fn write_executable(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}
