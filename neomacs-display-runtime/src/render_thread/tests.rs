use crate::thread_comm::FrameRef;
use super::RenderApp;
use crate::core::frame_glyphs::FrameGlyphBuffer;
use crate::thread_comm::{RenderCommand, ThreadComms, ToolBarItem};
use neomacs_display_protocol::glyph_matrix::FrameDisplayState;
use neomacs_display_protocol::{MenuBarItem, PopupMenuItem};
use neovm_core::window::GuiFrameGeometryHints;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use winit::keyboard::{Key, NamedKey};

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
        label: Some("render-thread test device"),
        ..Default::default()
    }))
    .ok()?;
    Some(device)
}

#[test]
fn test_translate_key_named() {
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::Escape)),
        0xff1b
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::Enter)),
        0xff0d
    );
    assert_eq!(RenderApp::translate_key(&Key::Named(NamedKey::Tab)), 0xff09);
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::Backspace)),
        0xff08
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::Delete)),
        0xffff
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::Home)),
        0xff50
    );
    assert_eq!(RenderApp::translate_key(&Key::Named(NamedKey::End)), 0xff57);
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::PageUp)),
        0xff55
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::PageDown)),
        0xff56
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::ArrowLeft)),
        0xff51
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::ArrowUp)),
        0xff52
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::ArrowRight)),
        0xff53
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::ArrowDown)),
        0xff54
    );
    assert_eq!(RenderApp::translate_key(&Key::Named(NamedKey::Space)), 0x20);
}

#[test]
fn test_translate_key_character() {
    assert_eq!(
        RenderApp::translate_key(&Key::Character("a".into())),
        'a' as u32
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Character("A".into())),
        'A' as u32
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Character("1".into())),
        '1' as u32
    );
}

#[test]
fn test_translate_key_function_keys() {
    assert_eq!(RenderApp::translate_key(&Key::Named(NamedKey::F1)), 0xffbe);
    assert_eq!(RenderApp::translate_key(&Key::Named(NamedKey::F12)), 0xffc9);
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::Insert)),
        0xff63
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::PrintScreen)),
        0xff61
    );
}

#[test]
fn test_translate_key_unknown() {
    assert_eq!(RenderApp::translate_key(&Key::Dead(None)), 0);
}

#[test]
fn test_render_thread_creation() {
    let comms = ThreadComms::new().expect("Failed to create ThreadComms");
    let (emacs, render) = comms.split();

    assert!(emacs.input_rx.is_empty());
    assert!(render.cmd_rx.is_empty());
}

#[test]
fn destroy_primary_window_command_prevents_lifecycle_recreate() {
    let mut app = make_test_app();
    app.frame_windows.adopt_primary_frame_id(0x1000);

    app.handle_window_command(RenderCommand::DestroyWindow { frame: FrameRef::Primary })
        .expect("destroy primary window");

    assert!(app.frame_windows.primary_window().is_none());
    assert!(app.primary_render_state().is_none());
    assert!(app.primary_current_frame().is_none());
    assert!(!app.primary_dirty());
    assert!(app.frame_windows.primary_window().is_none());
    assert_eq!(app.frame_windows.primary_frame_id(), None);
}

#[test]
fn destroy_adopted_primary_window_by_real_frame_id_prevents_lifecycle_recreate() {
    let mut app = make_test_app();
    app.frame_windows.adopt_primary_frame_id(0x1000);

    app.handle_window_command(RenderCommand::DestroyWindow {
        frame: FrameRef::Frame(0x1000),
    })
    .expect("destroy adopted primary window");

    assert!(app.frame_windows.primary_window().is_none());
    assert!(app.primary_render_state().is_none());
    assert!(!app.primary_dirty());
    assert!(app.frame_windows.primary_window().is_none());
    assert_eq!(app.frame_windows.primary_frame_id(), None);
    assert!(app.frame_windows.pending_destroys.is_empty());
}

#[test]
fn pre_bootstrap_primary_resize_updates_pending_size() {
    let mut app = make_test_app();
    let geometry_hints = GuiFrameGeometryHints {
        base_width: 24,
        base_height: 32,
        min_width: 48,
        min_height: 64,
        width_inc: 8,
        height_inc: 16,
    };

    app.handle_window_command(RenderCommand::ResizeWindow {
        frame: FrameRef::Primary,
        width: 1024,
        height: 768,
        geometry_hints,
    })
    .expect("pre-bootstrap primary resize");

    assert_eq!(app.primary_native_size(), (1024, 768));
    let primary = app.frame_windows.primary_window().unwrap();
    assert_eq!(
        primary.pending_geometry_hints,
        Some(geometry_hints)
    );
}

