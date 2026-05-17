use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, LockResult, Mutex, MutexGuard, PoisonError};

#[cfg(debug_assertions)]
use crate::topology::{self, GroupId};

// Shared internal representation for both owner and client references.
struct LockInner<T> {
    mutex: Arc<Mutex<T>>,
    #[cfg(debug_assertions)]
    group_id: GroupId,
}

/// The owning reference to a deadlock-free mutex. Exactly one owner exists per lock.
///
/// Not [`Clone`] by design — ownership is unique. Use [`DFMutex::client`] or [`crate::spawn`]
/// to create shareable [`DFMutexClient`] references.
pub struct DFMutex<T>(LockInner<T>);

/// A shareable client reference to a [`DFMutex`]. Any number of clients may exist per lock.
///
/// [`Clone`]able — clients are freely shareable across threads.
#[derive(Clone)]
pub struct DFMutexClient<T>(LockInner<T>);

/// RAII guard returned by [`DFMutex::lock`] and [`DFMutexClient::lock`].
///
/// The lock is released when this guard is dropped.
pub struct DFMutexGuard<'a, T> {
    inner: MutexGuard<'a, T>,
    #[cfg(debug_assertions)]
    group_id: GroupId,
    #[cfg(debug_assertions)]
    mutex_ptr: usize,
}

// ── LockInner ────────────────────────────────────────────────────────────────

impl<T> LockInner<T> {
    fn new(value: T, #[cfg(debug_assertions)] group_id: GroupId) -> Self {
        LockInner {
            mutex: Arc::new(Mutex::new(value)),
            #[cfg(debug_assertions)]
            group_id,
        }
    }

    fn client(&self) -> LockInner<T> {
        LockInner {
            mutex: Arc::clone(&self.mutex),
            #[cfg(debug_assertions)]
            group_id: self.group_id,
        }
    }

    fn lock(&self) -> LockResult<DFMutexGuard<'_, T>> {
        #[cfg(debug_assertions)]
        let group_id = self.group_id;
        #[cfg(debug_assertions)]
        let mutex_ptr = Arc::as_ptr(&self.mutex) as usize;

        #[cfg(debug_assertions)]
        topology::check_and_register_acquire(group_id, mutex_ptr);

        match self.mutex.lock() {
            Ok(guard) => Ok(DFMutexGuard {
                inner: guard,
                #[cfg(debug_assertions)]
                group_id,
                #[cfg(debug_assertions)]
                mutex_ptr,
            }),
            Err(poison) => Err(PoisonError::new(DFMutexGuard {
                inner: poison.into_inner(),
                #[cfg(debug_assertions)]
                group_id,
                #[cfg(debug_assertions)]
                mutex_ptr,
            })),
        }
    }
}

impl<T> Clone for LockInner<T> {
    fn clone(&self) -> Self {
        self.client()
    }
}

// ── DFMutex ──────────────────────────────────────────────────────────────────

impl<T> DFMutex<T> {
    /// Creates a new lock in an unlocked state with the given value.
    pub fn new(value: T) -> Self {
        DFMutex(LockInner::new(
            value,
            #[cfg(debug_assertions)]
            topology::next_id(),
        ))
    }

    #[cfg(debug_assertions)]
    pub(crate) fn new_in_group(value: T, group_id: GroupId) -> Self {
        DFMutex(LockInner::new(value, group_id))
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn new_in_group(value: T, _group_id: usize) -> Self {
        DFMutex(LockInner::new(value))
    }

    /// Creates a [`DFMutexClient`] sharing the same underlying lock.
    pub fn client(&self) -> DFMutexClient<T> {
        DFMutexClient(self.0.client())
    }

    /// Acquires the lock, blocking until available. Returns a guard that releases on drop.
    pub fn lock(&self) -> LockResult<DFMutexGuard<'_, T>> {
        self.0.lock()
    }
}

impl<T: Default> Default for DFMutex<T> {
    fn default() -> Self {
        DFMutex::new(T::default())
    }
}

impl<T> From<T> for DFMutex<T> {
    fn from(value: T) -> Self {
        DFMutex::new(value)
    }
}

impl<T: fmt::Debug> fmt::Debug for DFMutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("DFMutex");
        #[cfg(debug_assertions)]
        s.field("group_id", &self.0.group_id);
        match self.0.mutex.try_lock() {
            Ok(guard) => s.field("data", &*guard),
            Err(_) => s.field("data", &"<locked>"),
        };
        s.finish()
    }
}

// ── DFMutexClient ─────────────────────────────────────────────────────────────

impl<T> DFMutexClient<T> {
    /// Acquires the lock, blocking until available. Returns a guard that releases on drop.
    pub fn lock(&self) -> LockResult<DFMutexGuard<'_, T>> {
        self.0.lock()
    }
}

impl<T: Default> Default for DFMutexClient<T> {
    fn default() -> Self {
        DFMutex::default().client()
    }
}

impl<T: fmt::Debug> fmt::Debug for DFMutexClient<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("DFMutexClient");
        #[cfg(debug_assertions)]
        s.field("group_id", &self.0.group_id);
        match self.0.mutex.try_lock() {
            Ok(guard) => s.field("data", &*guard),
            Err(_) => s.field("data", &"<locked>"),
        };
        s.finish()
    }
}

// ── DFMutexGuard ─────────────────────────────────────────────────────────────

