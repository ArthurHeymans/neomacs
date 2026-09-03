use enumflags2::BitFlags;

mod snapshot;
mod types;

use types::{KqueueAction, KqueueVnodeAction};

#[cfg(target_os = "macos")]
pub(in super::super) fn action_from_lisp_name(name: &str) -> Option<KqueueAction> {
    KqueueAction::from_lisp_name(name)
}

#[cfg(test)]
use snapshot::DirectoryEntrySnapshot;
use snapshot::{DirectoryChange, DirectorySnapshot};

/// Decode all vnode bits in GNU's observable consing order
/// (`kqueue_callback`, src/kqueue.c).  This accepts a set, rather than a
/// single event enum, so simultaneous NOTE_* flags cannot be lost.
pub(super) fn vnode_actions(flags: BitFlags<KqueueVnodeAction>) -> Vec<KqueueAction> {
    [
        (KqueueVnodeAction::Revoke, KqueueAction::Revoke),
        (KqueueVnodeAction::Rename, KqueueAction::Rename),
        (KqueueVnodeAction::Link, KqueueAction::Link),
        (KqueueVnodeAction::Attrib, KqueueAction::Attrib),
        (KqueueVnodeAction::Extend, KqueueAction::Extend),
        (KqueueVnodeAction::Write, KqueueAction::Write),
        (KqueueVnodeAction::Delete, KqueueAction::Delete),
    ]
    .into_iter()
    .filter_map(|(native, lisp)| flags.contains(native).then_some(lisp))
    .collect()
}

pub(super) fn requested_vnode_actions(
    flags: BitFlags<KqueueVnodeAction>,
    requested: BitFlags<KqueueAction>,
) -> Vec<KqueueAction> {
    vnode_actions(flags)
        .into_iter()
        .filter(|action| requested.contains(*action))
        .collect()
}

#[cfg(target_os = "macos")]
mod native {
    use super::super::super::delivery::{
        self, DeliveryReceiver, DeliverySender, EVENT_CAPACITY, PublishOutcome,
    };
    use super::super::super::{
        DrainBatch, FileNotifyBackend, FileNotifyEvent, FileWatch, WatchActivity, WatchId,
        WatchIdAllocator, file_notify_error,
    };
    use super::*;
    use crate::emacs_core::error::Flow;
    use crate::emacs_core::process::WaitNotifier;
    use crate::emacs_core::value::Value;
    use rustix::event::kqueue::{
        Event, EventFilter, EventFlags, UserDefinedFlags, UserFlags, VnodeEvents, kevent, kqueue,
    };
    use rustix::fd::{AsRawFd, OwnedFd, RawFd};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::ptr;
    use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
    use std::thread::JoinHandle;

    const COMMAND_EVENT_IDENT: isize = 1;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct KqueueRequest {
        actions: BitFlags<KqueueAction>,
    }

    impl KqueueRequest {
        pub(crate) fn new(actions: BitFlags<KqueueAction>) -> Self {
            Self { actions }
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) struct KqueueEvent {
        watch_id: WatchId,
        actions: Vec<KqueueAction>,
        path: PathBuf,
        file1: Option<PathBuf>,
    }

    impl FileNotifyEvent for KqueueEvent {
        fn watch_id(&self) -> &WatchId {
            &self.watch_id
        }

        fn into_lisp(self, ctx: &crate::emacs_core::eval::Context) -> Value {
            // GNU kqueue events use a bare-fixnum descriptor and have no
            // trailing cookie (`kqueue_generate_event`, src/kqueue.c:94-104).
            let mut fields = vec![
                Value::fixnum(self.watch_id.slot()),
                Value::list(
                    self.actions
                        .into_iter()
                        .map(|action| Value::symbol(action.as_lisp_name()))
                        .collect(),
                ),
                super::super::super::lisp::file_name_to_lisp(ctx, &self.path),
            ];
            if let Some(file1) = self.file1 {
                fields.push(super::super::super::lisp::file_name_to_lisp(ctx, &file1));
            }
            Value::list(fields)
        }
    }

    #[derive(Debug)]
    struct NativeEvent {
        watch_id: WatchId,
        actions: BitFlags<KqueueVnodeAction>,
    }

    struct NativeWatch {
        _fd: OwnedFd,
        watch_id: WatchId,
        activity: WatchActivity,
    }

    enum Command {
        Add {
            fd: OwnedFd,
            watch_id: WatchId,
            activity: WatchActivity,
            actions: BitFlags<KqueueVnodeAction>,
            reply: SyncSender<Result<RawFd, String>>,
        },
        Remove {
            descriptor: i64,
            reply: SyncSender<bool>,
        },
        Shutdown,
    }