#[test]
fn pre_bootstrap_set_window_size_updates_native_fallback_size() {
    let mut app = make_test_app();

    app.handle_window_command(RenderCommand::SetWindowSize {
        width: 900,
        height: 700,
    })
    .expect("pre-bootstrap primary set size");

    assert_eq!(app.primary_native_size(), (900, 700));
}

#[test]
fn pre_bootstrap_window_decorations_update_native_fallback_chrome() {
    let mut app = make_test_app();

    app.handle_window_command(RenderCommand::SetWindowDecorated { decorated: false })
        .expect("pre-bootstrap primary decorations");

    assert!(!app.primary_chrome().decorations_enabled);
}

#[test]
fn adopt_primary_window_command_updates_existing_primary_render_state_identity() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));

    app.handle_window_command(RenderCommand::AdoptPrimaryFrame {
        frame: FrameRef::Frame(0x1000),
    })
    .expect("adopt primary frame");

    assert_eq!(app.frame_windows.primary_frame_id(), Some(0x1000));
    assert_eq!(
        app.primary_render_state().map(|frame| frame.emacs_frame_id),
        Some(0x1000)
    );
}

#[test]
fn adopted_primary_frame_id_targets_primary_popup_menu() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));
    app.frame_windows.adopt_primary_frame_id(0x1000);

    app.handle_ui_command(RenderCommand::ShowPopupMenu {
        frame: FrameRef::Frame(0x1000),
        x: 10.0,
        y: 20.0,
        items: vec![PopupMenuItem {
            label: "Open".to_string(),
            shortcut: String::new(),
            enabled: true,
            separator: false,
            submenu: false,
            depth: 0,
        }],
        title: None,
        fg: None,
        bg: None,
    })
    .expect("show popup on adopted primary");

    assert!(app.primary_popup_menu().is_some());
    assert!(app.primary_dirty());
}

#[test]
fn primary_toolbar_command_marks_render_state_dirty() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));

    app.handle_ui_command(RenderCommand::SetToolBar {
        items: vec![ToolBarItem {
            index: 7,
            icon_name: "open".to_string(),
            label: String::new(),
            help: String::new(),
            enabled: true,
            selected: false,
            is_separator: false,
        }],
        height: 34.0,
        fg_r: 1.0,
        fg_g: 1.0,
        fg_b: 1.0,
        bg_r: 0.0,
        bg_g: 0.0,
        bg_b: 0.0,
    })
    .expect("set primary toolbar");

    assert!(app.primary_tool_bar().is_some());
    assert!(app.primary_dirty());
}

#[test]
fn primary_menubar_command_marks_render_state_dirty() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));

    app.handle_ui_command(RenderCommand::SetMenuBar {
        items: vec![MenuBarItem {
            index: 7,
            label: "File".to_string(),
            key: "file".to_string(),
        }],
        height: 24.0,
        fg_r: 1.0,
        fg_g: 1.0,
        fg_b: 1.0,
        bg_r: 0.0,
        bg_g: 0.0,
        bg_b: 0.0,
    })
    .expect("set primary menu bar");

    assert!(app.primary_menu_bar().is_some());
    assert!(app.primary_dirty());
}

#[test]
fn primary_tooltip_command_marks_render_state_dirty() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));

    app.handle_ui_command(RenderCommand::ShowTooltip {
        frame: FrameRef::Primary,
        x: 10.0,
        y: 20.0,
        text: "tip".to_string(),
        fg_r: 1.0,
        fg_g: 1.0,
        fg_b: 1.0,
        bg_r: 0.0,
        bg_g: 0.0,
        bg_b: 0.0,
    })
    .expect("show primary tooltip");

    assert!(
        app.primary_render_state()
            .and_then(|frame| frame.overlays.tooltip.as_ref())
            .is_some()
    );
    assert!(app.primary_dirty());
}

#[test]
fn hide_popup_menu_marks_primary_chrome_dirty_without_popup() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));
    app.with_primary_chrome_interaction_mut(|chrome| chrome.menu_bar_active = Some(3));
    app.set_primary_dirty(false);

    app.handle_ui_command(RenderCommand::HidePopupMenu)
        .expect("hide popup menu");

    assert_eq!(app.primary_chrome_interaction().menu_bar_active, None);
    assert!(app.primary_dirty());
}

