//! Criterion benchmarks for thread affinity.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use numaperf::{CpuSet, ScopedPin, Topology};
use std::sync::Arc;

fn affinity_benchmarks(c: &mut Criterion) {
    let topo = Arc::new(Topology::discover().expect("topology discovery"));

    let mut group = c.benchmark_group("affinity");
    group.throughput(Throughput::Elements(1));

    // Get current affinity
    group.bench_function("get_affinity", |b| {
        b.iter(|| {
            let affinity = numaperf::get_affinity();
            black_box(affinity);
        });
    });

    // Pin/unpin cycle
    if !topo.numa_nodes().is_empty() {
        let node = &topo.numa_nodes()[0];
        if let Some(cpu_id) = node.cpus().iter().next() {
            let cpus = CpuSet::single(cpu_id);

            group.bench_function("pin_unpin_cycle", |b| {
                b.iter(|| {
                    if let Ok(pin) = ScopedPin::pin_current(cpus.clone()) {
                        black_box(&pin);
                        // Pin is restored when dropped
                    }
                });
            });

            // Pin and hold
            group.bench_function("pin_hold", |b| {
                b.iter_custom(|iters| {
                    let mut total = std::time::Duration::ZERO;
                    for _ in 0..iters {
                        let start = std::time::Instant::now();
                        if let Ok(pin) = ScopedPin::pin_current(cpus.clone()) {
                            // Hold the pin
                            std::hint::black_box(&pin);
                            total += start.elapsed();
                            // Drop happens here
                        }
                    }
                    total
                });
            });
        }
    }

    group.finish();
}

fn cpuset_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpuset");
    group.throughput(Throughput::Elements(1));

    // Parse CPU set
    group.bench_function("parse_single", |b| {
        b.iter(|| {
            let cpuset = CpuSet::parse("0");
            black_box(cpuset);
        });
    });

    group.bench_function("parse_range", |b| {
        b.iter(|| {
            let cpuset = CpuSet::parse("0-7");
            black_box(cpuset);
        });
    });

    group.bench_function("parse_complex", |b| {
        b.iter(|| {
            let cpuset = CpuSet::parse("0-3,8-11,16-19");
            black_box(cpuset);
        });
    });

    // CpuSet operations
    group.bench_function("contains", |b| {
        let cpuset = CpuSet::parse("0-31").expect("valid");
        b.iter(|| {
            for i in 0..32 {
                black_box(cpuset.contains(i));
            }
        });
    });

    group.bench_function("iter", |b| {
        let cpuset = CpuSet::parse("0-31").expect("valid");
        b.iter(|| {
            for cpu in cpuset.iter() {
                black_box(cpu);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, affinity_benchmarks, cpuset_benchmarks);
criterion_main!(benches);
