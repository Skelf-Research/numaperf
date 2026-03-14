//! Criterion benchmarks for scheduler.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use numaperf::{NumaExecutor, StealPolicy, Topology};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn task_submission_benchmarks(c: &mut Criterion) {
    let topo = Arc::new(Topology::discover().expect("topology discovery"));

    let mut group = c.benchmark_group("scheduler_submission");
    group.throughput(Throughput::Elements(100));

    // Submit to local node
    group.bench_function("submit_local", |b| {
        b.iter_custom(|iters| {
            let mut total = std::time::Duration::ZERO;
            for _ in 0..iters {
                let exec = NumaExecutor::builder(Arc::clone(&topo))
                    .steal_policy(StealPolicy::LocalOnly)
                    .workers_per_node(1)
                    .build()
                    .expect("executor");

                let node_id = topo.numa_nodes()[0].id();
                let completed = Arc::new(AtomicU64::new(0));

                let start = std::time::Instant::now();
                for _ in 0..100 {
                    let c = Arc::clone(&completed);
                    exec.submit_to_node(node_id, move || {
                        c.fetch_add(1, Ordering::Relaxed);
                    });
                }
                exec.shutdown();
                total += start.elapsed();
            }
            total
        });
    });

    group.finish();
}

fn steal_policy_benchmarks(c: &mut Criterion) {
    let topo = Arc::new(Topology::discover().expect("topology discovery"));

    let mut group = c.benchmark_group("scheduler_steal_policy");
    let num_tasks = 100;
    group.throughput(Throughput::Elements(num_tasks as u64));

    for policy in [
        StealPolicy::LocalOnly,
        StealPolicy::LocalThenSocketThenRemote,
        StealPolicy::Any,
    ] {
        let policy_name = match policy {
            StealPolicy::LocalOnly => "local_only",
            StealPolicy::LocalThenSocketThenRemote => "local_then_remote",
            StealPolicy::Any => "any",
        };

        group.bench_function(policy_name, |b| {
            b.iter_custom(|iters| {
                let mut total = std::time::Duration::ZERO;
                for _ in 0..iters {
                    let exec = NumaExecutor::builder(Arc::clone(&topo))
                        .steal_policy(policy)
                        .workers_per_node(2)
                        .build()
                        .expect("executor");

                    let completed = Arc::new(AtomicU64::new(0));

                    let start = std::time::Instant::now();
                    for i in 0..num_tasks {
                        let node_idx = i % topo.node_count();
                        let node_id = topo.numa_nodes()[node_idx].id();
                        let c = Arc::clone(&completed);
                        exec.submit_to_node(node_id, move || {
                            // Simulate some work
                            let sum: u64 = (0..100).sum();
                            black_box(sum);
                            c.fetch_add(1, Ordering::Relaxed);
                        });
                    }
                    exec.shutdown();
                    total += start.elapsed();
                }
                total
            });
        });
    }

    group.finish();
}

fn worker_scaling_benchmarks(c: &mut Criterion) {
    let topo = Arc::new(Topology::discover().expect("topology discovery"));

    let mut group = c.benchmark_group("scheduler_worker_scaling");
    let num_tasks = 200;
    group.throughput(Throughput::Elements(num_tasks as u64));

    for workers_per_node in [1, 2, 4] {
        group.bench_with_input(
            BenchmarkId::new("workers", workers_per_node),
            &workers_per_node,
            |b, &workers| {
                b.iter_custom(|iters| {
                    let mut total = std::time::Duration::ZERO;
                    for _ in 0..iters {
                        let exec = NumaExecutor::builder(Arc::clone(&topo))
                            .steal_policy(StealPolicy::LocalThenSocketThenRemote)
                            .workers_per_node(workers)
                            .build()
                            .expect("executor");

                        let completed = Arc::new(AtomicU64::new(0));

                        let start = std::time::Instant::now();
                        for i in 0..num_tasks {
                            let node_idx = i % topo.node_count();
                            let node_id = topo.numa_nodes()[node_idx].id();
                            let c = Arc::clone(&completed);
                            exec.submit_to_node(node_id, move || {
                                let sum: u64 = (0..1000).sum();
                                black_box(sum);
                                c.fetch_add(1, Ordering::Relaxed);
                            });
                        }
                        exec.shutdown();
                        total += start.elapsed();
                    }
                    total
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    task_submission_benchmarks,
    steal_policy_benchmarks,
    worker_scaling_benchmarks
);
criterion_main!(benches);
