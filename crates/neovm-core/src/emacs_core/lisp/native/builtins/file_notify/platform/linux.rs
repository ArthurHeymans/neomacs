use super::super::{FileNotifyBackend, FileNotifyEvent, FileWatch, WatchId, file_notify_error};
use crate::emacs_core::error::Flow;
use crate::emacs_core::process::WaitNotifier;
use crate::emacs_core::value::Value;
use notify::Watcher;
use notify::event::{AccessKind, AccessMode, ModifyKind, RenameMode};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct InotifyRequest {
    aspects: Vec<String>,
}

impl InotifyRequest {
    pub(in super::super) fn new(aspects: Vec<String>) -> Self {
        Self { aspects }
    }
}

#[derive(Clone, Debug)]
pub(in super::super) struct InotifyEvent {
    watch_id: WatchId,
    aspects: Vec<&'static str>,
    path: PathBuf,
    cookie: usize,
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
            Value::fixnum(i64::try_from(self.cookie).unwrap_or(i64::MAX)),
        ])
    }
}

#[derive(Default)]
pub(in super::super) struct InotifyBackend {
    watcher: Option<notify::RecommendedWatcher>,
    rx: Option<std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>>,
    watches: Vec<FileWatch<InotifyRequest>>,
    next_id: i64,
}

