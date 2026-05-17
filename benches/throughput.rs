use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dfmutex::{spawn as dfspawn, DFMutex, LockGroup};
use parking_lot::Mutex as PLMutex;
use std::sync::{Arc, Mutex as StdMutex};

const THREAD_COUNTS: &[usize] = &[1, 2, 4, 8, 16];
const PHILOSOPHER_COUNTS: &[usize] = &[3, 5, 8];

// ── Group 1: Uncontended flat ─────────────────────────────────────────────────
//
// Single thread, single lock, repeated lock/unlock. Measures the raw per-cycle
// overhead of DFMutex vs std::Mutex vs parking_lot: Arc cost + thread-local
// HELD_PTRS check. The global GRAPH mutex is never touched here (other_held=[]).

fn uncontended(c: &mut Criterion) {
    let mut group = c.benchmark_group("uncontended");

    group.bench_function("std_mutex", |b| {
        let m = StdMutex::new(0u64);
        b.iter(|| *m.lock().unwrap() += 1);
    });

    group.bench_function("dfmutex", |b| {
        let m = DFMutex::new(0u64);
        b.iter(|| *m.lock().unwrap() += 1);
    });

    group.bench_function("parking_lot", |b| {
        let m = PLMutex::new(0u64);
        b.iter(|| *m.lock() += 1);
    });

    group.finish();
}

// ── Group 2: Contended scaling ────────────────────────────────────────────────
//
// N threads each increment a shared counter once per iteration. Reveals how
// DFMutex throughput scales (or doesn't) relative to std and parking_lot as
// contention increases.

fn contended(c: &mut Criterion) {
    let mut group = c.benchmark_group("contended");

    for &n in THREAD_COUNTS {
        group.throughput(Throughput::Elements(n as u64));

        group.bench_with_input(BenchmarkId::new("std_mutex", n), &n, |b, &n| {
            let m = Arc::new(StdMutex::new(0u64));
            b.iter(|| {
                let handles: Vec<_> = (0..n)
                    .map(|_| {
                        let m = Arc::clone(&m);
                        std::thread::spawn(move || *m.lock().unwrap() += 1)
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("dfmutex", n), &n, |b, &n| {
            let m = DFMutex::new(0u64);
            b.iter(|| {
                let handles: Vec<_> =
                    (0..n).map(|_| dfspawn(&m, |c| *c.lock().unwrap() += 1)).collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("parking_lot", n), &n, |b, &n| {
            let m = Arc::new(PLMutex::new(0u64));
            b.iter(|| {
                let handles: Vec<_> = (0..n)
                    .map(|_| {
                        let m = Arc::clone(&m);
                        std::thread::spawn(move || *m.lock() += 1)
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

// ── Group 4: Dining philosophers (LockGroup) ──────────────────────────────────
//
// N philosophers each acquire their left and right fork (group peers) in
// arbitrary order. Measures LockGroup throughput under varying levels of
// peer-lock contention. std::Mutex is omitted: solving this safely without
// a group abstraction requires external ordering that isn't the library's job.

fn dining_philosophers(c: &mut Criterion) {
    let mut group = c.benchmark_group("dining_philosophers");

    for &n in PHILOSOPHER_COUNTS {
        group.throughput(Throughput::Elements(n as u64));

        group.bench_with_input(BenchmarkId::new("lock_group", n), &n, |b, &n| {
            let g = LockGroup::new();
            let forks: Vec<DFMutex<usize>> = (0..n).map(|i| g.mutex(i)).collect();

            b.iter(|| {
                let handles: Vec<_> = (0..n)
                    .map(|i| {
                        let left = forks[i].client();
                        let right = forks[(i + 1) % n].client();
                        std::thread::spawn(move || {
                            let _l = left.lock().unwrap();
                            let _r = right.lock().unwrap();
                        })
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, uncontended, contended, dining_philosophers);
criterion_main!(benches);
