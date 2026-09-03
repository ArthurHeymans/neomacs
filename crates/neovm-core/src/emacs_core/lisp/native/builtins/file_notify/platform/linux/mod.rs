//! Linux inotify adapter.
//!
//! This layer preserves the native mask and rename cookie until Lisp event
//! encoding.  A generic filesystem-event vocabulary is too lossy for GNU's
//! low-level `inotify-*` contract (notably `dont-follow`, `onlydir`, combined
//! bits, `isdir`, `unmount`, and terminal `ignored`).

use super::super::{
    DrainBatch, FileNotifyBackend, FileNotifyEvent, FileWatch, WatchActivity, WatchId,
    file_notify_error,
};
use crate::emacs_core::error::Flow;
use crate::emacs_core::process::WaitNotifier;
use crate::emacs_core::value::Value;
use inotify::{EventMask, WatchMask};
use std::path::{Path, PathBuf};

mod lisp;
mod worker;

pub(crate) use lisp::{inotify_add_watch, inotify_rm_watch, inotify_valid_p};

#[cfg(test)]
mod linux_test;

use worker::{NativeEvent, Worker};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct InotifyRequest {
    watch_mask: WatchMask,
    event_mask: EventMask,
}

impl InotifyRequest {
    pub(in super::super) fn new(aspects: Vec<String>) -> Self {
        let mut watch_mask = WatchMask::EXCL_UNLINK | WatchMask::MASK_ADD;
        let mut event_mask = EventMask::empty();
        for aspect in aspects {
            match aspect.as_str() {
                "access" => {
                    watch_mask.insert(WatchMask::ACCESS);
                    event_mask.insert(EventMask::ACCESS);
                }
                "attrib" => {
                    watch_mask.insert(WatchMask::ATTRIB);
                    event_mask.insert(EventMask::ATTRIB);
                }
                "close-write" => {
                    watch_mask.insert(WatchMask::CLOSE_WRITE);
                    event_mask.insert(EventMask::CLOSE_WRITE);
                }
                "close-nowrite" => {
                    watch_mask.insert(WatchMask::CLOSE_NOWRITE);
                    event_mask.insert(EventMask::CLOSE_NOWRITE);
                }
                "create" => {
                    watch_mask.insert(WatchMask::CREATE);
                    event_mask.insert(EventMask::CREATE);
                }
                "delete" => {
                    watch_mask.insert(WatchMask::DELETE);
                    event_mask.insert(EventMask::DELETE);
                }
                "delete-self" => {
                    watch_mask.insert(WatchMask::DELETE_SELF);
                    event_mask.insert(EventMask::DELETE_SELF);
                }
                "modify" => {
                    watch_mask.insert(WatchMask::MODIFY);
                    event_mask.insert(EventMask::MODIFY);
                }
                "move-self" => {
                    watch_mask.insert(WatchMask::MOVE_SELF);
                    event_mask.insert(EventMask::MOVE_SELF);
                }
                "moved-from" => {
                    watch_mask.insert(WatchMask::MOVED_FROM);
                    event_mask.insert(EventMask::MOVED_FROM);
                }
                "moved-to" => {
                    watch_mask.insert(WatchMask::MOVED_TO);
                    event_mask.insert(EventMask::MOVED_TO);
                }
                "open" => {
                    watch_mask.insert(WatchMask::OPEN);
                    event_mask.insert(EventMask::OPEN);
                }
                "move" => {
                    watch_mask.insert(WatchMask::MOVE);
                    event_mask.insert(EventMask::MOVED_FROM | EventMask::MOVED_TO);
                }
                "close" => {
                    watch_mask.insert(WatchMask::CLOSE);
                    event_mask.insert(EventMask::CLOSE_WRITE | EventMask::CLOSE_NOWRITE);
                }
                "dont-follow" => watch_mask.insert(WatchMask::DONT_FOLLOW),
                "onlydir" => watch_mask.insert(WatchMask::ONLYDIR),
                "all-events" | "t" => {
                    watch_mask.insert(WatchMask::ALL_EVENTS);
                    event_mask.insert(EventMask::from_bits_retain(WatchMask::ALL_EVENTS.bits()));
                }
                // These are kernel-generated result bits.  GNU accepts them in
                // ASPECT, while delivery remains unconditional when they occur.
                "ignored" | "unmount" => {}
                _ => unreachable!("Lisp validation rejects unknown inotify aspects"),
            }
        }
        Self {
            watch_mask,
            event_mask,
        }
    }

