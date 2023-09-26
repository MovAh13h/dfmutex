use std::sync::LockResult;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

#[derive(Debug)]
struct DFMutex<T> {
    internal: Arc<Mutex<T>>,
}

impl<T> DFMutex<T> {
    pub fn new(data: T) -> Self {
        DFMutex {
            internal: Arc::new(Mutex::new(data)),
        }
    }

    pub fn lock(&mut self) -> LockResult<MutexGuard<'_, T>> {
        self.internal.lock()
    }
}

impl<T> Clone for DFMutex<T> {
    fn clone(&self) -> Self {
        DFMutex { internal: Arc::clone(&self.internal) }
    }
}

fn spawn<D, T, F>(odfm: &DFMutex<D>, f: F) -> JoinHandle<T>
where
    F: FnOnce(DFMutex<D>) -> T + Send + 'static,
    D: Send + 'static,
    T: Send + 'static
{
    let codfm = odfm.clone();

    thread::spawn(move || {
        return f(codfm);
    })
}

mod test_commons {
    pub const TEST_ITERATIONS: std::ops::Range<i32> = 0..10;

    pub const TASK_BASE: u64 = 40;

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
            thread::sleep(Duration::new(2, 0));

            let data = dfm.lock().unwrap();

            println!("{}", data);
        };

        let a = spawn(&m, closure);
        let b = spawn(&m, closure);
        let c = spawn(&m, closure);
        let d = spawn(&m, closure);
        let e = spawn(&m, closure);
        let f = spawn(&m, closure);
        let g = spawn(&m, closure);
        let h = spawn(&m, closure);

        a.join().unwrap();
        b.join().unwrap();
        c.join().unwrap();
        d.join().unwrap();
        e.join().unwrap();
        f.join().unwrap();
        g.join().unwrap();
        h.join().unwrap();
    }

    #[test]
    pub fn random_time() {
        let m = DFMutex::new(String::from("Lorem Ipsum"));

        let closure = |mut dfm: DFMutex<String>| {
            let mut rng = thread_rng();
            thread::sleep(Duration::new(rng.gen_range(1..5), 0));

            let data = dfm.lock().unwrap();

            println!("{}", data);
        };

        let a = spawn(&m, closure);
        let b = spawn(&m, closure);
        let c = spawn(&m, closure);
        let d = spawn(&m, closure);
        let e = spawn(&m, closure);
        let f = spawn(&m, closure);
        let g = spawn(&m, closure);
        let h = spawn(&m, closure);

        a.join().unwrap();
        b.join().unwrap();
        c.join().unwrap();
        d.join().unwrap();
        e.join().unwrap();
        f.join().unwrap();
        g.join().unwrap();
        h.join().unwrap();
    }

    #[test]
    pub fn intensive_task() {
        let m = DFMutex::new(String::from("Lorem Ipsum"));

        let closure = |mut dfm: DFMutex<String>| {
            let r = compute_intensive_task();

            let data = dfm.lock().unwrap();

            println!("{} {}", data, r);
        };

        let a = spawn(&m, closure);
        let b = spawn(&m, closure);
        let c = spawn(&m, closure);
        let d = spawn(&m, closure);
        let e = spawn(&m, closure);
        let f = spawn(&m, closure);
        let g = spawn(&m, closure);
        let h = spawn(&m, closure);

        a.join().unwrap();
        b.join().unwrap();
        c.join().unwrap();
        d.join().unwrap();
        e.join().unwrap();
        f.join().unwrap();
        g.join().unwrap();
        h.join().unwrap();
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
                thread::sleep(Duration::new(2, 0));
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                println!("{} {}", m1d, m2d);
            };

            let a = spawn(&m, closure);
            let b = spawn(&m, closure);
            let c = spawn(&m, closure);
            let d = spawn(&m, closure);
            let e = spawn(&m, closure);
            let f = spawn(&m, closure);
            let g = spawn(&m, closure);
            let h = spawn(&m, closure);

            a.join().unwrap();
            b.join().unwrap();
            c.join().unwrap();
            d.join().unwrap();
            e.join().unwrap();
            f.join().unwrap();
            g.join().unwrap();
            h.join().unwrap();
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
                thread::sleep(Duration::new(rng.gen_range(1..5), 0));
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                println!("{} {}", m1d, m2d);
            };

            let a = spawn(&m, closure);
            let b = spawn(&m, closure);
            let c = spawn(&m, closure);
            let d = spawn(&m, closure);
            let e = spawn(&m, closure);
            let f = spawn(&m, closure);
            let g = spawn(&m, closure);
            let h = spawn(&m, closure);

            a.join().unwrap();
            b.join().unwrap();
            c.join().unwrap();
            d.join().unwrap();
            e.join().unwrap();
            f.join().unwrap();
            g.join().unwrap();
            h.join().unwrap();
        }
    }

    #[test]
    pub fn intensive_task() {
        for _ in TEST_ITERATIONS {
            let m1 = DFMutex::new(String::from("1"));
            let m2 = DFMutex::new(String::from("2"));
            let m = DFMutex::new((m1, m2));

            let closure = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let avg = compute_intensive_task();
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                println!("{} {} {}", m1d, m2d, avg);
            };

            let a = spawn(&m, closure);
            let b = spawn(&m, closure);
            let c = spawn(&m, closure);
            let d = spawn(&m, closure);
            let e = spawn(&m, closure);
            let f = spawn(&m, closure);
            let g = spawn(&m, closure);
            let h = spawn(&m, closure);

            a.join().unwrap();
            b.join().unwrap();
            c.join().unwrap();
            d.join().unwrap();
            e.join().unwrap();
            f.join().unwrap();
            g.join().unwrap();
            h.join().unwrap();
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
                thread::sleep(Duration::new(2, 0));
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                println!("{} {}", m1d, m2d);
            };

            let closure_b = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                thread::sleep(Duration::new(2, 0));
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m2d = m2.lock().unwrap();
                let m1d = m1.lock().unwrap();

                println!("{} {}", m2d, m1d);
            };

            let a = spawn(&m, closure_a);
            let b = spawn(&m, closure_b);
            let c = spawn(&m, closure_a);
            let d = spawn(&m, closure_b);
            let e = spawn(&m, closure_a);
            let f = spawn(&m, closure_b);
            let g = spawn(&m, closure_a);
            let h = spawn(&m, closure_b);

            a.join().unwrap();
            b.join().unwrap();
            c.join().unwrap();
            d.join().unwrap();
            e.join().unwrap();
            f.join().unwrap();
            g.join().unwrap();
            h.join().unwrap();
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
                thread::sleep(Duration::new(rng.gen_range(1..5), 0));
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                println!("{} {}", m1d, m2d);
            };

            let closure_b = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let mut rng = thread_rng();
                thread::sleep(Duration::new(rng.gen_range(1..5), 0));
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m2d = m2.lock().unwrap();
                let m1d = m1.lock().unwrap();

                println!("{} {}", m2d, m1d);
            };

            let a = spawn(&m, closure_a);
            let b = spawn(&m, closure_b);
            let c = spawn(&m, closure_a);
            let d = spawn(&m, closure_b);
            let e = spawn(&m, closure_a);
            let f = spawn(&m, closure_b);
            let g = spawn(&m, closure_a);
            let h = spawn(&m, closure_b);

            a.join().unwrap();
            b.join().unwrap();
            c.join().unwrap();
            d.join().unwrap();
            e.join().unwrap();
            f.join().unwrap();
            g.join().unwrap();
            h.join().unwrap();
        }
    }

    #[test]
    pub fn intensive_task() {
        for _ in TEST_ITERATIONS {
            let m1 = DFMutex::new(String::from("1"));
            let m2 = DFMutex::new(String::from("2"));
            let m = DFMutex::new((m1, m2));

            let closure_a = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let avg = compute_intensive_task();
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m1d = m1.lock().unwrap();
                let m2d = m2.lock().unwrap();

                println!("{} {} {}", m1d, m2d, avg);
            };

            let closure_b = |mut dfm: DFMutex<(DFMutex<String>, DFMutex<String>)>| {
                let avg = compute_intensive_task();
                let mut rng = thread_rng();
                thread::sleep(Duration::new(rng.gen_range(1..5), 0));
                let mut guard = dfm.lock().unwrap();
                let (m1, m2) = guard.deref_mut();

                let m2d = m2.lock().unwrap();
                let m1d = m1.lock().unwrap();

                println!("{} {} {}", m2d, m1d, avg);
            };

            let a = spawn(&m, closure_a);
            let b = spawn(&m, closure_b);
            let c = spawn(&m, closure_a);
            let d = spawn(&m, closure_b);
            let e = spawn(&m, closure_a);
            let f = spawn(&m, closure_b);
            let g = spawn(&m, closure_a);
            let h = spawn(&m, closure_b);

            a.join().unwrap();
            b.join().unwrap();
            c.join().unwrap();
            d.join().unwrap();
            e.join().unwrap();
            f.join().unwrap();
            g.join().unwrap();
            h.join().unwrap();
        }
    }
}














