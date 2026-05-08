//! Cooperative green-thread scheduler for Elisp threads.
//!
//! Implements Emacs-compatible cooperative threading: only one thread
//! runs at a time, yielding at explicit `thread-yield` calls and at
//! GC safepoints.  Each thread gets its own `Mutator` from the shared
//! GC heap for per-thread allocation.
//!
//! ## Thread lifecycle
//!
//! ```text
//! make-thread ──► Runnable ──► Dead
//!                    │
//!                    ├─ thread-yield ──► Runnable (back of queue)
//!                    ├─ thread-join ───► Waiting
//!                    └─ error ─────────► Dead (with error)
//!   Waiting ──► target finishes ──► Runnable
//! ```
//!
//! ## Design decisions
//!
//! - **Cooperative, not preemptive**: threads yield voluntarily.  This
//!   matches GNU Emacs semantics and avoids the need for a memory model.
//! - **Single OS thread for all Elisp threads**: the scheduler runs
//!   on one OS thread; Phase 3 lifts this to true parallelism.
//! - **Thread-local dynamic bindings**: `defvar` values are per-thread.
//! - **Mutators created on demand**: each time a thread is scheduled,
//!   a fresh `Mutator` is created from the shared `Arc<Heap>`.  This
//!   avoids lifetime issues and keeps the thread handles lightweight.

use std::collections::VecDeque;
use std::sync::Arc;

use neovm_gc::{Heap, Mutator};

use crate::value::LispValue;

/// Unique identifier for an Elisp thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ThreadId(u64);

impl ThreadId {
    pub(crate) const MAIN: Self = ThreadId(0);
}

/// Execution state of a single Elisp thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThreadState {
    /// Ready to run when the scheduler picks it.
    Runnable,
    /// Blocked waiting for another thread to finish.
    Waiting(ThreadId),
    /// Finished normally with a result value.
    Dead(LispValue),
    /// Finished with an error.
    Error(LispValue),
}

/// A single cooperative Elisp thread.
pub(crate) struct ThreadHandle {
    pub id: ThreadId,
    pub name: String,
    pub state: ThreadState,
    /// The function this thread is executing (nil when dead).
    pub body: LispValue,
    /// Per-thread dynamic binding stack.  Each entry is `(symbol, value)`
    /// pushed by `let`-style binding forms.
    pub dynamic_bindings: Vec<(LispValue, LispValue)>,
    /// Per-thread current buffer.
    pub current_buffer: Option<LispValue>,
}

/// Round-robin cooperative thread scheduler.
///
/// Owns a queue of `ThreadHandle`s and cycles through them, running
/// each runnable thread until it yields or finishes.  The main thread
/// (id 0) is always present and is the first to run.
pub(crate) struct ThreadScheduler {
    threads: VecDeque<ThreadHandle>,
    next_id: u64,
    /// The shared GC heap from which per-thread `Mutator`s are created.
    heap: Arc<Heap>,
}

impl ThreadScheduler {
    /// Create a new scheduler with the shared GC heap.  The main
    /// thread is created implicitly.
    pub fn new(heap: Arc<Heap>) -> Self {
        let main = ThreadHandle {
            id: ThreadId::MAIN,
            name: "main".into(),
            state: ThreadState::Runnable,
            body: LispValue::NIL,
            dynamic_bindings: Vec::new(),
            current_buffer: None,
        };
        let mut threads = VecDeque::new();
        threads.push_back(main);
        Self {
            threads,
            next_id: 1,
            heap,
        }
    }

