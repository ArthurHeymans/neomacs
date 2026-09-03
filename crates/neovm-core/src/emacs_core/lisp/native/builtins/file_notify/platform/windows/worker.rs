//! Exact `ReadDirectoryChangesW` ownership behind a small safe interface.
//!
//! `notify` deliberately normalizes Windows events and fixes one broad native
//! filter. GNU's low-level `w32notify-*` API exposes every native filter,
//! including last-access time, so this adapter owns the OS request directly.

use super::super::super::delivery::{DeliverySender, PublishOutcome};
use super::super::super::{WatchActivity, WatchId};
use super::{W32Event, codec};
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::ptr;
use std::thread::JoinHandle;
use windows_sys::Win32::Foundation::{
    ERROR_OPERATION_ABORTED, GetLastError, HANDLE, INVALID_HANDLE_VALUE, WAIT_FAILED, WAIT_OBJECT_0,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_OVERLAPPED,
    FILE_LIST_DIRECTORY, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    ReadDirectoryChangesW,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{
    CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects,
};

const BUFFER_CAPACITY: usize = 64 * 1024;

pub(super) enum WorkerMessage {
    Event(W32Event),
    Overflow(WatchId),
    Failed(String),
}

pub(super) struct Worker {
    stop_event: OwnedHandle,
    join: Option<JoinHandle<()>>,
}

impl Worker {
    pub(super) fn start(
        path: &Path,
        recursive: bool,
        native_filter: u32,
        watch_id: WatchId,
        activity: WatchActivity,
        events: DeliverySender<WorkerMessage>,
    ) -> Result<Self, String> {
        let (directory, watched_name) = if path.is_dir() {
            (path.to_path_buf(), None)
        } else {
            let parent = path
                .parent()
                .ok_or_else(|| "watched file has no parent directory".to_owned())?;
            (parent.to_path_buf(), path.file_name().map(PathBuf::from))
        };
        let directory_handle = open_directory(&directory)?;
        let io_event = create_event()?;
        let stop_event = create_event()?;
        let stop_event_raw = stop_event.as_raw_handle() as usize;

        let join = std::thread::Builder::new()
            .name("neomacs-w32notify".to_owned())
            .spawn(move || {
                run(
                    directory_handle,
                    io_event,
                    stop_event_raw,
                    watched_name,
                    recursive,
                    native_filter,
                    watch_id,
                    activity,
                    events,
                );
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            stop_event,
            join: Some(join),
        })
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // SAFETY: this controller owns the event handle until `join` completes;
        // the worker only borrows its raw value during that interval.
        unsafe {
            SetEvent(self.stop_event.as_raw_handle() as HANDLE);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn run(
    directory: OwnedHandle,
    io_event: OwnedHandle,
    stop_event: usize,
    watched_name: Option<PathBuf>,
    recursive: bool,
    native_filter: u32,
    watch_id: WatchId,
    activity: WatchActivity,
    events: DeliverySender<WorkerMessage>,
) {
    let directory_raw = directory.as_raw_handle() as HANDLE;
    let io_event_raw = io_event.as_raw_handle() as HANDLE;
    let stop_event_raw = stop_event as HANDLE;
    let handles = [stop_event_raw, io_event_raw];
    let mut buffer = vec![0_u8; BUFFER_CAPACITY];

    loop {
        let mut overlapped = OVERLAPPED {
            hEvent: io_event_raw,
            ..OVERLAPPED::default()
        };
        // SAFETY: all pointers reference live, writable storage until the
        // overlapped request completes or is cancelled below.
        let started = unsafe {
            ReadDirectoryChangesW(
                directory_raw,
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len() as u32,
                if recursive { 1 } else { 0 },
                native_filter,
                ptr::null_mut(),
                &mut overlapped,
                None,
            )
        };
        if started == 0 {
            fail(
                &activity,
                &events,
                std::io::Error::last_os_error().to_string(),
            );
            return;
        }

        // SAFETY: both event handles stay owned for the full wait.
        let ready = unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, INFINITE) };
        if ready == WAIT_OBJECT_0 {
            // SAFETY: cancelling and waiting on this exact live OVERLAPPED
            // keeps its stack storage and buffer alive through completion.
            unsafe {
                CancelIoEx(directory_raw, &overlapped);
                let mut ignored = 0;
                GetOverlappedResult(directory_raw, &overlapped, &mut ignored, 1);
            }
            return;
        }
        if ready == WAIT_FAILED || ready != WAIT_OBJECT_0 + 1 {
            fail(
                &activity,
                &events,
                std::io::Error::last_os_error().to_string(),
            );
            return;
        }

        let mut bytes = 0;
        // SAFETY: the I/O event is signalled, and OVERLAPPED and buffer are
        // still alive and unchanged since ReadDirectoryChangesW started.
        if unsafe { GetOverlappedResult(directory_raw, &overlapped, &mut bytes, 0) } == 0 {
            // SAFETY: GetLastError immediately follows the failed Win32 call.
            let code = unsafe { GetLastError() };
            if code == ERROR_OPERATION_ABORTED {
                return;
            }
            fail(
                &activity,
                &events,
                std::io::Error::from_raw_os_error(code as i32).to_string(),
            );
            return;
        }
        if bytes == 0 {
            if events.publish(WorkerMessage::Overflow(watch_id.clone())) == PublishOutcome::Closed {
                return;
            }
            continue;
        }

        let decoded = match codec::decode(&buffer[..bytes as usize]) {
            Ok(decoded) => decoded,
            Err(error) => {
                tracing::warn!(%error, "malformed Windows file-notification batch; rescanning");
                if events.publish(WorkerMessage::Overflow(watch_id.clone()))
                    == PublishOutcome::Closed
                {
                    return;
                }
                continue;
            }
        };
        for (action, path) in decoded {
            if watched_name.as_ref().is_some_and(|name| *name != path) {
                continue;
            }
            if events.publish(WorkerMessage::Event(W32Event {
                watch_id: watch_id.clone(),
                action,
                path,
            })) == PublishOutcome::Closed
            {
                return;
            }
        }
    }
}

fn fail(activity: &WatchActivity, events: &DeliverySender<WorkerMessage>, error: String) {
    activity.terminate();
    events.publish(WorkerMessage::Failed(error));
}

fn open_directory(path: &Path) -> Result<OwnedHandle, String> {
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: `wide` is a live NUL-terminated UTF-16 path; no security or
    // template pointers are supplied. Ownership transfers to OwnedHandle.
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED | FILE_FLAG_OPEN_REPARSE_POINT,
            ptr::null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // SAFETY: CreateFileW returned a new, valid, uniquely owned handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
}

fn create_event() -> Result<OwnedHandle, String> {
    // SAFETY: null attributes/name create an unnamed auto-reset event.
    let raw = unsafe { CreateEventW(ptr::null(), 0, 0, ptr::null()) };
    if raw.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // SAFETY: CreateEventW returned a new, valid, uniquely owned handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
}
