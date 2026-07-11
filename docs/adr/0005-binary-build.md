# ADR 0005: Single-binary builds via a self-appending runtime

**Status:** accepted

## Context

Deploying a Ludi application means installing a Lua interpreter, LuaRocks
and the ludi rock on the target machine. Go set the expectation that a
service can ship as one copyable file. The dev-time model (ADR 0001:
plain interpreter + native module) must not change — the binary is a
deploy artifact, not a new way to develop.

## Decision

Ship a `ludi` CLI whose binary doubles as the application runtime.

- `ludi build` copies the CLI's own executable and appends the project's
  `*.lua` files as a length-prefixed payload, closed by an 8-byte magic
  trailer (`LUDIPKG1`) — the bun/deno-compile technique. Build time is
  file I/O; no compiler runs.
- On startup the binary inspects its own trailer: payload present means
  "run the embedded app", absent means "act as the CLI".
- The runtime embeds a **static, vendored Lua 5.5**, the Rust core and
  the framework's Lua sources (`include_str!`). A custom `package`
  searcher resolves `require` from the bundle, so the filesystem is
  never consulted for modules.
- Cargo grows a `cli` feature (`mlua/vendored`); the `module` feature
  keeps building the cdylib rock. They are mutually exclusive per build
  invocation because mlua cannot be a module and vendor an interpreter
  at once.

## Alternatives considered

- **Generate a Rust project and `cargo build` per app** — rejected:
  requires the Rust toolchain on every developer machine and minutes of
  compile time; Ludi's promise is instant.
- **luastatic / C-toolchain bundlers** — rejected: needs a C compiler
  and per-platform fiddling; no story for the Rust core.
- **Container images as the only deploy story** — rejected as exclusive:
  containers remain possible, but a single file is strictly simpler for
  the common VPS case.

## Consequences

- The binary is tens of MB (interpreter + tokio + hyper); acceptable for
  a server artifact.
- A bundle always runs Lua 5.5 regardless of the development
  interpreter, so 5.1–5.4/LuaJIT-only code may need adjusting before
  bundling.
- Hot reload is meaningless inside a bundle; `LUDI_WATCH` is ignored
  with a warning (`ludi_core.bundled` flag).
- Module resolution order at run time: bundle first, then the host's
  normal `package.path`/`package.cpath`. Native rocks (`fredy_core`,
  `lsqlite3`, ...) are never packed — they load from the host, which
  requires the Lua state to allow C loaders (`Lua::unsafe_new`) and the
  binary to export the static Lua API to `dlopen`ed libraries
  (`-rdynamic` on Linux; macOS exports by default). Windows executables do
  not re-export their statically linked symbols, so a bundle that pulls in
  a native rock cannot resolve the Lua API there — pure-Lua bundles are
  unaffected and run anywhere.
- Therefore "runs anywhere" holds for pure-Lua applications; an app
  using native rocks needs those rocks (built for Lua 5.5) installed on
  the target machine. Embedding first-party native modules (fredy) into
  the runtime via cargo features is the planned next step.
