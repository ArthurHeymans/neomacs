use super::RenderApp;
use super::state::GuiChromeInteractionState;
use crate::core::frame_glyphs::FrameGlyphBuffer;
use crate::core::types::DisplayWindowId;
use crate::thread_comm::FrameRef;
use crate::thread_comm::{
    ThreadComms, ToolBarImageSource, ToolBarItem, ToolBarItemType, UiCommand, WindowCommand,
};
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

    app.handle_window(WindowCommand::DestroyWindow {
        frame: FrameRef::Primary,
    });

    assert!(app.frame_windows.primary_window().is_none());
    assert!(
        app.frame_windows
            .primary_window()
            .map(|ws| &ws.render)
            .is_none()
    );
    assert!(
        app.frame_windows
            .primary_window()
            .and_then(|ws| ws.render.compositor.current_frame.as_ref())
            .is_none()
    );
    assert!(
        !app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
    assert!(app.frame_windows.primary_window().is_none());
    assert_eq!(app.frame_windows.primary_frame_id(), None);
}

#[test]
fn destroy_adopted_primary_window_by_real_frame_id_prevents_lifecycle_recreate() {
    let mut app = make_test_app();
    app.frame_windows.adopt_primary_frame_id(0x1000);

    app.handle_window(WindowCommand::DestroyWindow {
        frame: FrameRef::Frame(0x1000),
    });

    assert!(app.frame_windows.primary_window().is_none());
    assert!(
        app.frame_windows
            .primary_window()
            .map(|ws| &ws.render)
            .is_none()
    );
    assert!(
        !app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
    assert!(app.frame_windows.primary_window().is_none());
    assert_eq!(app.frame_windows.primary_frame_id(), None);
    assert!(app.frame_windows.pending_destroys.is_empty());
}

#[test]
fn pending_dirty_primary_window_is_not_redrawable_active_work() {
    let mut app = make_test_app();
    let primary = app.frame_windows.primary_window_mut().unwrap();
    primary.render.compositor.dirty = true;

    assert!(
        app.frame_windows
            .primary_window()
            .unwrap()
            .render
            .compositor
            .dirty
    );
    assert!(
        !app.frame_windows.any_redrawable_top_level_dirty(),
        "a pending window has no native surface to receive RedrawRequested"
    );
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

    app.handle_window(WindowCommand::ResizeWindow {
        frame: FrameRef::Primary,
        width: 1024,
        height: 768,
        geometry_hints,
    });

    assert_eq!(
        app.frame_windows
            .primary_window()
            .map_or((0, 0), |ws| ws.native_size()),
        (1024, 768)
    );
    let primary = app.frame_windows.primary_window().unwrap();
    assert_eq!(primary.lifecycle.geometry_hints(), Some(geometry_hints));
}

#[test]
fn pre_bootstrap_set_window_size_updates_native_fallback_size() {
    let mut app = make_test_app();

    app.handle_window(WindowCommand::SetWindowSize {
        width: 900,
        height: 700,
    });

    assert_eq!(
        app.frame_windows
            .primary_window()
            .map_or((0, 0), |ws| ws.native_size()),
        (900, 700)
    );
}

#[test]
fn pre_bootstrap_window_decorations_update_native_fallback_chrome() {
    let mut app = make_test_app();

    app.handle_window(WindowCommand::SetWindowDecorated { decorated: false });

    assert!(
        !app.frame_windows
            .primary_window()
            .expect("primary window state")
            .chrome()
            .decorations_enabled
    );
}

#[test]
fn adopt_primary_window_command_updates_existing_primary_render_state_identity() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }

    app.handle_window(WindowCommand::AdoptPrimaryFrame {
        frame: FrameRef::Frame(0x1000),
    });

    assert_eq!(app.frame_windows.primary_frame_id(), Some(0x1000));
    assert_eq!(
        app.frame_windows
            .primary_window()
            .map(|ws| &ws.render)
            .map(|frame| frame.emacs_frame_id),
        Some(0x1000)
    );
}

#[test]
fn adopted_primary_frame_id_targets_primary_popup_menu() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }
    app.frame_windows.adopt_primary_frame_id(0x1000);

    app.handle_ui(UiCommand::ShowPopupMenu {
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
    });

    assert!(
        app.frame_windows
            .primary_window()
            .and_then(|ws| ws.render.overlays.popup_menu.as_ref())
            .is_some()
    );
    assert!(
        app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
}

