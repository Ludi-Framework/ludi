# ADR 0003: Every handler runs in a coroutine from v1

**Status:** accepted

## Context

The async standard library (ADR 0004) will let handlers suspend on I/O:
`db:query(...)` yields, the worker serves other requests, Rust resumes the
coroutine when the future resolves. That only works if handlers already run
inside coroutines. Retrofitting this later would either break running
applications or fork the API into sync/async variants.

## Decision

From v1, the dispatcher runs the middleware chain + handler inside a
`coroutine`, even though nothing yields yet:

```lua
local co = coroutine.create(run_request)
local ok, err = coroutine.resume(co, req, res)
```

The cost today is one coroutine allocation per request. The payoff is that
async APIs become a drop-in addition: existing handlers gain the ability to
yield without changing a line.

### Cancellation semantics

When a request dies while its coroutine is suspended (client disconnected,
timeout):

1. The pending Rust future is dropped (tokio futures are cancel-safe).
2. The coroutine is resumed **once** with a `"request cancelled"` error
   raised from the yield point — so user cleanup runs (`pcall` blocks,
   Lua 5.4 to-be-closed variables, transaction rollbacks).
3. Any async call made after cancellation fails immediately with the same
   error; the final result is discarded.
4. The coroutine is closed (`coroutine.close` on 5.4; dropped for GC on
   LuaJIT).

## Alternatives considered

- **Plain function call in v1, coroutines later** — rejected: async would
  be a breaking change or a parallel API.
- **Callback-style async** (`db:query(sql, function(rows) end)`) —
  rejected: pyramid of doom; coroutines are the idiomatic Lua answer.
- **Abandon suspended coroutines on cancel (never resume)** — rejected:
  leaks Lua-side resources (open transactions, locks); cleanup code never
  runs. Resume-with-error matches Go's context cancellation semantics.

## Consequences

- One coroutine per request (cheap: a coroutine is a small Lua thread
  object, collected by GC).
- Handler errors keep today's contract: caught, logged, client gets 500.
- The dispatcher owns the resume loop, so the future async bridge slots in
  behind `coroutine.yield` without touching user code.
