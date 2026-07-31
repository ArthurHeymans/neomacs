//! Evaluator-owned asynchronous image catalog.
//!
//! Ordinary redisplay only mutates evaluator-local state and probes renderer
//! completion with `try_lock`. Queue backpressure is handed to one submission
//! worker, so lookup never waits for the renderer thread.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use neomacs_display_runtime::render_thread::{ImageDecodeTerminal, SharedImageMetadata};
use neomacs_display_runtime::thread_comm::{AssetCommand, RenderCommand};
use neovm_core::emacs_core::image_catalog::{
    AxisSize, ImageCatalog, ImageLookup, ImageResolveRequest, ImageResolveSource, ImageSizeSpec,
    PendingImage, ReadyImage,
};
use neovm_core::heap_types::LispString;

use super::GuiEventLoopWaker;

const HOST_IMAGE_ID_START: u32 = 0x4000_0000;
static HOST_IMAGE_ID_ALLOCATOR: AtomicU32 = AtomicU32::new(HOST_IMAGE_ID_START);

fn next_host_image_id() -> u32 {
    HOST_IMAGE_ID_ALLOCATOR.fetch_add(1, Ordering::Relaxed)
}

/// Deep host-side module that owns image request identity, state transitions,
/// renderer scheduling, and completion observation.
pub(super) struct AsyncImageCatalog {
    cmd_tx: crossbeam_channel::Sender<RenderCommand>,
    render_waker: Option<GuiEventLoopWaker>,
    image_metadata: SharedImageMetadata,
    entries: RefCell<HashMap<ImageResolveRequest, ImageLookup>>,
    home_directory: Option<String>,
}

impl AsyncImageCatalog {
    pub(super) fn new(
        cmd_tx: crossbeam_channel::Sender<RenderCommand>,
        render_waker: Option<GuiEventLoopWaker>,
        image_metadata: SharedImageMetadata,
    ) -> Self {
        Self {
            cmd_tx,
            render_waker,
            image_metadata,
            entries: RefCell::new(HashMap::new()),
            home_directory: home_directory_from_environment(),
        }
    }

    /// Re-queue every known entry for decode + upload after the renderer's
    /// image cache was destroyed by a GPU device loss.
    ///
    /// The catalog's map keys are the full [`ImageResolveRequest`]s (source
    /// bytes/path plus sizing/realization), so each entry can rebuild its
    /// exact original load command. Entries and their image ids are KEPT —
    /// published frames still reference those ids, so re-uploading under the
    /// same id re-textures the renderer's retained CPU frame as soon as the
    /// decode lands, without waiting for a fresh redisplay. `Ready` metadata
    /// stays valid (same source, same parameters); `Pending` entries
    /// complete against the re-issued decode; `Failed` entries fail again
    /// identically.
    pub(super) fn invalidate_all(&self) {
        let entries = self.entries.borrow();
        for (request, state) in entries.iter() {
            let image_id = state.placement().image_id();
            let command = image_load_command(request, image_id);
            if let Err(error) =
                schedule_image_command(&self.cmd_tx, self.render_waker.as_ref(), command)
            {
                tracing::warn!(
                    image_id,
                    %error,
                    "failed to re-queue image decode after display reset"
                );
            }
        }
    }

    pub(super) fn resolve_sync(
        &self,
        request: ImageResolveRequest,
    ) -> Result<Option<ReadyImage>, String> {
        let request =
            normalize_image_file_request_with_home(request, self.home_directory.as_deref());
        let pending = match self.lookup(request.clone()) {
            ImageLookup::Ready(image) => return Ok(Some(image)),
            ImageLookup::Pending(image) => image,
            ImageLookup::Failed(failed) => return Err(failed.error),
        };
        let placement = pending.placement();

        let Some(terminal) = wait_for_image_metadata(
            &self.image_metadata,
            placement.image_id(),
            Duration::from_secs(1),
        ) else {
            return Ok(None);
        };
        let state = image_lookup_from_terminal(pending, terminal);
        self.entries.borrow_mut().insert(request, state.clone());

        match state {
            ImageLookup::Ready(image) => Ok(Some(image)),
            ImageLookup::Failed(failed) => Err(failed.error),
            ImageLookup::Pending(_) => unreachable!("terminal decode cannot remain pending"),
        }
    }
}