    struct Worker {
        commands: Sender<Command>,
        control_kqueue: OwnedFd,
        events: DeliveryReceiver<Result<NativeEvent, String>>,
        join: Option<JoinHandle<()>>,
    }

    impl Worker {
        fn start(notifier: Option<WaitNotifier>) -> Result<Self, Flow> {
            let worker_kqueue = kqueue().map_err(|error| {
                file_notify_error(
                    "File watching is not available",
                    Some(error.to_string()),
                    None,
                )
            })?;
            let control_kqueue = rustix::io::dup(&worker_kqueue).map_err(|error| {
                file_notify_error(
                    "File watching is not available",
                    Some(error.to_string()),
                    None,
                )
            })?;
            register_command_event(&worker_kqueue).map_err(|error| {
                file_notify_error("File watching is not available", Some(error), None)
            })?;

            let (command_tx, command_rx) = mpsc::channel();
            let (event_tx, event_rx) = delivery::channel(notifier);
            let join = std::thread::Builder::new()
                .name("neomacs-kqueue".to_owned())
                .spawn(move || worker_loop(worker_kqueue, command_rx, event_tx))
                .map_err(|error| {
                    file_notify_error(
                        "File watching is not available",
                        Some(error.to_string()),
                        None,
                    )
                })?;
            Ok(Self {
                commands: command_tx,
                control_kqueue,
                events: event_rx,
                join: Some(join),
            })
        }

        fn send_command(&self, command: Command) -> Result<(), String> {
            self.commands
                .send(command)
                .map_err(|_| "kqueue worker exited".to_owned())?;
            trigger_command_event(&self.control_kqueue)
        }

        fn add(
            &self,
            fd: OwnedFd,
            watch_id: WatchId,
            activity: WatchActivity,
            actions: BitFlags<KqueueVnodeAction>,
        ) -> Result<RawFd, String> {
            let (reply_tx, reply_rx) = mpsc::sync_channel(1);
            self.send_command(Command::Add {
                fd,
                watch_id,
                activity,
                actions,
                reply: reply_tx,
            })?;
            reply_rx
                .recv()
                .map_err(|_| "kqueue worker exited while adding a watch".to_owned())?
        }

        fn remove(&self, descriptor: i64) -> Result<bool, String> {
            let (reply_tx, reply_rx) = mpsc::sync_channel(1);
            self.send_command(Command::Remove {
                descriptor,
                reply: reply_tx,
            })?;
            reply_rx
                .recv()
                .map_err(|_| "kqueue worker exited while removing a watch".to_owned())
        }
    }

    impl Drop for Worker {
        fn drop(&mut self) {
            let _ = self.send_command(Command::Shutdown);
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }

    fn register_command_event(kqueue_fd: &OwnedFd) -> Result<(), String> {
        let change = Event::new(
            EventFilter::User {
                ident: COMMAND_EVENT_IDENT,
                flags: UserFlags::NOINPUT,
                user_flags: UserDefinedFlags::new(0),
            },
            EventFlags::ADD | EventFlags::CLEAR,
            ptr::null_mut(),
        );
        let events: &mut [Event] = &mut [];
        // SAFETY: the user filter contains no borrowed descriptor; kqueue_fd
        // is owned for this call and remains owned by the worker afterwards.
        unsafe { kevent(kqueue_fd, &[change], events, None) }
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn trigger_command_event(kqueue_fd: &OwnedFd) -> Result<(), String> {
        let change = Event::new(
            EventFilter::User {
                ident: COMMAND_EVENT_IDENT,
                flags: UserFlags::TRIGGER,
                user_flags: UserDefinedFlags::new(0),
            },
            EventFlags::empty(),
            ptr::null_mut(),
        );
        let events: &mut [Event] = &mut [];
        // SAFETY: this only triggers the previously registered user filter;
        // it refers to no vnode descriptor.
        unsafe { kevent(kqueue_fd, &[change], events, None) }
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn register_vnode(
        kqueue_fd: &OwnedFd,
        fd: RawFd,
        actions: BitFlags<KqueueVnodeAction>,
    ) -> Result<(), String> {
        let flags = to_rustix_vnode_events(actions);
        let change = Event::new(
            EventFilter::Vnode { vnode: fd, flags },
            EventFlags::ADD | EventFlags::ENABLE | EventFlags::CLEAR,
            ptr::null_mut(),
        );
        let events: &mut [Event] = &mut [];
        // SAFETY: `fd' is owned by the worker's watch map from successful
        // registration until the filter is removed by closing that fd.
        unsafe { kevent(kqueue_fd, &[change], events, None) }
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn to_rustix_vnode_events(actions: BitFlags<KqueueVnodeAction>) -> VnodeEvents {
        let mut flags = VnodeEvents::empty();
        for (action, native) in [
            (KqueueVnodeAction::Delete, VnodeEvents::DELETE),
            (KqueueVnodeAction::Write, VnodeEvents::WRITE),
            (KqueueVnodeAction::Extend, VnodeEvents::EXTEND),
            (KqueueVnodeAction::Attrib, VnodeEvents::ATTRIBUTES),
            (KqueueVnodeAction::Link, VnodeEvents::LINK),
            (KqueueVnodeAction::Rename, VnodeEvents::RENAME),
            (KqueueVnodeAction::Revoke, VnodeEvents::REVOKE),
        ] {
            if actions.contains(action) {
                flags.insert(native);
            }
        }
        flags
    }

    fn from_rustix_vnode_events(flags: VnodeEvents) -> BitFlags<KqueueVnodeAction> {
        let mut actions = BitFlags::empty();
        for (native, action) in [
            (VnodeEvents::DELETE, KqueueVnodeAction::Delete),
            (VnodeEvents::WRITE, KqueueVnodeAction::Write),
            (VnodeEvents::EXTEND, KqueueVnodeAction::Extend),
            (VnodeEvents::ATTRIBUTES, KqueueVnodeAction::Attrib),
            (VnodeEvents::LINK, KqueueVnodeAction::Link),
            (VnodeEvents::RENAME, KqueueVnodeAction::Rename),
            (VnodeEvents::REVOKE, KqueueVnodeAction::Revoke),
        ] {
            if flags.contains(native) {
                actions.insert(action);
            }
        }
        actions
    }

    fn worker_loop(
        kqueue_fd: OwnedFd,
        commands: Receiver<Command>,
        events: DeliverySender<Result<NativeEvent, String>>,
    ) {
        let mut watches = HashMap::<RawFd, NativeWatch>::new();
        loop {
            let mut ready = Vec::<Event>::with_capacity(32);
            // SAFETY: every vnode fd registered in this kqueue is owned by
            // `watches' for the full wait. Commands are applied only after
            // kevent returns, so no descriptor can be dropped concurrently.
            let wait_result = unsafe {
                kevent(
                    &kqueue_fd,
                    &[],
                    rustix::buffer::spare_capacity(&mut ready),
                    None,
                )
            };
            if let Err(error) = wait_result {
                for watch in watches.values() {
                    watch.activity.terminate();
                }
                events.publish(Err(error.to_string()));
                return;
            }

            let command_ready = ready.iter().any(|event| {
                matches!(
                    event.filter(),
                    EventFilter::User {
                        ident: COMMAND_EVENT_IDENT,
                        ..
                    }
                )
            });
            if command_ready {
                loop {
                    match commands.try_recv() {
                        Ok(Command::Add {
                            fd,
                            watch_id,
                            activity,
                            actions,
                            reply,
                        }) => {
                            let raw_fd = fd.as_raw_fd();
                            let result = register_vnode(&kqueue_fd, raw_fd, actions).map(|()| {
                                watches.insert(
                                    raw_fd,
                                    NativeWatch {
                                        _fd: fd,
                                        watch_id,
                                        activity,
                                    },
                                );
                                raw_fd
                            });
                            let _ = reply.send(result);
                        }
                        Ok(Command::Remove { descriptor, reply }) => {
                            let removed = i32::try_from(descriptor)
                                .ok()
                                .and_then(|fd| watches.remove(&fd));
                            if let Some(watch) = removed.as_ref() {
                                watch.activity.terminate();
                            }
                            let removed = removed.is_some();
                            let _ = reply.send(removed);
                        }
                        Ok(Command::Shutdown) => return,
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => return,
                    }
                }
            }

            for event in ready {
                let EventFilter::Vnode { vnode, flags } = event.filter() else {
                    continue;
                };
                let Some(watch) = watches.get(&vnode) else {
                    continue;
                };
                let watch_id = watch.watch_id.clone();
                let actions = from_rustix_vnode_events(flags);
                if actions.is_empty() {
                    continue;
                }
                let terminal = actions.intersects(
                    KqueueVnodeAction::Delete
                        | KqueueVnodeAction::Rename
                        | KqueueVnodeAction::Revoke,
                );
                if terminal {
                    let watch = watches
                        .remove(&vnode)
                        .expect("ready vnode remained registered");
                    watch.activity.terminate();
                }
                let published = events.publish(Ok(NativeEvent { watch_id, actions }));
                if published == PublishOutcome::Closed {
                    return;
                }
            }
        }
    }

    struct KqueueWatch {
        common: FileWatch<KqueueRequest>,
        native_descriptor: i64,
        activity: WatchActivity,
        directory: Option<DirectorySnapshot>,
    }

    impl KqueueWatch {
        fn translate(
            &mut self,
            mut native_actions: BitFlags<KqueueVnodeAction>,
        ) -> Result<Vec<KqueueEvent>, Flow> {
            let requested = self.common.request.actions;
            let watch_id = self.common.id.clone();
            let mut translated = Vec::new();

            if native_actions.contains(KqueueVnodeAction::Write)
                && let Some(old_snapshot) = self.directory.as_ref()
            {
                native_actions.remove(KqueueVnodeAction::Write);
                if self.common.path.is_dir() {
                    let new_snapshot =
                        DirectorySnapshot::read(&self.common.path).map_err(|error| {
                            file_notify_error(
                                "Error while reading watched directory",
                                Some(error.to_string()),
                                Some(Value::string(self.common.path.display().to_string())),
                            )
                        })?;
                    for change in old_snapshot.diff(&new_snapshot) {
                        let (action, path, file1) = match change {
                            DirectoryChange::Action { action, path } => (action, path, None),
                            DirectoryChange::Rename { from, to } => {
                                (KqueueAction::Rename, from, Some(to))
                            }
                        };
                        if requested.contains(action) {
                            translated.push(KqueueEvent {
                                watch_id: watch_id.clone(),
                                actions: vec![action],
                                path,
                                file1,
                            });
                        }
                    }
                    self.directory = Some(new_snapshot);
                } else if requested.contains(KqueueAction::Delete) {
                    translated.push(KqueueEvent {
                        watch_id: watch_id.clone(),
                        actions: vec![KqueueAction::Delete],
                        path: self.common.path.clone(),
                        file1: None,
                    });
                }
            }

            let actions = requested_vnode_actions(native_actions, requested);
            if !actions.is_empty() {
                translated.push(KqueueEvent {
                    watch_id,
                    actions,
                    path: self.common.path.clone(),
                    file1: None,
                });
            }
            Ok(translated)
        }
    }

    #[derive(Default)]
    pub(crate) struct KqueueBackend {
        worker: Option<Worker>,
        watches: Vec<KqueueWatch>,
        ids: WatchIdAllocator,
    }

    impl KqueueBackend {
        fn ensure_worker(&mut self, notifier: Option<WaitNotifier>) -> Result<&mut Worker, Flow> {
            if self.worker.is_none() {
                self.worker = Some(Worker::start(notifier)?);
            }
            Ok(self.worker.as_mut().expect("worker was initialized"))
        }

        fn requested_native_actions(
            actions: BitFlags<KqueueAction>,
        ) -> BitFlags<KqueueVnodeAction> {
            let mut native = BitFlags::empty();
            for (lisp, vnode) in [
                (KqueueAction::Delete, KqueueVnodeAction::Delete),
                (KqueueAction::Write, KqueueVnodeAction::Write),
                (KqueueAction::Extend, KqueueVnodeAction::Extend),
                (KqueueAction::Attrib, KqueueVnodeAction::Attrib),
                (KqueueAction::Link, KqueueVnodeAction::Link),
                (KqueueAction::Rename, KqueueVnodeAction::Rename),
                (KqueueAction::Revoke, KqueueVnodeAction::Revoke),
            ] {
                if actions.contains(lisp) {
                    native.insert(vnode);
                }
            }
            native
        }

        fn open_watch(path: &Path) -> Result<OwnedFd, Flow> {
            use rustix::fs::{Mode, OFlags};

            let flags = OFlags::from_bits_retain((libc::O_EVTONLY | libc::O_SYMLINK) as u32)
                | OFlags::NONBLOCK;
            rustix::fs::open(path, flags, Mode::empty()).map_err(|error| {
                file_notify_error(
                    "File cannot be opened",
                    Some(error.to_string()),
                    Some(Value::string(path.display().to_string())),
                )
            })
        }
    }

    impl FileNotifyBackend for KqueueBackend {
        type Request = KqueueRequest;
        type Event = KqueueEvent;

        fn add_watch(
            &mut self,
            path: &Path,
            request: Self::Request,
            notifier: Option<WaitNotifier>,
        ) -> Result<WatchId, Flow> {
            let actions = request.actions;
            let is_directory = path.is_dir();
            let fd = Self::open_watch(path)?;
            let watch_id = self.ids.allocate();
            let activity = WatchActivity::active();
            let native_descriptor = self
                .ensure_worker(notifier)?
                .add(
                    fd,
                    watch_id.clone(),
                    activity.clone(),
                    Self::requested_native_actions(actions),
                )
                .map_err(|error| {
                    file_notify_error(
                        "Cannot watch file",
                        Some(error),
                        Some(Value::string(path.display().to_string())),
                    )
                })?;
            let directory = if is_directory {
                match DirectorySnapshot::read(path) {
                    Ok(snapshot) => Some(snapshot),
                    Err(error) => {
                        let _ = self
                            .worker
                            .as_ref()
                            .expect("worker exists")
                            .remove(i64::from(native_descriptor));
                        return Err(file_notify_error(
                            "Cannot read watched directory",
                            Some(error.to_string()),
                            Some(Value::string(path.display().to_string())),
                        ));
                    }
                }
            } else {
                None
            };
            self.watches.push(KqueueWatch {
                common: FileWatch {
                    id: watch_id.clone(),
                    path: path.to_path_buf(),
                    request,
                },
                native_descriptor: i64::from(native_descriptor),
                activity,
                directory,
            });
            Ok(watch_id)
        }

        fn remove_watch(&mut self, descriptor: &WatchId) -> Result<bool, Flow> {
            let Some(index) = self
                .watches
                .iter()
                .position(|watch| watch.common.id == *descriptor)
            else {
                return Ok(false);
            };
            let native_descriptor = self.watches[index].native_descriptor;
            let _worker_had_watch = self
                .worker
                .as_ref()
                .expect("a live watch has a worker")
                .remove(native_descriptor)
                .map_err(|error| file_notify_error("Cannot remove watch", Some(error), None))?;
            self.watches.remove(index);
            if self.watches.is_empty() {
                self.worker = None;
            }
            // The descriptor's presence in `self.watches' is authoritative.
            // A terminal NOTE_* may already have made the worker close its fd
            // before the evaluator drains that event; GNU still considers an
            // explicit removal successful while its watch object is present.
            Ok(true)
        }

        fn valid_p(&self, descriptor: &WatchId) -> bool {
            self.watches
                .iter()
                .any(|watch| watch.common.id == *descriptor && watch.activity.is_active())
        }

        fn drain_events(&mut self) -> Result<DrainBatch<Self::Event>, Flow> {
            let mut raw_events = Vec::new();
            let mut overflowed = false;
            let mut failure = None;
            if let Some(worker) = self.worker.as_ref() {
                overflowed = worker.events.take_overflow();
                loop {
                    match worker.events.try_recv() {
                        Ok(Ok(event)) => raw_events.push(event),
                        Ok(Err(error)) => {
                            failure = Some(file_notify_error(
                                "Error while retrieving file system events",
                                Some(error),
                                None,
                            ));
                            break;
                        }
                        Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
                    }
                }
            }

            let mut translated = Vec::new();
            for event in raw_events {
                let Some(index) = self
                    .watches
                    .iter()
                    .position(|watch| watch.common.id == event.watch_id)
                else {
                    continue;
                };
                match self.watches[index].translate(event.actions) {
                    Ok(events) => translated.extend(events),
                    Err(error) if failure.is_none() => failure = Some(error),
                    Err(_) => {}
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
                tracing::warn!(
                    capacity = EVENT_CAPACITY,
                    "kqueue delivery queue overflowed; diffing watched directories conservatively"
                );
                for watch in &mut self.watches {
                    match watch.translate(KqueueVnodeAction::Write.into()) {
                        Ok(events) => translated.extend(events),
                        Err(error) if failure.is_none() => failure = Some(error),
                        Err(_) => {}
                    }
                }
            }
            if self.watches.is_empty() {
                self.worker = None;
            }
            Ok(DrainBatch {
                events: translated,
                terminated,
                failure,
            })
        }

        fn has_watches(&self) -> bool {
            !self.watches.is_empty()
        }
    }
}

#[cfg(target_os = "macos")]
pub(super) use native::{KqueueBackend, KqueueRequest};

#[cfg(target_os = "macos")]
mod lisp;
#[cfg(target_os = "macos")]
pub(crate) use lisp::{kqueue_add_watch, kqueue_rm_watch, kqueue_valid_p};

#[cfg(test)]
mod macos_test;
