//! Worker pool management for a single NUMA node.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use numaperf_affinity::ScopedPin;
use numaperf_core::{HardMode, NodeId, NumaError};
use numaperf_topo::Topology;

use crate::queue::NodeQueue;
use crate::steal::StealPolicy;

/// Shared state for all workers in the executor.
pub struct SharedState {
    /// The system topology.
    pub topo: Arc<Topology>,
    /// Per-node work queues.
    pub queues: Vec<Arc<NodeQueue>>,
    /// Work stealing policy.
    pub steal_policy: StealPolicy,
    /// Shutdown signal.
    pub shutdown: AtomicBool,
    /// Hard mode for worker pinning.
    pub hard_mode: HardMode,
}

impl SharedState {
    /// Signal all workers to shut down.
    pub fn signal_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been signaled.
    #[inline]
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

/// A pool of worker threads for a single NUMA node.
pub struct WorkerPool {
    /// The node this pool is assigned to.
    node_id: NodeId,
    /// Worker thread handles.
    threads: Vec<JoinHandle<()>>,
}

impl WorkerPool {
    /// Create a new worker pool for a node (soft mode).
    ///
    /// Spawns `num_workers` threads, all pinned to CPUs on `node_id`.
    /// Workers will continue even if pinning fails.
    pub fn new(
        node_id: NodeId,
        num_workers: usize,
        shared: Arc<SharedState>,
    ) -> Self {
        Self::new_with_mode(node_id, num_workers, shared, HardMode::Soft)
            .expect("soft mode worker pool creation should not fail")
    }

    /// Create a new worker pool with hard mode enforcement.
    ///
    /// In strict mode, this validates that workers can be pinned before spawning.
    /// If pinning validation fails, returns an error instead of spawning workers.
    ///
    /// In soft mode, this behaves like `new()`.
    pub fn new_with_mode(
        node_id: NodeId,
        num_workers: usize,
        shared: Arc<SharedState>,
        mode: HardMode,
    ) -> Result<Self, NumaError> {
        // In strict mode, validate that we can pin to this node before spawning workers
        if mode.is_strict() {
            let cpus = shared.topo.cpu_set(node_id);
            // Test pinning on the current thread - this validates permissions
            // We'll drop the pin guard immediately, the workers will pin themselves
            let _test_pin = ScopedPin::pin_current_with_mode(cpus, mode)?;
        }

        let mut threads = Vec::with_capacity(num_workers);

        for worker_idx in 0..num_workers {
            let shared = Arc::clone(&shared);
            let node = node_id;

            let handle = thread::Builder::new()
                .name(format!("numa-worker-{}-{}", node.as_u32(), worker_idx))
                .spawn(move || {
                    worker_loop(node, shared);
                })
                .expect("failed to spawn worker thread");

            threads.push(handle);
        }

        Ok(Self { node_id, threads })
    }

    /// Get the node ID this pool is assigned to.
    #[inline]
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get the number of workers in this pool.
    #[inline]
    pub fn worker_count(&self) -> usize {
        self.threads.len()
    }

    /// Wait for all workers to finish.
    ///
    /// This should only be called after signaling shutdown.
    pub fn join(self) {
        for handle in self.threads {
            let _ = handle.join();
        }
    }
}

/// The main worker loop.
///
/// Each worker:
/// 1. Pins itself to the node's CPUs
/// 2. Processes tasks from its local queue
/// 3. Steals from other queues when idle
/// 4. Parks briefly when no work is available
fn worker_loop(node_id: NodeId, shared: Arc<SharedState>) {
    // Pin to this node's CPUs with hard mode enforcement
    let cpus = shared.topo.cpu_set(node_id);
    let _pin = ScopedPin::pin_current_with_mode(cpus, shared.hard_mode).ok();

    let local_queue = &shared.queues[node_id.as_u32() as usize];

    loop {
        // Check for shutdown
        if shared.is_shutdown() {
            // Drain remaining local tasks before exiting
            while let Some(task) = local_queue.pop() {
                run_task_safely(task);
            }
            break;
        }

        // 1. Try local queue first
        if let Some(task) = local_queue.pop() {
            run_task_safely(task);
            continue;
        }

        // 2. Try stealing from other nodes
        if shared.steal_policy.allows_stealing() {
            if let Some(task) = try_steal(node_id, &shared) {
                run_task_safely(task);
                continue;
            }
        }

        // 3. No work available - park briefly
        thread::park_timeout(Duration::from_micros(100));
    }
}

/// Try to steal a task from another node's queue.
fn try_steal(node_id: NodeId, shared: &SharedState) -> Option<crate::task::Task> {
    let steal_order = shared.steal_policy.steal_order(node_id, &shared.topo);

    for target_node in steal_order {
        let target_queue = &shared.queues[target_node.as_u32() as usize];

        // Only try to steal if there's work
        if !target_queue.is_empty() {
            if let Some(task) = target_queue.steal() {
                return Some(task);
            }
        }
    }

    None
}

/// Run a task, catching any panics.
fn run_task_safely(task: crate::task::Task) {
    // Catch panics to prevent one bad task from killing the worker
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        task.run();
    }));

    if let Err(_panic) = result {
        // Log or handle the panic
        // For now, we just continue to the next task
        eprintln!("Task panicked in worker thread");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_shared_state(num_nodes: usize) -> Arc<SharedState> {
        let cpus = numaperf_core::CpuSet::parse("0-7").unwrap();
        let topo = Arc::new(Topology::single_node(cpus));

        let queues: Vec<Arc<NodeQueue>> = (0..num_nodes).map(|_| Arc::new(NodeQueue::new())).collect();

        Arc::new(SharedState {
            topo,
            queues,
            steal_policy: StealPolicy::default(),
            shutdown: AtomicBool::new(false),
            hard_mode: HardMode::Soft,
        })
    }

    #[test]
    fn test_shared_state_shutdown() {
        let shared = make_shared_state(1);
        assert!(!shared.is_shutdown());

        shared.signal_shutdown();
        assert!(shared.is_shutdown());
    }

    #[test]
    fn test_worker_pool_soft_mode() {
        let shared = make_shared_state(1);
        let pool = WorkerPool::new_with_mode(
            NodeId::new(0),
            1,
            Arc::clone(&shared),
            HardMode::Soft,
        );
        assert!(pool.is_ok());

        let pool = pool.unwrap();
        assert_eq!(pool.worker_count(), 1);

        shared.signal_shutdown();
        pool.join();
    }

    #[test]
    fn test_worker_pool_strict_mode() {
        let shared = make_shared_state(1);

        // Strict mode - may succeed or fail depending on permissions
        let result = WorkerPool::new_with_mode(
            NodeId::new(0),
            1,
            Arc::clone(&shared),
            HardMode::Strict,
        );

        // Either succeeds or returns HardModeUnavailable
        match result {
            Ok(pool) => {
                shared.signal_shutdown();
                pool.join();
            }
            Err(NumaError::HardModeUnavailable { .. }) => {
                // Expected if we don't have permission
            }
            Err(NumaError::NotSupported { .. }) => {
                // Expected on platforms without affinity support
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}