impl InotifyBackend {
    fn ensure_watcher(&mut self, notifier: Option<WaitNotifier>) -> Result<(), Flow> {
        if self.watcher.is_some() {
            return Ok(());
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let watcher = notify::RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if tx.send(res).is_ok()
                    && let Some(notifier) = notifier.as_ref()
                    && let Err(error) = notifier.notify()
                {
                    tracing::error!(%error, "failed to wake evaluator for file notification");
                }
            },
            notify::Config::default(),
        )
        .map_err(|e| {
            file_notify_error("File watching is not available", Some(e.to_string()), None)
        })?;
        self.watcher = Some(watcher);
        self.rx = Some(rx);
        Ok(())
    }

    fn allocate_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn watch_requests(watch: &FileWatch<InotifyRequest>, aspect: &str) -> bool {
        watch
            .request
            .aspects
            .iter()
            .any(|requested| match requested.as_str() {
                "t" | "all-events" => true,
                "move" => matches!(aspect, "moved-from" | "moved-to"),
                "close" => matches!(aspect, "close-write" | "close-nowrite"),
                requested => requested == aspect,
            })
    }

    fn watch_matches_path(watch: &FileWatch<InotifyRequest>, event_path: &Path) -> bool {
        if watch.is_directory {
            event_path == watch.path || event_path.parent() == Some(watch.path.as_path())
        } else {
            event_path == watch.path
        }
    }

    fn reported_path(watch: &FileWatch<InotifyRequest>, event_path: &Path) -> PathBuf {
        if watch.is_directory && event_path != watch.path {
            event_path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| event_path.to_path_buf())
        } else {
            event_path.to_path_buf()
        }
    }

    fn event_aspects(
        event: &notify::Event,
        path_index: usize,
        watch: &FileWatch<InotifyRequest>,
    ) -> Vec<&'static str> {
        if event.need_rescan() {
            return vec!["q-overflow"];
        }

        match event.kind {
            notify::EventKind::Access(AccessKind::Open(_)) => vec!["open"],
            notify::EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
                vec!["close-write"]
            }
            notify::EventKind::Access(AccessKind::Close(_)) => vec!["close-nowrite"],
            notify::EventKind::Access(_) => vec!["access"],
            notify::EventKind::Create(_) => vec!["create"],
            notify::EventKind::Modify(ModifyKind::Data(_)) => vec!["modify"],
            notify::EventKind::Modify(ModifyKind::Metadata(_)) => vec!["attrib"],
            notify::EventKind::Modify(ModifyKind::Name(RenameMode::From)) => vec!["moved-from"],
            notify::EventKind::Modify(ModifyKind::Name(RenameMode::To)) => vec!["moved-to"],
            notify::EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                if path_index == 0 {
                    vec!["moved-from"]
                } else {
                    vec!["moved-to"]
                }
            }
            notify::EventKind::Modify(ModifyKind::Name(_)) => vec!["move-self"],
            notify::EventKind::Modify(_) => vec!["modify"],
            notify::EventKind::Remove(_) => {
                if event.paths.get(path_index) == Some(&watch.path) {
                    vec!["delete-self"]
                } else {
                    vec!["delete"]
                }
            }
            notify::EventKind::Any | notify::EventKind::Other => Vec::new(),
        }
    }

    fn translate_event(&self, event: notify::Event) -> Vec<InotifyEvent> {
        let cookie = event.tracker().unwrap_or(0);
        let mut translated = Vec::new();

        for watch in &self.watches {
            for (path_index, path) in event.paths.iter().enumerate() {
                if !Self::watch_matches_path(watch, path) {
                    continue;
                }
                let aspects = Self::event_aspects(&event, path_index, watch);
                let aspects: Vec<_> = aspects
                    .into_iter()
                    .filter(|aspect| {
                        matches!(*aspect, "q-overflow" | "ignored" | "unmount")
                            || Self::watch_requests(watch, aspect)
                    })
                    .collect();
                if aspects.is_empty() {
                    continue;
                }
                translated.push(InotifyEvent {
                    watch_id: watch.id.clone(),
                    aspects,
                    path: Self::reported_path(watch, path),
                    cookie,
                });
            }
        }

        translated
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
        self.ensure_watcher(notifier)?;

        if !path.exists() {
            return Err(file_notify_error(
                "Could not add watch for file",
                Some("No such file or directory".to_string()),
                Some(Value::string(path.display().to_string())),
            ));
        }
        if request.aspects.iter().any(|aspect| aspect == "onlydir") && !path.is_dir() {
            return Err(file_notify_error(
                "Could not add watch for file",
                Some("Not a directory".to_owned()),
                Some(Value::string(path.display().to_string())),
            ));
        }
        let path_already_watched = self.watches.iter().any(|watch| watch.path == path);
        if !path_already_watched && let Some(ref mut watcher) = self.watcher {
            watcher
                .watch(path, notify::RecursiveMode::NonRecursive)
                .map_err(|e| {
                    file_notify_error(
                        "Could not add watch for file",
                        Some(e.to_string()),
                        Some(Value::string(path.display().to_string())),
                    )
                })?;
        }

        let id = self.allocate_id();
        let descriptor = WatchId::new(id, 0);
        self.watches.push(FileWatch {
            id: descriptor.clone(),
            path: path.to_path_buf(),
            is_directory: path.is_dir(),
            request,
        });

        Ok(descriptor)
    }

    fn remove_watch(&mut self, descriptor: &WatchId) -> Result<bool, Flow> {
        let Some(pos) = self
            .watches
            .iter()
            .position(|watch| watch.id == *descriptor)
        else {
            return Ok(false);
        };

        let removed = self.watches.remove(pos);
        let path_still_watched = self.watches.iter().any(|watch| watch.path == removed.path);
        if !path_still_watched && let Some(ref mut watcher) = self.watcher {
            let _ = watcher.unwatch(&removed.path);
        }

        if self.watches.is_empty() {
            self.watcher = None;
            self.rx = None;
        }

        Ok(true)
    }

    fn valid_p(&self, descriptor: &WatchId) -> bool {
        self.watches.iter().any(|watch| watch.id == *descriptor)
    }

    fn drain_events(&mut self) -> Result<Vec<Self::Event>, Flow> {
        let mut raw_events = Vec::new();
        if let Some(rx) = self.rx.as_ref() {
            loop {
                match rx.try_recv() {
                    Ok(Ok(event)) => raw_events.push(event),
                    Ok(Err(error)) => {
                        return Err(file_notify_error(
                            "Error while retrieving file system events",
                            Some(error.to_string()),
                            None,
                        ));
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }

        let mut translated = Vec::new();
        for event in raw_events {
            translated.extend(self.translate_event(event));
        }
        Ok(translated)
    }

    fn has_watches(&self) -> bool {
        !self.watches.is_empty()
    }
}
