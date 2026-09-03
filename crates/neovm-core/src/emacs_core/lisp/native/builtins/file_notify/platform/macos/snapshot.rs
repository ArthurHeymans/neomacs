//! Pure directory snapshot model used to synthesize kqueue child events.
//!
//! Kqueue reports that a directory changed, but not which child changed.  GNU
//! Emacs therefore compares directory snapshots.  Keeping that policy pure
//! lets us test it without a macOS kernel or a live worker thread.

use super::KqueueAction;
#[cfg(target_os = "macos")]
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DirectoryEntrySnapshot {
    pub(super) inode: u64,
    pub(super) name: PathBuf,
    pub(super) modified: (i64, i64),
    pub(super) changed: (i64, i64),
    pub(super) size: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct DirectorySnapshot {
    entries: Vec<DirectoryEntrySnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum DirectoryChange {
    Action { action: KqueueAction, path: PathBuf },
    Rename { from: PathBuf, to: PathBuf },
}

impl DirectorySnapshot {
    #[cfg(test)]
    pub(super) fn from_entries(entries: Vec<DirectoryEntrySnapshot>) -> Self {
        Self { entries }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn read(directory: &Path) -> std::io::Result<Self> {
        use std::os::unix::fs::MetadataExt;

        let mut entries = Vec::new();
        for result in std::fs::read_dir(directory)? {
            let entry = result?;
            let metadata = std::fs::symlink_metadata(entry.path())?;
            entries.push(DirectoryEntrySnapshot {
                inode: metadata.ino(),
                name: PathBuf::from(entry.file_name()),
                modified: (metadata.mtime(), metadata.mtime_nsec()),
                changed: (metadata.ctime(), metadata.ctime_nsec()),
                size: metadata.size(),
            });
        }
        Ok(Self { entries })
    }

    /// Reproduce GNU `kqueue_compare_dir_list' as a pure transition.  Keeping
    /// the old and new snapshots explicit makes rename pairing, replacement,
    /// and metadata classification independently testable from kqueue I/O.
    pub(super) fn diff(&self, new: &Self) -> Vec<DirectoryChange> {
        let mut available_new = new.entries.clone();
        let mut pending = Vec::<DirectoryEntrySnapshot>::new();
        let mut renamed_destinations = Vec::<DirectoryEntrySnapshot>::new();
        let mut changes = Vec::new();

        for old_entry in &self.entries {
            if let Some(index) = available_new
                .iter()
                .position(|new_entry| new_entry.inode == old_entry.inode)
            {
                let new_entry = available_new.remove(index);
                if *old_entry == new_entry {
                    continue;
                }
                if old_entry.name == new_entry.name {
                    if old_entry.modified != new_entry.modified {
                        changes.push(DirectoryChange::Action {
                            action: KqueueAction::Write,
                            path: old_entry.name.clone(),
                        });
                    }
                    if old_entry.changed != new_entry.changed {
                        changes.push(DirectoryChange::Action {
                            action: KqueueAction::Attrib,
                            path: old_entry.name.clone(),
                        });
                    }
                } else {
                    changes.push(DirectoryChange::Rename {
                        from: old_entry.name.clone(),
                        to: new_entry.name.clone(),
                    });
                    renamed_destinations.push(new_entry);
                }
                continue;
            }

            if let Some(index) = available_new
                .iter()
                .position(|new_entry| new_entry.name == old_entry.name)
            {
                pending.push(available_new.remove(index));
                continue;
            }

            if let Some(index) = pending
                .iter()
                .position(|new_entry| new_entry.inode == old_entry.inode)
            {
                let new_entry = pending.remove(index);
                changes.push(DirectoryChange::Rename {
                    from: old_entry.name.clone(),
                    to: new_entry.name,
                });
                continue;
            }

            if let Some(index) = renamed_destinations
                .iter()
                .position(|new_entry| new_entry.name == old_entry.name)
            {
                renamed_destinations.remove(index);
                continue;
            }

            changes.push(DirectoryChange::Action {
                action: KqueueAction::Delete,
                path: old_entry.name.clone(),
            });
        }

        for entry in available_new {
            changes.push(DirectoryChange::Action {
                action: KqueueAction::Create,
                path: entry.name.clone(),
            });
            if entry.size > 0 {
                changes.push(DirectoryChange::Action {
                    action: KqueueAction::Write,
                    path: entry.name,
                });
            }
        }
        for entry in pending {
            changes.push(DirectoryChange::Action {
                action: KqueueAction::Write,
                path: entry.name,
            });
        }

        changes
    }
}
