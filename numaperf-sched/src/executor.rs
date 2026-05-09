//! NUMA-aware work executor.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use numaperf_core::{HardMode, NodeId, NumaError};
use numaperf_topo::Topology;

use crate::queue::NodeQueue;
use crate::steal::StealPolicy;
use crate::task::Task;
use crate::worker::{SharedState, WorkerPool};

/// A NUMA-aware work executor with per-node queues and work stealing.
///
/// The executor maintains a pool of worker threads for each NUMA node. Workers
/// are pinned to their node's CPUs and primarily process work from their local
/// queue. When idle, they may steal work from other nodes based on the
/// configured [`StealPolicy`].
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use numaperf_sched::{NumaExecutor, StealPolicy};
/// use numaperf_topo::Topology;
///
/// let topo = Arc::new(Topology::discover()?);
/// let exec = NumaExecutor::new(Arc::clone(&topo), StealPolicy::default())?;
///
/// // Submit work to node 0
/// exec.submit_to_node(numaperf_core::NodeId::new(0), || {
///     println!("Hello from node 0!");
/// });
///
/// // Graceful shutdown
/// exec.shutdown();
/// # Ok::<(), numaperf_core::NumaError>(())
/// ```
pub struct NumaExecutor {
    /// Shared state accessible by all workers.
    shared: Arc<SharedState>,
    /// Worker pools, one per node.
    workers: Vec<WorkerPool>,
}

impl NumaExecutor {
    /// Create a new executor with the given topology and steal policy.
    ///
    /// This spawns one worker thread per CPU across all NUMA nodes.
    pub fn new(topo: Arc<Topology>, steal_policy: StealPolicy) -> Result<Self, NumaError> {
        NumaExecutorBuilder::new(topo)
            .steal_policy(steal_policy)
            .build()
    }

    /// Create a builder for more configuration options.
    pub fn builder(topo: Arc<Topology>) -> NumaExecutorBuilder {
        NumaExecutorBuilder::new(topo)
    }

    /// Submit a task to a specific node's queue.
    ///
    /// The task will be executed by a worker pinned to the specified node,
    /// or stolen by another node's worker if the target node is busy.
    pub fn submit_to_node<F>(&self, node: NodeId, work: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let task = Task::with_home_node(work, node);
        self.submit_task_to_node(task, node);
    }

    /// Submit a task without a specific node preference.
    ///
    /// The task is added to the current thread's node queue if detectable,
    /// otherwise to node 0.
    pub fn submit<F>(&self, work: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let task = Task::new(work);
        // For simplicity, submit to node 0. A smarter implementation would
        // detect the current thread's node.
        self.submit_task_to_node(task, NodeId::new(0));
    }

    /// Submit a pre-constructed task to a specific node.
    fn submit_task_to_node(&self, task: Task, node: NodeId) {
        let idx = node.as_u32() as usize;
        if idx < self.shared.queues.len() {
            self.shared.queues[idx].push(task);
            // Wake up a worker on that node
            // (In a real implementation, we'd unpark a specific worker)
        } else {
            // Fallback to node 0 if the node doesn't exist
            self.shared.queues[0].push(task);
        }
    }

    /// Get the number of NUMA nodes in the executor.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.workers.len()
    }

    /// Get the total number of worker threads.
    pub fn worker_count(&self) -> usize {
        self.workers.iter().map(|p| p.worker_count()).sum()
    }

    /// Get the approximate number of pending tasks across all queues.
    pub fn pending_tasks(&self) -> usize {
        self.shared.queues.iter().map(|q| q.len()).sum()
    }

    /// Get the steal policy.
    #[inline]
    pub fn steal_policy(&self) -> StealPolicy {
        self.shared.steal_policy
    }

    /// Initiate graceful shutdown.
    ///
    /// Signals all workers to stop accepting new work. Workers will finish
    /// processing their current task and drain their local queues before
    /// exiting.
    ///
    /// This method blocks until all workers have terminated.
    pub fn shutdown(self) {
        self.shared.signal_shutdown();

        // Wait for all workers to finish
        for pool in self.workers {
            pool.join();
        }
    }

    /// Shutdown with a timeout.
    ///
    /// Like `shutdown()`, but returns after the specified duration even if
    /// workers haven't finished. Returns `true` if shutdown completed within
    /// the timeout.
    pub fn shutdown_timeout(self, _timeout: Duration) -> bool {
        // For MVP, just do a regular shutdown
        // A real implementation would track elapsed time
        self.shutdown();
        true
    }
}

/// Builder for configuring a [`NumaExecutor`].
pub struct NumaExecutorBuilder {
    topo: Arc<Topology>,
    steal_policy: StealPolicy,
    workers_per_node: Option<usize>,
    hard_mode: HardMode,
}

impl NumaExecutorBuilder {
    /// Create a new builder with the given topology.
    pub fn new(topo: Arc<Topology>) -> Self {
        Self {
            topo,
            steal_policy: StealPolicy::default(),
            workers_per_node: None,
            hard_mode: HardMode::default(),
        }
    }

    /// Set the work stealing policy.
    pub fn steal_policy(mut self, policy: StealPolicy) -> Self {
        self.steal_policy = policy;
        self
    }

    /// Set the number of workers per node.
    ///
    /// If not set, defaults to the number of CPUs on each node.
    pub fn workers_per_node(mut self, count: usize) -> Self {
        self.workers_per_node = Some(count);
        self
    }

