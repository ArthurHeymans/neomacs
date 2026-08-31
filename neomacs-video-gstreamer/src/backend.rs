use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};
use neomacs_display_protocol::types::VideoId;

use crate::{
    FrameTiming, VideoColorimetry, VideoCommand, VideoCommandError, VideoFrameFormat,
    VideoGeometry, VideoWake,
};

pub(crate) struct DecodedFrame<F> {
    pub(crate) lease: F,
    pub(crate) timing: FrameTiming,
    pub(crate) geometry: VideoGeometry,
    pub(crate) format: VideoFrameFormat,
    pub(crate) colorimetry: VideoColorimetry,
}

pub(crate) enum BackendEvent<F> {
    Opened {
        id: VideoId,
        width: u32,
        height: u32,
        initial_state: crate::VideoSessionState,
    },
    Frame {
        id: VideoId,
        frame: DecodedFrame<F>,
    },
    FramesReplaced {
        id: VideoId,
        count: u64,
    },
    StateChanged {
        id: VideoId,
        state: crate::VideoSessionState,
    },
    Looped {
        id: VideoId,
        remaining: crate::LoopMode,
    },
    Ended {
        id: VideoId,
    },
    Failed {
        id: VideoId,
        error: VideoCommandError,
    },
}

pub(crate) struct BackendPublisher<F> {
    events: Sender<BackendEvent<F>>,
    latest_frames: Arc<Mutex<HashMap<VideoId, LatestPublishedFrame<F>>>>,
    wake: VideoWake,
}

impl<F> Clone for BackendPublisher<F> {
    fn clone(&self) -> Self {
        Self {
            events: self.events.clone(),
            latest_frames: Arc::clone(&self.latest_frames),
            wake: self.wake.clone(),
        }
    }
}

pub(crate) struct BackendInbox<F> {
    events: Receiver<BackendEvent<F>>,
    latest_frames: Arc<Mutex<HashMap<VideoId, LatestPublishedFrame<F>>>>,
}

struct LatestPublishedFrame<F> {
    frame: DecodedFrame<F>,
    replaced: u64,
}

pub(crate) fn backend_bridge<F>(wake: VideoWake) -> (BackendPublisher<F>, BackendInbox<F>) {
    let (events, incoming) = crossbeam_channel::unbounded();
    let latest_frames = Arc::new(Mutex::new(HashMap::new()));
    (
        BackendPublisher {
            events,
            latest_frames: Arc::clone(&latest_frames),
            wake,
        },
        BackendInbox {
            events: incoming,
            latest_frames,
        },
    )
}

impl<F> BackendPublisher<F> {
    pub(crate) fn event(&self, event: BackendEvent<F>) {
        if self.events.send(event).is_ok() {
            self.wake.notify_for_backend();
        }
    }

    pub(crate) fn frame(&self, id: VideoId, frame: DecodedFrame<F>) {
        let mut latest = lock_unpoisoned(&self.latest_frames);
        let replaced = latest
            .remove(&id)
            .map_or(0, |previous| previous.replaced.saturating_add(1));
        latest.insert(id, LatestPublishedFrame { frame, replaced });
        drop(latest);
        self.wake.notify_for_backend();
    }
}

impl<F> BackendInbox<F> {
    pub(crate) fn drain(&self) -> Vec<BackendEvent<F>> {
        let mut events: Vec<_> = self.events.try_iter().collect();
        let frames = std::mem::take(&mut *lock_unpoisoned(&self.latest_frames));
        for (id, published) in frames {
            events.push(BackendEvent::Frame {
                id,
                frame: published.frame,
            });
            if published.replaced != 0 {
                events.push(BackendEvent::FramesReplaced {
                    id,
                    count: published.replaced,
                });
            }
        }
        events
    }

    pub(crate) fn remove_frame(&self, id: VideoId) {
        lock_unpoisoned(&self.latest_frames).remove(&id);
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(crate) trait DecoderBackend {
    type Frame;

    fn command(&mut self, command: VideoCommand) -> Result<(), VideoCommandError>;
    fn drain_events(&mut self) -> Vec<BackendEvent<Self::Frame>>;
}

#[cfg(test)]
#[path = "backend_test.rs"]
mod tests;
