//! The `ludi` binary. Two personalities, decided by its own trailing bytes:
//!
//! - **CLI** (plain executable): subcommands live one per module below —
//!   `build` packs the current directory into a self-contained binary.
//! - **Runtime** (bundled executable): runs the embedded application. All
//!   `require`s resolve from the bundle; nothing is read from disk.

mod build;
mod runtime;

use std::process::ExitCode;

use ludi_core::bundle;

const USAGE: &str = "\
ludi - build self-contained binaries from Ludi applications

Usage:
  ludi build [entry.lua] [-o <output>]   pack the current directory into a binary
  ludi version                           print the version

The build embeds a static Lua 5.5, the framework and every *.lua file
under the current directory (hidden dirs, target/, lua_modules/ and
node_modules/ excluded). The output runs anywhere: no Lua, no LuaRocks.

Defaults: entry is app.lua, server.lua, main.lua or init.lua when exactly
one of them exists (ambiguity asks for an explicit entry); output is the
current directory's name.";

fn main() -> ExitCode {
    match bundle::read_own_bundle() {
        Ok(Some(bundle)) => runtime::run(bundle),
        Ok(None) => cli(),
        Err(err) => {
            eprintln!("ludi: cannot read own executable: {err}");
            ExitCode::FAILURE
        }
    }
}

fn cli() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("build") => build::run(&args[1..]),
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
