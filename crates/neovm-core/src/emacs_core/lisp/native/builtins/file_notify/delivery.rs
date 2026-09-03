//! Bounded cross-thread delivery with coalesced overflow state.
//!
//! Filesystem bursts are external input and must not grow evaluator memory
//! without bound.  Producers therefore publish into a fixed-capacity queue.
//! Once full, an atomic latch records that consumers must conservatively
//! rescan; a wakeup is still sent so overflow cannot remain invisible.

use crate::emacs_core::process::WaitNotifier;
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) const EVENT_CAPACITY: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PublishOutcome {
    Published,
    Overflowed,
    Closed,
}

pub(super) struct DeliverySender<T> {
    sender: Sender<T>,
    overflowed: Arc<AtomicBool>,
    notifier: Option<WaitNotifier>,
}

impl<T> Clone for DeliverySender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            overflowed: Arc::clone(&self.overflowed),
            notifier: self.notifier.clone(),
        }
    }
}

pub(super) struct DeliveryReceiver<T> {
    receiver: Receiver<T>,
    overflowed: Arc<AtomicBool>,
}

pub(super) fn channel<T>(
    notifier: Option<WaitNotifier>,
) -> (DeliverySender<T>, DeliveryReceiver<T>) {
    channel_with_capacity(EVENT_CAPACITY, notifier)
}

fn channel_with_capacity<T>(
    capacity: usize,
    notifier: Option<WaitNotifier>,
) -> (DeliverySender<T>, DeliveryReceiver<T>) {
    let (sender, receiver) = crossbeam_channel::bounded(capacity);
    let overflowed = Arc::new(AtomicBool::new(false));
    (
        DeliverySender {
            sender,
            overflowed: Arc::clone(&overflowed),
            notifier,
        },
        DeliveryReceiver {
            receiver,
            overflowed,
        },
    )
}

impl<T> DeliverySender<T> {
    pub(super) fn publish(&self, item: T) -> PublishOutcome {
        let outcome = match self.sender.try_send(item) {
            Ok(()) => PublishOutcome::Published,
            Err(TrySendError::Full(_)) => {
                self.overflowed.store(true, Ordering::Release);
                PublishOutcome::Overflowed
            }
            Err(TrySendError::Disconnected(_)) => return PublishOutcome::Closed,
        };
        if let Some(notifier) = self.notifier.as_ref()
            && let Err(error) = notifier.notify()
        {
            tracing::error!(%error, "failed to wake evaluator for file notification");
        }
        outcome
    }
}

impl<T> DeliveryReceiver<T> {
    pub(super) fn try_recv(&self) -> Result<T, TryRecvError> {
        self.receiver.try_recv()
    }

    pub(super) fn take_overflow(&self) -> bool {
        self.overflowed.swap(false, Ordering::AcqRel)
    }
}

#[cfg(test)]
#[path = "delivery_test.rs"]
mod tests;
