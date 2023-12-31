use std::sync::LockResult;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

///
/// Deadlock-free Mutex locks
///
/// DFMutex is a library that provides a ***guaranteed deadlock-free*** Mutex
/// implementation for the Rust language. Based on the research paper Higher-Order
/// Leak and Deadlock Free Locks by Jules Jacobs and Stephanie Balzer. 
/// 
/// Example
/// ```rust
/// use dfmutex::{DFMutex, spawn};
/// 
/// fn main() {
///     // Create a Mutex with any owned value
///     let m = DFMutex::new(String::from("Lorem Ipsum"));
/// 
///     // Create a closure to pass in the thread.
///     // The type of the created Mutex above should be same as the
///     // argument to the closure.
///     let closure = |mut dfm: DFMutex<String>| {
///         let data = dfm.lock().unwrap();
///         
///         // Use the data
///         println!("{}", data);
///     };
/// 
///     // Spawn 8 threads and store their handles
///     let mut handles = Vec::new();
///     for _ in 0..8 {
///         handles.push(spawn(&m, closure));    
///     }
/// 
///     // Join all the threads
///     for handle in handles.into_iter() {
///         handle.join().unwrap();
///     }
/// }
/// ```

/// A deadlock-free mutual exclusion primitive useful for protecting shared data
#[derive(Debug)]
pub struct DFMutex<T> {
    internal: Arc<Mutex<T>>,
}

impl<T> DFMutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    pub fn new(data: T) -> Self {
        DFMutex {
            internal: Arc::new(Mutex::new(data)),
        }
    }

    /// Acquires a mutex, blocking the current thread until it is able to do so.
    pub fn lock(&mut self) -> LockResult<MutexGuard<'_, T>> {
        self.internal.lock()
    }

    fn clone(&self) -> Self {
        DFMutex { internal: Arc::clone(&self.internal) }
    }
}

/// Spawns a new thread, returning a [`JoinHandle`] for it.
pub fn spawn<D, T, F>(odfm: &DFMutex<D>, f: F) -> JoinHandle<T>
where
    F: FnOnce(DFMutex<D>) -> T + Send + 'static,
    D: Send + 'static,
    T: Send + 'static,
{
    let codfm = odfm.clone();

    thread::spawn(move || f(codfm))
}

#[allow(dead_code)]
mod test_commons {
    pub const TEST_ITERATIONS: std::ops::Range<i32> = 0..10;
    pub const THREADS_RANGE: std::ops::Range<i32> = 0..8;

    const TASK_BASE: u64 = 40;

    fn fibonacci(n: u64) -> u64 {
        if n <= 1 {
            return n;
        }
        fibonacci(n - 1) + fibonacci(n - 2)
    }

    pub fn compute_intensive_task() -> u64 {
        fibonacci(TASK_BASE)
    } 
}

#[cfg(test)]
mod single_lock {
    use rand::Rng;
    use rand::thread_rng;
    use std::thread;
    use std::time::Duration;

    use super::DFMutex;
    use super::spawn;
    use super::test_commons::*;

    #[test]
    pub fn constant_time() {
        let m = DFMutex::new(String::from("Lorem Ipsum"));

        let closure = |mut dfm: DFMutex<String>| {
            let data = dfm.lock().unwrap();
            thread::sleep(Duration::new(1, 0));
            println!("{}", data);
        };

        let mut handles = Vec::new();

        for _ in THREADS_RANGE {
            handles.push(spawn(&m, closure));    
        }

        for handle in handles.into_iter() {
            handle.join().unwrap();
        }
    }

    #[test]
    pub fn random_time() {
        let m = DFMutex::new(String::from("Lorem Ipsum"));

        let closure = |mut dfm: DFMutex<String>| {
            let mut rng = thread_rng();
            let data = dfm.lock().unwrap();
            thread::sleep(Duration::new(rng.gen_range(1..3), 0));

            println!("{}", data);
        };

        let mut handles = Vec::new();

        for _ in THREADS_RANGE {
            handles.push(spawn(&m, closure));
        }

        for handle in handles.into_iter() {
            handle.join().unwrap();
        }
    }