impl ImageCatalog for AsyncImageCatalog {
    fn lookup(&self, request: ImageResolveRequest) -> ImageLookup {
        let request =
            normalize_image_file_request_with_home(request, self.home_directory.as_deref());
        let mut entries = self.entries.borrow_mut();
        if !entries.contains_key(&request) {
            let image_id = next_host_image_id();
            let (width, height) = placeholder_image_dimensions(&request);
            let pending = PendingImage::new(image_id, width, height);
            let command = image_load_command(&request, image_id);
            let state =
                match schedule_image_command(&self.cmd_tx, self.render_waker.as_ref(), command) {
                    Ok(()) => ImageLookup::Pending(pending),
                    Err(error) => ImageLookup::Failed(pending.failed(error)),
                };
            entries.insert(request.clone(), state);
        }

        let state = entries
            .get_mut(&request)
            .expect("image catalog entry inserted above");
        let ImageLookup::Pending(pending) = state else {
            return state.clone();
        };
        let placement = pending.placement();
        let (lock, _) = &*self.image_metadata;
        let terminal = match lock.try_lock() {
            Ok(images) => images.get(&placement.image_id()).cloned(),
            Err(std::sync::TryLockError::WouldBlock) => return state.clone(),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                poisoned.into_inner().get(&placement.image_id()).cloned()
            }
        };
        let Some(terminal) = terminal else {
            return state.clone();
        };
        *state = image_lookup_from_terminal(pending.clone(), terminal);
        state.clone()
    }

    fn invalidate(&self, source: &ImageResolveSource) {
        let normalized_source = normalize_image_file_request_with_home(
            ImageResolveRequest {
                source: source.clone(),
                size: Default::default(),
                rotation: Default::default(),
                fg_color: 0,
                bg_color: 0,
                realization: Default::default(),
            },
            self.home_directory.as_deref(),
        )
        .source;
        let removed = {
            let mut entries = self.entries.borrow_mut();
            let requests = entries
                .keys()
                .filter(|request| request.source == normalized_source)
                .cloned()
                .collect::<Vec<_>>();
            requests
                .into_iter()
                .filter_map(|request| entries.remove(&request))
                .map(|state| state.placement().image_id())
                .collect::<Vec<_>>()
        };

        for id in removed {
            let command = RenderCommand::Asset(AssetCommand::ImageFree { id });
            if let Err(error) =
                schedule_image_command(&self.cmd_tx, self.render_waker.as_ref(), command)
            {
                tracing::warn!(id, %error, "failed to schedule invalidated image release");
            }
        }
    }
}

fn image_lookup_from_terminal(pending: PendingImage, terminal: ImageDecodeTerminal) -> ImageLookup {
    let image_id = pending.placement().image_id();
    match terminal {
        ImageDecodeTerminal::Ready(metadata) => {
            ImageLookup::Ready(ReadyImage { image_id, metadata })
        }
        ImageDecodeTerminal::Failed(error) => ImageLookup::Failed(pending.failed(error)),
    }
}

pub(super) fn wait_for_image_metadata(
    shared: &SharedImageMetadata,
    id: u32,
    timeout: Duration,
) -> Option<ImageDecodeTerminal> {
    let (lock, cvar) = &**shared;
    let deadline = Instant::now() + timeout;
    let mut images = match lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    loop {
        if let Some(terminal) = images.get(&id).cloned() {
            return Some(terminal);
        }
        let remaining = deadline.checked_duration_since(Instant::now())?;
        match cvar.wait_timeout(images, remaining) {
            Ok((guard, result)) => {
                images = guard;
                if result.timed_out() {
                    return images.get(&id).cloned();
                }
            }
            Err(poisoned) => {
                let (guard, _) = poisoned.into_inner();
                images = guard;
            }
        }
    }
}

struct DeferredRenderCommand {
    target: crossbeam_channel::Sender<RenderCommand>,
    waker: Option<GuiEventLoopWaker>,
    command: RenderCommand,
}

fn deferred_render_command_sender() -> &'static crossbeam_channel::Sender<DeferredRenderCommand> {
    static SENDER: OnceLock<crossbeam_channel::Sender<DeferredRenderCommand>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = crossbeam_channel::unbounded::<DeferredRenderCommand>();
        let _ = std::thread::Builder::new()
            .name("neomacs-image-command-submit".to_owned())
            .spawn(move || {
                while let Ok(deferred) = rx.recv() {
                    let command = expand_deferred_image_path(deferred.command);
                    if deferred.target.send(command).is_ok()
                        && let Some(waker) = deferred.waker
                    {
                        waker.wake();
                    }
                }
            });
        tx
    })
}

