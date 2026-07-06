# ADR 0004: Async APIs bridge Lua coroutines to Rust futures

**Status:** accepted (design; implementation planned as phase 2)

## Context

A handler that blocks the VM — a synchronous SQL driver, `io.*`, luasocket
— stalls every request behind it, no matter how many workers exist. The
framework must offer I/O that suspends the handler instead of blocking the
VM. This API surface is the hardest part of the framework to change after
release, which is why its design precedes the worker model on the roadmap.

## Decision

Expose asynchronous Rust APIs behind **synchronous-looking Lua calls**:

```lua
app:get("/users", function(req, res)
    local rows = db:query("select * from users")  -- suspends, doesn't block
    res:json(rows)
end)
```

Under the hood:

```
handler coroutine ── yield ──▶ Rust future ──▶ tokio
        ▲                                        │
        └────────── resume(result) ◀── completed ┘
```

1. The stdlib function submits a future to the runtime and calls
   `coroutine.yield`.
2. The VM thread is free to run other request coroutines.
3. When the future completes, the runtime resumes the coroutine with the
   result (or raises the error at the yield point).

Errors surface as normal Lua errors at the call site — `pcall` works;
callbacks and promise objects are never part of the public API.
Cancellation follows ADR 0003.

Initial stdlib scope: HTTP client, SQL (PostgreSQL first), filesystem,
timers/sleep.

## Alternatives considered

- **Callbacks** — rejected: inverts control flow, nests badly, alien to
  the Express-style API.
- **Promise/future objects in Lua** — rejected: two-step ergonomics
  (`:await()`) for no gain; coroutines already give direct style.
- **Blocking Lua drivers + more workers** — rejected: workers multiply the
  ceiling but every blocked VM still wastes a core; latency collapses under
  slow upstreams.

## Consequences

- Handlers stay sequential-looking; concurrency is invisible.
- Requires the coroutine-per-request dispatcher (ADR 0003) and a runtime
  scheduler mapping suspended coroutines to in-flight futures.
- Third-party blocking libraries still block the VM; the docs must steer
  users to the ludi stdlib for I/O.
