//! # DFMutex — Deadlock-Free Mutexes for Rust
//!
//! `dfmutex` provides mutexes that detect and prevent deadlocks, based on
//! [*Higher-Order Leak and Deadlock Free Locks*](https://dl.acm.org/doi/abs/10.1145/3571229)
//! by Jules Jacobs and Stephanie Balzer (POPL 2023).
//!
//! ## Core types
//!
//! | Type | Role |
//! |------|------|
//! | [`DFMutex<T>`] | Owning reference — exactly one per lock, not [`Clone`] |
//! | [`DFMutexClient<T>`] | Client reference — freely [`Clone`]able, send to other threads |
//! | [`DFMutexGuard<'_, T>`] | RAII guard returned by `lock()` — releases the lock on drop |
//! | [`LockGroup`] | Set of peer locks that may be acquired in any order |
//!
//! ## Quick start
//!
//! ```rust
//! use dfmutex::{DFMutex, spawn};
//!
//! let counter = DFMutex::new(0u64);
//! let handles: Vec<_> = (0..8)
//!     .map(|_| spawn(&counter, |c| *c.lock().unwrap() += 1))
//!     .collect();
//! for h in handles { h.join().unwrap(); }
//! assert_eq!(*counter.lock().unwrap(), 8);
//! ```
//!
//! ## Deadlock freedom
//!
//! Two invariants make deadlock impossible:
//!
//! 1. **Single owner** — [`DFMutex<T>`] is not [`Clone`]; exactly one owner exists per
//!    lock. Use [`DFMutex::client`] or [`spawn`] to share access across threads.
//!
//! 2. **Acyclic acquisition order** — locks must be acquired in a consistent global order.
//!    In `debug_assertions` builds, acquiring a lock that would form a cycle in the
//!    lock-order graph panics immediately rather than silently deadlocking:
//!
//! ```text
//! thread 'main' panicked at 'DFMutex: potential deadlock detected — acquiring lock
//! group 1 while holding [0] would create a cycle.'
//! ```
//!
//! Nested locks are fine as long as the order is consistent everywhere:
//!
//! ```rust
//! use dfmutex::{DFMutex, spawn};
//!
//! let outer = DFMutex::new(DFMutex::new(0u64));
//! spawn(&outer, |c| {
//!     let g = c.lock().unwrap(); // acquire outer
//!     *g.lock().unwrap() += 1;   // acquire inner — always outer-before-inner
//! }).join().unwrap();
//! ```
//!
//! ## Lock groups — peer locks in any order
//!
//! When locks are conceptually equal peers (e.g. forks in the dining philosophers
//! problem), a [`LockGroup`] lets them be acquired in any order without triggering
//! the cycle detector:
//!
//! ```rust
//! use dfmutex::LockGroup;
//!
//! let group = LockGroup::new();
//! let fork_a = group.mutex(());
//! let fork_b = group.mutex(());
//!
//! // Any acquisition order is safe within the same group.
//! let ca = fork_a.client();
//! let cb = fork_b.client();
//! let _ga = ca.lock().unwrap();
//! let _gb = cb.lock().unwrap();
//! ```
//!
//! ## Debug vs release
//!
//! The cycle detector and reentrant-lock detector only run under `debug_assertions`
//! (the default for `cargo test` and `cargo build`). In release builds the topology
//! code is stripped entirely — the only overhead relative to a plain `Arc<Mutex<T>>`
//! is the `Arc` itself.
//!
//! To benchmark with the detectors active, use the provided `bench-assertions` profile:
//!
//! ```sh
//! cargo bench --profile bench-assertions
//! ```

#![deny(missing_docs)]

pub use group::LockGroup;
pub use mutex::{DFMutex, DFMutexClient, DFMutexGuard};
pub use spawn::spawn;

mod group;
mod mutex;
mod spawn;
#[cfg(debug_assertions)]
pub(crate) mod topology;
