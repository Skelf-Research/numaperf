//! Criterion benchmarks for sharded data structures.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use numaperf::{ShardedCounter, Topology};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn sharded_counter_benchmarks(c: &mut Criterion) {
    let topo = Arc::new(Topology::discover().expect("topology discovery"));

    let mut group = c.benchmark_group("sharded_counter");
    group.throughput(Throughput::Elements(1));

    // Single-threaded increment
    group.bench_function("increment_single", |b| {
        let counter = ShardedCounter::new(&topo);
        b.iter(|| {
            counter.increment();
        });
    });

    // Sum operation (read all shards)
    group.bench_function("sum", |b| {
        let counter = ShardedCounter::new(&topo);
        for _ in 0..1000 {
            counter.increment();
        }
        b.iter(|| {
            black_box(counter.sum());
        });
    });

    // Compare with global atomic baseline
    group.bench_function("global_atomic_baseline", |b| {
        let counter = AtomicU64::new(0);
        b.iter(|| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
    });

    group.finish();
}

fn sharded_counter_scaling(c: &mut Criterion) {
    let topo = Arc::new(Topology::discover().expect("topology discovery"));

    let mut group = c.benchmark_group("sharded_counter_scaling");
    group.throughput(Throughput::Elements(1000));

    for threads in [1, 2, 4, 8].iter().filter(|&&t| t <= num_cpus()) {
        group.bench_with_input(
            BenchmarkId::new("increment", threads),
            threads,
            |b, &threads| {
                let counter = Arc::new(ShardedCounter::new(&topo));
                b.iter(|| {
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            let c = Arc::clone(&counter);
                            std::thread::spawn(move || {
                                for _ in 0..1000 / threads {
                                    c.increment();
                                }
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

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

criterion_group!(benches, sharded_counter_benchmarks, sharded_counter_scaling);
criterion_main!(benches);
