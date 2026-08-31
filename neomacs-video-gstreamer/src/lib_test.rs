use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use neomacs_video_backend_abi as abi;

use super::{
    LinuxDrmDevice, PlaybackAction, VideoCommand, VideoSource, decode_command, decode_drm_device,
};

#[test]
fn malformed_drm_device_pair_is_rejected_at_the_abi_boundary() {
    let options = abi::BackendCreateOptions {
        transfer_policy: abi::TRANSFER_ALLOW_CPU,
        renderer_drm_major: -1,
        renderer_drm_minor: 128,
        wake: None,
        wake_userdata: core::ptr::null_mut(),
    };

    assert_eq!(
        decode_drm_device(&options),
        Err("invalid renderer DRM device numbers (-1, 128)".to_owned())
    );
}

#[test]
fn valid_drm_device_pair_round_trips_from_the_abi() {
    let options = abi::BackendCreateOptions {
        transfer_policy: abi::TRANSFER_ALLOW_CPU,
        renderer_drm_major: 226,
        renderer_drm_minor: 128,
        wake: None,
        wake_userdata: core::ptr::null_mut(),
    };

    assert_eq!(
        decode_drm_device(&options),
        Ok(Some(LinuxDrmDevice::from_device_numbers(226, 128)))
    );
}

#[test]
fn non_utf8_uri_is_rejected_at_the_abi_boundary() {
    let source = [0xff];
    let command = abi::BackendCommand {
        kind: abi::COMMAND_OPEN,
        id: 7,
        source_kind: abi::SOURCE_URI,
        source_ptr: source.as_ptr(),
        source_len: source.len(),
        ..abi::BackendCommand::default()
    };

    assert_eq!(
        decode_command(&command),
        Err("video URI is not valid UTF-8".to_owned())
    );
}

#[test]
fn unix_file_path_bytes_round_trip_without_utf8_conversion() {
    let source = [b'/', b't', b'm', b'p', b'/', 0xff];
    let command = abi::BackendCommand {
        kind: abi::COMMAND_OPEN,
        id: 7,
        source_kind: abi::SOURCE_FILE,
        source_ptr: source.as_ptr(),
        source_len: source.len(),
        ..abi::BackendCommand::default()
    };

    let decoded = decode_command(&command).expect("Unix file paths may contain arbitrary bytes");
    assert_eq!(
        decoded,
        VideoCommand::Open {
            id: neomacs_display_protocol::types::VideoId::new(7),
            source: VideoSource::File(PathBuf::from(std::ffi::OsString::from_vec(source.to_vec()))),
            initial_playback: super::InitialPlayback::Paused,
            loop_mode: super::LoopMode::Off,
        }
    );
}

#[test]
fn invalid_playback_rate_is_rejected_at_the_abi_boundary() {
    let command = abi::BackendCommand {
        kind: abi::COMMAND_SET_RATE,
        id: 7,
        playback_rate: f64::NAN,
        ..abi::BackendCommand::default()
    };

    assert!(matches!(
        decode_command(&command),
        Err(message) if message.contains("finite")
    ));
    assert!(!matches!(
        decode_command(&command),
        Ok(VideoCommand::Playback {
            action: PlaybackAction::SetRate(_),
            ..
        })
    ));
}
