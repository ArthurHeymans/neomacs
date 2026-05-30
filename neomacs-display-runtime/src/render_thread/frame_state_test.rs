use super::*;
use crate::core::face::Face;
use crate::core::frame_glyphs::{CursorStyle, FrameGlyphBuffer, GlyphRowRole};
use crate::thread_comm::ThreadComms;
use neomacs_display_protocol::types::Color;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn make_test_app() -> RenderApp {
    let comms = ThreadComms::new().expect("Failed to create ThreadComms");
    let (_emacs, render) = comms.split();
    RenderApp::new(
        render,
        800,
        600,
        "test".to_string(),
        Arc::new((Mutex::new(HashMap::new()), std::sync::Condvar::new())),
        Arc::new((Mutex::new(Vec::new()), std::sync::Condvar::new())),
        true,
        #[cfg(feature = "neo-term")]
        Arc::new(Mutex::new(HashMap::new())),
    )
}

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

fn face(id: u32) -> Face {
    Face {
        id,
        ..Face::default()
    }
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
fn refresh_faces_rebuilds_from_primary_fallback_frames() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows.primary_window().map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() { window_state.render = __render; }app.faces.insert(99, face(99));

    let mut root = FrameGlyphBuffer::with_size(80.0, 32.0);
    root.faces.insert(7, face(7));
    if let Some(ws) = app.frame_windows.primary_window_mut() { ws.render.set_current_frame(Some(root)) };

    let mut child = FrameGlyphBuffer::with_size(40.0, 16.0);
    child.frame_id = 0x2000;
    child.parent_id = 0;
    child.faces.insert(8, face(8));
    app.frame_windows.primary_window_mut().expect("primary child frames mut").render.compositor.child_frames.update_frame(child);

    app.refresh_faces_from_frames();

    assert!(app.faces.contains_key(&7));
    assert!(app.faces.contains_key(&8));
    assert!(!app.faces.contains_key(&99));
}
