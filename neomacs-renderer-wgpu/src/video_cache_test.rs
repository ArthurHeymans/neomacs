use super::{
    CachedVideo, NativeVideoSessionId, VideoCache, VideoGpuAccounting, VideoGpuAccountingChange,
    VideoState, remap_event,
};
use neomacs_display_protocol::types::VideoId;
use neomacs_video::{VideoEvent, VideoSessionState};
use std::collections::HashMap;

#[test]
fn typed_session_states_have_one_renderer_compatibility_mapping() {
    assert_eq!(
        VideoState::from(VideoSessionState::Opening),
        VideoState::Loading
    );
    assert_eq!(
        VideoState::from(VideoSessionState::Playing),
        VideoState::Playing
    );
    assert_eq!(
        VideoState::from(VideoSessionState::Paused),
        VideoState::Paused
    );
    assert_eq!(
        VideoState::from(VideoSessionState::Ended),
        VideoState::EndOfStream
    );
    assert_eq!(
        VideoState::from(VideoSessionState::Failed),
        VideoState::Error
    );
    assert_eq!(
        VideoState::from(VideoSessionState::Closed),
        VideoState::Stopped
    );
}

#[test]
fn video_gpu_pool_accounting_tracks_aggregate_texture_lifetime() {
    let mut accounting = VideoGpuAccounting::default();

    assert_eq!(
        accounting.observe(1920 * 1080 * 4),
        VideoGpuAccountingChange::Register(1920 * 1080 * 4)
    );
    assert_eq!(
        accounting.observe(1920 * 1080 * 4),
        VideoGpuAccountingChange::Unchanged
    );
    assert_eq!(
        accounting.observe(3 * 1920 * 1080 * 4),
        VideoGpuAccountingChange::Register(3 * 1920 * 1080 * 4)
    );
    assert_eq!(accounting.observe(0), VideoGpuAccountingChange::Free);
    assert_eq!(accounting.observe(0), VideoGpuAccountingChange::Unchanged);
}

#[test]
fn native_session_identity_is_distinct_from_stable_video_identity() {
    let stable = VideoId::new(7);
    let old_native = NativeVideoSessionId(VideoId::new(41));
    let new_native = NativeVideoSessionId(VideoId::new(42));

    assert_ne!(old_native, new_native);
    assert_ne!(old_native.protocol(), stable);
    assert_eq!(
        remap_event(
            VideoEvent::Ended {
                id: new_native.protocol(),
            },
            stable,
        ),
        VideoEvent::Ended { id: stable }
    );
}

#[test]
fn terminal_failure_detaches_the_ephemeral_native_incarnation() {
    let stable = VideoId::new(7);
    let native = NativeVideoSessionId(VideoId::new(41));
    let mut cache = VideoCache {
        system: None,
        initialization_error: None,
        videos: HashMap::from([(
            stable,
            CachedVideo {
                id: stable,
                width: 1920,
                height: 1080,
                state: VideoState::Playing,
                frame_count: 3,
                native_id: Some(native),
                parked: None,
            },
        )]),
        next_id: 8,
        next_native_id: 42,
        native_to_video: HashMap::from([(native, stable)]),
        accounting: Vec::new(),
        gpu_accounting: VideoGpuAccounting::default(),
        last_service: Default::default(),
    };

    cache.detach_failed_native_session(stable, native, "import failed".into());

    let video = &cache.videos[&stable];
    assert_eq!(video.state, VideoState::Error);
    assert_eq!(video.native_id, None);
    assert_eq!(video.parked, None);
    assert!(!cache.native_to_video.contains_key(&native));
}