impl<T> Deref for DFMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> DerefMut for DFMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: fmt::Debug> fmt::Debug for DFMutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for DFMutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T> Drop for DFMutexGuard<'_, T> {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        topology::register_release(self.group_id, self.mutex_ptr);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawn;
    use std::time::Duration;

    const THREADS: usize = 8;
    const STRESS_ITERS: usize = 50;

    #[test]
    fn single_lock_counter() {
        let m = DFMutex::new(0u64);
        let handles: Vec<_> = (0..THREADS).map(|_| spawn(&m, |c| *c.lock().unwrap() += 1)).collect();
        for h in handles { h.join().unwrap(); }
        assert_eq!(*m.lock().unwrap(), THREADS as u64);
    }

    #[test]
    fn single_lock_stress() {
        for _ in 0..STRESS_ITERS {
            let m = DFMutex::new(0u64);
            let handles: Vec<_> = (0..THREADS).map(|_| spawn(&m, |c| *c.lock().unwrap() += 1)).collect();
            for h in handles { h.join().unwrap(); }
            assert_eq!(*m.lock().unwrap(), THREADS as u64);
        }
    }

    #[test]
    fn nested_locks_both_counters_correct() {
        let m = DFMutex::new((DFMutex::new(0u64), DFMutex::new(0u64)));
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                spawn(&m, |outer| {
                    let g = outer.lock().unwrap();
                    *g.0.lock().unwrap() += 1;
                    *g.1.lock().unwrap() += 1;
                })
            })
            .collect();
        for h in handles { h.join().unwrap(); }
        let g = m.lock().unwrap();
        assert_eq!(*g.0.lock().unwrap(), THREADS as u64);
        assert_eq!(*g.1.lock().unwrap(), THREADS as u64);
    }

    #[test]
    fn client_clone_shares_lock() {
        let m = DFMutex::new(42u64);
        let c1 = m.client();
        let c2 = c1.clone();
        assert_eq!(*c1.lock().unwrap(), 42);
        assert_eq!(*c2.lock().unwrap(), 42);
        *m.lock().unwrap() = 99;
        assert_eq!(*c1.lock().unwrap(), 99);
        assert_eq!(*c2.lock().unwrap(), 99);
    }

    #[test]
    fn from_and_default() {
        let m: DFMutex<u64> = DFMutex::from(7);
        assert_eq!(*m.lock().unwrap(), 7);
        let d: DFMutex<u64> = DFMutex::default();
        assert_eq!(*d.lock().unwrap(), 0);
    }

    #[test]
    fn lock_with_random_delays() {
        use rand::{thread_rng, Rng};
        let m = DFMutex::new(0u64);
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                spawn(&m, |c| {
                    let delay = thread_rng().gen_range(0..10);
                    std::thread::sleep(Duration::from_millis(delay));
                    *c.lock().unwrap() += 1;
                })
            })
            .collect();
        for h in handles { h.join().unwrap(); }
        assert_eq!(*m.lock().unwrap(), THREADS as u64);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "potential deadlock detected")]
    fn cycle_detection_panics() {
        // Simulate two threads acquiring in opposite order on one thread.
        // After thread-1 acquires A then B, edge A→B is recorded globally.
        // When thread-2 holds B and tries A, the cycle A→B→A is detected.
        let a = DFMutex::new(());
        let b = DFMutex::new(());

        { let _ga = a.lock().unwrap(); let _gb = b.lock().unwrap(); }
        // Edge A→B is now in the global graph.

        let _gb2 = b.lock().unwrap();
        let _ga2 = a.lock().unwrap(); // panics: B→A creates cycle with A→B
    }

    // ── Reentrant detection ───────────────────────────────────────────────────
    //
    // std::sync::Mutex is not reentrant: locking the same underlying mutex twice
    // on the same thread silently deadlocks. The cycle detector's same-group filter
    // previously masked this case. These tests verify it is caught before blocking.

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "reentrant lock detected")]
    fn reentrant_via_cloned_client_panics() {
        // c1 and c2 share the same Arc<Mutex<T>> (and the same group_id).
        // Locking both on the same thread must panic before reaching mutex.lock().
        let m = DFMutex::new(0u64);
        let c1 = m.client();
        let c2 = c1.clone();
        let _g1 = c1.lock().unwrap();
        let _g2 = c2.lock().unwrap(); // must panic here, not deadlock
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "reentrant lock detected")]
    fn reentrant_via_owner_panics() {
        // The owner calling lock() twice on the same thread is equally unsafe.
        let m = DFMutex::new(0u64);
        let _g1 = m.lock().unwrap();
        let _g2 = m.lock().unwrap(); // must panic here, not deadlock
    }

    #[test]
    fn cloned_clients_on_separate_threads_are_not_reentrant() {
        // Two different threads each holding the same cloned client is normal
        // contention — they run sequentially, not concurrently, on the same mutex.
        // This must never trigger the reentrant detector.
        let m = DFMutex::new(0u64);
        let c1 = m.client();
        let c2 = c1.clone();
        let h1 = std::thread::spawn(move || *c1.lock().unwrap() += 1);
        let h2 = std::thread::spawn(move || *c2.lock().unwrap() += 1);
        h1.join().unwrap();
        h2.join().unwrap();
        assert_eq!(*m.lock().unwrap(), 2);
    }
}