    /// Create a fresh `Mutator` from the shared GC heap.  Callers
    /// should use this when a thread is about to execute Elisp that
    /// allocates through the GC path.
    pub fn create_mutator(&self) -> Mutator<'_> {
        self.heap.mutator()
    }

    /// Return a reference to the currently-running thread.
    pub fn current(&self) -> &ThreadHandle {
        self.threads.front().expect("scheduler has no threads")
    }

    /// Return a mutable reference to the currently-running thread.
    pub fn current_mut(&mut self) -> &mut ThreadHandle {
        self.threads.front_mut().expect("scheduler has no threads")
    }

    /// Return the current thread id.
    pub fn current_id(&self) -> ThreadId {
        self.current().id
    }

    /// Return a reference to the shared GC heap.
    pub fn heap(&self) -> &Arc<Heap> {
        &self.heap
    }

    /// Create a new thread with `name` and `body` function.  The
    /// thread starts as `Runnable` and will execute when the
    /// scheduler picks it.
    ///
    /// Returns the new thread's id.
    pub fn make_thread(&mut self, name: String, body: LispValue) -> ThreadId {
        let id = ThreadId(self.next_id);
        self.next_id += 1;
        let handle = ThreadHandle {
            id,
            name,
            state: ThreadState::Runnable,
            body,
            dynamic_bindings: Vec::new(),
            current_buffer: None,
        };
        // Insert after the current thread so the new thread runs
        // in this scheduling round.
        self.threads.insert(1, handle);
        id
    }

    /// The current thread voluntarily yields.  It moves to the back
    /// of the runnable queue.
    pub fn thread_yield(&mut self) {
        if self.threads.len() <= 1 {
            return;
        }
        let current = self.threads.pop_front().expect("scheduler has no threads");
        self.threads.push_back(current);
    }

    /// Block the current thread until `target` finishes.  The current
    /// thread is marked `Waiting(target)` and will be re-queued when
    /// the target transitions to `Dead`.
    ///
    /// Returns `true` if the current thread was actually blocked (the
    /// target was still alive).  Returns `false` if the target is
    /// already dead — the caller can immediately read the result.
    pub fn thread_join(&mut self, target: ThreadId) -> bool {
        let target_state = self
            .threads
            .iter()
            .find(|t| t.id == target)
            .map(|t| t.state);
        match target_state {
            Some(ThreadState::Dead(_)) | Some(ThreadState::Error(_)) => false,
            _ => {
                self.current_mut().state = ThreadState::Waiting(target);
                let current = self.threads.pop_front().expect("scheduler has no threads");
                self.threads.push_back(current);
                true
            }
        }
    }

    /// Signal an error in the target thread.  The target is marked
    /// `Error(error)` and any threads waiting on it are woken.
    pub fn thread_signal(&mut self, target: ThreadId, error: LispValue) {
        if let Some(handle) = self.threads.iter_mut().find(|t| t.id == target) {
            handle.state = ThreadState::Error(error);
            handle.body = LispValue::NIL;
        }
        self.wake_waiters(target);
    }

    /// Mark the current thread as finished with `result`, and wake
    /// any threads waiting on it.
    pub fn finish_current(&mut self, result: LispValue) {
        let id = self.current_id();
        self.current_mut().state = ThreadState::Dead(result);
        self.current_mut().body = LispValue::NIL;
        self.wake_waiters(id);
    }

    /// Mark the current thread as errored with `error`, and wake
    /// any threads waiting on it.
    pub fn error_current(&mut self, error: LispValue) {
        let id = self.current_id();
        self.current_mut().state = ThreadState::Error(error);
        self.current_mut().body = LispValue::NIL;
        self.wake_waiters(id);
    }

    /// Wake all threads that are waiting for `finished_id`.
    fn wake_waiters(&mut self, finished_id: ThreadId) {
        for handle in self.threads.iter_mut() {
            if handle.state == ThreadState::Waiting(finished_id) {
                handle.state = ThreadState::Runnable;
            }
        }
    }

    /// Get the result value of a dead thread.  Returns `None` if the
    /// thread is still alive or doesn't exist.
    pub fn thread_result(&self, id: ThreadId) -> Option<LispValue> {
        self.threads.iter().find(|t| t.id == id).and_then(|t| match t.state {
            ThreadState::Dead(val) => Some(val),
            ThreadState::Error(err) => Some(err),
            _ => None,
        })
    }

    /// Check whether a thread is still alive (runnable or waiting).
    pub fn thread_alive_p(&self, id: ThreadId) -> bool {
        self.threads
            .iter()
            .any(|t| t.id == id && matches!(t.state, ThreadState::Runnable | ThreadState::Waiting(_)))
    }

    /// Check whether the scheduler should continue running.
    pub fn has_runnable(&self) -> bool {
        self.threads
            .iter()
            .any(|t| matches!(t.state, ThreadState::Runnable))
    }

    /// Number of threads in the scheduler.
    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neovm_gc::{Heap, HeapConfig};

    fn test_heap() -> Arc<Heap> {
        Arc::new(Heap::new(HeapConfig::default()))
    }

    #[test]
    fn scheduler_starts_with_main_thread() {
        let scheduler = ThreadScheduler::new(test_heap());
        assert_eq!(scheduler.thread_count(), 1);
        assert_eq!(scheduler.current_id(), ThreadId::MAIN);
        assert!(scheduler.has_runnable());
    }

    #[test]
    fn make_thread_creates_runnable_thread() {
        let mut scheduler = ThreadScheduler::new(test_heap());
        let id = scheduler.make_thread("worker".into(), LispValue::NIL);
        assert_eq!(scheduler.thread_count(), 2);
        assert!(scheduler.thread_alive_p(id));
    }

    #[test]
    fn thread_yield_rotates_queue() {
        let mut scheduler = ThreadScheduler::new(test_heap());
        let worker_id = scheduler.make_thread("worker".into(), LispValue::NIL);
        assert_eq!(scheduler.current_id(), ThreadId::MAIN);
        scheduler.thread_yield();
        assert_eq!(scheduler.current_id(), worker_id);
        scheduler.thread_yield();
        assert_eq!(scheduler.current_id(), ThreadId::MAIN);
    }

    #[test]
    fn thread_join_blocks_until_target_finishes() {
        let mut scheduler = ThreadScheduler::new(test_heap());
        let worker_id = scheduler.make_thread("worker".into(), LispValue::NIL);
        scheduler.thread_yield();
        assert_eq!(scheduler.current_id(), worker_id);
        scheduler.finish_current(LispValue::expect_fixnum(42));
        scheduler.thread_yield(); // back to main
        let blocked = scheduler.thread_join(worker_id);
        assert!(!blocked); // already dead
        assert_eq!(
            scheduler.thread_result(worker_id),
            Some(LispValue::expect_fixnum(42))
        );
    }

    #[test]
    fn thread_signal_propagates_error() {
        let mut scheduler = ThreadScheduler::new(test_heap());
        let worker_id = scheduler.make_thread("worker".into(), LispValue::NIL);
        let error = LispValue::expect_fixnum(99);
        scheduler.thread_signal(worker_id, error);
        assert!(!scheduler.thread_alive_p(worker_id));
        assert_eq!(scheduler.thread_result(worker_id), Some(error));
    }

    #[test]
    fn yield_is_noop_with_single_thread() {
        let mut scheduler = ThreadScheduler::new(test_heap());
        let before = scheduler.current_id();
        scheduler.thread_yield();
        assert_eq!(before, scheduler.current_id());
    }
}
