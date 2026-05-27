use super::*;
use crate::core::frame_glyphs::{CursorStyle, FrameGlyphBuffer, GlyphRowRole};
use crate::render_thread::cursor::{CursorState, CursorTarget};
use crate::render_thread::frame_windows::GuiFrameRenderState;
use crate::thread_comm::ThreadComms;
use neomacs_display_protocol::types::Color;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn make_test_device() -> Option<wgpu::Device> {
    let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    instance_descriptor.backends = wgpu::Backends::all();
    let instance = wgpu::Instance::new(instance_descriptor);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok()?;
    let (device, _queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("frame-state test device"),
        ..Default::default()
    }))
    .ok()?;
    Some(device)
}

#[test]
fn apply_extra_spacing_remaps_cursor_by_slot_id() {
    let mut frame = FrameGlyphBuffer::with_size(80.0, 32.0);
    frame.set_draw_context(1, GlyphRowRole::Text, None);
    frame.add_char('a', 0.0, 0.0, 8.0, 16.0, 12.0, false);
    frame.add_char('b', 8.0, 0.0, 8.0, 16.0, 12.0, false);
    let target_slot = frame.glyphs[1].slot_id().expect("slot id");

    frame.add_cursor(1, 2.0, 0.0, 2.0, 16.0, CursorStyle::Bar(2.0), Color::WHITE);
    frame.window_cursors[0].slot_id = target_slot;

    frame.set_phys_cursor(PhysCursor {
        window_id: 1,
        charpos: 1,
        row: 0,
        col: 1,
        slot_id: target_slot,
        x: 2.0,
        y: 0.0,
        width: 2.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::Bar(2.0),
        color: Color::WHITE,
        cursor_fg: Color::BLACK,
    });

    RenderApp::apply_extra_spacing(
        &mut frame.glyphs,
        &mut frame.window_cursors,
        &mut frame.phys_cursor,
        0.0,
        1.0,
    );

    match &frame.glyphs[1] {
        FrameGlyph::Char { x, .. } => assert_eq!(*x, 9.0),
        other => panic!("expected char glyph, got {:?}", other),
    }
    assert_eq!(frame.window_cursors[0].x, 9.0);
    assert_eq!(frame.window_cursors[0].y, 0.0);
    let cursor = frame.phys_cursor.as_ref().expect("phys cursor");
    assert_eq!(cursor.x, 9.0);
    assert_eq!(cursor.y, 0.0);
}

#[test]
fn apply_visual_cursor_animations_rewrites_visual_cursor_rect() {
    let comms = ThreadComms::new().expect("ThreadComms::new failed");
    let (_emacs, render) = comms.split();
    let image_dimensions = Arc::new((Mutex::new(HashMap::new()), std::sync::Condvar::new()));
    let shared_monitors = Arc::new((Mutex::new(Vec::new()), std::sync::Condvar::new()));
    let mut app = RenderApp::new(
        render,
        320,
        200,
        "test".to_string(),
        image_dimensions,
        shared_monitors,
        true,
        #[cfg(feature = "neo-term")]
        Arc::new(Mutex::new(HashMap::new())),
    );
    let Some(device) = make_test_device() else {
        return;
    };
    app.primary_frame = Some(GuiFrameRenderState::new(
        0,
        &device,
        app.scale_factor,
        false,
    ));

    let mut frame = FrameGlyphBuffer::with_size(320.0, 200.0);
    frame.add_cursor(
        -1,
        100.0,
        20.0,
        8.0,
        16.0,
        CursorStyle::Bar(8.0),
        Color::WHITE,
    );
    app.set_primary_current_frame(Some(frame));

    let mut state = CursorState::default();
    state.set_target(CursorTarget {
        window_id: -1,
        x: 80.0,
        y: 20.0,
        width: 8.0,
        height: 16.0,
        style: CursorStyle::Bar(8.0),
        color: Color::WHITE,
        frame_id: 0,
    });
    state.set_target(CursorTarget {
        window_id: -1,
        x: 100.0,
        y: 20.0,
        width: 8.0,
        height: 16.0,
        style: CursorStyle::Bar(8.0),
        color: Color::WHITE,
        frame_id: 0,
    });
    app.primary_frame
        .as_mut()
        .expect("primary frame")
        .visual_cursors
        .insert(-1, state);

    app.apply_visual_cursor_animations();

    let cursor = &app.primary_current_frame().expect("frame").window_cursors[0];
    assert_eq!(cursor.x, 80.0);
    assert_eq!(cursor.y, 20.0);
    assert_eq!(cursor.width, 8.0);
    assert_eq!(cursor.height, 16.0);
}
