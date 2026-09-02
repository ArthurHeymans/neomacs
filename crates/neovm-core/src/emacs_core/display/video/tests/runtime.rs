//! Lisp-visible video session tests.

use super::eval::{Context, DisplayHost, GuiFrameHostRequest};
use super::value::Value;
use super::video::{VideoDisplayReference, parse_video_display_reference};
use neomacs_display_protocol::VideoId;
use neomacs_video_model::{PlaybackAction, VideoOpenRequest};
use std::sync::{Arc, Mutex};

const STUB_VIDEO_ID: VideoId = VideoId::new(42);

#[derive(Clone, Default)]
struct RecordingVideoDisplayHost {
    opens: Arc<Mutex<Vec<VideoOpenRequest>>>,
    controls: Arc<Mutex<Vec<(VideoId, PlaybackAction)>>>,
    destroys: Arc<Mutex<Vec<VideoId>>>,
}

impl DisplayHost for RecordingVideoDisplayHost {
    fn realize_gui_frame(&mut self, _request: GuiFrameHostRequest) -> Result<(), String> {
        Ok(())
    }

    fn resize_gui_frame(&mut self, _request: GuiFrameHostRequest) -> Result<(), String> {
        Ok(())
    }

    fn create_video(&self, request: VideoOpenRequest) -> Result<VideoId, String> {
        self.opens.lock().expect("video host opens").push(request);
        Ok(STUB_VIDEO_ID)
    }

    fn control_video(&self, id: VideoId, action: PlaybackAction) -> Result<(), String> {
        self.controls
            .lock()
            .expect("video host controls")
            .push((id, action));
        Ok(())
    }

    fn destroy_video(&self, id: VideoId) -> Result<(), String> {
        self.destroys.lock().expect("video host destroys").push(id);
        Ok(())
    }
}

fn video_context() -> (Context, RecordingVideoDisplayHost) {
    let host = RecordingVideoDisplayHost::default();
    let mut ctx = Context::new();
    ctx.set_display_host(Box::new(host.clone()));
    (ctx, host)
}

fn eval(ctx: &mut Context, source: &str) -> Value {
    ctx.eval_str(source).expect("video form should evaluate")
}

#[test]
fn advertised_video_session_functions_are_bound() {
    crate::test_utils::init_test_tracing();
    let mut ctx = Context::new();

    let available = ctx
        .eval_str(
            r#"(and (fboundp 'neomacs-video-p)
                     (fboundp 'neomacs-video-load)
                     (fboundp 'neomacs-video-play)
                     (fboundp 'neomacs-video-pause)
                     (fboundp 'neomacs-video-stop)
                     (fboundp 'neomacs-video-set-loop)
                     (fboundp 'neomacs-video-destroy))"#,
        )
        .expect("video function probe should evaluate");

    assert!(available.is_truthy());
}

#[test]
fn load_returns_typed_handle_and_controls_route_the_same_video_id() {
    crate::test_utils::init_test_tracing();
    let (mut ctx, host) = video_context();

    let handle = eval(
        &mut ctx,
        r#"(setq video-test-handle (neomacs-video-load "movie.mp4"))"#,
    );
    assert!(handle.is_video_handle());
    assert!(eval(&mut ctx, "(neomacs-video-p video-test-handle)").is_truthy());
    assert_eq!(handle.as_video_handle(), Some(STUB_VIDEO_ID));
    assert_eq!(handle.as_int(), None);

    eval(
        &mut ctx,
        r#"(progn
             (neomacs-video-play video-test-handle)
             (neomacs-video-pause video-test-handle)
             (neomacs-video-stop video-test-handle)
             (neomacs-video-set-loop video-test-handle -1))"#,
    );

    assert_eq!(host.opens.lock().expect("video host opens").len(), 1);
    assert_eq!(
        *host.controls.lock().expect("video host controls"),
        vec![
            (STUB_VIDEO_ID, PlaybackAction::Play),
            (STUB_VIDEO_ID, PlaybackAction::Pause),
            (STUB_VIDEO_ID, PlaybackAction::Stop),
            (
                STUB_VIDEO_ID,
                PlaybackAction::SetLoop(neomacs_video_model::LoopMode::Infinite),
            ),
        ]
    );
}

#[test]
fn dropping_video_handle_destroys_the_host_session_once() {
    crate::test_utils::init_test_tracing();
    let (mut ctx, host) = video_context();

    eval(
        &mut ctx,
        r#"(setq video-test-handle (neomacs-video-load "movie.mp4"))"#,
    );
    eval(&mut ctx, "(garbage-collect)");
    assert!(
        host.destroys
            .lock()
            .expect("video host destroys")
            .is_empty()
    );

    eval(&mut ctx, "(setq video-test-handle nil)");
    eval(&mut ctx, "(garbage-collect)");
    assert_eq!(
        *host.destroys.lock().expect("video host destroys"),
        vec![STUB_VIDEO_ID]
    );

    eval(&mut ctx, "(garbage-collect)");
    assert_eq!(
        *host.destroys.lock().expect("video host destroys"),
        vec![STUB_VIDEO_ID]
    );
}

#[test]
fn display_reference_keeps_open_sessions_distinct_from_source_requests() {
    crate::test_utils::init_test_tracing();
    let handle = Value::make_video_handle(STUB_VIDEO_ID);
    let items = vec![Value::symbol("video"), Value::keyword("id"), handle];
    assert_eq!(
        parse_video_display_reference(&items, false),
        Some(VideoDisplayReference::Session(STUB_VIDEO_ID))
    );

    let raw_id = vec![
        Value::symbol("video"),
        Value::keyword("id"),
        Value::fixnum(i64::from(STUB_VIDEO_ID.get())),
    ];
    assert_eq!(parse_video_display_reference(&raw_id, false), None);

    let ambiguous = vec![
        Value::symbol("video"),
        Value::keyword("id"),
        handle,
        Value::keyword("file"),
        Value::string("movie.mp4"),
    ];
    assert_eq!(parse_video_display_reference(&ambiguous, false), None);

    let stateful_display = vec![
        Value::symbol("video"),
        Value::keyword("id"),
        handle,
        Value::keyword("autoplay"),
        Value::T,
    ];
    assert_eq!(
        parse_video_display_reference(&stateful_display, false),
        None
    );
}
