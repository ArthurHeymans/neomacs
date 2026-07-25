use super::{FileNotifyBackend, FileNotifyWatchDescriptor, FileWatch, file_notify_error};
use crate::emacs_core::error::Flow;
use crate::emacs_core::value::Value;
use notify::Watcher;

#[derive(Default)]
pub(super) struct NotifyRsInotifyBackend {
    watcher: Option<notify::RecommendedWatcher>,
    _rx: Option<std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>>,
    watches: Vec<FileWatch>,
    next_id: i64,
}

impl NotifyRsInotifyBackend {
    fn ensure_watcher(&mut self) -> Result<(), Flow> {
        if self.watcher.is_some() {
            return Ok(());
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let watcher = notify::RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )
        .map_err(|e| {
            file_notify_error("File watching is not available", Some(e.to_string()), None)
        })?;
        self.watcher = Some(watcher);
        self._rx = Some(rx);
        Ok(())
    }

    fn allocate_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl FileNotifyBackend for NotifyRsInotifyBackend {
    fn allocated_p(&self) -> bool {
        self.watcher.is_some()
    }

    fn watch_list(&self) -> Vec<FileWatch> {
        self.watches.clone()
    }

    fn add_watch(&mut self, path: &std::path::Path) -> Result<FileNotifyWatchDescriptor, Flow> {
        self.ensure_watcher()?;

        if !path.exists() {
            return Err(file_notify_error(
                "Could not add watch for file",
                Some("No such file or directory".to_string()),
                Some(Value::string(path.display().to_string())),
            ));
        }
        if let Some(ref mut watcher) = self.watcher {
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
        let descriptor = FileNotifyWatchDescriptor::new(id, 0);
        self.watches.push(FileWatch {
            id,
            generation: descriptor.generation(),
            path: path.display().to_string(),
        });

        Ok(descriptor)
    }

    fn remove_watch(&mut self, descriptor: &FileNotifyWatchDescriptor) -> Result<bool, Flow> {
        let Some(pos) = self
            .watches
            .iter()
            .position(|w| w.id == descriptor.id() && w.generation == descriptor.generation())
        else {
            return Ok(false);
        };

        let removed = self.watches.remove(pos);
        if let Some(ref mut watcher) = self.watcher {
            let path = std::path::Path::new(&removed.path);
            let _ = watcher.unwatch(path);
        }

        if self.watches.is_empty() {
            self.watcher = None;
            self._rx = None;
        }

        Ok(true)
    }

    fn valid_p(&self, descriptor: &FileNotifyWatchDescriptor) -> bool {
        self.watches
            .iter()
            .any(|w| w.id == descriptor.id() && w.generation == descriptor.generation())
    }
}