fn schedule_image_command(
    target: &crossbeam_channel::Sender<RenderCommand>,
    waker: Option<&GuiEventLoopWaker>,
    command: RenderCommand,
) -> Result<(), String> {
    if requires_deferred_path_expansion(&command) {
        return defer_render_command(target, waker, command);
    }
    match target.try_send(command) {
        Ok(()) => {
            if let Some(waker) = waker {
                waker.wake();
            }
            Ok(())
        }
        Err(crossbeam_channel::TrySendError::Full(command)) => {
            defer_render_command(target, waker, command)
        }
        Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
            Err("failed to queue image load: channel disconnected".to_owned())
        }
    }
}

fn defer_render_command(
    target: &crossbeam_channel::Sender<RenderCommand>,
    waker: Option<&GuiEventLoopWaker>,
    command: RenderCommand,
) -> Result<(), String> {
    deferred_render_command_sender()
        .send(DeferredRenderCommand {
            target: target.clone(),
            waker: waker.cloned(),
            command,
        })
        .map_err(|error| format!("failed to defer image load command: {error}"))
}

fn requires_deferred_path_expansion(command: &RenderCommand) -> bool {
    matches!(
        command,
        RenderCommand::Asset(AssetCommand::ImageLoadFile { path, .. })
            if path.starts_with('~')
    )
}

fn expand_deferred_image_path(mut command: RenderCommand) -> RenderCommand {
    if let RenderCommand::Asset(AssetCommand::ImageLoadFile { path, .. }) = &mut command
        && path.starts_with('~')
    {
        *path = neovm_core::emacs_core::fileio::expand_file_name(path, None);
    }
    command
}

fn image_load_command(request: &ImageResolveRequest, image_id: u32) -> RenderCommand {
    match &request.source {
        ImageResolveSource::File(path) => RenderCommand::Asset(AssetCommand::ImageLoadFile {
            id: image_id,
            path: path.as_utf8_str().unwrap_or_default().to_owned(),
            size: request.size,
            rotation: request.rotation,
            realization: request.realization,
            fg_color: request.fg_color,
            bg_color: request.bg_color,
        }),
        ImageResolveSource::Data(data) => RenderCommand::Asset(AssetCommand::ImageLoadData {
            id: image_id,
            data: data.clone(),
            size: request.size,
            rotation: request.rotation,
            realization: request.realization,
            fg_color: request.fg_color,
            bg_color: request.bg_color,
        }),
    }
}

fn placeholder_image_dimensions(request: &ImageResolveRequest) -> (u32, u32) {
    let (width, height) = request.size.placeholder_extent().unwrap_or((1, 1));
    (
        request.realization.layout_dimension(width),
        request.realization.layout_dimension(height),
    )
}

fn home_directory_from_environment() -> Option<String> {
    std::env::var_os("HOME")
        .or({
            #[cfg(windows)]
            {
                std::env::var_os("APPDATA").or_else(|| std::env::var_os("USERPROFILE"))
            }
            #[cfg(not(windows))]
            {
                None
            }
        })
        .map(|home| home.to_string_lossy().into_owned())
}

fn normalize_image_file_request_with_home(
    mut request: ImageResolveRequest,
    home_directory: Option<&str>,
) -> ImageResolveRequest {
    let ImageResolveSource::File(path) = &request.source else {
        return request;
    };
    let Some(path) = path.as_utf8_str() else {
        return request;
    };
    let Some(home_directory) = home_directory else {
        return request;
    };
    let expanded = if path == "~" {
        home_directory.to_owned()
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("{}/{rest}", home_directory.trim_end_matches('/'))
    } else {
        // Named-user lookup can involve NSS/LDAP. Preserve it for expansion
        // by the submission worker instead of performing I/O in redisplay.
        return request;
    };
    let expanded = if neovm_core::emacs_core::fileio::file_name_absolute_p(&expanded) {
        // Absolute-path cleanup is lexical and cannot consult cwd or NSS.
        neovm_core::emacs_core::fileio::expand_file_name(&expanded, None)
    } else {
        expanded
    };
    request.source = ImageResolveSource::File(LispString::from_utf8(&expanded));
    request
}

#[cfg(test)]
mod tests {
    use super::*;
    use neovm_core::emacs_core::image_catalog::{
        ImageDefaultScale, ImageScaleEnvironment, ImageScalePolicy,
    };
    use std::sync::{Arc, Condvar, Mutex};

    fn file_request(path: &str) -> ImageResolveRequest {
        ImageResolveRequest {
            source: ImageResolveSource::File(LispString::from_utf8(path)),
            size: ImageSizeSpec::new(AxisSize::AtMost(24), AxisSize::AtMost(24)),
            rotation: Default::default(),
            fg_color: 0,
            bg_color: 0,
            realization: Default::default(),
        }
    }

