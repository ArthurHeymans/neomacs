use crate::background::{
    BackgroundCollector, BackgroundCollectorConfig, BackgroundCollectorStats,
    SharedBackgroundError, SharedHeapStatus,
};
use crate::collector_state::CollectorSharedSnapshot;
use crate::heap::AllocError;
use crate::plan::BackgroundCollectionStatus;
use crate::runtime::SharedCollectorRuntime;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Background worker configuration for an autonomous collector thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackgroundWorkerConfig {
    /// Background collector coordinator configuration used by the worker.
    pub collector: BackgroundCollectorConfig,
    /// Sleep duration after an idle worker round.
    pub idle_sleep: Duration,
    /// Sleep duration after one ready-to-finish or finished round.
    pub busy_sleep: Duration,
}

impl Default for BackgroundWorkerConfig {
    fn default() -> Self {
        Self {
            collector: BackgroundCollectorConfig::default(),
            idle_sleep: Duration::from_millis(1),
            busy_sleep: Duration::ZERO,
        }
    }
}

/// Runtime statistics for one autonomous background worker.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BackgroundWorkerStats {
    /// Number of worker loop iterations executed.
    pub loops: u64,
    /// Number of worker iterations that observed an idle collector.
    pub idle_loops: u64,
    /// Number of worker iterations that entered signal-backed waiting.
    pub wait_loops: u64,
    /// Number of idle worker iterations satisfied entirely from shared snapshot state.
    pub snapshot_idle_loops: u64,
    /// Number of worker waits woken early by one shared-heap signal.
    pub signal_wakeups: u64,
    /// Number of signal-backed wakes that observed one real background-scheduler state change.
    pub background_change_wakeups: u64,
    /// Number of signal-backed wakes ignored because background-scheduler state stayed the same.
    pub ignored_signal_wakeups: u64,
    /// Number of worker iterations that skipped due to heap lock contention.
    pub contention_loops: u64,
    /// Background collector coordinator statistics accumulated by the worker.
    pub collector: BackgroundCollectorStats,
}

/// Public snapshot of one background worker and its backing shared heap state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackgroundWorkerStatus {
    /// Current autonomous worker statistics.
    pub worker: BackgroundWorkerStats,
    /// Current shared heap snapshot backing the worker.
    pub heap: SharedHeapStatus,
}

/// Background worker failure modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackgroundWorkerError {
    /// The worker encountered one heap/collector allocation error.
    Collection(AllocError),
    /// Shared heap or worker stats were poisoned by another panic.
    LockPoisoned,
    /// The worker thread panicked before returning.
    WorkerPanicked,
}

/// Join handle and control surface for one autonomous background collector thread.
#[derive(Debug)]
pub struct BackgroundWorker {
    stop: Arc<AtomicBool>,
    stats: Arc<BackgroundWorkerCounters>,
    runtime: SharedCollectorRuntime,
    handle: Option<JoinHandle<Result<(), BackgroundWorkerError>>>,
}

#[derive(Debug, Default)]
struct BackgroundWorkerCounters {
    loops: AtomicU64,
    idle_loops: AtomicU64,
    wait_loops: AtomicU64,
    snapshot_idle_loops: AtomicU64,
    signal_wakeups: AtomicU64,
    background_change_wakeups: AtomicU64,
    ignored_signal_wakeups: AtomicU64,
    contention_loops: AtomicU64,
    collector_ticks: AtomicU64,
    collector_rounds: AtomicU64,
    collector_sessions_started: AtomicU64,
    collector_sessions_finished: AtomicU64,
}

impl BackgroundWorkerCounters {
    fn snapshot(&self) -> BackgroundWorkerStats {
        BackgroundWorkerStats {
            loops: self.loops.load(Ordering::Relaxed),
            idle_loops: self.idle_loops.load(Ordering::Relaxed),
            wait_loops: self.wait_loops.load(Ordering::Relaxed),
            snapshot_idle_loops: self.snapshot_idle_loops.load(Ordering::Relaxed),
            signal_wakeups: self.signal_wakeups.load(Ordering::Relaxed),
            background_change_wakeups: self.background_change_wakeups.load(Ordering::Relaxed),
            ignored_signal_wakeups: self.ignored_signal_wakeups.load(Ordering::Relaxed),
            contention_loops: self.contention_loops.load(Ordering::Relaxed),
            collector: BackgroundCollectorStats {
                ticks: self.collector_ticks.load(Ordering::Relaxed),
                rounds: self.collector_rounds.load(Ordering::Relaxed),
                sessions_started: self.collector_sessions_started.load(Ordering::Relaxed),
                sessions_finished: self.collector_sessions_finished.load(Ordering::Relaxed),
            },
        }
    }

