# Architecture

Ludi is a web framework where application code is Lua and the runtime is
Rust. This document records the execution model and the planned evolution.
Individual decisions, with their alternatives and trade-offs, live in
[docs/adr/](adr/).

## Execution model

**v1 (current): one Lua VM per process.**

```
                 hyper + tokio (worker threads)
                          │
                   mpsc channel (one Job per request)
                          │
                 Lua thread — the interpreter that ran `lua app.lua`
                 routing, middlewares, handlers
```

- The user starts the process (`lua app.lua`); the framework is a native
  module loaded with `require`. The Lua VM is owned by the interpreter,
  not by Ludi.
- Rust is transport only: HTTP parsing, limits, socket I/O. It knows
  nothing about routes.
- Every request crosses the boundary once as a plain table
  (`method, path, query, headers, body`) and returns as a plain table
  (`status, headers, body`).

**Planned: one Lua VM per worker (see [ADR 0002](adr/0002-worker-model.md)).**

```
   Thread 1          Thread 2          Thread N
   hyper accept      hyper accept      hyper accept
   loop (SO_REUSEPORT, kernel balances connections)
      │                 │                 │
   Lua VM 1          Lua VM 2          Lua VM N
```

Each worker thread owns an independent hyper server and an independent
`lua_State` that re-executed the application entrypoint — the Node cluster
model. No dispatcher, no shared channel, no locks.

## Concurrency model

- **Lua code is always single-threaded.** A handler never runs in parallel
  with another handler on the same VM. There are no locks and no data races
  in application code.
- Concurrency comes from the runtime: socket I/O is parallel in Rust, and
  (planned) blocking operations are asynchronous via coroutines — the
  handler yields, the worker serves other requests, Rust resumes the
  coroutine when the future completes ([ADR 0004](adr/0004-async-ffi.md)).
- Every handler runs inside a coroutine from v1, even though nothing
  yields yet ([ADR 0003](adr/0003-coroutines.md)). This makes the future
  async API a non-breaking change.

## State

- Global Lua state (module-level tables, counters, caches) is shared
  within one VM only.
- With multiple workers each VM re-runs `app.lua` and owns its own state:
  a `local counter = 0` becomes N independent counters. Same semantics as
  Node cluster. Cross-worker state belongs in external storage (database,
  cache server).

## Request lifecycle

1. hyper worker reads the request, enforces the body size limit.
2. Request crosses to the Lua side; the router matches method + path
   segments, extracting `:params`.
3. Global middlewares, route middlewares, handler — run inside one
   coroutine, chained by explicit `next()` calls.
4. Handler errors are caught (`pcall`): the client gets a `500`, the error
   is logged to stderr, never leaked.
5. The response table crosses back; hyper writes it out.

Cancellation (client disconnected, timeout): the pending Rust future is
dropped and the coroutine is resumed once with a `"request cancelled"`
error raised from the yield point, so user cleanup (`pcall`,
to-be-closed variables) runs. See [ADR 0003](adr/0003-coroutines.md).

## Planned evolution (in order)

1. ✅ Single VM, stable public API.
2. Async standard library (HTTP client, SQL, filesystem) on the
   coroutine bridge — the API surface is the hard-to-change part, so it
   comes first.
3. Multiple workers: SO_REUSEPORT, one hyper + one VM per worker.
4. Memory optimizations: fewer copies on the request/response path.

## Non-goals

- Exposing Rust, tokio or mlua concepts to application code.
- Multi-threaded Lua. Parallelism is per-VM, never intra-VM.
- Being a batteries-included MVC framework. Ludi is the HTTP layer.