    fn accepts(&self, mask: EventMask) -> bool {
        mask.intersects(self.event_mask)
            || mask.intersects(
                EventMask::IGNORED | EventMask::ISDIR | EventMask::Q_OVERFLOW | EventMask::UNMOUNT,
            )
    }
}

#[derive(Clone, Debug)]
pub(in super::super) struct InotifyEvent {
    watch_id: WatchId,
    aspects: Vec<&'static str>,
    path: PathBuf,
    cookie: u32,
}

impl FileNotifyEvent for InotifyEvent {
    fn watch_id(&self) -> &WatchId {
        &self.watch_id
    }

    fn into_lisp(self) -> Value {
        // GNU inotify events are `(DESCRIPTOR ASPECTS NAME COOKIE)`.
        Value::list(vec![
            self.watch_id.to_inotify_lisp(),
            Value::list(self.aspects.into_iter().map(Value::symbol).collect()),
            Value::string(self.path.display().to_string()),
            Value::fixnum(i64::from(self.cookie)),
        ])
    }
}

#[derive(Clone, Debug)]
struct InotifyWatch {
    common: FileWatch<InotifyRequest>,
    native_descriptor: i32,
    activity: WatchActivity,
}

#[derive(Default)]
pub(in super::super) struct InotifyBackend {
    worker: Option<Worker>,
    watches: Vec<InotifyWatch>,
    next_id: i64,
}

impl InotifyBackend {
    fn ensure_worker(&mut self, notifier: Option<WaitNotifier>) -> Result<&mut Worker, Flow> {
        if self.worker.is_none() {
            self.worker = Some(Worker::start(notifier).map_err(|error| {
                file_notify_error("File watching is not available", Some(error), None)
            })?);
        }
        Ok(self.worker.as_mut().expect("worker was initialized"))
    }

    fn allocate_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("file notification descriptor space exhausted");
        id
    }

    fn aspects(mask: EventMask) -> Vec<&'static str> {
        // GNU's C implementation conses in the opposite order from its bit
        // probes, producing this observable list order.
        [
            (EventMask::UNMOUNT, "unmount"),
            (EventMask::Q_OVERFLOW, "q-overflow"),
            (EventMask::ISDIR, "isdir"),
            (EventMask::IGNORED, "ignored"),
            (EventMask::OPEN, "open"),
            (EventMask::MOVED_TO, "moved-to"),
            (EventMask::MOVED_FROM, "moved-from"),
            (EventMask::MOVE_SELF, "move-self"),
            (EventMask::MODIFY, "modify"),
            (EventMask::DELETE_SELF, "delete-self"),
            (EventMask::DELETE, "delete"),
            (EventMask::CREATE, "create"),
            (EventMask::CLOSE_NOWRITE, "close-nowrite"),
            (EventMask::CLOSE_WRITE, "close-write"),
            (EventMask::ATTRIB, "attrib"),
            (EventMask::ACCESS, "access"),
        ]
        .into_iter()
        .filter_map(|(bit, name)| mask.contains(bit).then_some(name))
        .collect()
    }

    fn translate_event(&self, event: NativeEvent) -> Vec<InotifyEvent> {
        let queue_overflow = event.mask.contains(EventMask::Q_OVERFLOW);
        self.watches
            .iter()
            .filter(|watch| queue_overflow || watch.native_descriptor == event.descriptor)
            .filter(|watch| watch.common.request.accepts(event.mask))
            .map(|watch| InotifyEvent {
                watch_id: watch.common.id.clone(),
                aspects: Self::aspects(event.mask),
                path: event
                    .name
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| watch.common.path.clone()),
                cookie: event.cookie,
            })
            .collect()
    }

    fn overflow_events(&self) -> Vec<InotifyEvent> {
        self.watches
            .iter()
            .map(|watch| InotifyEvent {
                watch_id: watch.common.id.clone(),
                aspects: vec!["q-overflow"],
                path: watch.common.path.clone(),
                cookie: 0,
            })
            .collect()
    }
}

