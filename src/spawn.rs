use std::thread::{self, JoinHandle};

use crate::mutex::{DFMutex, DFMutexClient};

/// Spawns a new thread, passing a [`DFMutexClient`] of `dfm` to the closure.
///
/// This is the primary way to share a lock across threads while preserving the
/// deadlock-freedom invariant. The owner `dfm` stays in the calling thread; the
/// spawned thread receives its own client reference.
///
/// Returns the [`JoinHandle`] for the spawned thread.
///
/// # Panics
///
/// Panics if the OS fails to create the thread (same behaviour as
/// [`std::thread::spawn`]).
///
/// # Example
///
/// ```rust
/// use dfmutex::{DFMutex, spawn};
///
/// let m = DFMutex::new(0u64);
/// let handles: Vec<_> = (0..8).map(|_| spawn(&m, |c| *c.lock().unwrap() += 1)).collect();
/// for h in handles { h.join().unwrap(); }
/// assert_eq!(*m.lock().unwrap(), 8);
/// ```
pub fn spawn<D, T, F>(dfm: &DFMutex<D>, f: F) -> JoinHandle<T>
where
    F: FnOnce(DFMutexClient<D>) -> T + Send + 'static,
    D: Send + 'static,
    T: Send + 'static,
{
    let client = dfm.client();
    thread::spawn(move || f(client))
}
