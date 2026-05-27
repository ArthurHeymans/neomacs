use super::RenderApp;
use crate::thread_comm::{RenderCommand, ThreadComms};
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

    app.handle_window_command(RenderCommand::DestroyWindow { emacs_frame_id: 0 })
        .expect("destroy primary window");

    assert!(app.window.is_none());
    assert!(app.surface.is_none());
    assert!(app.surface_config.is_none());
    assert!(app.primary_frame.is_none());
    assert!(app.primary_current_frame().is_none());
    assert!(!app.primary_dirty());
    assert!(app.primary_window_destroyed);
    assert_eq!(app.frame_windows.primary_frame_id(), None);
}

#[test]
fn adopt_primary_window_command_updates_existing_primary_render_state_identity() {
    let mut app = make_test_app();
    let Some(device) = make_test_device() else {
        return;
    };
    app.primary_frame = Some(super::frame_windows::GuiFrameRenderState::new(
        0,
        &device,
        app.scale_factor,
        app.primary_fps_enabled(),
    ));

    app.handle_window_command(RenderCommand::AdoptPrimaryFrame {
        emacs_frame_id: 0x1000,
    })
    .expect("adopt primary frame");

    assert_eq!(app.frame_windows.primary_frame_id(), Some(0x1000));
    assert_eq!(
        app.primary_frame.as_ref().map(|frame| frame.emacs_frame_id),
        Some(0x1000)
    );
}