impl FileNotifyBackend for InotifyBackend {
    type Request = InotifyRequest;
    type Event = InotifyEvent;

    fn add_watch(
        &mut self,
        path: &Path,
        request: Self::Request,
        notifier: Option<WaitNotifier>,
    ) -> Result<WatchId, Flow> {
        let add_result = self
            .ensure_worker(notifier)?
            .add(path.to_path_buf(), request.watch_mask);
        let (native_descriptor, activity) = match add_result {
            Ok((descriptor, activity)) => (descriptor, activity),
            Err(error) => {
                if self.watches.is_empty() {
                    self.worker = None;
                }
                return Err(file_notify_error(
                    "Could not add watch for file",
                    Some(error),
                    Some(Value::string(path.display().to_string())),
                ));
            }
        };
        let descriptor = WatchId::new(self.allocate_id(), 0);
        self.watches.push(InotifyWatch {
            common: FileWatch {
                id: descriptor.clone(),
                path: path.to_path_buf(),
                request,
            },
            native_descriptor,
            activity,
        });
        Ok(descriptor)
    }

    fn remove_watch(&mut self, descriptor: &WatchId) -> Result<bool, Flow> {
        let Some(index) = self
            .watches
            .iter()
            .position(|watch| watch.common.id == *descriptor)
        else {
            return Ok(false);
        };
        let removed = self.watches.remove(index);
        if !self
            .watches
            .iter()
            .any(|watch| watch.native_descriptor == removed.native_descriptor)
        {
            self.worker
                .as_ref()
                .expect("a live watch has a worker")
                .remove(removed.native_descriptor)
                .map_err(|error| file_notify_error("Could not rm watch", Some(error), None))?;
        }
        if self.watches.is_empty() {
            self.worker = None;
        }
        Ok(true)
    }

    fn valid_p(&self, descriptor: &WatchId) -> bool {
        self.watches
            .iter()
            .any(|watch| watch.common.id == *descriptor && watch.activity.is_active())
    }

    fn drain_events(&mut self) -> Result<DrainBatch<Self::Event>, Flow> {
        let mut events = Vec::new();
        let mut overflowed = false;
        let mut failure = None;
        if let Some(worker) = self.worker.as_ref() {
            if worker.take_overflow() {
                overflowed = true;
                tracing::warn!(
                    capacity = super::super::delivery::EVENT_CAPACITY,
                    "inotify delivery queue overflowed; requesting conservative rescan"
                );
            }
            loop {
                match worker.try_recv() {
                    Ok(Ok(event)) => {
                        events.extend(self.translate_event(event));
                    }
                    Ok(Err(error)) => {
                        failure = Some(file_notify_error(
                            "Error while retrieving file system events",
                            Some(error),
                            None,
                        ));
                        break;
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                    Err(crossbeam_channel::TryRecvError::Disconnected) => break,
                }
            }
        }
        let terminated = self
            .watches
            .iter()
            .filter(|watch| !watch.activity.is_active())
            .map(|watch| watch.common.id.clone())
            .collect::<Vec<_>>();
        self.watches.retain(|watch| watch.activity.is_active());
        if overflowed {
            events.extend(self.overflow_events());
        }
        if self.watches.is_empty() {
            self.worker = None;
        }
        Ok(DrainBatch {
            events,
            terminated,
            failure,
        })
    }

    fn has_watches(&self) -> bool {
        !self.watches.is_empty()
    }
}
