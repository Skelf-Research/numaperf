//! Per-node work queue with stealing support.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::task::Task;

/// A work queue for a single NUMA node.
///
/// The queue supports:
/// - Local push/pop (LIFO for cache locality)
/// - Remote stealing (FIFO to steal older tasks first)
///
/// This implementation uses a simple `Mutex<VecDeque>`. For higher performance
/// under contention, consider using `crossbeam-deque` in the future.
pub struct NodeQueue {
    /// The task queue, protected by a mutex.
    tasks: Mutex<VecDeque<Task>>,
    /// Approximate count of tasks (may be slightly stale).
    len: AtomicUsize,
}

impl NodeQueue {
    /// Create a new empty queue.
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(VecDeque::new()),
            len: AtomicUsize::new(0),
        }
    }

    /// Push a task onto the local end of the queue.
    ///
    /// Local workers push to the back and pop from the back (LIFO),
    /// which provides better cache locality for recently submitted work.
    pub fn push(&self, task: Task) {
        let mut queue = self.tasks.lock().unwrap();
        queue.push_back(task);
        self.len.fetch_add(1, Ordering::Relaxed);
    }

    /// Pop a task from the local end of the queue.
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&self) -> Option<Task> {
        let mut queue = self.tasks.lock().unwrap();
        let task = queue.pop_back();
        if task.is_some() {
            self.len.fetch_sub(1, Ordering::Relaxed);
        }
        task
    }

    /// Steal a task from the remote end of the queue.
    ///
    /// Stealers take from the front (FIFO), getting older tasks that are
    /// less likely to be in the local worker's cache anyway.
    ///
    /// Returns `None` if the queue is empty.
    pub fn steal(&self) -> Option<Task> {
        let mut queue = self.tasks.lock().unwrap();
        let task = queue.pop_front();
        if task.is_some() {
            self.len.fetch_sub(1, Ordering::Relaxed);
        }
        task
    }

    /// Get the approximate number of tasks in the queue.
    ///
    /// This is a relaxed read and may not reflect concurrent modifications.
    #[inline]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    /// Check if the queue is approximately empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for NodeQueue {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: NodeQueue uses internal synchronization (Mutex)
unsafe impl Send for NodeQueue {}
unsafe impl Sync for NodeQueue {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    #[test]
    fn test_push_pop() {
        let queue = NodeQueue::new();
        assert!(queue.is_empty());

        let counter = Arc::new(AtomicUsize::new(0));

        // Push 3 tasks
        for i in 1..=3 {
            let c = Arc::clone(&counter);
            queue.push(Task::new(move || {
                c.fetch_add(i, Ordering::SeqCst);
            }));
        }

        assert_eq!(queue.len(), 3);

        // Pop should return in LIFO order (3, 2, 1)
        queue.pop().unwrap().run();
        assert_eq!(counter.load(Ordering::SeqCst), 3);

        queue.pop().unwrap().run();
        assert_eq!(counter.load(Ordering::SeqCst), 5); // 3 + 2

        queue.pop().unwrap().run();
        assert_eq!(counter.load(Ordering::SeqCst), 6); // 3 + 2 + 1

        assert!(queue.is_empty());
        assert!(queue.pop().is_none());
    }

    #[test]
    fn test_steal() {
        let queue = NodeQueue::new();
        let counter = Arc::new(AtomicUsize::new(0));

        // Push 3 tasks
        for i in 1..=3 {
            let c = Arc::clone(&counter);
            queue.push(Task::new(move || {
                c.fetch_add(i, Ordering::SeqCst);
            }));
        }

        // Steal should return in FIFO order (1, 2, 3)
        queue.steal().unwrap().run();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        queue.steal().unwrap().run();
        assert_eq!(counter.load(Ordering::SeqCst), 3); // 1 + 2

        queue.steal().unwrap().run();
        assert_eq!(counter.load(Ordering::SeqCst), 6); // 1 + 2 + 3

        assert!(queue.is_empty());
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let queue = Arc::new(NodeQueue::new());
        let counter = Arc::new(AtomicUsize::new(0));

        // Spawn producers
        let mut handles = vec![];
        for _ in 0..4 {
            let q = Arc::clone(&queue);
            let c = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let cc = Arc::clone(&c);
                    q.push(Task::new(move || {
                        cc.fetch_add(1, Ordering::Relaxed);
                    }));
                }
            }));
        }

        // Wait for producers
        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(queue.len(), 400);

        // Drain the queue
        while let Some(task) = queue.pop() {
            task.run();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 400);
    }
}
