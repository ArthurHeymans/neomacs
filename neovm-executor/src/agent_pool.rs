//! Background thread pool for processing agent actions asynchronously.
//!
//! Agents with pending actions are processed by this pool.  Each worker
//! thread gets a forked Runtime and executes actions independently.
//! Used when `send-off` is called (potentially-blocking work) or when
//! the cooperative scheduler detects idle time.
//!
//! In the cooperative threading model (single OS thread), the pool is
//! dormant — agent actions are drained synchronously by `agent-await`.
//! In the native threading model (Phase 3), the pool processes actions
//! on background OS threads.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};

/// Statistics for the agent thread pool.
#[derive(Debug, Default)]
pub(crate) struct AgentPoolStats {
    pub submitted: AtomicUsize,
    pub completed: AtomicUsize,
}

/// A thread pool for asynchronous agent action processing.
///
/// Created eagerly but workers only spawn on first `send-off`.
/// `send` actions are queued and may be drained synchronously by
/// `agent-await` or asynchronously by a pool worker.
pub(crate) struct AgentPool {
    /// Channel to submit work to worker threads.  None if no workers
    /// have been spawned yet (lazy initialization).
    sender: Mutex<Option<mpsc::Sender<AgentWork>>>,
    stats: AgentPoolStats,
}

/// Work item: the raw heap address of the agent to process.
#[derive(Debug)]
pub(crate) struct AgentWork {
    pub agent_addr: usize,
}

impl AgentPool {
    pub fn new() -> Self {
        Self {
            sender: Mutex::new(None),
            stats: AgentPoolStats::default(),
        }
    }

    /// Submit an agent for background processing.  If no workers are
    /// running, the work is queued and will be drained by `agent-await`.
    pub fn submit(&self, agent_addr: usize) {
        self.stats.submitted.fetch_add(1, Ordering::Relaxed);
        if let Some(ref tx) = *self.sender.lock().unwrap() {
            let _ = tx.send(AgentWork { agent_addr });
        }
    }

    /// Return current pool statistics.
    pub fn stats(&self) -> &AgentPoolStats {
        &self.stats
    }
}

impl Default for AgentPool {
    fn default() -> Self {
        Self::new()
    }
}