    #[test]
    fn evaluator_normalization_uses_cached_home_without_user_lookup() {
        let request = normalize_image_file_request_with_home(
            file_request("~/Pictures/icon.png"),
            Some("/cached/home"),
        );

        assert!(matches!(
            request.source,
            ImageResolveSource::File(path)
                if path.as_utf8_str() == Some("/cached/home/Pictures/icon.png")
        ));
    }

    #[test]
    fn named_user_expansion_is_deferred_off_the_evaluator_thread() {
        let request = normalize_image_file_request_with_home(
            file_request("~some-user/Pictures/icon.png"),
            Some("/cached/home"),
        );
        let command = image_load_command(&request, 7);

        assert!(matches!(
            &request.source,
            ImageResolveSource::File(path)
                if path.as_utf8_str() == Some("~some-user/Pictures/icon.png")
        ));
        assert!(requires_deferred_path_expansion(&command));
    }

    #[test]
    fn pending_slot_and_decode_command_share_one_resolved_realization() {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let metadata = Arc::new((Mutex::new(HashMap::new()), Condvar::new()));
        let catalog = AsyncImageCatalog::new(cmd_tx, None, metadata);
        let mut request = file_request("/tmp/icon.svg");
        // Neither axis pinned: the placeholder falls back to the realization.
        request.size = ImageSizeSpec::new(AxisSize::Native, AxisSize::AtMost(24));
        request.realization = ImageScaleEnvironment::new(7.2, 1.75, ImageDefaultScale::Auto)
            .resolve(ImageScalePolicy::Default);

        let placement = catalog.lookup(request).placement();

        assert_eq!(placement.width(), 18);
        assert_eq!(placement.height(), 18);
        assert!(matches!(
            cmd_rx.try_recv().expect("image load command"),
            RenderCommand::Asset(AssetCommand::ImageLoadFile {
                realization,
                ..
            }) if (realization.layout_scale() - (1.3 / 1.75)).abs() < 0.0001
                && (realization.device_scale() - 1.75).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn invalidate_all_requeues_every_entry_under_its_existing_id() {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let metadata = Arc::new((Mutex::new(HashMap::new()), Condvar::new()));
        let catalog = AsyncImageCatalog::new(cmd_tx, None, metadata);

        let first = catalog
            .lookup(file_request("/tmp/one.png"))
            .placement()
            .image_id();
        let second = catalog
            .lookup(file_request("/tmp/two.png"))
            .placement()
            .image_id();
        // Drain the two initial load commands.
        assert!(cmd_rx.try_recv().is_ok());
        assert!(cmd_rx.try_recv().is_ok());

        catalog.invalidate_all();

        let mut requeued_ids = Vec::new();
        while let Ok(command) = cmd_rx.try_recv() {
            match command {
                RenderCommand::Asset(AssetCommand::ImageLoadFile { id, .. }) => {
                    requeued_ids.push(id);
                }
                other => panic!("unexpected command re-queued: {other:?}"),
            }
        }
        requeued_ids.sort_unstable();
        let mut expected = vec![first, second];
        expected.sort_unstable();
        assert_eq!(requeued_ids, expected, "same ids, one command per entry");

        // The entries survive: a later lookup reuses the id, no new load.
        let again = catalog
            .lookup(file_request("/tmp/one.png"))
            .placement()
            .image_id();
        assert_eq!(again, first);
        assert!(cmd_rx.try_recv().is_err());
    }

    #[test]
    fn invalidating_file_source_frees_old_identity_and_next_lookup_reloads() {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let metadata = Arc::new((Mutex::new(HashMap::new()), Condvar::new()));
        let catalog = AsyncImageCatalog::new(cmd_tx, None, metadata);
        let request = file_request("/tmp/watched.svg");

        let first = catalog.lookup(request.clone()).placement().image_id();
        assert!(matches!(
            cmd_rx.try_recv().expect("initial image load"),
            RenderCommand::Asset(AssetCommand::ImageLoadFile { id, .. }) if id == first
        ));

        catalog.invalidate(&request.source);
        assert!(matches!(
            cmd_rx.try_recv().expect("old image identity freed"),
            RenderCommand::Asset(AssetCommand::ImageFree { id }) if id == first
        ));

        let second = catalog.lookup(request).placement().image_id();
        assert_ne!(first, second);
        assert!(matches!(
            cmd_rx.try_recv().expect("replacement image load"),
            RenderCommand::Asset(AssetCommand::ImageLoadFile { id, .. }) if id == second
        ));
    }
}
