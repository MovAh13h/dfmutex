# DFMutex

[![Crates.io](https://img.shields.io/crates/v/dfmutex.svg)](https://crates.io/crates/dfmutex)
[![Docs.rs](https://docs.rs/dfmutex/badge.svg)](https://docs.rs/dfmutex)

Deadlock-free mutexes for Rust.

`dfmutex` guarantees deadlock freedom through two invariants enforced by the type system
and, in debug builds, a runtime cycle detector:

1. **Single owner** — `DFMutex<T>` is not `Clone`; exactly one owner exists per lock.
2. **Acyclic acquisition order** — locks must always be acquired in a consistent global order.
   In debug builds the cycle detector panics immediately instead of silently deadlocking.

Based on [*Higher-Order Leak and Deadlock Free Locks*](https://dl.acm.org/doi/abs/10.1145/3571229)
by Jules Jacobs and Stephanie Balzer (POPL 2023).

## Installation

```toml
[dependencies]
dfmutex = "0.2"
```

## Quick start

```rust
use dfmutex::{DFMutex, spawn};

let counter = DFMutex::new(0u64);
let handles: Vec<_> = (0..8)
    .map(|_| spawn(&counter, |c| *c.lock().unwrap() += 1))
    .collect();
for h in handles { h.join().unwrap(); }
assert_eq!(*counter.lock().unwrap(), 8);
```

## Core types

| Type | Role |
|------|------|
| `DFMutex<T>` | Owning reference — exactly one per lock, not `Clone` |
| `DFMutexClient<T>` | Client reference — freely `Clone`able, sendable to other threads |
| `DFMutexGuard<'_, T>` | RAII guard — derefs to `T`, releases the lock on drop |
| `LockGroup` | Set of peer locks acquirable in any order |

## How it works

### Single owner

`DFMutex<T>` is not `Clone`. To share access, call `.client()` to get a `DFMutexClient<T>`,
or use `spawn(&lock, |client| ...)` which hands a client to the new thread automatically:

```rust
use dfmutex::{DFMutex, spawn};

let m = DFMutex::new(String::from("hello"));
spawn(&m, |c| println!("{}", c.lock().unwrap())).join().unwrap();
```

### Acyclic lock-order graph

The classic deadlock is two threads each holding one lock and waiting for the other.
`dfmutex` prevents this by requiring a consistent global acquisition order. In debug
builds, a runtime cycle detector tracks every ordering in a global DAG. Any acquire
that would form a cycle panics immediately:

```text
thread 'main' panicked at 'DFMutex: potential deadlock detected — acquiring lock group 1
while holding [0] would create a cycle. Ensure lock acquisition order is consistent,
or use LockGroup for peer locks.'
```

Nested locks are fine as long as the order is consistent everywhere:

```rust
use dfmutex::{DFMutex, spawn};

let outer = DFMutex::new(DFMutex::new(0u64));
spawn(&outer, |c| {
    let g = c.lock().unwrap(); // always acquire outer first
    *g.lock().unwrap() += 1;   // then inner
}).join().unwrap();
```

### Lock groups — peer locks in any order

When locks are equal peers (e.g. forks in the dining philosophers problem), a `LockGroup`
exempts them from the ordering constraint:

```rust
use dfmutex::{DFMutex, LockGroup};
use std::thread;

let group = LockGroup::new();
let forks: Vec<DFMutex<()>> = (0..5).map(|_| group.mutex(())).collect();

let handles: Vec<_> = (0..5)
    .map(|i| {
        let left  = forks[i].client();
        let right = forks[(i + 1) % 5].client();
        thread::spawn(move || {
            let _l = left.lock().unwrap();  // any order is fine within a group
            let _r = right.lock().unwrap();
        })
    })
    .collect();

for h in handles { h.join().unwrap(); }
```

## Debug vs release

| Mode | Cycle detector | Reentrant detector | Overhead vs `Arc<Mutex<T>>` |
|------|---------------|-------------------|-----------------------------|
| `cargo build` / `cargo test` | ✅ active | ✅ active | thread-local ops + global graph lock on nested acquire |
| `cargo build --release` / `cargo bench` | ❌ stripped | ❌ stripped | `Arc` only |

The topology machinery is entirely behind `#[cfg(debug_assertions)]`. Release builds are
as lean as a hand-rolled `Arc<Mutex<T>>`.

## Benchmarks

Two benchmark suites are included, each runnable in release mode (cycle detector off) or
under the `bench-assertions` profile (cycle detector on) to isolate overhead.

### Commands

```sh
# Run all benchmarks in release mode
cargo bench

# Uncontended single-lock, contended N-thread scaling, dining philosophers (LockGroup)
# Compares DFMutex vs std::Mutex vs parking_lot across 1–16 threads
cargo bench --bench throughput

# Single-thread nested lock depth (depth 1/2/3) and concurrent nested locking
# Release mode: measures thread-local HELD tracking overhead only
cargo bench --bench topology_cost

# Re-run either suite with the cycle detector fully active
# Use this to see the true cost of debug builds
cargo bench --bench throughput   --profile bench-assertions
cargo bench --bench topology_cost --profile bench-assertions
```

### Results (Apple M-series, release mode)

#### Uncontended — single thread, repeated lock/unlock

| | Time | vs `std::Mutex` |
|---|---|---|
| `std::Mutex` | 8.68 ns | baseline |
| `DFMutex` | **8.65 ns** | ≈ 0% |
| `parking_lot` | 8.50 ns | −2% |

DFMutex in release mode is statistically identical to `std::Mutex`. The topology fields
are stripped by `#[cfg(debug_assertions)]`; the only remaining structure is `Arc<Mutex<T>>`.

#### Contended — N threads competing for one lock

| Threads | `std::Mutex` | `DFMutex` | `parking_lot` |
|---------|------------|---------|-------------|
| 1 | 19.9 µs | 19.9 µs | 19.8 µs |
| 2 | 42.7 µs | 41.6 µs | 41.0 µs |
| 4 | 67.5 µs | 68.0 µs | 66.5 µs |
| 8 | 116.6 µs | 117.2 µs | 116.1 µs |
| 16 | 227.8 µs | 226.9 µs | 231.2 µs |

All three are within measurement noise at every thread count. The bottleneck is OS
thread scheduling, not the mutex implementation.

#### Nested lock depth — release vs debug

| Scenario | Release | Debug | Added cost |
|---|---|---|---|
| `std::Mutex` depth=1 | 8.7 ns | 8.7 ns | — |
| `std::Mutex` depth=2 | 13.4 ns | 13.5 ns | — |
| `DFMutex` depth=1 | 8.7 ns | **30.2 ns** | +21.5 ns |
| `DFMutex` depth=2 | 13.4 ns | **174 ns** | +161 ns |
| `DFMutex` depth=3 | 18.3 ns | **392 ns** | +374 ns |

In debug builds:
- **Flat lock (+21.5 ns)** — reentrant check (`HELD_PTRS.contains()`) and thread-local
  HELD push/pop. No global lock involved.
- **First nesting level (+161 ns)** — the global `GRAPH` mutex is acquired here for the
  cycle check and edge insertion.
- **Each additional nesting level (+~215 ns)** — another round-trip through the global
  `GRAPH` mutex.

#### Concurrent nested locking — debug overhead

| Threads | `std::Mutex` | `DFMutex` release | `DFMutex` debug | Overhead |
|---------|------------|-----------------|---------------|---------|
| 2 | 39.6 µs | 39.8 µs | 43.5 µs | +9.7% |
| 4 | 67.2 µs | 67.2 µs | 73.7 µs | +9.7% |
| 8 | 118.1 µs | 117.7 µs | 126.8 µs | +7.8% |

The global `GRAPH` mutex adds roughly **8–10% overhead** to concurrent nested locking in
debug builds, as multiple threads contend on the cycle-check lock simultaneously.

#### Summary

| Mode | Scenario | Cost |
|---|---|---|
| Release | Any | Identical to `std::Mutex` — zero overhead |
| Debug | Flat lock (depth=1) | +21 ns (thread-local only) |
| Debug | Nested (depth≥2) | +~150–220 ns per level (global GRAPH mutex) |
| Debug | Concurrent nested | ~8–10% slower than `std::Mutex` |

## Acknowledgements

- Jules Jacobs (Radboud University, The Netherlands)
- Stephanie Balzer (Carnegie Mellon University, USA)

Paper: [Higher-Order Leak and Deadlock Free Locks](https://dl.acm.org/doi/abs/10.1145/3571229), POPL 2023.
