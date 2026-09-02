//! Lisp-facing video-session interface.
//!
//! Video decoding and GPU import belong to the display host's render thread.
//! This module owns only the evaluator seam: typed Lisp handles and commands
//! addressed to the stable video-session identity wrapped by those handles.

mod subrs;
#[cfg(test)]
pub(crate) use subrs::SUBRS;
pub(crate) use subrs::register_subrs;

use super::error::{EvalResult, Flow, signal};
use super::eval::{Context, VideoResolveRequest, VideoResolveSource};
use super::value::Value;
use neomacs_display_protocol::VideoId;
use neomacs_video_model::{
    InitialPlayback, LoopMode, PlaybackAction, VideoOpenRequest, VideoSource,
};
use std::path::PathBuf;

/// Exactly one valid identity for a Lisp `(video ...)` display specification.
///
/// A session handle is already open and stateful.  A source request is
/// declarative and must be resolved by the display host.  Keeping the two in
/// an enum makes opening a second decoder for a handle unrepresentable after
/// this parsing boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VideoDisplayReference {
    Session(VideoId),
    Resolve(VideoResolveRequest),
}

/// Parse the identity and initial playback policy shared by inline video and
/// shader-channel display specs.
///
/// `default_autoplay` differs by presentation: ordinary inline declarations
/// default paused, while a video sampled only by a shader channel must play to
/// produce useful frames.  Explicit state options are rejected for `:id`
/// handles because that session is controlled through the native handle API.
pub fn parse_video_display_reference(
    items: &[Value],
    default_autoplay: bool,
) -> Option<VideoDisplayReference> {
    if items.first()?.as_symbol_name() != Some("video") || !(items.len() - 1).is_multiple_of(2) {
        return None;
    }

    let mut session = None;
    let mut source = None;
    let mut loop_count = 0;
    let mut autoplay = default_autoplay;
    let mut has_playback_options = false;
    let mut index = 1;
    while index + 1 < items.len() {
        let value = items[index + 1];
        match items[index].as_symbol_name() {
            Some(":id") => {
                if session.is_some() || source.is_some() {
                    return None;
                }
                session = Some(value.as_video_handle()?);
            }
            Some(":file") => {
                if session.is_some() || source.is_some() {
                    return None;
                }
                source = Some(VideoResolveSource::File(value.as_lisp_string()?.clone()));
            }
            Some(":uri") => {
                if session.is_some() || source.is_some() {
                    return None;
                }
                source = Some(VideoResolveSource::Uri(value.as_lisp_string()?.clone()));
            }
            Some(":loop" | ":loop-count") => {
                has_playback_options = true;
                loop_count = if value.is_nil() {
                    0
                } else if value.is_symbol_named("t") {
                    -1
                } else {
                    let count = i32::try_from(value.as_int()?).ok()?;
                    (count >= -1).then_some(count)?
                };
            }
            Some(":autoplay") => {
                has_playback_options = true;
                autoplay = value.is_truthy();
            }
            _ => {}
        }
        index += 2;
    }

    match (session, source) {
        (Some(id), None) if !has_playback_options => Some(VideoDisplayReference::Session(id)),
        (None, Some(source)) => Some(VideoDisplayReference::Resolve(VideoResolveRequest {
            source,
            loop_count,
            autoplay,
        })),
        _ => None,
    }
}

fn video_error(message: impl Into<String>) -> super::error::Flow {
    signal("error", vec![Value::string(message.into())])
}

fn wrong_type(predicate: &str, value: Value) -> Flow {
    signal("wrong-type-argument", vec![Value::symbol(predicate), value])
}

fn display_host<'eval>(
    eval: &'eval Context,
    operation: &str,
) -> Result<&'eval dyn super::eval::DisplayHost, Flow> {
    eval.display_host.as_deref().ok_or_else(|| {
        video_error(format!(
            "{operation}: no GUI video display host in this session"
        ))
    })
}

fn video_id(value: Value) -> Result<VideoId, Flow> {
    value
        .as_video_handle()
        .ok_or_else(|| wrong_type("neomacs-video-p", value))
}

fn predicate(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    Ok(Value::bool(args[0].is_video_handle()))
}

fn source(value: Value) -> Result<VideoSource, Flow> {
    let text = value
        .as_lisp_string()
        .ok_or_else(|| wrong_type("stringp", value))?
        .as_utf8_str()
        .ok_or_else(|| video_error("neomacs-video-load: source must be UTF-8"))?;
    let is_uri = text.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme.bytes().enumerate().all(|(index, byte)| match byte {
                b'a'..=b'z' | b'A'..=b'Z' => true,
                b'0'..=b'9' | b'+' | b'-' | b'.' => index > 0,
                _ => false,
            })
    });
    Ok(if is_uri {
        VideoSource::Uri(text.to_owned())
    } else {
        VideoSource::File(PathBuf::from(text))
    })
}

fn loop_mode(value: Value) -> Result<LoopMode, Flow> {
    if value.is_nil() {
        return Ok(LoopMode::Off);
    }
    let count = value
        .as_int()
        .ok_or_else(|| wrong_type("integerp", value))?;
    let count = i32::try_from(count)
        .map_err(|_| video_error("neomacs-video-load: loop count is outside the i32 range"))?;
    LoopMode::from_legacy(count).map_err(|error| video_error(error.to_string()))
}

/// `(neomacs-video-load SOURCE &optional LOOP-COUNT AUTOPLAY)`.
///
/// Allocate one compositor-owned playback session. The returned opaque value
/// is deliberately not an integer: Lisp can copy and compare it, but cannot
/// accidentally use a glyph, image, or stale renderer id as a video session.
fn load(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    let request = VideoOpenRequest {
        source: source(args[0])?,
        loop_mode: loop_mode(args.get(1).copied().unwrap_or(Value::NIL))?,
        initial_playback: if args.get(2).is_some_and(|value| value.is_truthy()) {
            InitialPlayback::Playing
        } else {
            InitialPlayback::Paused
        },
    };
    let id = display_host(eval, "neomacs-video-load")?
        .create_video(request)
        .map_err(video_error)?;
    Ok(Value::make_video_handle(id))
}

fn control(eval: &Context, value: Value, operation: &str, action: PlaybackAction) -> EvalResult {
    let id = video_id(value)?;
    display_host(eval, operation)?
        .control_video(id, action)
        .map_err(video_error)?;
    Ok(Value::T)
}

fn play(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    control(eval, args[0], "neomacs-video-play", PlaybackAction::Play)
}

fn pause(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    control(eval, args[0], "neomacs-video-pause", PlaybackAction::Pause)
}

fn stop(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    control(eval, args[0], "neomacs-video-stop", PlaybackAction::Stop)
}

fn set_loop(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    let mode = loop_mode(args[1])?;
    control(
        eval,
        args[0],
        "neomacs-video-set-loop",
        PlaybackAction::SetLoop(mode),
    )
}

fn destroy(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    let id = video_id(args[0])?;
    display_host(eval, "neomacs-video-destroy")?
        .destroy_video(id)
        .map_err(video_error)?;
    Ok(Value::T)
}