    fn add_loops(&self, delta: u64) {
        self.loops.fetch_add(delta, Ordering::Relaxed);
    }

    fn add_idle_loops(&self, delta: u64) {
        self.idle_loops.fetch_add(delta, Ordering::Relaxed);
    }

    fn add_wait_loops(&self, delta: u64) {
        self.wait_loops.fetch_add(delta, Ordering::Relaxed);
    }

    fn add_snapshot_idle_loops(&self, delta: u64) {
        self.snapshot_idle_loops.fetch_add(delta, Ordering::Relaxed);
    }

    fn add_signal_wakeups(&self, delta: u64) {
        self.signal_wakeups.fetch_add(delta, Ordering::Relaxed);
    }

    fn add_background_change_wakeups(&self, delta: u64) {
        self.background_change_wakeups
            .fetch_add(delta, Ordering::Relaxed);
    }

    fn add_ignored_signal_wakeups(&self, delta: u64) {
        self.ignored_signal_wakeups
            .fetch_add(delta, Ordering::Relaxed);
    }

    fn add_contention_loops(&self, delta: u64) {
        self.contention_loops.fetch_add(delta, Ordering::Relaxed);
    }

    fn store_collector(&self, collector: BackgroundCollectorStats) {
        self.collector_ticks
            .store(collector.ticks, Ordering::Relaxed);
        self.collector_rounds
            .store(collector.rounds, Ordering::Relaxed);
        self.collector_sessions_started
            .store(collector.sessions_started, Ordering::Relaxed);
        self.collector_sessions_finished
            .store(collector.sessions_finished, Ordering::Relaxed);
    }
}

impl BackgroundWorker {
    pub(crate) fn spawn(runtime: SharedCollectorRuntime, config: BackgroundWorkerConfig) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(BackgroundWorkerCounters::default());
        let worker_stop = Arc::clone(&stop);
        let worker_stats = Arc::clone(&stats);
        let worker_runtime = runtime.clone();
        let handle =
            thread::spawn(move || worker_loop(worker_runtime, config, worker_stop, worker_stats));
        Self {
            stop,
            stats,
            runtime,
            handle: Some(handle),
        }
    }

    /// Request that the worker stop after its current loop iteration.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
        self.runtime.notify_waiters();
        self.runtime.notify_background_waiters();
    }

    /// Return whether the worker thread has already finished.
    pub fn is_finished(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
    }

    /// Return a snapshot of current worker statistics.
    pub fn stats(&self) -> Result<BackgroundWorkerStats, BackgroundWorkerError> {
        Ok(self.stats.snapshot())
    }

    /// Return a combined snapshot of worker and shared heap state.
    pub fn status(&self) -> Result<BackgroundWorkerStatus, BackgroundWorkerError> {
        Ok(BackgroundWorkerStatus {
            worker: self.stats()?,
            heap: self
                .runtime
                .status()
                .map_err(|_| BackgroundWorkerError::LockPoisoned)?,
        })
    }

    /// Stop the worker and join its thread, returning final worker statistics.
    pub fn join(mut self) -> Result<BackgroundWorkerStats, BackgroundWorkerError> {
        self.request_stop();
        let Some(handle) = self.handle.take() else {
            return self.stats();
        };
        match handle.join() {
            Ok(Ok(())) => self.stats(),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(BackgroundWorkerError::WorkerPanicked),
        }
    }
}

fn background_wait_duration(
    status: &BackgroundCollectionStatus,
    config: &BackgroundWorkerConfig,
) -> Duration {
    match status {
        BackgroundCollectionStatus::Idle => config.idle_sleep,
        BackgroundCollectionStatus::ReadyToFinish(_) | BackgroundCollectionStatus::Finished(_) => {
            config.busy_sleep
        }
        BackgroundCollectionStatus::Progress(_) => Duration::ZERO,
    }
}