#[test]
fn primary_toolbar_command_marks_render_state_dirty() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }

    app.handle_ui(UiCommand::SetToolBar {
        items: vec![ToolBarItem {
            index: 7,
            key: "open".to_string(),
            image: Some(ToolBarImageSource::File {
                path: "etc/images/open.xpm".to_string(),
            }),
            label: String::new(),
            help: String::new(),
            enabled: true,
            selected: false,
            item_type: ToolBarItemType::Button,
            wrap: false,
        }],
        height: 34.0,
        fg_r: 1.0,
        fg_g: 1.0,
        fg_b: 1.0,
        bg_r: 0.0,
        bg_g: 0.0,
        bg_b: 0.0,
    });

    assert!(
        app.frame_windows
            .primary_window()
            .and_then(|ws| ws.render.chrome.tool_bar.as_ref())
            .is_some()
    );
    assert!(
        app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
}

#[test]
fn primary_menubar_command_marks_render_state_dirty() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }

    app.handle_ui(UiCommand::SetMenuBar {
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
    });

    assert!(
        app.frame_windows
            .primary_window()
            .and_then(|ws| ws.render.chrome.menu_bar.as_ref())
            .is_some()
    );
    assert!(
        app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
}

#[test]
fn primary_tooltip_command_marks_render_state_dirty() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }

    app.handle_ui(UiCommand::ShowTooltip {
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
    });

    assert!(
        app.frame_windows
            .primary_window()
            .map(|ws| &ws.render)
            .and_then(|frame| frame.overlays.tooltip.as_ref())
            .is_some()
    );
    assert!(
        app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
}

#[test]
fn hide_popup_menu_marks_primary_chrome_dirty_without_popup() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }
    if let Some(ws) = app.frame_windows.primary_window_mut() {
        ws.render
            .with_chrome_interaction_mut(|chrome| chrome.menu_bar_active = Some(3))
    } else {
        false
    };
    if let Some(ws) = app.frame_windows.primary_window_mut() {
        ws.render.compositor.dirty = false
    };

    app.handle_ui(UiCommand::HidePopupMenu);

    assert_eq!(
        app.frame_windows
            .primary_window()
            .map_or(GuiChromeInteractionState::default(), |ws| ws
                .render
                .chrome
                .interaction)
            .menu_bar_active,
        None
    );
    assert!(
        app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
}

#[test]
fn popup_menu_for_unknown_secondary_does_not_fall_back_to_primary() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }

    app.handle_ui(UiCommand::ShowPopupMenu {
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
    });

    assert!(
        app.frame_windows
            .primary_window()
            .and_then(|ws| ws.render.overlays.popup_menu.as_ref())
            .is_none()
    );
    assert!(
        !app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
}

#[test]
fn tooltip_for_unknown_secondary_does_not_fall_back_to_primary() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }

    app.handle_ui(UiCommand::ShowTooltip {
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
    });

    assert!(
        app.frame_windows
            .primary_window()
            .map(|ws| &ws.render)
            .and_then(|frame| frame.overlays.tooltip.as_ref())
            .is_none()
    );
    assert!(
        !app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
}

#[test]
fn adopted_primary_frame_id_targets_primary_visual_bell() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }
    app.frame_windows.adopt_primary_frame_id(0x1000);

    app.handle_ui(UiCommand::VisualBell {
        frame: FrameRef::Frame(0x1000),
    });

    assert!(
        app.frame_windows
            .primary_window()
            .map(|ws| &ws.render)
            .and_then(|frame| frame.overlays.visual_bell_start)
            .is_some()
    );
    assert!(
        app.frame_windows
            .primary_window()
            .is_some_and(|ws| ws.render.compositor.dirty)
    );
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
        DisplayWindowId::new(7),
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
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }
    app.frame_windows.adopt_primary_frame_id(0x1000);
    if let Some(ws) = app.frame_windows.primary_window_mut() {
        ws.render
            .set_current_frame(Some(FrameGlyphBuffer::with_size(800.0, 600.0)), None)
    };

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
    let __render = super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.frame_windows
            .primary_window()
            .map_or(1.0, |ws| ws.scale_factor()),
        app.frame_windows.fps_enabled,
    );
    if let Some(window_state) = app.frame_windows.primary_window_mut() {
        window_state.render = __render;
    }
    if let Some(ws) = app.frame_windows.primary_window_mut() {
        ws.render
            .set_current_frame(Some(FrameGlyphBuffer::with_size(800.0, 600.0)), None)
    };
    if let Some(ws) = app.frame_windows.primary_window_mut() {
        ws.render.compositor.dirty = false
    };

    let mut secondary = FrameGlyphBuffer::with_size(320.0, 240.0);
    secondary.frame_id = neomacs_display_protocol::types::DisplayFrameId::new(0x2000);
    secondary.parent_id = neomacs_display_protocol::types::DisplayFrameId::new(0);
    emacs
        .frame_tx
        .send(FrameDisplayState::from_frame_glyph_buffer(&secondary))
        .expect("queue secondary snapshot");

    app.poll_frame();

    assert_eq!(
        app.frame_windows
            .primary_window()
            .and_then(|ws| ws.render.compositor.current_frame.as_ref())
            .map(|frame| frame.width),
        Some(800.0)
    );
}
