//! Criterion benchmarks for memory allocation.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use numaperf::{MemPolicy, NodeId, NodeMask, NumaRegion, Prefault, Topology};
use std::sync::Arc;

fn memory_allocation_benchmarks(c: &mut Criterion) {
    let topo = Arc::new(Topology::discover().expect("topology discovery"));

    let mut group = c.benchmark_group("memory_allocation");

    // Different sizes
    for size in [4096, 64 * 1024, 1024 * 1024] {
        let size_name = if size < 1024 * 1024 {
            format!("{}kb", size / 1024)
        } else {
            format!("{}mb", size / (1024 * 1024))
        };

        group.throughput(Throughput::Bytes(size as u64));

        // Local policy
        group.bench_with_input(
            BenchmarkId::new("local", &size_name),
            &size,
            |b, &size| {
                b.iter(|| {
                    let region = NumaRegion::anon(
                        size,
                        MemPolicy::Local,
                        Default::default(),
                        Prefault::None,
                    )
                    .expect("allocation");
                    black_box(region);
                });
            },
        );

        // Bind policy
        if !topo.numa_nodes().is_empty() {
            let node0 = NodeMask::single(topo.numa_nodes()[0].id());
            group.bench_with_input(
                BenchmarkId::new("bind", &size_name),
                &size,
                |b, &size| {
                    b.iter(|| {
                        let region = NumaRegion::anon(
                            size,
                            MemPolicy::Bind(node0.clone()),
                            Default::default(),
                            Prefault::None,
                        )
                        .expect("allocation");
                        black_box(region);
                    });
                },
            );
        }

        // Preferred policy
        group.bench_with_input(
            BenchmarkId::new("preferred", &size_name),
            &size,
            |b, &size| {
                b.iter(|| {
                    let region = NumaRegion::anon(
                        size,
                        MemPolicy::Preferred(NodeId::new(0)),
                        Default::default(),
                        Prefault::None,
                    )
                    .expect("allocation");
                    black_box(region);
                });
            },
        );
    }

    group.finish();
}

fn prefault_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_prefault");
    let size = 64 * 1024; // 64KB

    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("none", |b| {
        b.iter(|| {
            let region = NumaRegion::anon(size, MemPolicy::Local, Default::default(), Prefault::None)
                .expect("allocation");
            black_box(region);
        });
    });

    group.bench_function("touch", |b| {
        b.iter(|| {
            let region = NumaRegion::anon(size, MemPolicy::Local, Default::default(), Prefault::Touch)
                .expect("allocation");
            black_box(region);
        });
    });

    group.finish();
}

fn memory_access_benchmarks(c: &mut Criterion) {
    let topo = Arc::new(Topology::discover().expect("topology discovery"));
    let size = 1024 * 1024; // 1MB

    let mut group = c.benchmark_group("memory_access");
    group.throughput(Throughput::Bytes(size as u64));

    // Local memory access
    group.bench_function("local_sequential", |b| {
        let mut region = NumaRegion::anon(size, MemPolicy::Local, Default::default(), Prefault::Touch)
            .expect("allocation");
        let slice = region.as_mut_slice();
        b.iter(|| {
            for byte in slice.iter_mut() {
                *byte = byte.wrapping_add(1);
            }
            black_box(&slice);
        });
    });

    // Interleaved memory access (if multiple nodes)
    if topo.node_count() > 1 {
        let mut all_nodes = NodeMask::new();
        for node in topo.numa_nodes() {
            all_nodes.add(node.id());
        }

        group.bench_function("interleaved_sequential", |b| {
            let mut region = NumaRegion::anon(
                size,
                MemPolicy::Interleave(all_nodes.clone()),
                Default::default(),
                Prefault::Touch,
            )
            .expect("allocation");
            let slice = region.as_mut_slice();
            b.iter(|| {
                for byte in slice.iter_mut() {
                    *byte = byte.wrapping_add(1);
                }
                black_box(&slice);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    memory_allocation_benchmarks,
    prefault_benchmarks,
    memory_access_benchmarks
);
criterion_main!(benches);
