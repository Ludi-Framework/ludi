# Ludi

An Express-style web framework for Lua, powered by Rust.

Routing, middlewares and handlers are plain Lua. Underneath, a native module
written in Rust ([hyper](https://hyper.rs) + [tokio](https://tokio.rs)) does
the HTTP heavy lifting. No OpenResty, no nginx, no separate runtime — just
`require("ludi")` and run your app with any Lua interpreter.

```lua
local ludi = require("ludi")

local app = ludi.new()

app:get("/users/:id", function(req, res)
    res:json({ id = req.params.id })
end)

app:listen(3000, function()
    print("Listening on http://localhost:3000")
end)
```

```bash
lua app.lua
# Listening on http://localhost:3000
```

## Installation

```bash
luarocks install ludi
```

Prebuilt binary rocks are published for Linux and macOS (Lua 5.4, 5.5 and LuaJIT),
so no Rust toolchain is needed. On platforms without a prebuilt rock,
LuaRocks falls back to building from source, which requires
[Rust](https://rustup.rs) installed.

For a per-project install (like `node_modules`):

```bash
luarocks install ludi --tree lua_modules
```

## Guide

### Routing

All standard HTTP methods are available: `get`, `post`, `put`, `delete`,
`patch`, `options`, `head`.

```lua
app:get("/hello", function(req, res)
    res:send("hi")
end)

app:post("/items", function(req, res)
    res:status(201):json({ ok = true })
end)
```

Path segments prefixed with `:` are captured as params:

```lua
app:get("/users/:id/posts/:post_id", function(req, res)
    res:json({ user = req.params.id, post = req.params.post_id })
end)
```

Unmatched requests get a `404` with a JSON body. Errors thrown inside a
handler are caught and answered with a `500` (the error is logged to stderr,
never leaked to the client).

### Middlewares

A middleware is a function `(req, res, next)`. The chain only advances when
`next()` is called — not calling it halts the request, express-style.

```lua
-- global: runs for every request
app:use(function(req, res, next)
    print(req.method .. " " .. req.path)
    next()
end)

-- per route: single function or a list, before the handler
local function auth(req, res, next)
    if req.headers["authorization"] then
        next()
    else
        res:status(401):json({ error = "Unauthorized" })
    end
end

app:get("/private", auth, function(req, res)
    res:send("secret")
end)

app:get("/very-private", { auth, audit }, function(req, res)
    res:send("very secret")
end)
```

Execution order: global middlewares → route middlewares → handler.

### Request

| Field / method | Description |
| --- | --- |
| `req.method` | HTTP method, uppercase (`"GET"`) |
| `req.path` | Path without query string (`"/users/42"`) |
| `req.params` | Path params captured by the router (`{ id = "42" }`) |
| `req.query` | Parsed query string (`?a=1` → `{ a = "1" }`) |
| `req.headers` | Request headers, lowercase keys (`req.headers["content-type"]`) |
| `req.body` | Raw body as a string (up to 1 MB; larger requests get `413`) |
| `req:json()` | Decodes the body as JSON; returns a table, or `nil` plus an error |

### Response

All methods are chainable.

| Method | Description |
| --- | --- |
| `res:status(code)` | Sets the status code (default `200`) |
| `res:header(name, value)` | Sets a response header |
| `res:send(data)` | String → `text/plain`; table → JSON with `application/json` |
| `res:json(data)` | Alias of `send` for tables |

Explicit headers win over the `Content-Type` that `send` sets by default.

### Listening

```lua
app:listen(3000)   -- blocks; defaults to 3000 when omitted

app:listen(3000, function()   -- callback runs once the port is bound,
    print("Ready!")           -- before any request is served
end)
```

Ludi prints nothing on startup — logging is the application's choice, in the
callback. A port that cannot be bound raises a Lua error.

### Hot reload (development)

```bash
LUDI_WATCH=1 lua app.lua
```

With `LUDI_WATCH=1`, Ludi watches every `*.lua` file under the working
directory and hot-reloads the application on change — no restart, no
rebinding the port, in-flight requests are never interrupted:

1. A change is detected (new, edited or deleted `.lua` file).
2. Application modules are dropped from `package.loaded`, so `require`
   re-reads them. The Lua stdlib and ludi itself are kept.
3. The entrypoint (`app.lua`) is re-executed; its `app:listen()` call is
   intercepted and the new app replaces the old one in place.

A reload that fails — syntax error, error at load time — is logged to
stderr and the previous version keeps serving until the next change.
State living in module locals is reset on reload, like restarting the
process; the `listen` callback does not run again.

Intended for development only: leave `LUDI_WATCH` unset in production
(the watcher polls the filesystem and reloads on any deploy write).

### Building a binary

Like Go, a Ludi application can ship as a single self-contained
executable — no Lua, no LuaRocks on the target machine:

```bash
ludi build                    # entry auto-detected when unambiguous
ludi build server.lua -o api  # or explicit entry and output name
./api
```

Without an explicit entry, `ludi build` accepts the conventional names
`app.lua`, `server.lua`, `main.lua` and `init.lua` — but only when
exactly one of them exists. With two or more present there is no safe
guess (an `app.lua` next to a `server.lua` is usually a module, not the
runner), so the build asks for the entrypoint instead of guessing wrong.

`ludi build` packs every `*.lua` file under the current directory (same
skip rules as the watcher) into a copy of the `ludi` runtime: a static
Lua 5.5, the framework and the application in one file. At run time
`require` resolves from the bundle; nothing is read from disk. The build
is instant — no toolchain involved.

The `ludi` CLI is attached to each GitHub release (`ludi-linux-x86_64`,
`ludi-darwin-arm64`), or build it from source with `make cli`.

Notes:

- A bundled binary always runs Lua 5.5, regardless of the interpreter
  used in development.
- Modules resolve from the bundle first, then from the host's normal
  LuaRocks trees — so native rocks (e.g. `fredy_core`) work, but must be
  installed on the target machine. Pure-Lua apps run anywhere.
- `LUDI_WATCH` is ignored inside a bundle (nothing on disk to watch).
- See [ADR 0005](docs/adr/0005-binary-build.md) for the design.

## Architecture

```
                    ┌────────────────────────────┐
 HTTP clients ───▶  │  Rust: hyper + tokio       │   ludi_core (native module)
                    │  worker threads            │
                    └────────────┬───────────────┘
                          mpsc channel (one Job per request)
                    ┌────────────▼───────────────┐
                    │  Lua thread (your app)     │   routing, middlewares,
                    │  single-threaded, Node-like│   handlers — plain Lua
                    └────────────────────────────┘
```

- Rust is transport only: it parses HTTP, enforces limits, and forwards each
  request through a channel. It knows nothing about routes.
- All Lua runs on the single thread that called `app:listen()`. Worker
  threads never touch the interpreter, so there are no cross-thread
  callbacks and no FFI.
- Each hyper worker awaits its response on a oneshot channel, so slow
  handlers don't block connection accept.

Design decisions and the planned evolution (async stdlib, worker model)
are documented in [docs/architecture.md](docs/architecture.md) and
[docs/adr/](docs/adr/).

Repository layout (same shape as [Lapis](https://github.com/leafo/lapis)):

```
ludi/          Lua package (init, router, request, response, middleware, json, reload)
src/           Rust native module (lib, server, bridge, types, watch)
spec/          busted specs
examples/      runnable examples
```

## Development

Requires Rust, a Lua (5.1+ or LuaJIT — 5.5 recommended) with headers, and
[busted](https://lunarmodules.github.io/busted/) for the Lua specs.

```bash
make dev              # build the native module for Lua 5.5 (recommended)
make dev LUA=lua54    # ... or for Lua 5.4
make dev LUA=luajit   # ... or for LuaJIT
make run              # build + run examples/hello.lua
make test             # cargo test + busted
```

Or via LuaRocks: `luarocks make` builds and installs the rockspec locally,
and `luarocks test` runs the busted suite. If `require("ludi")` isn't found
after installing, load the LuaRocks paths into your shell:
`eval "$(luarocks path)"`.

The Lua version is selected by a cargo feature, e.g.
`cargo build --release --features lua54`. When installing through
LuaRocks, `luarocks-build-rust-mlua` picks the right feature automatically.

## Releases

Tagging `v*` runs the release workflow: binary rocks are built for each
OS/Lua combination and attached to the GitHub release.

## License

[MIT](LICENSE)