fn worker_loop(
    runtime: SharedCollectorRuntime,
    config: BackgroundWorkerConfig,
    stop: Arc<AtomicBool>,
    stats: Arc<BackgroundWorkerCounters>,
) -> Result<(), BackgroundWorkerError> {
    let mut collector = BackgroundCollector::new(config.collector);

    let wait_for_signal = |stats: &Arc<BackgroundWorkerCounters>,
                           runtime: &SharedCollectorRuntime,
                           stop: &Arc<AtomicBool>,
                           observed_signal_epoch: &mut u64,
                           observed_background: &mut CollectorSharedSnapshot,
                           timeout: Duration|
     -> Result<(), BackgroundWorkerError> {
        if timeout.is_zero() {
            return Ok(());
        }

        stats.add_wait_loops(1);

        let (signal_changed, collector_changed) = runtime
            .wait_for_collector_change(
                observed_signal_epoch,
                observed_background,
                timeout,
                Some(stop),
            )
            .map_err(|_| BackgroundWorkerError::LockPoisoned)?;

        if signal_changed {
            stats.add_signal_wakeups(1);
        }
        if collector_changed {
            stats.add_background_change_wakeups(1);
        } else if signal_changed && !stop.load(Ordering::Acquire) {
            stats.add_ignored_signal_wakeups(1);
        }

        Ok(())
    };

    while !stop.load(Ordering::Acquire) {
        let snapshot = runtime
            .collector_snapshot()
            .map_err(|_| BackgroundWorkerError::LockPoisoned)?;
        if let Some(status) = collector.snapshot_tick(&snapshot) {
            stats.add_loops(1);
            if matches!(status, BackgroundCollectionStatus::Idle) {
                stats.add_idle_loops(1);
                stats.add_snapshot_idle_loops(1);
            }
            stats.store_collector(collector.stats());
            let (mut observed_signal_epoch, mut observed_background) = runtime
                .collector_observation()
                .map_err(|_| BackgroundWorkerError::LockPoisoned)?;
            let wait_for = match status {
                BackgroundCollectionStatus::Idle => config.idle_sleep,
                BackgroundCollectionStatus::ReadyToFinish(_)
                | BackgroundCollectionStatus::Finished(_) => config.busy_sleep,
                BackgroundCollectionStatus::Progress(_) => Duration::ZERO,
            };
            wait_for_signal(
                &stats,
                &runtime,
                &stop,
                &mut observed_signal_epoch,
                &mut observed_background,
                wait_for,
            )?;
            continue;
        }

        let status = match collector.try_tick_shared_after_snapshot(&runtime) {
            Ok(status) => status,
            Err(SharedBackgroundError::Collection(error)) => {
                return Err(BackgroundWorkerError::Collection(error));
            }
            Err(SharedBackgroundError::LockPoisoned) => {
                return Err(BackgroundWorkerError::LockPoisoned);
            }
            Err(SharedBackgroundError::WouldBlock) => {
                let blocked_status = collector.blocked_status_from_snapshot(&snapshot);
                stats.add_loops(1);
                stats.add_contention_loops(1);
                if blocked_status.is_none() {
                    stats.add_idle_loops(1);
                }
                stats.store_collector(collector.stats());
                let (mut observed_signal_epoch, mut observed_background) = runtime
                    .collector_observation()
                    .map_err(|_| BackgroundWorkerError::LockPoisoned)?;
                let wait_for = blocked_status
                    .as_ref()
                    .map(|status| background_wait_duration(status, &config))
                    .unwrap_or(config.idle_sleep);
                wait_for_signal(
                    &stats,
                    &runtime,
                    &stop,
                    &mut observed_signal_epoch,
                    &mut observed_background,
                    wait_for,
                )?;
                continue;
            }
        };

        let (mut observed_signal_epoch, mut observed_background) = runtime
            .collector_observation()
            .map_err(|_| BackgroundWorkerError::LockPoisoned)?;

        stats.add_loops(1);
        if matches!(status, BackgroundCollectionStatus::Idle) {
            stats.add_idle_loops(1);
        }
        stats.store_collector(collector.stats());

        let sleep_for = background_wait_duration(&status, &config);
        wait_for_signal(
            &stats,
            &runtime,
            &stop,
            &mut observed_signal_epoch,
            &mut observed_background,
            sleep_for,
        )?;
    }

    Ok(())
}