    #[test]
    pub fn intensive_task() {
        let m = DFMutex::new(String::from("Lorem Ipsum"));

        let closure = |mut dfm: DFMutex<String>| {
            let data = dfm.lock().unwrap();

            let r = compute_intensive_task();

            println!("{} {}", data, r);
        };

        let mut handles = Vec::new();

        for _ in THREADS_RANGE {
            handles.push(spawn(&m, closure));    
        }

        for handle in handles.into_iter() {
            handle.join().unwrap();
        }
    }
}

#[cfg(test)]
mod lock_pair_straight_order {
    use std::ops::DerefMut;
    use rand::Rng;
    use rand::thread_rng;
    use std::thread;
    use std::time::Duration;

    use super::DFMutex;
    use super::spawn;
    use super::test_commons::*;


    #[test]
    pub fn constant_time() {
        for _ in TEST_ITERATIONS {
            let m1 = DFMutex::new(String::from("1"));
            let m2 = DFMutex::new(String::from("2"));
            let m = DFMutex::new((m1, m2));

            let closure = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                thread::sleep(Duration::new(1, 0));

                println!("{} {}", m1d, m2d);
            };

            let mut handles = Vec::new();

            for _ in THREADS_RANGE {
                handles.push(spawn(&m, closure));
            }

            for handle in handles.into_iter() {
                handle.join().unwrap();
            }
        }
    }

    #[test]
    pub fn random_time() {
        for _ in TEST_ITERATIONS {
            let m1 = DFMutex::new(String::from("1"));
            let m2 = DFMutex::new(String::from("2"));
            let m = DFMutex::new((m1, m2));

            let closure = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut rng = thread_rng();
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                thread::sleep(Duration::new(rng.gen_range(1..3), 0));

                println!("{} {}", m1d, m2d);
            };

            let mut handles = Vec::new();

            for _ in THREADS_RANGE {
                handles.push(spawn(&m, closure));    
            }

            for handle in handles.into_iter() {
                handle.join().unwrap();
            }        }
    }

    #[test]
    pub fn intensive_task() {
        for _ in TEST_ITERATIONS {
            let m1 = DFMutex::new(String::from("1"));
            let m2 = DFMutex::new(String::from("2"));
            let m = DFMutex::new((m1, m2));

            let closure = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                let r = compute_intensive_task();

                println!("{} {} {}", m1d, m2d, r);
            };

            let mut handles = Vec::new();

            for _ in THREADS_RANGE {
                handles.push(spawn(&m, closure));    
            }

            for handle in handles.into_iter() {
                handle.join().unwrap();
            }
        }
    }
}

#[cfg(test)]
mod lock_pair_swapped_order {
    use std::ops::DerefMut;
    use rand::Rng;
    use rand::thread_rng;
    use std::thread;
    use std::time::Duration;

    use super::DFMutex;
    use super::spawn;
    use super::test_commons::*;


    #[test]
    pub fn constant_time() {
        for _ in TEST_ITERATIONS {
            let m1 = DFMutex::new(String::from("1"));
            let m2 = DFMutex::new(String::from("2"));
            let m = DFMutex::new((m1, m2));

            let closure_a = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                thread::sleep(Duration::new(1, 0));

                println!("{} {}", m1d, m2d);
            };

            let closure_b = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m2d = m2.lock().unwrap();
                let m1d = m1.lock().unwrap();

                thread::sleep(Duration::new(1, 0));

                println!("{} {}", m2d, m1d);
            };

            let mut flag = true;
            let mut handles = Vec::new();

            for _ in THREADS_RANGE {
                if flag {
                    handles.push(spawn(&m, closure_a));    
                } else {
                    handles.push(spawn(&m, closure_b));
                }
                flag = !flag;
            }

