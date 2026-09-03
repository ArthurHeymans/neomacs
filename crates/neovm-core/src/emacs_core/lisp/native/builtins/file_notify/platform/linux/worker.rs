//! Inotify ownership and blocking I/O.
//!
//! The worker is the only owner of the inotify instance and its native watch
//! descriptors.  The evaluator sends typed commands and receives owned event
//! records; neither side shares kernel handles or Lisp values.

use super::super::super::WatchActivity;
use super::super::super::delivery::{self, DeliveryReceiver, DeliverySender};
use crate::emacs_core::process::WaitNotifier;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use polling::{Event, Events, PollMode, Poller};
use std::collections::HashMap;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

const INOTIFY_KEY: usize = 1;

#[derive(Clone, Debug)]
pub(super) struct NativeEvent {
    pub(super) descriptor: i32,
    pub(super) mask: EventMask,
    pub(super) cookie: u32,
    pub(super) name: Option<OsString>,
}

enum Command {
    Add {
        path: PathBuf,
        mask: WatchMask,
        reply: Sender<Result<(i32, WatchActivity), String>>,
    },
    Remove {
        descriptor: i32,
        reply: Sender<Result<bool, String>>,
    },
    Shutdown,
}

pub(super) struct Worker {
    commands: Sender<Command>,
    poller: Arc<Poller>,
    events: DeliveryReceiver<Result<NativeEvent, String>>,
    join: Option<JoinHandle<()>>,
}

impl Worker {
    pub(super) fn start(notifier: Option<WaitNotifier>) -> Result<Self, String> {
        let inotify = Inotify::init().map_err(|error| error.to_string())?;
        let poller = Arc::new(Poller::new().map_err(|error| error.to_string())?);
        // SAFETY: the worker owns `inotify` until it deletes the registration
        // after its wait loop.  No command can drop or replace that value.
        unsafe { poller.add_with_mode(&inotify, Event::readable(INOTIFY_KEY), PollMode::Level) }
            .map_err(|error| error.to_string())?;

        let (command_tx, command_rx) = crossbeam_channel::bounded(64);
        let (event_tx, event_rx) = delivery::channel(notifier);
        let worker_poller = Arc::clone(&poller);
        let join = std::thread::Builder::new()
            .name("neomacs-inotify".to_owned())
            .spawn(move || worker_loop(inotify, worker_poller, command_rx, event_tx))
            .map_err(|error| error.to_string())?;

        Ok(Self {
            commands: command_tx,
            poller,
            events: event_rx,
            join: Some(join),
        })
    }

    fn send_command(&self, command: Command) -> Result<(), String> {
        self.commands
            .send(command)
            .map_err(|_| "inotify worker exited".to_owned())?;
        self.poller.notify().map_err(|error| error.to_string())
    }

    pub(super) fn add(
        &self,
        path: PathBuf,
        mask: WatchMask,
    ) -> Result<(i32, WatchActivity), String> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        self.send_command(Command::Add {
            path,
            mask,
            reply: reply_tx,
        })?;
        reply_rx
            .recv()
            .map_err(|_| "inotify worker exited while adding a watch".to_owned())?
    }

    pub(super) fn remove(&self, descriptor: i32) -> Result<bool, String> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        self.send_command(Command::Remove {
            descriptor,
            reply: reply_tx,
        })?;
        reply_rx
            .recv()
            .map_err(|_| "inotify worker exited while removing a watch".to_owned())?
    }

    pub(super) fn try_recv(&self) -> Result<Result<NativeEvent, String>, TryRecvError> {
        self.events.try_recv()
    }

    pub(super) fn take_overflow(&self) -> bool {
        self.events.take_overflow()
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

fn worker_loop(
    mut inotify: Inotify,
    poller: Arc<Poller>,
    commands: Receiver<Command>,
    events: DeliverySender<Result<NativeEvent, String>>,
) {
    let mut descriptors = HashMap::<i32, (WatchDescriptor, WatchActivity)>::new();
    let mut poll_events = Events::new();
    let mut buffer = vec![0; 64 * 1024];
    let mut running = true;

    while running {
        running = apply_commands(&mut inotify, &commands, &mut descriptors);
        if !running {
            break;
        }

        poll_events.clear();
        if let Err(error) = poller.wait(&mut poll_events, None) {
            terminate_all(&descriptors);
            events.publish(Err(error.to_string()));
            break;
        }

        running = apply_commands(&mut inotify, &commands, &mut descriptors);
        if !running {
            break;
        }
        if !poll_events
            .iter()
            .any(|event| event.key == INOTIFY_KEY && event.readable)
        {
            continue;
        }

        loop {
            match inotify.read_events(&mut buffer) {
                Ok(raw_events) => {
                    let mut any = false;
                    for event in raw_events {
                        any = true;
                        let descriptor = event.wd.get_watch_descriptor_id();
                        let terminal = event.mask.contains(EventMask::IGNORED);
                        if terminal && let Some((_, activity)) = descriptors.remove(&descriptor) {
                            activity.terminate();
                        }
                        events.publish(Ok(NativeEvent {
                            descriptor,
                            mask: event.mask,
                            cookie: event.cookie,
                            name: event.name.map(ToOwned::to_owned),
                        }));
                    }
                    if !any {
                        break;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    terminate_all(&descriptors);
                    events.publish(Err(error.to_string()));
                    running = false;
                    break;
                }
            }
        }
    }

    let _ = poller.delete(&inotify);
}

fn apply_commands(
    inotify: &mut Inotify,
    commands: &Receiver<Command>,
    descriptors: &mut HashMap<i32, (WatchDescriptor, WatchActivity)>,
) -> bool {
    loop {
        match commands.try_recv() {
            Ok(Command::Add { path, mask, reply }) => {
                let result = inotify
                    .watches()
                    .add(path, mask)
                    .map(|descriptor| {
                        let id = descriptor.get_watch_descriptor_id();
                        let activity = match descriptors.entry(id) {
                            std::collections::hash_map::Entry::Occupied(entry) => {
                                entry.get().1.clone()
                            }
                            std::collections::hash_map::Entry::Vacant(entry) => {
                                let activity = WatchActivity::active();
                                entry.insert((descriptor, activity.clone()));
                                activity
                            }
                        };
                        (id, activity)
                    })
                    .map_err(|error| error.to_string());
                let _ = reply.send(result);
            }
            Ok(Command::Remove { descriptor, reply }) => {
                let result = match descriptors.remove(&descriptor) {
                    Some((watch_descriptor, activity)) => inotify
                        .watches()
                        .remove(watch_descriptor)
                        .map(|()| {
                            activity.terminate();
                            true
                        })
                        .map_err(|error| error.to_string()),
                    None => Ok(false),
                };
                let _ = reply.send(result);
            }
            Ok(Command::Shutdown) => return false,
            Err(TryRecvError::Empty) => return true,
            Err(TryRecvError::Disconnected) => return false,
        }
    }
}

fn terminate_all(descriptors: &HashMap<i32, (WatchDescriptor, WatchActivity)>) {
    for (_, activity) in descriptors.values() {
        activity.terminate();
    }
}
