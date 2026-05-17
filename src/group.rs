use crate::mutex::DFMutex;

#[cfg(debug_assertions)]
use crate::topology;

/// A set of locks that may be acquired in any order without deadlock.
///
/// Normally, [`DFMutex`] locks must be acquired in a consistent global order — the cycle
/// detector enforces this in debug builds. `LockGroup` relaxes that constraint for a
/// specific set of locks: members of the same group are treated as peers, so no ordering
/// is imposed between them.
///
/// This is the correct abstraction for the dining philosophers problem and any situation
/// where a fixed pool of equal-status resources must be shared freely.
///
/// # Example — dining philosophers
///
/// ```rust
/// use dfmutex::{DFMutex, LockGroup};
/// use std::thread;
///
/// let group = LockGroup::new();
/// let forks: Vec<DFMutex<()>> = (0..5).map(|_| group.mutex(())).collect();
///
/// let handles: Vec<_> = (0..5)
///     .map(|i| {
///         let left  = forks[i].client();
///         let right = forks[(i + 1) % 5].client();
///         thread::spawn(move || {
///             let _l = left.lock().unwrap();
///             let _r = right.lock().unwrap();
///         })
///     })
///     .collect();
///
/// for h in handles { h.join().unwrap(); }
/// ```
pub struct LockGroup {
    id: usize,
}

impl LockGroup {
    /// Creates a new lock group.
    ///
    /// All locks created from this group via [`LockGroup::mutex`] are peers and may be
    /// acquired in any order. Locks from *different* groups are still subject to the
    /// global ordering constraint.
    ///
    /// # Example
    ///
    /// ```rust
    /// let group = dfmutex::LockGroup::new();
    /// let a = group.mutex(1u32);
    /// let b = group.mutex(2u32);
    /// // a and b can be locked in any order.
    /// ```
    pub fn new() -> Self {
        LockGroup {
            #[cfg(debug_assertions)]
            id: topology::next_id(),
            #[cfg(not(debug_assertions))]
            id: 0,
        }
    }

    /// Creates a new [`DFMutex`] belonging to this group.
    ///
    /// The returned lock is a peer of every other lock created from the same group:
    /// it can be acquired in any order relative to them without triggering the
    /// cycle detector.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dfmutex::LockGroup;
    ///
    /// let group = LockGroup::new();
    /// let lock_a = group.mutex(String::from("resource A"));
    /// let lock_b = group.mutex(String::from("resource B"));
    ///
    /// // Clients for both locks can be acquired in any order.
    /// let ca = lock_a.client();
    /// let cb = lock_b.client();
    /// let _ga = ca.lock().unwrap();
    /// let _gb = cb.lock().unwrap();
    /// ```
    pub fn mutex<T>(&self, value: T) -> DFMutex<T> {
        DFMutex::new_in_group(value, self.id)
    }
}

impl Default for LockGroup {
    /// Creates a new lock group, equivalent to [`LockGroup::new`].
    fn default() -> Self {
        LockGroup::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    const PHILOSOPHERS: usize = 5;
    const DINNER_ROUNDS: usize = 20;

    #[test]
    fn dining_philosophers() {
        for round in 0..DINNER_ROUNDS {
            let group = LockGroup::new();
            let forks: Vec<DFMutex<usize>> = (0..PHILOSOPHERS).map(|i| group.mutex(i)).collect();

            let mut handles = Vec::new();
            for i in 0..PHILOSOPHERS {
                let left = forks[i].client();
                let right = forks[(i + 1) % PHILOSOPHERS].client();

                handles.push(thread::spawn(move || {
                    thread::sleep(Duration::from_micros(100));
                    let _l = left.lock().unwrap();
                    let _r = right.lock().unwrap();
                    thread::sleep(Duration::from_micros(100));
                }));
            }

            for h in handles {
                h.join().unwrap_or_else(|_| panic!("philosopher panicked on round {}", round));
            }
        }
    }

    #[test]
    fn group_locks_any_acquisition_order() {
        // Two threads each acquire the same two group locks in opposite order.
        // Without LockGroup this would trigger the cycle detector; with it, no panic.
        let group = LockGroup::new();
        let a = group.mutex(0u64);
        let b = group.mutex(0u64);

        let ca1 = a.client();
        let cb1 = b.client();
        let ca2 = a.client();
        let cb2 = b.client();

        let h1 = thread::spawn(move || {
            let _ga = ca1.lock().unwrap();
            let _gb = cb1.lock().unwrap();
        });
        let h2 = thread::spawn(move || {
            let _gb = cb2.lock().unwrap();
            let _ga = ca2.lock().unwrap();
        });

        h1.join().unwrap();
        h2.join().unwrap();
    }

    #[test]
    fn group_shared_counter() {
        let group = LockGroup::new();
        let m = group.mutex(0u64);
        let handles: Vec<_> = (0..8).map(|_| {
            let c = m.client();
            thread::spawn(move || *c.lock().unwrap() += 1)
        }).collect();
        for h in handles { h.join().unwrap(); }
        assert_eq!(*m.lock().unwrap(), 8);
    }
}
