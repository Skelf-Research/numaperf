//! Task wrapper for scheduled work.

use numaperf_core::NodeId;

/// A unit of work to be executed by the scheduler.
///
/// Tasks wrap a closure and optionally specify a "home node" - the preferred
/// NUMA node where the task should execute for best data locality.
pub struct Task {
    /// The work to execute.
    work: Box<dyn FnOnce() + Send + 'static>,
    /// The preferred node for execution, if any.
    home_node: Option<NodeId>,
}

impl Task {
    /// Create a new task with no node preference.
    pub fn new<F>(work: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            work: Box::new(work),
            home_node: None,
        }
    }

    /// Create a new task with a preferred home node.
    ///
    /// The scheduler will try to execute this task on the specified node,
    /// but may run it elsewhere if that node's workers are busy.
    pub fn with_home_node<F>(work: F, home_node: NodeId) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            work: Box::new(work),
            home_node: Some(home_node),
        }
    }

    /// Get the preferred home node, if specified.
    #[inline]
    pub fn home_node(&self) -> Option<NodeId> {
        self.home_node
    }

    /// Execute the task.
    ///
    /// This consumes the task.
    #[inline]
    pub fn run(self) {
        (self.work)();
    }
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("home_node", &self.home_node)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_task_runs() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = Arc::clone(&executed);

        let task = Task::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
        });

        assert!(!executed.load(Ordering::SeqCst));
        task.run();
        assert!(executed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_task_with_home_node() {
        let task = Task::with_home_node(|| {}, NodeId::new(2));
        assert_eq!(task.home_node(), Some(NodeId::new(2)));
    }

    #[test]
    fn test_task_without_home_node() {
        let task = Task::new(|| {});
        assert_eq!(task.home_node(), None);
    }
}
