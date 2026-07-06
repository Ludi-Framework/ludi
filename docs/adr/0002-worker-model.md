# ADR 0002: Worker model — single VM now, one VM per worker later

**Status:** accepted (phase 1 implemented; phase 3 planned)

## Context

A `lua_State` is not thread-safe and cannot be cloned (closures, registry,
upvalues, userdata). Meanwhile hyper/tokio serve requests from a pool of
worker threads. Some execution model must reconcile "many transport
threads" with "single-threaded Lua".

A single Lua consumer has a throughput ceiling of `1/service_time`, where
service time includes the whole bridge (channel wakeup, table conversion,
pcall) — a few microseconds per request even for trivial handlers. The
ceiling is high (hundreds of thousands of req/s) but real.

## Decision

**Phase 1 (v1, current):** one Lua VM — the user's interpreter — plus an
mpsc channel. Hyper workers push a `Job` per request; the Lua thread
consumes serially. Simple, correct, no locks, ships early.

**Phase 3 (planned):** opt-in workers, hidden behind the same API:

```lua
app:listen(3000, { workers = "auto" })   -- N = CPU cores
```

Implementation: N threads, each with its **own hyper accept loop** on the
same port via `SO_REUSEPORT` (kernel balances connections) and its **own
Lua VM** created by the module, which re-executes the application
entrypoint — the Node cluster model. No dispatcher, no shared channel, no
locks anywhere on the hot path.

## Alternatives considered

- **Single global mpsc forever** — rejected as the end state: serial
  consumer ceiling; wastes hyper/tokio parallelism under load.
- **`Mutex<Lua>` called directly from any tokio thread** — rejected:
  serializes everything the channel serialized, plus lock contention.
- **One VM per tokio worker with direct dispatch** — not implementable:
  tokio tasks migrate between threads, so a request has no stable "its
  worker's VM" without pinning, which tokio does not support well.
- **Sharded channels (N Lua threads, N channels, round-robin)** — viable
  intermediate, but SO_REUSEPORT achieves the same isolation with zero
  cross-thread dispatch; not worth building both.

## Consequences

- v1 behavior is the documented default; workers are additive and opt-in.
- With workers, global Lua state is per-VM (see architecture.md — Node
  cluster semantics); top-level side effects run once per worker.
- The entrypoint must be re-executable: `app:listen()` in a worker VM
  registers routes instead of binding the port again.
