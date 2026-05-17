use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dfmutex::DFMutex;
use std::sync::Mutex;

// ── Group 3: Nested lock depth ────────────────────────────────────────────────
//
// Measures the marginal cost of each additional nesting level. In release mode
// this captures thread-local overhead (HELD_PTRS push/pop, borrow_mut). Run
// with `cargo bench --profile bench-assertions` to also capture the global
// GRAPH mutex cost that fires when other_held is non-empty.
//
// std::Mutex at depth=2 is included as a zero-topology baseline so the
// dfmutex overhead is clearly attributable to the topology machinery.

fn nested_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_depth");

    group.bench_function("std_mutex/depth=1", |b| {
        let m = Mutex::new(0u64);
        b.iter(|| *m.lock().unwrap() += 1);
    });

    group.bench_function("std_mutex/depth=2", |b| {
        let outer = Mutex::new(Mutex::new(0u64));
        b.iter(|| {
            let g = outer.lock().unwrap();
            *g.lock().unwrap() += 1;
        });
    });

    group.bench_function("dfmutex/depth=1", |b| {
        let m = DFMutex::new(0u64);
        b.iter(|| *m.lock().unwrap() += 1);
    });

    // Depth=2: acquiring the inner lock sees other_held=[outer_group].
    // In debug mode this acquires the global GRAPH mutex to check/add an edge.
    group.bench_function("dfmutex/depth=2", |b| {
        let outer = DFMutex::new(DFMutex::new(0u64));
        b.iter(|| {
            let g = outer.lock().unwrap();
            *g.lock().unwrap() += 1;
        });
    });

    // Depth=3: inner acquisition sees other_held=[outer, middle].
    // In debug mode: GRAPH mutex acquired, two edges checked/added.
    group.bench_function("dfmutex/depth=3", |b| {
        let m3 = DFMutex::new(DFMutex::new(DFMutex::new(0u64)));
        b.iter(|| {
            let g3 = m3.lock().unwrap();
            let g2 = g3.lock().unwrap();
            *g2.lock().unwrap() += 1;
        });
    });

    group.finish();
}

// ── Group 5: Cycle-detector overhead (best run under bench-assertions) ────────
//
// Isolates the cycle-detector cost under concurrent nested locking. Two threads
// each hold one lock and acquire a second — the ordering is consistent so no
// panic fires, but the GRAPH mutex is contested between threads.
//
// Run with:
//   cargo bench --profile bench-assertions --bench topology_cost
//
// Compare the numbers here against the release-mode run to see the raw cost
// of the topology machinery under concurrent nested access.

fn concurrent_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_nested");

    for &n in &[2usize, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("dfmutex_nested", n),
            &n,
            |b, &n| {
                // n locks in a fixed hierarchy: thread i acquires lock i then lock i+1.
                // Consistent ordering → never panics, but triggers n-1 GRAPH mutex
                // acquisitions per round.
                let locks: Vec<DFMutex<u64>> = (0..n + 1).map(|_| DFMutex::new(0u64)).collect();

                b.iter(|| {
                    let handles: Vec<_> = (0..n)
                        .map(|i| {
                            let lo = locks[i].client();
                            let hi = locks[i + 1].client();
                            std::thread::spawn(move || {
                                let _gl = lo.lock().unwrap();
                                let _gh = hi.lock().unwrap();
                            })
                        })
                        .collect();
                    for h in handles {
                        h.join().unwrap();
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("std_mutex_nested", n),
            &n,
            |b, &n| {
                use std::sync::Arc;
                let locks: Vec<Arc<Mutex<u64>>> =
                    (0..n + 1).map(|_| Arc::new(Mutex::new(0u64))).collect();

                b.iter(|| {
                    let handles: Vec<_> = (0..n)
                        .map(|i| {
                            let lo = Arc::clone(&locks[i]);
                            let hi = Arc::clone(&locks[i + 1]);
                            std::thread::spawn(move || {
                                let _gl = lo.lock().unwrap();
                                let _gh = hi.lock().unwrap();
                            })
                        })
                        .collect();
                    for h in handles {
                        h.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, nested_depth, concurrent_nested);
criterion_main!(benches);