            for handle in handles.into_iter() {
                handle.join().unwrap();
            }
        }
    }

    #[test]
    pub fn random_time() {
        for _ in TEST_ITERATIONS {
            let m1 = DFMutex::new(String::from("1"));
            let m2 = DFMutex::new(String::from("2"));
            let m = DFMutex::new((m1, m2));

            let closure_a = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut rng = thread_rng();
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                thread::sleep(Duration::new(rng.gen_range(1..3), 0));

                println!("{} {}", m1d, m2d);
            };

            let closure_b = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut rng = thread_rng();
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m2d = m2.lock().unwrap();
                let m1d = m1.lock().unwrap();

                thread::sleep(Duration::new(rng.gen_range(1..3), 0));

                println!("{} {}", m2d, m1d);
            };

            let mut flag = true;
            let mut handles = Vec::new();

            for _ in THREADS_RANGE {
                if flag {
                    handles.push(spawn(&m, closure_a));    
                } else {
                    handles.push(spawn(&m, closure_b));
                }
                flag = !flag;
            }

            for handle in handles.into_iter() {
                handle.join().unwrap();
            }
        }
    }

    #[test]
    pub fn intensive_task() {
        for _ in TEST_ITERATIONS {
            let m1 = DFMutex::new(String::from("1"));
            let m2 = DFMutex::new(String::from("2"));
            let m = DFMutex::new((m1, m2));

            let closure_a = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                let r = compute_intensive_task();

                println!("{} {} {}", m1d, m2d, r);
            };

            let closure_b = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m2d = m2.lock().unwrap();
                let m1d = m1.lock().unwrap();

                let r = compute_intensive_task();

                println!("{} {} {}", m2d, m1d, r);
            };

            let mut flag = true;
            let mut handles = Vec::new();

            for _ in THREADS_RANGE {
                if flag {
                    handles.push(spawn(&m, closure_a));    
                } else {
                    handles.push(spawn(&m, closure_b));
                }
                flag = !flag;
            }

            for handle in handles.into_iter() {
                handle.join().unwrap();
            }
        }
    }
}


#[cfg(test)]
mod dining_philisophers {
    use std::thread;
    use std::time::Duration;

    use super::DFMutex;

    const ITERATIONS: std::ops::Range<i32> = 0..500;
    const FORK_RANGE: std::ops::RangeInclusive<i32> = 1..=5;

    struct Philosopher {
        id: i32,
        left: DFMutex<String>,
        right: DFMutex<String>,
    }

    impl Philosopher {
        pub fn new(id: i32, left: DFMutex<String>, right: DFMutex<String>) -> Self {
            Self { id, left, right }
        }

        pub fn think(&self) {
            thread::sleep(Duration::new(0, 100000));
        }

        pub fn eat(&mut self) {
            let left_fork = self.left.lock().unwrap();
            println!("{} Acquired L -> {}", self.id, left_fork);
            let right_fork = self.right.lock().unwrap();
            println!("{} Acquired R -> {}", self.id, right_fork);

            thread::sleep(Duration::new(0, 100000));

            drop(left_fork);
            drop(right_fork);
        }
    }

    #[ignore = "Test is deadlock prone"]
    #[test]
    pub fn std() {
        for i in ITERATIONS {
            println!("===== Iteration {} =====", i);

            let mut forks = Vec::new();

            for i in FORK_RANGE {
                forks.push(DFMutex::new(format!("Fork {}", i)));
            }

            let mut philosophers: Vec<Philosopher> = Vec::new();

            philosophers.push(Philosopher::new(1, forks[0].clone(), forks[1].clone()));
            philosophers.push(Philosopher::new(2, forks[1].clone(), forks[2].clone()));
            philosophers.push(Philosopher::new(3, forks[2].clone(), forks[3].clone()));
            philosophers.push(Philosopher::new(4, forks[3].clone(), forks[4].clone()));
            philosophers.push(Philosopher::new(5, forks[4].clone(), forks[0].clone()));

            let mut handles = Vec::new();
            for _ in FORK_RANGE {
                let mut phil = philosophers.pop().unwrap();
                handles.push(thread::spawn(move || {
                    phil.think();
                    phil.eat();
                }));
            }

            for i in handles.into_iter() {
                i.join().unwrap();
            }
        }
    }
}