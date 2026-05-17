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

```sh
cargo bench                              # release mode, all groups
cargo bench --bench throughput           # uncontended, contended, dining philosophers
cargo bench --bench topology_cost        # nested-depth and concurrent-nested overhead
cargo bench --profile bench-assertions   # same benchmarks with cycle detector active
```

## Acknowledgements

- Jules Jacobs (Radboud University, The Netherlands)
- Stephanie Balzer (Carnegie Mellon University, USA)

Paper: [Higher-Order Leak and Deadlock Free Locks](https://dl.acm.org/doi/abs/10.1145/3571229), POPL 2023.
