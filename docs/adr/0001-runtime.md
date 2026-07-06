# ADR 0001: Native Lua module with a Rust runtime

**Status:** accepted

## Context

The framework targets Lua developers: install with LuaRocks, `require("ludi")`,
run with a plain Lua interpreter — the Express experience. The HTTP engine
is Rust (hyper + tokio). Something has to bridge the two languages, and the
choice constrains safety, distribution and which Lua implementations work.

## Decision

Ship Ludi as a **native Lua module written in Rust with mlua** (module
mode). The crate compiles to a `cdylib` exporting `luaopen_ludi_core`; Lua's
own `require` loads it through `package.cpath`, exactly like lua-cjson or
luasocket. Rust never owns the main VM — the user's interpreter does.

Distribution is LuaRocks with `luarocks-build-rust-mlua`, publishing
prebuilt binary rocks per platform/Lua so end users need no Rust toolchain.

## Alternatives considered

- **LuaJIT FFI (`ffi.load` on a cdylib)** — rejected. LuaJIT-only; callbacks
  created with `ffi.cast` are limited in number, leak unless freed, and are
  not safe to invoke from foreign threads (the original prototype crashed
  under load); string lifetimes across the boundary were managed by hand.
- **Rust binary embedding Lua** (`ludi run app.lua`) — rejected. Clean
  architecture, but breaks the package model: users would install a runtime
  instead of a library, and lose their own interpreter/tooling.

## Consequences

- Works on Lua 5.1–5.4 and LuaJIT; no `unsafe` at the boundary.
- The Lua version is selected at compile time (cargo features `lua54`,
  `luajit`, ...), so binary rocks are built per platform × Lua version.
- Because the user's interpreter owns the main VM, additional VMs (worker
  model) must be created by the module and re-execute the entrypoint —
  see ADR 0002.
