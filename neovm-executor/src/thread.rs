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
pub(crate) struct ThreadId(pub(crate) u64);

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
///
/// The inner state is wrapped in `Arc<Mutex<>>` so the scheduler can
/// be cloned and shared between native OS threads.
#[derive(Clone)]
pub(crate) struct ThreadScheduler {
    inner: Arc<std::sync::Mutex<SchedulerInner>>,
}

struct SchedulerInner {
    threads: VecDeque<ThreadHandle>,
    next_id: u64,
    heap: Arc<Heap>,
}

impl ThreadScheduler {
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
            inner: Arc::new(std::sync::Mutex::new(SchedulerInner {
                threads,
                next_id: 1,
                heap,
            })),
        }
    }

    /// Return a clone of the shared GC heap Arc.  Callers can create
    /// a Mutator from it directly: `scheduler.heap_clone().mutator()`.
    pub fn heap_clone(&self) -> Arc<Heap> {
        Arc::clone(&self.inner.lock().unwrap().heap)
    }

    /// Current thread id — reads the front of the queue under lock,
    /// then releases.  The id is stable for the current OS thread.
    pub fn current_id(&self) -> ThreadId {
        let s = self.inner.lock().unwrap();
        s.threads.front().expect("scheduler has no threads").id
    }

    pub fn make_thread(&self, name: String, body: LispValue) -> ThreadId {
        let mut s = self.inner.lock().unwrap();
        let id = ThreadId(s.next_id);
        s.next_id += 1;
        let handle = ThreadHandle {
            id,
            name,
            state: ThreadState::Runnable,
            body,
            dynamic_bindings: Vec::new(),
            current_buffer: None,
        };
        s.threads.insert(1, handle);
        id
    }

    pub fn thread_yield(&self) {
        let mut s = self.inner.lock().unwrap();
        if s.threads.len() <= 1 {
            return;
        }
        let current = s.threads.pop_front().expect("scheduler has no threads");
        s.threads.push_back(current);
    }

    pub fn thread_join(&self, target: ThreadId) -> bool {
        let mut s = self.inner.lock().unwrap();
        let target_state = s.threads.iter().find(|t| t.id == target).map(|t| t.state);
        match target_state {
            Some(ThreadState::Dead(_)) | Some(ThreadState::Error(_)) => false,
            _ => {
                if let Some(current) = s.threads.front_mut() {
                    current.state = ThreadState::Waiting(target);
                }
                let current = s.threads.pop_front().expect("scheduler has no threads");
                s.threads.push_back(current);
                true
            }
        }
    }

    pub fn thread_signal(&self, target: ThreadId, error: LispValue) {
        let mut s = self.inner.lock().unwrap();
        if let Some(handle) = s.threads.iter_mut().find(|t| t.id == target) {
            handle.state = ThreadState::Error(error);
            handle.body = LispValue::NIL;
        }
        Self::wake_waiters_inner(&mut s.threads, target);
    }

    pub fn finish_current(&self, result: LispValue) {
        let mut s = self.inner.lock().unwrap();
        let id = s.threads.front().expect("scheduler has no threads").id;
        if let Some(current) = s.threads.front_mut() {
            current.state = ThreadState::Dead(result);
            current.body = LispValue::NIL;
        }
        Self::wake_waiters_inner(&mut s.threads, id);
    }

    pub fn error_current(&self, error: LispValue) {
        let mut s = self.inner.lock().unwrap();
        let id = s.threads.front().expect("scheduler has no threads").id;
        if let Some(current) = s.threads.front_mut() {
            current.state = ThreadState::Error(error);
            current.body = LispValue::NIL;
        }
        Self::wake_waiters_inner(&mut s.threads, id);
    }

    fn wake_waiters_inner(threads: &mut VecDeque<ThreadHandle>, finished_id: ThreadId) {
        for handle in threads.iter_mut() {
            if handle.state == ThreadState::Waiting(finished_id) {
                handle.state = ThreadState::Runnable;
            }
        }
    }

    pub fn thread_result(&self, id: ThreadId) -> Option<LispValue> {
        let s = self.inner.lock().unwrap();
        s.threads.iter().find(|t| t.id == id).and_then(|t| match t.state {
            ThreadState::Dead(val) => Some(val),
            ThreadState::Error(err) => Some(err),
            _ => None,
        })
    }

    pub fn thread_alive_p(&self, id: ThreadId) -> bool {
        let s = self.inner.lock().unwrap();
        s.threads
            .iter()
            .any(|t| t.id == id && matches!(t.state, ThreadState::Runnable | ThreadState::Waiting(_)))
    }

    pub fn has_runnable(&self) -> bool {
        let s = self.inner.lock().unwrap();
        s.threads.iter().any(|t| matches!(t.state, ThreadState::Runnable))
    }

    pub fn thread_count(&self) -> usize {
        self.inner.lock().unwrap().threads.len()
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
        let scheduler = ThreadScheduler::new(test_heap());
        let id = scheduler.make_thread("worker".into(), LispValue::NIL);
        assert_eq!(scheduler.thread_count(), 2);
        assert!(scheduler.thread_alive_p(id));
    }

    #[test]
    fn thread_yield_rotates_queue() {
        let scheduler = ThreadScheduler::new(test_heap());
        let worker_id = scheduler.make_thread("worker".into(), LispValue::NIL);
        assert_eq!(scheduler.current_id(), ThreadId::MAIN);
        scheduler.thread_yield();
        assert_eq!(scheduler.current_id(), worker_id);
        scheduler.thread_yield();
        assert_eq!(scheduler.current_id(), ThreadId::MAIN);
    }

    #[test]
    fn thread_join_blocks_until_target_finishes() {
        let scheduler = ThreadScheduler::new(test_heap());
        let worker_id = scheduler.make_thread("worker".into(), LispValue::NIL);
        scheduler.thread_yield();
        assert_eq!(scheduler.current_id(), worker_id);
        scheduler.finish_current(LispValue::expect_fixnum(42));
        scheduler.thread_yield();
        let blocked = scheduler.thread_join(worker_id);
        assert!(!blocked);
        assert_eq!(
            scheduler.thread_result(worker_id),
            Some(LispValue::expect_fixnum(42))
        );
    }

    #[test]
    fn thread_signal_propagates_error() {
        let scheduler = ThreadScheduler::new(test_heap());
        let worker_id = scheduler.make_thread("worker".into(), LispValue::NIL);
        let error = LispValue::expect_fixnum(99);
        scheduler.thread_signal(worker_id, error);
        assert!(!scheduler.thread_alive_p(worker_id));
        assert_eq!(scheduler.thread_result(worker_id), Some(error));
    }

    #[test]
    fn yield_is_noop_with_single_thread() {
        let scheduler = ThreadScheduler::new(test_heap());
        let before = scheduler.current_id();
        scheduler.thread_yield();
        assert_eq!(before, scheduler.current_id());
    }
}
