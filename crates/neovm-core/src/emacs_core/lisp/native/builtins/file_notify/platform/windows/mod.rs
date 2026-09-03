#[cfg(target_os = "windows")]
use super::super::{DrainBatch, FileNotifyBackend, FileWatch, file_notify_error};
use super::super::{FileNotifyEvent, WatchId};
#[cfg(target_os = "windows")]
use crate::emacs_core::error::Flow;
use crate::emacs_core::value::Value;
use enumflags2::BitFlags;
use notify::event::{AccessKind, ModifyKind, RenameMode};
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;

#[enumflags2::bitflags]
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum W32Filter {
    FileName = 1 << 0,
    DirectoryName = 1 << 1,
    Attributes = 1 << 2,
    Size = 1 << 3,
    LastWriteTime = 1 << 4,
    LastAccessTime = 1 << 5,
    CreationTime = 1 << 6,
    SecurityDescriptor = 1 << 7,
    Subtree = 1 << 8,
}

impl W32Filter {
    pub(in super::super) fn from_lisp_name(name: &str) -> Option<Self> {
        match name {
            "file-name" => Some(Self::FileName),
            "directory-name" => Some(Self::DirectoryName),
            "attributes" => Some(Self::Attributes),
            "size" => Some(Self::Size),
            "last-write-time" => Some(Self::LastWriteTime),
            "last-access-time" => Some(Self::LastAccessTime),
            "creation-time" => Some(Self::CreationTime),
            "security-desc" => Some(Self::SecurityDescriptor),
            "subtree" => Some(Self::Subtree),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct W32Request {
    filters: BitFlags<W32Filter>,
}

impl W32Request {
    pub(in super::super) fn new(filters: BitFlags<W32Filter>) -> Self {
        Self { filters }
    }

    fn recursive(&self) -> bool {
        self.filters.contains(W32Filter::Subtree)
    }

    fn accepts_names(&self) -> bool {
        self.filters
            .intersects(W32Filter::FileName | W32Filter::DirectoryName)
    }

    fn accepts_modification(&self) -> bool {
        self.filters.intersects(
            W32Filter::Attributes
                | W32Filter::Size
                | W32Filter::LastWriteTime
                | W32Filter::LastAccessTime
                | W32Filter::CreationTime
                | W32Filter::SecurityDescriptor,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WatchMode {
    Direct,
    Recursive,
}

impl WatchMode {
    fn from_recursive(recursive: bool) -> Self {
        if recursive {
            Self::Recursive
        } else {
            Self::Direct
        }
    }

    fn covers(self, requested: Self) -> bool {
        self == Self::Recursive || self == requested
    }

    #[cfg(target_os = "windows")]
    fn into_notify(self) -> notify::RecursiveMode {
        match self {
            Self::Direct => notify::RecursiveMode::NonRecursive,
            Self::Recursive => notify::RecursiveMode::Recursive,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum W32Action {
    Added,
    Removed,
    Modified,
    RenamedFrom,
    RenamedTo,
}

impl W32Action {
    const fn as_lisp_name(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Modified => "modified",
            Self::RenamedFrom => "renamed-from",
            Self::RenamedTo => "renamed-to",
        }
    }
}

#[derive(Clone, Debug)]
pub(in super::super) struct W32Event {
    watch_id: WatchId,
    action: W32Action,
    path: PathBuf,
}

impl FileNotifyEvent for W32Event {
    fn watch_id(&self) -> &WatchId {
        &self.watch_id
    }

    fn into_lisp(self) -> Value {
        // GNU w32notify events are `(DESCRIPTOR ACTION FILE)` and use a
        // pointer-like integer as the opaque descriptor.
        Value::list(vec![
            Value::fixnum(self.watch_id.slot()),
            Value::symbol(self.action.as_lisp_name()),
            Value::string(self.path.display().to_string()),
        ])
    }
}

fn event_actions(event: &notify::Event, request: &W32Request) -> Vec<(usize, W32Action)> {
    match event.kind {
        notify::EventKind::Create(_) if request.accepts_names() => vec![(0, W32Action::Added)],
        notify::EventKind::Remove(_) if request.accepts_names() => vec![(0, W32Action::Removed)],
        notify::EventKind::Modify(ModifyKind::Name(RenameMode::From))
            if request.accepts_names() =>
        {
            vec![(0, W32Action::RenamedFrom)]
        }
        notify::EventKind::Modify(ModifyKind::Name(RenameMode::To)) if request.accepts_names() => {
            vec![(0, W32Action::RenamedTo)]
        }
        notify::EventKind::Modify(ModifyKind::Name(RenameMode::Both))
            if request.accepts_names() =>
        {
            vec![(0, W32Action::RenamedFrom), (1, W32Action::RenamedTo)]
        }
        notify::EventKind::Modify(ModifyKind::Name(_)) if request.accepts_names() => event
            .paths
            .iter()
            .enumerate()
            .map(|(index, _)| (index, W32Action::Modified))
            .collect(),
        notify::EventKind::Modify(_) if request.accepts_modification() => {
            vec![(0, W32Action::Modified)]
        }
        notify::EventKind::Access(AccessKind::Any | AccessKind::Other)
            if request.filters.contains(W32Filter::LastAccessTime) =>
        {
            vec![(0, W32Action::Modified)]
        }
        _ => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
mod native {
    use super::super::super::delivery::{self, DeliveryReceiver, DeliverySender, EVENT_CAPACITY};
    use super::*;
    use crate::emacs_core::process::WaitNotifier;
    use notify::Watcher;
    use std::collections::HashMap;

    struct W32Watch {
        common: FileWatch<W32Request>,
        is_directory: bool,
    }

    struct PhysicalWatch {
        _watcher: notify::ReadDirectoryChangesWatcher,
        mode: WatchMode,
    }

    #[derive(Default)]
    pub(crate) struct W32NotifyBackend {
        tx: Option<DeliverySender<Result<notify::Event, notify::Error>>>,
        rx: Option<DeliveryReceiver<Result<notify::Event, notify::Error>>>,
        watches: Vec<W32Watch>,
        physical_watches: HashMap<PathBuf, PhysicalWatch>,
        next_id: i64,
    }

    impl W32NotifyBackend {
        fn ensure_delivery(&mut self, notifier: Option<WaitNotifier>) {
            if self.tx.is_some() {
                return;
            }
            let (tx, rx) = delivery::channel(notifier);
            self.tx = Some(tx);
            self.rx = Some(rx);
        }

        fn allocate_id(&mut self) -> i64 {
            let id = self.next_id;
            self.next_id = self
                .next_id
                .checked_add(1)
                .expect("file notification descriptor space exhausted");
            id
        }

        fn configure_path(&mut self, path: &Path, requested: WatchMode) -> Result<(), Flow> {
            if self
                .physical_watches
                .get(path)
                .is_some_and(|watch| watch.mode.covers(requested))
            {
                return Ok(());
            }

            // Construct and arm the replacement before dropping the old
            // watcher. A failed recursive upgrade therefore leaves every
            // existing logical watch physically active.
            let tx = self.tx.as_ref().expect("delivery was initialized").clone();
            let mut watcher = notify::ReadDirectoryChangesWatcher::new(
                move |result| {
                    tx.publish(result);
                },
                notify::Config::default(),
            )
            .map_err(|error| {
                file_notify_error(
                    "Watching filesystem events is not supported",
                    Some(error.to_string()),
                    None,
                )
            })?;
            watcher
                .watch(path, requested.into_notify())
                .map_err(|error| {
                    file_notify_error(
                        "Cannot watch file",
                        Some(error.to_string()),
                        Some(Value::string(path.display().to_string())),
                    )
                })?;
            self.physical_watches.insert(
                path.to_path_buf(),
                PhysicalWatch {
                    _watcher: watcher,
                    mode: requested,
                },
            );
            Ok(())
        }

        fn watch_matches_path(watch: &W32Watch, event_path: &Path) -> bool {
            if watch.is_directory {
                if watch.common.request.recursive() {
                    event_path.starts_with(&watch.common.path)
                } else {
                    event_path == watch.common.path
                        || event_path.parent() == Some(watch.common.path.as_path())
                }
            } else {
                event_path == watch.common.path
            }
        }

        fn reported_path(watch: &W32Watch, event_path: &Path) -> PathBuf {
            if watch.is_directory {
                event_path
                    .strip_prefix(&watch.common.path)
                    .ok()
                    .filter(|path| !path.as_os_str().is_empty())
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| event_path.to_path_buf())
            } else {
                event_path
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| event_path.to_path_buf())
            }
        }

        fn translate_event(&self, event: notify::Event) -> Vec<W32Event> {
            let mut translated = Vec::new();
            for watch in &self.watches {
                for (path_index, action) in event_actions(&event, &watch.common.request) {
                    let Some(path) = event.paths.get(path_index) else {
                        continue;
                    };
                    if !Self::watch_matches_path(watch, path) {
                        continue;
                    }
                    translated.push(W32Event {
                        watch_id: watch.common.id.clone(),
                        action,
                        path: Self::reported_path(watch, path),
                    });
                }
            }
            translated
        }
    }

    impl FileNotifyBackend for W32NotifyBackend {
        type Request = W32Request;
        type Event = W32Event;

        fn add_watch(
            &mut self,
            path: &Path,
            request: Self::Request,
            notifier: Option<WaitNotifier>,
        ) -> Result<WatchId, Flow> {
            self.ensure_delivery(notifier);
            if !path.exists() {
                return Err(file_notify_error(
                    "Cannot watch file",
                    Some("No such file or directory".to_owned()),
                    Some(Value::string(path.display().to_string())),
                ));
            }
            self.configure_path(path, WatchMode::from_recursive(request.recursive()))?;
            let descriptor = WatchId::new(self.allocate_id(), 0);
            self.watches.push(W32Watch {
                common: FileWatch {
                    id: descriptor.clone(),
                    path: path.to_path_buf(),
                    request,
                },
                is_directory: path.is_dir(),
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
                .any(|watch| watch.common.path == removed.common.path)
            {
                self.physical_watches.remove(&removed.common.path);
            }
            if self.watches.is_empty() {
                self.tx = None;
                self.rx = None;
                self.physical_watches.clear();
            }
            Ok(true)
        }

        fn valid_p(&self, descriptor: &WatchId) -> bool {
            self.watches
                .iter()
                .any(|watch| watch.common.id == *descriptor)
        }

        fn drain_events(&mut self) -> Result<DrainBatch<Self::Event>, Flow> {
            let mut raw_events = Vec::new();
            if let Some(rx) = self.rx.as_ref() {
                let overflowed = rx.take_overflow();
                loop {
                    match rx.try_recv() {
                        Ok(Ok(event)) => raw_events.push(event),
                        Ok(Err(error)) => {
                            tracing::warn!(%error, "w32 file notification event was lost");
                        }
                        Err(crossbeam_channel::TryRecvError::Empty) => break,
                        Err(crossbeam_channel::TryRecvError::Disconnected) => break,
                    }
                }
                if overflowed {
                    tracing::warn!(
                        capacity = EVENT_CAPACITY,
                        "Windows file-notification queue overflowed; emitting conservative changes"
                    );
                    return Ok(DrainBatch {
                        events: self
                            .watches
                            .iter()
                            .map(|watch| W32Event {
                                watch_id: watch.common.id.clone(),
                                action: W32Action::Modified,
                                path: watch.common.path.clone(),
                            })
                            .chain(
                                raw_events
                                    .into_iter()
                                    .flat_map(|event| self.translate_event(event)),
                            )
                            .collect(),
                        terminated: Vec::new(),
                    });
                }
            }
            Ok(DrainBatch {
                events: raw_events
                    .into_iter()
                    .flat_map(|event| self.translate_event(event))
                    .collect(),
                terminated: Vec::new(),
            })
        }

        fn has_watches(&self) -> bool {
            !self.watches.is_empty()
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) use native::W32NotifyBackend;

#[cfg(test)]
mod windows_test;
