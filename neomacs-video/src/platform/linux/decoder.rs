use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Arc;

use neomacs_display_protocol::types::VideoId;
use neomacs_video_backend_abi as abi;

use crate::backend::{BackendEvent, DecoderBackend};
use crate::sampling::LinuxDrmDevice;
use crate::{FrameTransferPolicy, VideoCommand, VideoCommandError, VideoWake};

use super::codec::{
    decode_event, encode_command, encode_renderer_drm_device, encode_supported_formats,
    encode_transfer_policy,
};
use super::frame::LinuxFrameLease;
use super::loader::{
    BACKEND_OVERRIDE_ENV, LoadedBackend, configured_backend_library_candidates,
    load_backend_from_candidates,
};

pub(crate) struct GstreamerDecoder {
    backend: Arc<LoadedBackend>,
    instance: Option<NonNull<c_void>>,
    _wake_context: Box<VideoWake>,
    sessions: HashSet<VideoId>,
}

impl GstreamerDecoder {
    pub(super) fn new(
        wake: VideoWake,
        transfer_policy: FrameTransferPolicy,
        renderer_drm_device: Option<LinuxDrmDevice>,
        renderer_features: wgpu::Features,
    ) -> Result<Self, String> {
        let executable = std::env::current_exe()
            .map_err(|error| format!("cannot locate Neomacs executable: {error}"))?;
        let configured = std::env::var_os(BACKEND_OVERRIDE_ENV);
        let candidates = configured_backend_library_candidates(&executable, configured.as_deref())
            .map_err(|error| error.to_string())?;
        let backend =
            Arc::new(load_backend_from_candidates(&candidates).map_err(|error| error.to_string())?);
        let mut wake_context = Box::new(wake);
        let (renderer_drm_major, renderer_drm_minor) =
            encode_renderer_drm_device(renderer_drm_device)?;
        let options = abi::BackendCreateOptions {
            transfer_policy: encode_transfer_policy(transfer_policy),
            supported_formats: encode_supported_formats(renderer_features),
            renderer_drm_major,
            renderer_drm_minor,
            wake: Some(wake_from_plugin),
            wake_userdata: (&mut *wake_context as *mut VideoWake).cast(),
        };
        let instance = backend.create(&options)?;
        Ok(Self {
            backend,
            instance: Some(instance),
            _wake_context: wake_context,
            sessions: HashSet::new(),
        })
    }

    fn instance(&self) -> NonNull<c_void> {
        self.instance.expect("decoder instance exists until drop")
    }
}

impl DecoderBackend for GstreamerDecoder {
    type Frame = LinuxFrameLease;

    fn command(&mut self, command: VideoCommand) -> Result<(), VideoCommandError> {
        let opened = match &command {
            VideoCommand::Open { id, .. } => Some((true, *id)),
            VideoCommand::Close { id } => Some((false, *id)),
            VideoCommand::Playback { .. } | VideoCommand::Presentation { .. } => None,
        };
        let (encoded, source_storage) = encode_command(&command);
        let _source_storage = source_storage;
        self.backend
            .command(self.instance(), &encoded)
            .map_err(VideoCommandError::from)?;
        if let Some((is_open, id)) = opened {
            if is_open {
                self.sessions.insert(id);
            } else {
                self.sessions.remove(&id);
            }
        }
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent<Self::Frame>> {
        let mut events = Vec::new();
        loop {
            let mut encoded = abi::BackendEvent::default();
            match self.backend.poll_event(self.instance(), &mut encoded) {
                Ok(abi::POLL_EMPTY) => break,
                Ok(abi::POLL_EVENT) => {
                    let id = VideoId::new(encoded.id);
                    match decode_event(Arc::clone(&self.backend), encoded) {
                        Ok(event) => events.push(event),
                        Err(message) => events.push(BackendEvent::Failed {
                            id,
                            error: VideoCommandError::Backend { message },
                        }),
                    }
                }
                Ok(_) => unreachable!("poll status validated by LoadedBackend"),
                Err(message) => {
                    events.extend(
                        self.sessions
                            .iter()
                            .copied()
                            .map(|id| BackendEvent::Failed {
                                id,
                                error: VideoCommandError::Backend {
                                    message: message.clone(),
                                },
                            }),
                    );
                    break;
                }
            }
        }
        events
    }
}

impl Drop for GstreamerDecoder {
    fn drop(&mut self) {
        if let Some(instance) = self.instance.take() {
            self.backend.destroy(instance);
        }
    }
}

unsafe extern "C" fn wake_from_plugin(userdata: *mut c_void) {
    // SAFETY: the boxed wake context outlives the plugin instance, and plugin
    // destroy joins all worker callbacks before returning.
    if let Some(wake) = unsafe { userdata.cast::<VideoWake>().as_ref() } {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| wake.notify()));
    }
}
