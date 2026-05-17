use crate::mutex::DFMutex;

#[cfg(debug_assertions)]
use crate::topology::{self, GroupId};

/// A set of locks that may be acquired in any order without deadlock.
///
/// Locks created from the same `LockGroup` are treated as peers in the sharing topology —
/// no ordering constraint is imposed between them. This is the correct abstraction for
/// problems like dining philosophers where independent resources must be shared freely.
///
/// # Example
///
/// ```rust
/// use dfmutex::LockGroup;
///
/// let group = LockGroup::new();
/// let fork_a = group.mutex(String::from("fork A"));
/// let fork_b = group.mutex(String::from("fork B"));
///
/// // Clients for fork_a and fork_b can be acquired in any order across threads.
/// let ca = fork_a.client();
/// let cb = fork_b.client();
/// let _ga = ca.lock().unwrap();
/// let _gb = cb.lock().unwrap();
/// ```
pub struct LockGroup {
    #[cfg(debug_assertions)]
    id: GroupId,
    #[cfg(not(debug_assertions))]
    _private: (),
}

impl LockGroup {
    /// Creates a new lock group.
    pub fn new() -> Self {
        LockGroup {
            #[cfg(debug_assertions)]
            id: topology::next_id(),
            #[cfg(not(debug_assertions))]
            _private: (),
        }
    }

    /// Creates a new [`DFMutex`] belonging to this group.
    pub fn mutex<T>(&self, value: T) -> DFMutex<T> {
        #[cfg(debug_assertions)]
        return DFMutex::new_in_group(value, self.id);
        #[cfg(not(debug_assertions))]
        return DFMutex::new_in_group(value, 0);
    }
}

impl Default for LockGroup {
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
