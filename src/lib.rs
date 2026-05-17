//! # DFMutex — Deadlock-Free Mutex Locks
//!
//! Based on [Higher-Order Leak and Deadlock Free Locks](https://dl.acm.org/doi/abs/10.1145/3571229)
//! by Jules Jacobs and Stephanie Balzer.
//!
//! ## Core types
//!
//! | Type | Role |
//! |------|------|
//! | [`DFMutex<T>`] | Owning reference — exactly one per lock, not [`Clone`] |
//! | [`DFMutexClient<T>`] | Client reference — freely [`Clone`]able, shareable across threads |
//! | [`LockGroup`] | Set of peer locks acquirable in any order (solves dining philosophers) |
//!
//! ## Deadlock freedom
//!
//! Deadlock freedom rests on two invariants enforced by the type system and, in debug builds,
//! by a runtime cycle detector:
//!
//! 1. **Single owner** — [`DFMutex<T>`] is not [`Clone`]; only one owner exists per lock.
//! 2. **Acyclic acquisition order** — locks must always be acquired in a consistent order
//!    across threads. In `debug_assertions` builds, acquiring a lock that would create a
//!    cycle in the global lock-order graph causes an immediate `panic!` rather than a silent
//!    deadlock.
//!
//! Locks belonging to the same [`LockGroup`] are peers and may be acquired in any order.
//!
//! ## Quick start
//!
//! ```rust
//! use dfmutex::{DFMutex, spawn};
//!
//! let m = DFMutex::new(0u64);
//! let handles: Vec<_> = (0..8).map(|_| spawn(&m, |c| *c.lock().unwrap() += 1)).collect();
//! for h in handles { h.join().unwrap(); }
//! assert_eq!(*m.lock().unwrap(), 8);
//! ```

pub use group::LockGroup;
pub use mutex::{DFMutex, DFMutexClient, DFMutexGuard};
pub use spawn::spawn;

mod group;
mod mutex;
mod spawn;
#[cfg(debug_assertions)]
pub(crate) mod topology;