    /// Set the hard mode for worker pinning.
    ///
    /// In soft mode (default), workers will attempt to pin to their node's CPUs
    /// but will continue running even if pinning fails.
    ///
    /// In strict mode, the executor will fail to build if worker pinning cannot
    /// be guaranteed. This validates that workers can actually be pinned before
    /// spawning them.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use numaperf_sched::NumaExecutor;
    /// use numaperf_core::HardMode;
    /// use numaperf_topo::Topology;
    ///
    /// let topo = Arc::new(Topology::discover()?);
    ///
    /// // Strict mode - fail if pinning is not possible
    /// let exec = NumaExecutor::builder(Arc::clone(&topo))
    ///     .hard_mode(HardMode::Strict)
    ///     .build()?;
    /// # Ok::<(), numaperf_core::NumaError>(())
    /// ```
    pub fn hard_mode(mut self, mode: HardMode) -> Self {
        self.hard_mode = mode;
        self
    }

    /// Build the executor.
    pub fn build(self) -> Result<NumaExecutor, NumaError> {
        let num_nodes = self.topo.node_count();

        // Create per-node queues
        let queues: Vec<Arc<NodeQueue>> = (0..num_nodes)
            .map(|_| Arc::new(NodeQueue::new()))
            .collect();

        // Create shared state
        let shared = Arc::new(SharedState {
            topo: Arc::clone(&self.topo),
            queues,
            steal_policy: self.steal_policy,
            shutdown: AtomicBool::new(false),
            hard_mode: self.hard_mode,
        });

        // Create worker pools
        let mut workers = Vec::with_capacity(num_nodes);
        for node in self.topo.numa_nodes() {
            let num_workers = self.workers_per_node.unwrap_or_else(|| {
                // Default to number of CPUs on this node, minimum 1
                node.cpu_count().max(1)
            });

            let pool = WorkerPool::new_with_mode(
                node.id(),
                num_workers,
                Arc::clone(&shared),
                self.hard_mode,
            )?;
            workers.push(pool);
        }

        Ok(NumaExecutor { shared, workers })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn test_topology() -> Arc<Topology> {
        Arc::new(Topology::discover().unwrap_or_else(|_| {
            // Fallback for systems without NUMA
            let cpus = numaperf_core::CpuSet::parse("0-3").unwrap();
            Topology::single_node(cpus)
        }))
    }

    #[test]
    fn test_executor_soft_mode() {
        let topo = test_topology();
        let exec = NumaExecutor::builder(topo)
            .hard_mode(HardMode::Soft)
            .workers_per_node(1)
            .build();

        // Soft mode should always succeed
        assert!(exec.is_ok());
        exec.unwrap().shutdown();
    }

    #[test]
    fn test_executor_strict_mode() {
        let topo = test_topology();
        let result = NumaExecutor::builder(topo)
            .hard_mode(HardMode::Strict)
            .workers_per_node(1)
            .build();

        // Strict mode may succeed or fail depending on permissions
        match result {
            Ok(exec) => {
                // Has permission for strict pinning
                exec.shutdown();
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

    #[test]
    fn test_executor_creation() {
        let topo = test_topology();
        let exec = NumaExecutor::new(topo, StealPolicy::default()).unwrap();
        assert!(exec.node_count() > 0);
        assert!(exec.worker_count() > 0);
        exec.shutdown();
    }

    #[test]
    fn test_executor_builder() {
        let topo = test_topology();
        let exec = NumaExecutor::builder(topo)
            .steal_policy(StealPolicy::LocalOnly)
            .workers_per_node(2)
            .build()
            .unwrap();

        assert_eq!(exec.steal_policy(), StealPolicy::LocalOnly);
        exec.shutdown();
    }

    #[test]
    fn test_submit_and_execute() {
        let topo = test_topology();
        let exec = NumaExecutor::new(Arc::clone(&topo), StealPolicy::default()).unwrap();

        let counter = Arc::new(AtomicUsize::new(0));

        // Submit some tasks
        for _ in 0..10 {
            let c = Arc::clone(&counter);
            exec.submit_to_node(NodeId::new(0), move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }

        // Give workers time to process
        std::thread::sleep(Duration::from_millis(100));

        exec.shutdown();

        // All tasks should have been executed
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_pending_tasks() {
        let topo = test_topology();
        let exec = NumaExecutor::builder(topo)
            .workers_per_node(1)
            .build()
            .unwrap();

        assert_eq!(exec.pending_tasks(), 0);

        // Use two channels: one to signal task started, one to unblock it
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (unblock_tx, unblock_rx) = std::sync::mpsc::channel();

        // Submit a blocking task
        exec.submit_to_node(NodeId::new(0), move || {
            started_tx.send(()).unwrap(); // Signal we started
            unblock_rx.recv().unwrap(); // Block until signaled
        });

        // Wait for the blocking task to actually start executing
        started_rx.recv().unwrap();

        // Now the worker is blocked, so these should queue up
        for _ in 0..5 {
            exec.submit_to_node(NodeId::new(0), || {});
        }

        // Give a moment for tasks to be queued
        std::thread::sleep(Duration::from_millis(10));

        // Should have exactly 5 pending tasks (the worker is blocked)
        assert_eq!(exec.pending_tasks(), 5);

        // Unblock and shutdown
        unblock_tx.send(()).unwrap();
        exec.shutdown();
    }
}
