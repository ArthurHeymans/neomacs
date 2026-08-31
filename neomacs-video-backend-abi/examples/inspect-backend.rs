#![cfg(target_os = "linux")]

use std::path::PathBuf;
use std::ptr::NonNull;

use neomacs_video_backend_abi as abi;

fn main() -> Result<(), String> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| "usage: inspect-backend PATH".to_owned())?;
    // SAFETY: this audit intentionally loads the exact release artifact under
    // test; its entry point and table are validated before any operation runs.
    let library = unsafe { libloading::Library::new(&path) }
        .map_err(|error| format!("failed to load {}: {error}", path.display()))?;
    // SAFETY: the symbol and function signature are fixed by ABI v2.
    let entry = unsafe { library.get::<abi::BackendEntryFn>(abi::BACKEND_ENTRY_SYMBOL) }
        .map_err(|error| format!("failed to resolve backend entry point: {error}"))?;
    // SAFETY: calling the entry point has no inputs and returns immutable
    // process-lifetime table storage owned by the loaded library.
    let api = NonNull::new(unsafe { entry() }.cast_mut())
        .ok_or_else(|| "backend entry point returned null".to_owned())?;
    // Read only the stable prefix until it proves that the full table exists.
    // SAFETY: an ABI entry point must return at least one readable header.
    let header = unsafe { api.cast::<abi::BackendApiHeader>().as_ptr().read() };
    header
        .validate()
        .map_err(|error| format!("invalid backend header: {error:?}"))?;
    // SAFETY: header validation proved the complete v2 table is present.
    let api = unsafe { api.as_ptr().read() };
    api.validate()
        .map_err(|error| format!("invalid backend table: {error:?}"))?;
    println!("{}: valid video backend ABI v2", path.display());
    Ok(())
}