#[test]
fn popup_menu_for_unknown_secondary_does_not_fall_back_to_primary() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));

    app.handle_ui_command(RenderCommand::ShowPopupMenu {
        frame: FrameRef::Frame(0x2000),
        x: 10.0,
        y: 20.0,
        items: vec![PopupMenuItem {
            label: "Open".to_string(),
            shortcut: String::new(),
            enabled: true,
            separator: false,
            submenu: false,
            depth: 0,
        }],
        title: None,
        fg: None,
        bg: None,
    })
    .expect("unknown secondary popup is handled as no-op");

    assert!(app.primary_popup_menu().is_none());
    assert!(!app.primary_dirty());
}

#[test]
fn tooltip_for_unknown_secondary_does_not_fall_back_to_primary() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));

    app.handle_ui_command(RenderCommand::ShowTooltip {
        frame: FrameRef::Frame(0x2000),
        x: 10.0,
        y: 20.0,
        text: "secondary".to_string(),
        fg_r: 1.0,
        fg_g: 1.0,
        fg_b: 1.0,
        bg_r: 0.0,
        bg_g: 0.0,
        bg_b: 0.0,
    })
    .expect("unknown secondary tooltip is handled as no-op");

    assert!(
        app.primary_render_state()
            .and_then(|frame| frame.overlays.tooltip.as_ref())
            .is_none()
    );
    assert!(!app.primary_dirty());
}

#[test]
fn adopted_primary_frame_id_targets_primary_visual_bell() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));
    app.frame_windows.adopt_primary_frame_id(0x1000);

    app.handle_ui_command(RenderCommand::VisualBell {
        frame: FrameRef::Frame(0x1000),
    })
    .expect("visual bell on adopted primary");

    assert!(
        app.primary_render_state()
            .and_then(|frame| frame.overlays.visual_bell_start)
            .is_some()
    );
    assert!(app.primary_dirty());
}

#[test]
fn managed_primary_visual_bell_uses_frame_renderer_effects() {
    let mut render = make_test_device()
        .map(|device| super::frame_windows::GuiFrameRenderState::new(0x1000, &device, 1.0, false));
    let Some(render) = render.as_mut() else {
        return;
    };
    let mut frame = FrameGlyphBuffer::with_size(800.0, 600.0);
    frame.add_window_info(
        7,
        1,
        1,
        50,
        50,
        0.0,
        0.0,
        400.0,
        300.0,
        20.0,
        0.0,
        0.0,
        true,
        false,
        17.0,
        String::new(),
        false,
    );
    render.compositor.current_frame = Some(frame);

    render.trigger_visual_bell(true, true, 120, std::time::Instant::now());

    assert!(render.overlays.visual_bell_start.is_some());
    assert!(render.compositor.renderer_effects.has_transient_effects());
    assert!(render.compositor.dirty);
}

#[test]
fn adopted_primary_pointer_target_uses_real_frame_id() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));
    app.frame_windows.adopt_primary_frame_id(0x1000);
    app.set_primary_current_frame(Some(FrameGlyphBuffer::with_size(800.0, 600.0)));

    let (x, y, frame_id) = app.pointer_target_at(12.0, 34.0);

    assert_eq!((x, y), (12.0, 34.0));
    assert_eq!(frame_id, 0x1000);
}

#[test]
fn unknown_secondary_frame_snapshot_does_not_fall_back_to_primary() {
    let comms = ThreadComms::new().expect("Failed to create ThreadComms");
    let (emacs, render) = comms.split();
    let mut app = RenderApp::new(
        render,
        800,
        600,
        "test".to_string(),
        Arc::new((Mutex::new(HashMap::new()), std::sync::Condvar::new())),
        Arc::new((Mutex::new(Vec::new()), std::sync::Condvar::new())),
        true,
        #[cfg(feature = "neo-term")]
        Arc::new(Mutex::new(HashMap::new())),
    );
    let Some(device) = make_test_device() else {
        return;
    };
    app.set_primary_render_state_for_tests(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.primary_scale_factor(),
        app.frame_windows.fps_enabled,
    ));
    app.set_primary_current_frame(Some(FrameGlyphBuffer::with_size(800.0, 600.0)));
    app.set_primary_dirty(false);

    let mut secondary = FrameGlyphBuffer::with_size(320.0, 240.0);
    secondary.frame_id = 0x2000;
    secondary.parent_id = 0;
    emacs
        .frame_tx
        .send(FrameDisplayState::from_frame_glyph_buffer(&secondary))
        .expect("queue secondary snapshot");

    app.poll_frame();

    assert_eq!(
        app.primary_current_frame().map(|frame| frame.width),
        Some(800.0)
    );
}
