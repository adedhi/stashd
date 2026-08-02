# stashd

A Redis-inspired key-value store server written in Rust, from scratch. Speaks the real RESP (REdis Serialization Protocol) wire format, so standard Redis clients — including `redis-cli` — can connect to it and run commands directly.

## Features

- **Core commands**: `SET`, `GET`, `DEL`
- **Expiration**: `EXPIRE`, `TTL`, `PERSIST`
- **RESP protocol support**: parses and encodes the same wire format real Redis uses, verified against actual `redis-cli`
- **Concurrent clients**: one thread per connection, with shared state protected by a mutex-guarded hash map
- **Durability**: every write command is appended to a write-ahead log (`stashd.log`); on startup, the log is replayed to restore state
- **Log compaction**: the log is periodically rewritten down to the minimum commands needed to reconstruct current state, so it doesn't grow unbounded
- **Graceful shutdown**: a final compaction runs automatically on Ctrl+C, so no uncompacted writes are left behind when the server stops

## Running it

```bash
cargo run
```

The server listens on `0.0.0.0:7878`. Connect with `redis-cli`:

```bash
redis-cli -p 7878
> SET foo bar
OK
> GET foo
"bar"
> EXPIRE foo 60
OK
> TTL foo
(integer) 59
> PERSIST foo
(integer) 1
> DEL foo
OK
```

## Architecture

The codebase is split into focused modules, each owning one concern:

| File | Responsibility |
|---|---|
| `main.rs` | Wires everything together: creates the store, opens/replays the log, spawns the background compaction thread, registers the shutdown handler, starts the server |
| `store.rs` | The data model — `Entry` (value + optional expiry) and `Store`, a thread-safe shared hash map (`Arc<Mutex<HashMap<String, Entry>>>`) |
| `resp.rs` | The RESP protocol layer — parses incoming commands off the wire and encodes outgoing replies. Knows nothing about what any command *does* |
| `network.rs` | The command dispatcher — accepts connections, spawns a thread per client, matches parsed commands against handlers, mutates the store, and writes replies. Knows nothing about wire-level byte framing |
| `persistence.rs` | The write-ahead log — appending commands, replaying them on startup, and compacting the log down to current state |

This separation means the protocol layer (`resp.rs`), the command logic (`network.rs`), and the durability layer (`persistence.rs`) can each be reasoned about independently.

### Concurrency model

Each client connection runs on its own OS thread (`thread::spawn` per accepted connection). All threads share one `Store`, wrapped in `Arc` (so multiple threads can hold a reference to it) and `Mutex` (so only one thread can read or write it at a time). The write-ahead log file is protected the same way, so concurrent writers can't interleave log lines.

### Durability model

Every mutating command (`SET`, `DEL`, `EXPIRE`, `PERSIST`) is appended to `stashd.log` as it's applied to the in-memory store. On startup, that log is replayed line by line to rebuild the store's state before the server starts accepting connections — so a restart doesn't lose data.

Left alone, an append-only log grows forever, even though the underlying dataset may be small and stable. A background thread compacts the log every 5 minutes: it snapshots the current store into a minimal set of `SET`/`EXPIRE` lines, writes them to a temp file, and atomically renames that file over the real log — so a crash mid-compaction can never leave a corrupted, unreplayable log. The same compaction also runs once on shutdown (Ctrl+C), so recent writes aren't left sitting uncompacted between scheduled runs.

## Testing

Integration tests (`integration.rs`) spin up a real server on a test port and connect to it as a real TCP client:

- `handles_concurrent_clients` — fires `SET` commands from 5 threads simultaneously and verifies each gets a clean response
- `expired_key_is_not_found` — sets a key with a short expiry, confirms it's readable immediately, then confirms it's gone after the expiry passes

Run with:

```bash
cargo test
```

## Design notes

- **Expiration is lazy**, not swept by a background timer: an expired key is only removed the moment it's next accessed (`GET`, `TTL`), matching one of the strategies real Redis uses.
- **Expiry timestamps are stored absolute**, not as a relative TTL — so replaying or compacting the log doesn't shift when a key actually expires, regardless of when that replay happens.
- **Log compaction uses write-to-temp-then-rename** rather than writing to the log file directly, so the log is never left in a partially-written state if the process dies mid-compaction.