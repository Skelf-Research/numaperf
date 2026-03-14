//! Benchmark runners for each category.

use numaperf::{
    CpuSet, MemPolicy, NodeMask, NumaExecutor, NumaRegion, Prefault,
    ScopedPin, ShardedCounter, StealPolicy, Topology,
};
use numaperf_bench::BenchmarkResult;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Run sharded data structure benchmarks.
pub fn run_sharded_benchmarks(
    topo: &Arc<Topology>,
    iterations: u64,
    _threads: usize,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    // Sharded counter increment (single thread)
    {
        let counter = ShardedCounter::new(topo);
        let start = Instant::now();
        for _ in 0..iterations {
            counter.increment();
        }
        let duration = start.elapsed();

        results.push(BenchmarkResult {
            name: "sharded_counter_increment".to_string(),
            duration,
            operations: iterations,
            locality_ratio: Some(1.0), // Single thread is always local
        });
    }

    // Sharded counter vs global atomic (baseline comparison)
    {
        let global = AtomicU64::new(0);
        let start = Instant::now();
        for _ in 0..iterations {
            global.fetch_add(1, Ordering::Relaxed);
        }
        let duration = start.elapsed();

        results.push(BenchmarkResult {
            name: "global_atomic_increment".to_string(),
            duration,
            operations: iterations,
            locality_ratio: None,
        });
    }

    // Sharded counter sum (read all shards)
    {
        let counter = ShardedCounter::new(topo);
        for _ in 0..100 {
            counter.increment();
        }
        let start = Instant::now();
        for _ in 0..iterations {
            std::hint::black_box(counter.sum());
        }
        let duration = start.elapsed();

        results.push(BenchmarkResult {
            name: "sharded_counter_sum".to_string(),
            duration,
            operations: iterations,
            locality_ratio: None,
        });
    }

    Ok(results)
}

/// Run memory allocation benchmarks.
pub fn run_memory_benchmarks(
    topo: &Arc<Topology>,
    iterations: u64,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    let size = 64 * 1024; // 64KB per allocation

    // Local policy allocation
    {
        let start = Instant::now();
        for _ in 0..iterations.min(100) {
            let region = NumaRegion::anon(size, MemPolicy::Local, Default::default(), Prefault::None)?;
            std::hint::black_box(&region);
        }
        let duration = start.elapsed();
        let ops = iterations.min(100);

        results.push(BenchmarkResult {
            name: "memory_alloc_local_64kb".to_string(),
            duration,
            operations: ops,
            locality_ratio: Some(1.0),
        });
    }

    // Bind policy allocation
    if !topo.numa_nodes().is_empty() {
        let node0 = NodeMask::single(topo.numa_nodes()[0].id());
        let start = Instant::now();
        for _ in 0..iterations.min(100) {
            let region = NumaRegion::anon(size, MemPolicy::Bind(node0.clone()), Default::default(), Prefault::None)?;
            std::hint::black_box(&region);
        }
        let duration = start.elapsed();
        let ops = iterations.min(100);

        results.push(BenchmarkResult {
            name: "memory_alloc_bind_64kb".to_string(),
            duration,
            operations: ops,
            locality_ratio: Some(1.0),
        });
    }

    // Prefault comparison
    {
        let start = Instant::now();
        for _ in 0..iterations.min(50) {
            let region = NumaRegion::anon(size, MemPolicy::Local, Default::default(), Prefault::Touch)?;
            std::hint::black_box(&region);
        }
        let duration = start.elapsed();
        let ops = iterations.min(50);

        results.push(BenchmarkResult {
            name: "memory_alloc_prefault_64kb".to_string(),
            duration,
            operations: ops,
            locality_ratio: Some(1.0),
        });
    }

    Ok(results)
}

/// Run scheduler benchmarks.
pub fn run_scheduler_benchmarks(
    topo: &Arc<Topology>,
    iterations: u64,
    _threads: usize,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    // Task submission throughput
    {
        let exec = NumaExecutor::builder(Arc::clone(topo))
            .steal_policy(StealPolicy::LocalThenSocketThenRemote)
            .workers_per_node(2)
            .build()?;

        let completed = Arc::new(AtomicU64::new(0));
        let num_tasks = iterations.min(1000) as usize;

        let start = Instant::now();
        for _ in 0..num_tasks {
            let c = Arc::clone(&completed);
            let node_id = topo.numa_nodes()[0].id();
            exec.submit_to_node(node_id, move || {
                c.fetch_add(1, Ordering::Relaxed);
            });
        }
        exec.shutdown();
        let duration = start.elapsed();

        results.push(BenchmarkResult {
            name: "scheduler_submit_throughput".to_string(),
            duration,
            operations: num_tasks as u64,
            locality_ratio: None,
        });
    }

    // Local-only policy throughput
    {
        let exec = NumaExecutor::builder(Arc::clone(topo))
            .steal_policy(StealPolicy::LocalOnly)
            .workers_per_node(2)
            .build()?;

        let completed = Arc::new(AtomicU64::new(0));
        let num_tasks = iterations.min(1000) as usize;

        let start = Instant::now();
        for i in 0..num_tasks {
            let node_idx = i % topo.node_count();
            let node_id = topo.numa_nodes()[node_idx].id();
            let c = Arc::clone(&completed);
            exec.submit_to_node(node_id, move || {
                c.fetch_add(1, Ordering::Relaxed);
            });
        }
        exec.shutdown();
        let duration = start.elapsed();

        results.push(BenchmarkResult {
            name: "scheduler_local_only".to_string(),
            duration,
            operations: num_tasks as u64,
            locality_ratio: Some(1.0), // LocalOnly guarantees locality
        });
    }

    Ok(results)
}

/// Run affinity benchmarks.
pub fn run_affinity_benchmarks(
    topo: &Arc<Topology>,
    iterations: u64,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    // Pin/unpin cycle
    if !topo.numa_nodes().is_empty() {
        let node = &topo.numa_nodes()[0];
        if let Some(cpu_id) = node.cpus().iter().next() {
            let cpus = CpuSet::single(cpu_id);

            let start = Instant::now();
            for _ in 0..iterations.min(100) {
                let pin = ScopedPin::pin_current(cpus.clone());
                if let Ok(_p) = pin {
                    // Pin is automatically restored when dropped
                }
            }
            let duration = start.elapsed();
            let ops = iterations.min(100);

            results.push(BenchmarkResult {
                name: "affinity_pin_unpin_cycle".to_string(),
                duration,
                operations: ops,
                locality_ratio: None,
            });
        }
    }

    // Get affinity
    {
        let start = Instant::now();
        for _ in 0..iterations {
            let _affinity = numaperf::get_affinity();
        }
        let duration = start.elapsed();

        results.push(BenchmarkResult {
            name: "affinity_get_current".to_string(),
            duration,
            operations: iterations,
            locality_ratio: None,
        });
    }

    Ok(results)
}
