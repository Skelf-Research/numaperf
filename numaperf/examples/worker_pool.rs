//! NUMA-aware parallel task execution.
//!
//! This example demonstrates creating a NUMA-aware executor with per-node
//! worker pools and configurable work stealing.
//!
//! Run with: cargo run -p numaperf --example worker_pool

use numaperf::{Capabilities, NumaExecutor, StealPolicy, Topology};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

fn main() -> Result<(), numaperf::NumaError> {
    println!("=== numaperf: Worker Pool Example ===\n");

    // Check capabilities
    let caps = Capabilities::detect();
    println!("NUMA nodes detected: {}", caps.numa_node_count);
    println!("Hard mode supported: {}", caps.supports_hard_mode());
    println!();

    // Discover topology
    let topo = Arc::new(Topology::discover()?);
    println!("Creating executor with {} nodes", topo.node_count());

    // Create executor with 2 workers per node
    let exec = NumaExecutor::builder(Arc::clone(&topo))
        .steal_policy(StealPolicy::LocalThenSocketThenRemote)
        .workers_per_node(2)
        .build()?;

    println!("Executor created:");
    println!("  Total workers: {}", exec.worker_count());
    println!("  Steal policy: {}", exec.steal_policy());
    println!();

    // Track work completion per node
    let total_tasks = 100;
    let completed = Arc::new(AtomicUsize::new(0));

    println!("Submitting {} tasks...", total_tasks);
    let start = Instant::now();

    // Distribute tasks across nodes
    for i in 0..total_tasks {
        let node_idx = i % topo.node_count();
        let node_id = topo.numa_nodes()[node_idx].id();
        let c = Arc::clone(&completed);

        exec.submit_to_node(node_id, move || {
            // Simulate some work
            let sum: u64 = (0..1000).sum();
            std::hint::black_box(sum);

            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    println!("All tasks submitted, waiting for completion...");

    // Graceful shutdown waits for all tasks
    exec.shutdown();

    let elapsed = start.elapsed();
    let completed_count = completed.load(Ordering::SeqCst);

    println!();
    println!("Results:");
    println!("  Completed: {} tasks", completed_count);
    println!("  Time: {:?}", elapsed);
    println!(
        "  Throughput: {:.0} tasks/sec",
        completed_count as f64 / elapsed.as_secs_f64()
    );

    Ok(())
}
