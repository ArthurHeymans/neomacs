use super::*;
use crate::core::frame_glyphs::FrameGlyphBuffer;
use crate::core::types::Color;
use crate::render_thread::frame_windows::{FrameLifecycle, GuiFrameRenderState};
use crate::render_thread::pointer_events::PointerOwner;
use crate::render_thread::state::{
    ActivePointerAppearance, PointerAppearancePhase, PointerAppearanceState,
    PresentedAppearanceKey, PresentedInteractionKey, TabBarPressCapture,
};
use neomacs_display_protocol::frame_chrome::InteractionId;
use neomacs_display_protocol::frame_chrome::PresentationId;
use neomacs_display_protocol::{
    FaceId, FrameRect, PointerAppearanceId,
    PointerAppearancePhase as ProtocolPointerAppearancePhase, PointerDrawMode, PresentedPaintSpan,
    PresentedPointerAppearance, PresentedPointerRegion, PresentedPrimitiveKind,
};
use winit::keyboard::{Key, NamedKey, SmolStr};
use winit::window::ResizeDirection;

/// Build a minimal `RenderApp` suitable for testing `detect_resize_edge`
/// and `titlebar_hit_test`.  Only the fields those methods read are
/// meaningful; everything else is set to harmless defaults.
fn make_test_app(width: u32, height: u32, scale_factor: f64) -> RenderApp {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::thread_comm::ThreadComms;

    let comms = ThreadComms::new().expect("ThreadComms::new failed");
    let (_emacs, render) = comms.split();
    let image_metadata = Arc::new((Mutex::new(HashMap::new()), std::sync::Condvar::new()));
    let shared_monitors = Arc::new((Mutex::new(Vec::new()), std::sync::Condvar::new()));

    let mut app = RenderApp::new(
        render,
        width,
        height,
        "test".to_string(),
        image_metadata,
        shared_monitors,
        true,
        #[cfg(feature = "neo-term")]
        Arc::new(Mutex::new(HashMap::new())),
    );
    {
        let primary = app.frame_windows.primary_window_mut().unwrap();
        if let FrameLifecycle::Pending {
            scale_factor: sf, ..
        } = &mut primary.lifecycle
        {
            *sf = scale_factor;
        }
    }
    app
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
        label: Some("input test device"),
        ..Default::default()
    }))
    .ok()?;
    Some(device)
}

fn appearance_key(presentation: u64, appearance: usize) -> PresentedAppearanceKey {
    PresentedAppearanceKey::new(
        PresentationId::new(presentation),
        PointerAppearanceId::try_from(appearance).expect("appearance id"),
    )
}

#[test]
fn active_pointer_appearance_selects_only_its_exact_presented_frame() {
    let mut frame = FrameGlyphBuffer::with_size(100.0, 20.0);
    frame.presentation_id = PresentationId::new(7);
    frame
        .faces
        .insert(FaceId::new(9), crate::core::face::Face::new(FaceId::new(9)));
    frame.add_char('a', 0.0, 0.0, 10.0, 20.0, 15.0, false);
    frame
        .install_presented_pointer(
            vec![PresentedPointerRegion::new(
                FrameRect::new(0.0, 0.0, 10.0, 20.0).unwrap(),
                None,
                Some(PointerAppearanceId::try_from(0usize).unwrap()),
            )],
            vec![PresentedPointerAppearance::new(
                vec![PresentedPaintSpan::new(
                    PresentedPrimitiveKind::Glyph,
                    0,
                    1,
                    FrameRect::new(0.0, 0.0, 10.0, 20.0).unwrap(),
                )],
                PointerDrawMode::Face(FaceId::new(9)),
                PointerDrawMode::Face(FaceId::new(9)),
            )],
        )
        .unwrap();
    let active =
        ActivePointerAppearance::new(appearance_key(7, 0), PointerAppearancePhase::Pressed);

    let selection = active.selection_for(&frame).expect("matching snapshot");
    assert_eq!(
        selection.appearance(),
        PointerAppearanceId::try_from(0usize).unwrap()
    );
    assert_eq!(selection.phase(), ProtocolPointerAppearancePhase::Pressed);

    frame.presentation_id = PresentationId::new(8);
    assert_eq!(active.selection_for(&frame), None);
}

#[test]
fn pointer_appearance_is_qualified_by_presentation_and_phase() {
    let first = appearance_key(7, 1);
    let second = appearance_key(7, 2);
    let replacement = appearance_key(8, 1);
    let mut state = PointerAppearanceState::default();

    assert!(state.hover(Some(first)));
    assert_eq!(
        state.active(),
        Some(ActivePointerAppearance::new(
            first,
            PointerAppearancePhase::Hover
        ))
    );
    assert!(!state.hover(Some(first)), "same visual range is stable");

    assert!(state.press());
    assert_eq!(
        state.active().unwrap().phase(),
        PointerAppearancePhase::Pressed
    );
    assert!(
        !state.press(),
        "repeated press does not change the draw phase"
    );

    assert!(state.hover(Some(second)));
    assert_eq!(
        state.active(),
        Some(ActivePointerAppearance::new(
            second,
            PointerAppearancePhase::Hover
        ))
    );
    assert!(state.hover(Some(first)));
    assert_eq!(
        state.active().unwrap().phase(),
        PointerAppearancePhase::Pressed
    );

    assert!(state.release());
    assert_eq!(
        state.active().unwrap().phase(),
        PointerAppearancePhase::Hover
    );
    assert!(!state.release(), "repeated release is visually stable");
    assert!(state.hover(Some(replacement)));
    assert_eq!(
        state.active().unwrap().presentation(),
        PresentationId::new(8)
    );
    assert!(state.hover(None));
    assert_eq!(state.active(), None);
}

#[test]
fn pressed_visual_stays_captured_while_hover_follows_pointer() {
    let pressed_visual = appearance_key(11, 3);
    let other_visual = appearance_key(11, 4);
    let mut state = PointerAppearanceState::default();

    state.hover(Some(pressed_visual));
    state.press();
    state.hover(Some(other_visual));

    assert_eq!(state.pressed(), Some(pressed_visual));
    assert_eq!(state.active().unwrap().key(), other_visual);
    assert_eq!(
        state.active().unwrap().phase(),
        PointerAppearancePhase::Hover
    );
    assert!(
        state.release(),
        "clearing the captured visual is a state change even while another visual is active"
    );
    assert_eq!(state.active().unwrap().key(), other_visual);
    assert_eq!(
        state.active().unwrap().phase(),
        PointerAppearancePhase::Hover
    );
    assert_eq!(state.pressed(), None);

    state.hover(Some(pressed_visual));
    state.press();
    state.hover(Some(appearance_key(12, 4)));
    assert!(
        state.retire(PresentationId::new(11)),
        "retiring a captured visual is a state change even if another presentation is hovered"
    );
    assert_eq!(
        state.active().unwrap().presentation(),
        PresentationId::new(12)
    );
    assert_eq!(state.pressed(), None);

    state.hover(Some(pressed_visual));
    assert_eq!(
        state.active().unwrap().phase(),
        PointerAppearancePhase::Hover
    );
}

#[test]
fn visual_transitions_do_not_mutate_evaluator_press_capture() {
    let mut render = GuiFrameRenderState::new_without_device(0x42, false);
    let target = PresentedInteractionKey::new(PresentationId::new(11), InteractionId::new(99));
    render.chrome.interaction.capture_tab_bar(Some(target));

    render.pointer_appearance.hover(Some(appearance_key(11, 3)));
    render.pointer_appearance.press();
    render.pointer_appearance.hover(Some(appearance_key(11, 4)));
    render.pointer_appearance.release();

    assert_eq!(
        render.chrome.interaction.tab_bar_capture(),
        Some(TabBarPressCapture::new(Some(target)))
    );
}

#[test]
fn cursor_leave_clears_hover_but_preserves_visual_and_input_capture() {
    let pressed_visual = appearance_key(11, 3);
    let mut render = GuiFrameRenderState::new_without_device(0x42, false);
    let target = PresentedInteractionKey::new(PresentationId::new(11), InteractionId::new(99));
    render.chrome.interaction.capture_tab_bar(Some(target));
    render.pointer_appearance.hover(Some(pressed_visual));
    render.pointer_appearance.press();
    render.set_dirty(false);

    assert!(render.clear_pointer_hover());

    assert_eq!(render.pointer_appearance.active(), None);
    assert_eq!(render.pointer_appearance.pressed(), Some(pressed_visual));
    assert_eq!(
        render.chrome.interaction.tab_bar_capture(),
        Some(TabBarPressCapture::new(Some(target)))
    );
    assert!(render.compositor.dirty);
}

#[test]
fn tab_release_uses_the_original_capture_after_hover_moves_or_leaves() {
    let captured = PresentedInteractionKey::new(PresentationId::new(21), InteractionId::new(7));
    let mut render = GuiFrameRenderState::new_without_device(0x42, false);
    render.chrome.interaction.capture_tab_bar(Some(captured));
    render.pointer_appearance.hover(Some(appearance_key(21, 1)));
    render.pointer_appearance.press();
    render.pointer_appearance.hover(Some(appearance_key(21, 2)));

    let event = RenderApp::take_tab_bar_release_events(&mut render, 500.0, 700.0)
        .into_iter()
        .next()
        .expect("captured release event");

    assert!(matches!(
        event,
        crate::thread_comm::InputEvent::PresentedPointer {
            presentation: 21,
            interaction: 7,
            pressed: false,
            x: 500.0,
            y: 700.0,
            ..
        }
    ));
    assert_eq!(render.chrome.interaction.tab_bar_capture(), None);
}

#[test]
fn blank_tab_band_capture_suppresses_release_without_fake_interaction() {
    let mut render = GuiFrameRenderState::new_without_device(0x42, false);
    render.chrome.interaction.capture_tab_bar(None);

    assert_eq!(
        render.chrome.interaction.tab_bar_capture(),
        Some(TabBarPressCapture::new(None))
    );
    assert!(RenderApp::take_tab_bar_release_events(&mut render, 40.0, 10.0).is_empty());
    assert_eq!(render.chrome.interaction.tab_bar_capture(), None);
}

#[test]
fn captured_release_precedes_deferred_presentation_retirement() {
    let target = PresentedInteractionKey::new(PresentationId::new(41), InteractionId::new(9));
    let mut render = GuiFrameRenderState::new_without_device(0x42, false);
    render.chrome.interaction.capture_tab_bar(Some(target));
    assert_eq!(render.route_presentation_retirement(41), None);

    let events = RenderApp::take_tab_bar_release_events(&mut render, 1.0, 2.0);
    assert!(matches!(
        events.as_slice(),
        [
            crate::thread_comm::InputEvent::PresentedPointer {
                presentation: 41,
                interaction: 9,
                pressed: false,
                ..
            },
            crate::thread_comm::InputEvent::PresentationRetired { presentation: 41 }
        ]
    ));
}

#[test]
fn cancellation_flushes_pinned_retirement_without_release() {
    let target = PresentedInteractionKey::new(PresentationId::new(51), InteractionId::new(10));
    let mut render = GuiFrameRenderState::new_without_device(0x42, false);
    render.chrome.interaction.capture_tab_bar(Some(target));
    render.chrome.interaction.toolbar_press_captured = true;
    render.chrome.interaction.toolbar_pressed = Some(3);
    render.pointer_appearance.hover(Some(appearance_key(51, 1)));
    render.pointer_appearance.press();
    assert_eq!(render.route_presentation_retirement(51), None);

    let (changed, retirements) = render.cancel_pointer_interaction();
    assert!(changed);
    assert_eq!(retirements, vec![51]);
    assert_eq!(render.pointer_appearance.active(), None);
    assert_eq!(render.pointer_appearance.pressed(), None);
    assert_eq!(render.chrome.interaction.tab_bar_capture(), None);
    assert!(!render.chrome.interaction.toolbar_press_captured);
    assert_eq!(render.chrome.interaction.toolbar_pressed, None);
    assert!(!render.pointer_inside);
}

#[test]
fn focus_cancellation_dirties_toolbar_and_compact_only_state() {
    let mut render = GuiFrameRenderState::new_without_device(0x42, false);
    render.pointer_inside = true;
    render.chrome.interaction.toolbar_hovered = Some(1);
    render.chrome.interaction.toolbar_pressed = Some(1);
    render.chrome.interaction.toolbar_press_captured = true;
    render.chrome.interaction.compact_bar_tool_hovered = Some(2);
    render.chrome.interaction.compact_bar_tool_pressed = Some(2);
    render.set_dirty(false);

    let (changed, retirements) = render.cancel_pointer_interaction();

    assert!(changed);
    assert!(retirements.is_empty());
    assert!(render.compositor.dirty);
    assert!(!render.pointer_inside);
    assert_eq!(render.chrome.interaction.toolbar_hovered, None);
    assert_eq!(render.chrome.interaction.toolbar_pressed, None);
    assert_eq!(render.chrome.interaction.compact_bar_tool_hovered, None);
    assert_eq!(render.chrome.interaction.compact_bar_tool_pressed, None);
}

#[test]
fn programmatic_popup_open_suppresses_underlying_hover_immediately() {
    let key = appearance_key(61, 1);
    let mut render = GuiFrameRenderState::new_without_device(0x42, false);
    render.pointer_appearance.hover(Some(key));
    render.pointer_appearance.press();
    render.set_dirty(false);

    render.set_popup_menu(Some(neomacs_renderer_wgpu::PopupMenuState::new(
        0.0,
        0.0,
        vec![],
        None,
        13.0,
        17.0,
        8.0,
    )));

    assert_eq!(render.pointer_appearance.active(), None);
    assert_eq!(render.pointer_appearance.pressed(), Some(key));
    assert!(render.compositor.dirty);
}

#[test]
fn non_root_owner_clears_stale_root_chrome_hover() {
    let mut render = GuiFrameRenderState::new_without_device(0x42, false);
    render.chrome.interaction.menu_bar_hovered = Some(1);
    render.chrome.interaction.compact_bar_menu_hovered = Some(2);
    render.chrome.interaction.compact_bar_tool_hovered = Some(3);
    render.chrome.interaction.toolbar_hovered = Some(4);

    assert!(RenderApp::suppress_root_chrome_hover(&mut render));
    assert_eq!(render.chrome.interaction.menu_bar_hovered, None);
    assert_eq!(render.chrome.interaction.compact_bar_menu_hovered, None);
    assert_eq!(render.chrome.interaction.compact_bar_tool_hovered, None);
    assert_eq!(render.chrome.interaction.toolbar_hovered, None);
}

#[test]
fn tab_capture_survives_tab_bar_removal_until_release() {
    let captured = PresentedInteractionKey::new(PresentationId::new(31), InteractionId::new(8));
    let mut interaction = crate::render_thread::state::GuiChromeInteractionState::default();
    interaction.capture_tab_bar(Some(captured));

    interaction.clear_tab_bar();

    assert_eq!(
        interaction.tab_bar_capture(),
        Some(TabBarPressCapture::new(Some(captured)))
    );
}

#[test]
fn topmost_child_blocks_root_chrome_ownership() {
    let mut app = make_test_app(800, 600, 1.0);
    let render = ensure_primary_frame(&mut app).expect("primary render");
    render.set_emacs_frame_id(0x42);
    let mut child = FrameGlyphBuffer::with_size(100.0, 100.0);
    child.frame_id = neomacs_display_protocol::DisplayFrameId::new(0x99);
    child.parent_id = neomacs_display_protocol::DisplayFrameId::new(0x42);
    child.parent_x = 0.0;
    child.parent_y = 40.0;
    render.update_child_frame(child);
    let window = app.frame_windows.primary_window().unwrap();

    assert!(matches!(
        RenderApp::pointer_owner(window, 20.0, 56.0),
        PointerOwner::Child { frame_id: 0x99, .. }
    ));
}

#[test]
fn popup_owns_pointer_above_underlying_presented_content() {
    let mut app = make_test_app(800, 600, 1.0);
    let render = ensure_primary_frame(&mut app).expect("primary render");
    render.set_popup_menu(Some(neomacs_renderer_wgpu::PopupMenuState::new(
        0.0,
        0.0,
        vec![],
        None,
        13.0,
        17.0,
        8.0,
    )));
    let window = app.frame_windows.primary_window().unwrap();

    let owner = RenderApp::pointer_owner(window, 20.0, 56.0);
    assert_eq!(owner, PointerOwner::Popup);
    assert!(
        owner.target().is_none(),
        "popup suppresses underlying appearance"
    );
    assert!(
        owner.permits_root_chrome(),
        "popup branch retains explicit chrome delegation"
    );
}

fn ensure_primary_frame(app: &mut RenderApp) -> Option<&mut GuiFrameRenderState> {
    if app
        .frame_windows
        .primary_window()
        .map(|ws| &ws.render)
        .is_none()
    {
        let device = make_test_device()?;
        let __render = GuiFrameRenderState::new(
            0,
            &device,
            app.frame_windows
                .primary_window()
                .map_or(1.0, |ws| ws.scale_factor()),
            false,
        );
        if let Some(window_state) = app.frame_windows.primary_window_mut() {
            window_state.render = __render;
        }
    }
    app.frame_windows
        .primary_window_mut()
        .map(|ws| &mut ws.render)
}

#[test]
fn frame_chrome_toolbar_origin_comes_from_authoritative_band_bounds() {
    use neomacs_display_protocol::frame_chrome::{
        ChromeBandRequest, FrameChrome, FrameChromeContent, FrameChromeKind, FrameSize,
        MenuBarContent, ToolBarContent,
    };

    let mut frame = FrameGlyphBuffer::with_size(800.0, 600.0);
    frame.frame_chrome = FrameChrome::layout(
        FrameSize::new(800.0, 600.0).expect("frame size"),
        vec![
            ChromeBandRequest::new(
                FrameChromeKind::MenuBar,
                19.0,
                FrameChromeContent::MenuBar(MenuBarContent::empty()),
            ),
            ChromeBandRequest::new(
                FrameChromeKind::ToolBar,
                41.0,
                FrameChromeContent::ToolBar(ToolBarContent::empty()),
            ),
        ],
    )
    .expect("frame chrome");

    let bounds = crate::render_thread::render_pass::frame_chrome_toolbar_bounds(&frame)
        .expect("toolbar band");
    assert_eq!(bounds.y(), 19.0);
    assert_eq!(bounds.height(), 41.0);
}

#[test]
fn chrome_hit_uses_absolute_semantic_hit_regions() {
    use neomacs_display_protocol::frame_chrome::{
        BandRect, ChromeAction, ChromeBandRequest, ChromeDisplayRow, ChromeHitRegion, FrameChrome,
        FrameChromeContent, FrameChromeKind, FrameSize, MenuBarContent, PositionedChromeItem,
        ToolBarContent,
    };
    use neomacs_display_protocol::{MenuBarItem, ToolBarItem, ToolBarItemType};

    let menu = MenuBarContent::new(
        vec![PositionedChromeItem::new(
            BandRect::new(8.0, 0.0, 48.0, 18.0).expect("menu bounds"),
            MenuBarItem {
                index: 0,
                label: "File".into(),
                key: "file".into(),
            },
            ChromeAction::OpenMenu {
                index: 0,
                key: "file".into(),
            },
        )],
        Color::WHITE,
        Color::BLACK,
    );
    let tool = ToolBarContent::new(
        vec![PositionedChromeItem::new(
            BandRect::new(5.0, 0.0, 24.0, 34.0).expect("tool bounds"),
            ToolBarItem {
                index: 0,
                key: "open".into(),
                image: None,
                label: String::new(),
                help: String::new(),
                enabled: true,
                selected: false,
                item_type: ToolBarItemType::Button,
                wrap: false,
            },
            ChromeAction::InvokeToolBarItem { index: 0 },
        )],
        Color::WHITE,
        Color::BLACK,
        24,
        5,
    );
    let tab = ChromeBandRequest::new(
        FrameChromeKind::TabBar,
        18.0,
        FrameChromeContent::DisplayRow(ChromeDisplayRow::empty_tab_bar()),
    )
    .with_hit_regions(vec![ChromeHitRegion::new(
        BandRect::new(8.0, 0.0, 80.0, 18.0).expect("tab bounds"),
        ChromeAction::Presented {
            interaction: InteractionId::new(4),
        },
    )]);
    let mut frame = FrameGlyphBuffer::with_size(800.0, 600.0);
    frame.presentation_id = PresentationId::new(9);
    frame.frame_chrome = FrameChrome::layout(
        FrameSize::new(800.0, 600.0).expect("frame size"),
        vec![
            ChromeBandRequest::new(
                FrameChromeKind::MenuBar,
                18.0,
                FrameChromeContent::MenuBar(menu),
            ),
            ChromeBandRequest::new(
                FrameChromeKind::ToolBar,
                34.0,
                FrameChromeContent::ToolBar(tool),
            ),
            tab,
        ],
    )
    .expect("frame chrome");

    assert!(matches!(
        frame_chrome_hit(&frame, 20.0, 30.0),
        Some((ChromeAction::InvokeToolBarItem { index: 0 }, bounds))
            if bounds.y() == 18.0
    ));
    assert!(matches!(
        frame_chrome_hit(&frame, 20.0, 56.0),
        Some((ChromeAction::Presented { interaction }, bounds))
            if interaction.get() == 4 && bounds.y() == 52.0
    ));
    frame
        .install_presented_pointer(
            vec![neomacs_display_protocol::PresentedPointerRegion::new(
                neomacs_display_protocol::FrameRect::new(8.0, 52.0, 80.0, 18.0)
                    .expect("pointer bounds"),
                Some(InteractionId::new(12)),
                None,
            )],
            vec![],
        )
        .expect("displayed pointer map");
    let mut app = make_test_app(800, 600, 1.0);
    let Some(primary_frame) = ensure_primary_frame(&mut app) else {
        return;
    };
    primary_frame.compositor.current_frame = Some(frame);
    assert_eq!(app.toolbar_y_origin(), 18.0);
    assert_eq!(app.toolbar_hit_test(20.0, 30.0), Some(0));
    assert_eq!(app.tab_bar_hit_test(20.0, 56.0), Some((9, 12)));
    let hit = app.menu_bar_hit_test(20.0, 10.0).expect("menu hit");
    assert_eq!(hit.index, 0);
    assert_eq!(hit.key, "file");
    assert_eq!(hit.menu_x, 8.0);
    assert_eq!(hit.anchor.x, 8.0);
    assert_eq!(hit.anchor.y, 0.0);
    assert_eq!(hit.anchor.width, 48.0);
    assert_eq!(hit.anchor.height, 18.0);
}

#[test]
fn empty_and_trailing_frame_chrome_space_owns_pointer_input() {
    use neomacs_display_protocol::frame_chrome::{
        ChromeBandRequest, CompactBarContent, FrameChrome, FrameChromeContent, FrameChromeKind,
        FrameSize, MenuBarContent,
    };

    let mut frame = FrameGlyphBuffer::with_size(200.0, 100.0);
    frame.frame_chrome = FrameChrome::layout(
        FrameSize::new(200.0, 100.0).expect("frame size"),
        vec![ChromeBandRequest::new(
            FrameChromeKind::CompactBar,
            20.0,
            FrameChromeContent::CompactBar(CompactBarContent::empty()),
        )],
    )
    .expect("empty compact band");

    assert!(frame_chrome_owns_pointer(&frame, 190.0, 10.0));
    assert!(!frame_chrome_owns_pointer(&frame, 190.0, 50.0));

    frame.frame_chrome = FrameChrome::layout(
        FrameSize::new(200.0, 100.0).expect("frame size"),
        vec![ChromeBandRequest::new(
            FrameChromeKind::MenuBar,
            18.0,
            FrameChromeContent::MenuBar(MenuBarContent::empty()),
        )],
    )
    .expect("empty menu band");

    assert!(frame_chrome_owns_pointer(&frame, 190.0, 10.0));
    assert!(!frame_chrome_owns_pointer(&frame, 190.0, 28.0));
}

// ===================================================================
// translate_key — Function keys
// ===================================================================

#[test]
fn translate_key_f1_through_f12() {
    let expected: Vec<(NamedKey, u32)> = vec![
        (NamedKey::F1, 0xffbe),
        (NamedKey::F2, 0xffbf),
        (NamedKey::F3, 0xffc0),
        (NamedKey::F4, 0xffc1),
        (NamedKey::F5, 0xffc2),
        (NamedKey::F6, 0xffc3),
        (NamedKey::F7, 0xffc4),
        (NamedKey::F8, 0xffc5),
        (NamedKey::F9, 0xffc6),
        (NamedKey::F10, 0xffc7),
        (NamedKey::F11, 0xffc8),
        (NamedKey::F12, 0xffc9),
    ];
    for (named, keysym) in expected {
        assert_eq!(
            RenderApp::translate_key(&Key::Named(named)),
            keysym,
            "F-key mismatch for {:?}",
            named
        );
    }
}

// ===================================================================
// translate_key — Navigation / editing keys
// ===================================================================

#[test]
fn translate_key_navigation_keys() {
    let cases: Vec<(NamedKey, u32)> = vec![
        (NamedKey::Escape, 0xff1b),
        (NamedKey::Enter, 0xff0d),
        (NamedKey::Tab, 0xff09),
        (NamedKey::Backspace, 0xff08),
        (NamedKey::Delete, 0xffff),
        (NamedKey::Insert, 0xff63),
        (NamedKey::Home, 0xff50),
        (NamedKey::End, 0xff57),
        (NamedKey::PageUp, 0xff55),
        (NamedKey::PageDown, 0xff56),
    ];
    for (named, keysym) in cases {
        assert_eq!(
            RenderApp::translate_key(&Key::Named(named)),
            keysym,
            "Navigation key mismatch for {:?}",
            named
        );
    }
}

// ===================================================================
// translate_key — Arrow keys
// ===================================================================

#[test]
fn translate_key_arrow_keys() {
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
}

// ===================================================================
// translate_key — Space
// ===================================================================

#[test]
fn translate_key_space() {
    assert_eq!(RenderApp::translate_key(&Key::Named(NamedKey::Space)), 0x20);
}

// ===================================================================
// translate_key — Other named keys (PrintScreen, ScrollLock, Pause)
// ===================================================================

#[test]
fn translate_key_other_named() {
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::PrintScreen)),
        0xff61
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::ScrollLock)),
        0xff14
    );
    assert_eq!(
        RenderApp::translate_key(&Key::Named(NamedKey::Pause)),
        0xff13
    );
}

// ===================================================================
// translate_key — Modifier keys should return 0 (suppressed)
// ===================================================================

#[test]
fn translate_key_modifier_keys_suppressed() {
    let modifiers = vec![
        NamedKey::Shift,
        NamedKey::Control,
        NamedKey::Alt,
        NamedKey::Super,
        NamedKey::CapsLock,
        NamedKey::NumLock,
    ];
    for named in modifiers {
        assert_eq!(
            RenderApp::translate_key(&Key::Named(named)),
            0,
            "Modifier {:?} should be suppressed (return 0)",
            named
        );
    }
}

// ===================================================================
// translate_key — Character keys
// ===================================================================

#[test]
fn translate_key_ascii_characters() {
    for ch in 'a'..='z' {
        let key = Key::Character(SmolStr::new(ch.to_string()));
        assert_eq!(
            RenderApp::translate_key(&key),
            ch as u32,
            "Character key mismatch for '{}'",
            ch
        );
    }
}

#[test]
fn translate_key_digit_characters() {
    for ch in '0'..='9' {
        let key = Key::Character(SmolStr::new(ch.to_string()));
        assert_eq!(
            RenderApp::translate_key(&key),
            ch as u32,
            "Digit key mismatch for '{}'",
            ch
        );
    }
}

#[test]
fn translate_key_special_characters() {
    let specials = vec![
        ('!', 0x21),
        ('@', 0x40),
        ('#', 0x23),
        ('/', 0x2f),
        ('-', 0x2d),
        ('=', 0x3d),
        ('[', 0x5b),
        (']', 0x5d),
        (';', 0x3b),
        ('\'', 0x27),
    ];
    for (ch, code) in specials {
        let key = Key::Character(SmolStr::new(ch.to_string()));
        assert_eq!(
            RenderApp::translate_key(&key),
            code,
            "Special char mismatch for '{}'",
            ch
        );
    }
}

#[test]
fn translate_key_unicode_character() {
    // Multi-byte Unicode characters should return the Unicode code point
    let key = Key::Character(SmolStr::new("\u{00e9}")); // e-acute
    assert_eq!(RenderApp::translate_key(&key), 0xe9);

    let key = Key::Character(SmolStr::new("\u{4e2d}")); // CJK character
    assert_eq!(RenderApp::translate_key(&key), 0x4e2d);
}

#[test]
fn translate_key_empty_character_string() {
    let key = Key::Character(SmolStr::new(""));
    assert_eq!(RenderApp::translate_key(&key), 0);
}

// ===================================================================
// translate_key — Unrecognized / dead keys
// ===================================================================

#[test]
fn translate_key_dead_returns_zero() {
    let key = Key::Dead(None);
    assert_eq!(RenderApp::translate_key(&key), 0);
}

#[test]
fn translate_key_unidentified_returns_zero() {
    let key = Key::Unidentified(winit::keyboard::NativeKey::Unidentified);
    assert_eq!(RenderApp::translate_key(&key), 0);
}

#[test]
fn translate_committed_text_prefers_uppercase_ascii_without_command_modifiers() {
    assert_eq!(
        RenderApp::translate_committed_text("A", 0),
        Some(vec!['A' as u32])
    );
}

#[test]
fn translate_committed_text_prefers_shifted_punctuation_without_command_modifiers() {
    assert_eq!(
        RenderApp::translate_committed_text("!", 0),
        Some(vec!['!' as u32])
    );
}

#[test]
fn translate_committed_text_ignores_control_only_text() {
    assert_eq!(RenderApp::translate_committed_text("\u{8}", 0), None);
    assert_eq!(RenderApp::translate_committed_text("\r", 0), None);
}

#[test]
fn named_backspace_does_not_use_control_text_path() {
    assert!(!RenderApp::should_use_committed_text(&Key::Named(
        NamedKey::Backspace
    )));
    assert!(!RenderApp::should_use_committed_text(&Key::Named(
        NamedKey::Delete
    )));
    assert!(RenderApp::should_use_committed_text(&Key::Character(
        "x".into()
    )));
}

#[test]
fn translate_committed_text_skips_command_modified_input() {
    assert_eq!(
        RenderApp::translate_committed_text("x", NEOMACS_META_MASK),
        None
    );
    assert_eq!(
        RenderApp::translate_committed_text("x", NEOMACS_CTRL_MASK),
        None
    );
    assert_eq!(
        RenderApp::translate_committed_text("x", NEOMACS_SUPER_MASK),
        None
    );
}

#[test]
fn translate_control_text_preserves_single_control_bytes() {
    assert_eq!(RenderApp::translate_control_text("\u{0e}"), Some(0x0e)); // C-n
    assert_eq!(RenderApp::translate_control_text("\u{10}"), Some(0x10)); // C-p
    assert_eq!(RenderApp::translate_control_text("\r"), Some(0x0d));
}

#[test]
fn translate_control_text_ignores_printable_and_multi_char_text() {
    assert_eq!(RenderApp::translate_control_text("n"), None);
    assert_eq!(RenderApp::translate_control_text("np"), None);
    assert_eq!(RenderApp::translate_control_text(""), None);
}

// ===================================================================
// detect_resize_edge — decorations enabled (always None)
// ===================================================================

#[test]
fn resize_edge_returns_none_when_decorations_enabled() {
    let app = make_test_app(800, 600, 1.0);
    // Default chrome has decorations_enabled = true
    assert!(
        app.frame_windows
            .primary_window()
            .expect("primary window state")
            .chrome()
            .decorations_enabled
    );
    assert_eq!(app.detect_resize_edge(0.0, 0.0), None);
    assert_eq!(app.detect_resize_edge(400.0, 300.0), None);
}

// ===================================================================
// detect_resize_edge — corners (5px border zone)
// ===================================================================

#[test]
fn resize_edge_top_left_corner() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    assert_eq!(
        app.detect_resize_edge(0.0, 0.0),
        Some(ResizeDirection::NorthWest)
    );
    assert_eq!(
        app.detect_resize_edge(4.9, 4.9),
        Some(ResizeDirection::NorthWest)
    );
}

#[test]
fn resize_edge_top_right_corner() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    // w=800, border=5 => on_right when x >= 795
    assert_eq!(
        app.detect_resize_edge(795.0, 0.0),
        Some(ResizeDirection::NorthEast)
    );
    assert_eq!(
        app.detect_resize_edge(799.0, 4.0),
        Some(ResizeDirection::NorthEast)
    );
}

#[test]
fn resize_edge_bottom_left_corner() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    // h=600, border=5 => on_bottom when y >= 595
    assert_eq!(
        app.detect_resize_edge(0.0, 595.0),
        Some(ResizeDirection::SouthWest)
    );
    assert_eq!(
        app.detect_resize_edge(4.0, 599.0),
        Some(ResizeDirection::SouthWest)
    );
}

#[test]
fn resize_edge_bottom_right_corner() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    assert_eq!(
        app.detect_resize_edge(795.0, 595.0),
        Some(ResizeDirection::SouthEast)
    );
    assert_eq!(
        app.detect_resize_edge(799.0, 599.0),
        Some(ResizeDirection::SouthEast)
    );
}

// ===================================================================
// detect_resize_edge — edges (not corners)
// ===================================================================

#[test]
fn resize_edge_left() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    // Left edge, but not in top or bottom border zone
    assert_eq!(
        app.detect_resize_edge(0.0, 300.0),
        Some(ResizeDirection::West)
    );
    assert_eq!(
        app.detect_resize_edge(4.9, 300.0),
        Some(ResizeDirection::West)
    );
}

#[test]
fn resize_edge_right() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    assert_eq!(
        app.detect_resize_edge(795.0, 300.0),
        Some(ResizeDirection::East)
    );
    assert_eq!(
        app.detect_resize_edge(799.0, 300.0),
        Some(ResizeDirection::East)
    );
}

#[test]
fn resize_edge_top() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    // Top edge, but not in left or right border zone
    assert_eq!(
        app.detect_resize_edge(400.0, 0.0),
        Some(ResizeDirection::North)
    );
    assert_eq!(
        app.detect_resize_edge(400.0, 4.9),
        Some(ResizeDirection::North)
    );
}

#[test]
fn resize_edge_bottom() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    assert_eq!(
        app.detect_resize_edge(400.0, 595.0),
        Some(ResizeDirection::South)
    );
    assert_eq!(
        app.detect_resize_edge(400.0, 599.0),
        Some(ResizeDirection::South)
    );
}

// ===================================================================
// detect_resize_edge — interior (no edge)
// ===================================================================

#[test]
fn resize_edge_interior_returns_none() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    // Center of the window — well inside border zone
    assert_eq!(app.detect_resize_edge(400.0, 300.0), None);
    // Just inside each border
    assert_eq!(app.detect_resize_edge(5.0, 5.0), None);
    assert_eq!(app.detect_resize_edge(794.9, 594.9), None);
}

// ===================================================================
// detect_resize_edge — boundary values at exactly the border threshold
// ===================================================================

#[test]
fn resize_edge_boundary_exact() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    // x=5.0 is NOT on_left (on_left requires x < 5.0)
    assert_eq!(app.detect_resize_edge(5.0, 300.0), None);
    // x=4.999... is still on_left
    assert_eq!(
        app.detect_resize_edge(4.999, 300.0),
        Some(ResizeDirection::West)
    );
    // y=5.0 is NOT on_top
    assert_eq!(app.detect_resize_edge(300.0, 5.0), None);
    // x=795.0 IS on_right (on_right requires x >= 795.0)
    assert_eq!(
        app.detect_resize_edge(795.0, 300.0),
        Some(ResizeDirection::East)
    );
    // x=794.9 is NOT on_right
    assert_eq!(app.detect_resize_edge(794.9, 300.0), None);
    // y=595.0 IS on_bottom
    assert_eq!(
        app.detect_resize_edge(300.0, 595.0),
        Some(ResizeDirection::South)
    );
    // y=594.9 is NOT on_bottom
    assert_eq!(app.detect_resize_edge(300.0, 594.9), None);
}

// ===================================================================
// detect_resize_edge — small window where border zones might overlap
// ===================================================================

#[test]
fn resize_edge_small_window() {
    let mut app = make_test_app(10, 10, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    // At (0,0) — top-left corner (left and top overlap)
    assert_eq!(
        app.detect_resize_edge(0.0, 0.0),
        Some(ResizeDirection::NorthWest)
    );
    // At (9,9) — bottom-right corner
    assert_eq!(
        app.detect_resize_edge(9.0, 9.0),
        Some(ResizeDirection::SouthEast)
    );
    // At (5,5) — the center, which is also exactly at the border threshold
    // on_left = 5 < 5 = false, on_right = 5 >= 5 = true
    // on_top = 5 < 5 = false, on_bottom = 5 >= 5 = true
    assert_eq!(
        app.detect_resize_edge(5.0, 5.0),
        Some(ResizeDirection::SouthEast)
    );
}

// ===================================================================
// titlebar_hit_test — decorations enabled (always 0)
// ===================================================================

#[test]
fn titlebar_returns_zero_when_decorations_enabled() {
    let app = make_test_app(800, 600, 1.0);
    assert!(
        app.frame_windows
            .primary_window()
            .expect("primary window state")
            .chrome()
            .decorations_enabled
    );
    assert_eq!(app.titlebar_hit_test(0.0, 0.0), 0);
    assert_eq!(app.titlebar_hit_test(400.0, 10.0), 0);
}

// ===================================================================
// titlebar_hit_test — fullscreen (always 0)
// ===================================================================

#[test]
fn titlebar_returns_zero_when_fullscreen() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .is_fullscreen = true;
    assert_eq!(app.titlebar_hit_test(400.0, 10.0), 0);
}

// ===================================================================
// titlebar_hit_test — zero titlebar height (always 0)
// ===================================================================

#[test]
fn titlebar_returns_zero_when_height_is_zero() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 0.0;
    assert_eq!(app.titlebar_hit_test(400.0, 10.0), 0);
}

#[test]
fn titlebar_returns_zero_when_height_is_negative() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = -5.0;
    assert_eq!(app.titlebar_hit_test(400.0, 0.0), 0);
}

// ===================================================================
// titlebar_hit_test — below title bar (always 0)
// ===================================================================

#[test]
fn titlebar_returns_zero_below_titlebar() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 30.0;
    // y >= titlebar_height means below
    assert_eq!(app.titlebar_hit_test(400.0, 30.0), 0);
    assert_eq!(app.titlebar_hit_test(400.0, 100.0), 0);
}

// ===================================================================
// titlebar_hit_test — button areas
// Window width (logical) = 800 / 1.0 = 800.  btn_w = 46.
//   close:    x >= 800-46  = 754
//   maximize: x >= 800-92  = 708  and x < 754
//   minimize: x >= 800-138 = 662  and x < 708
//   drag:     x < 662
// ===================================================================

#[test]
fn titlebar_close_button() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 30.0;
    assert_eq!(app.titlebar_hit_test(754.0, 15.0), 2);
    assert_eq!(app.titlebar_hit_test(799.0, 0.0), 2);
}

#[test]
fn titlebar_maximize_button() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 30.0;
    assert_eq!(app.titlebar_hit_test(708.0, 15.0), 3);
    assert_eq!(app.titlebar_hit_test(753.9, 15.0), 3);
}

#[test]
fn titlebar_minimize_button() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 30.0;
    assert_eq!(app.titlebar_hit_test(662.0, 15.0), 4);
    assert_eq!(app.titlebar_hit_test(707.9, 15.0), 4);
}

#[test]
fn titlebar_drag_area() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 30.0;
    assert_eq!(app.titlebar_hit_test(0.0, 15.0), 1);
    assert_eq!(app.titlebar_hit_test(300.0, 15.0), 1);
    assert_eq!(app.titlebar_hit_test(661.9, 15.0), 1);
}

// ===================================================================
// titlebar_hit_test — with scale_factor > 1
// Logical width = physical_width / scale_factor = 1600 / 2.0 = 800
// So button positions in logical pixels are the same as the 800px case.
// ===================================================================

#[test]
fn titlebar_with_scale_factor() {
    let mut app = make_test_app(1600, 1200, 2.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 30.0;
    // Logical width = 1600/2.0 = 800
    // close_x = 800-46 = 754, max_x = 708, min_x = 662
    assert_eq!(app.titlebar_hit_test(760.0, 10.0), 2); // close
    assert_eq!(app.titlebar_hit_test(720.0, 10.0), 3); // maximize
    assert_eq!(app.titlebar_hit_test(670.0, 10.0), 4); // minimize
    assert_eq!(app.titlebar_hit_test(100.0, 10.0), 1); // drag
}

// ===================================================================
// titlebar_hit_test — boundary between buttons
// ===================================================================

#[test]
fn titlebar_button_boundaries() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 30.0;
    // Exact boundary: close_x = 754
    assert_eq!(app.titlebar_hit_test(754.0, 15.0), 2); // close
    assert_eq!(app.titlebar_hit_test(753.9, 15.0), 3); // maximize (just left of close)
    // Exact boundary: max_x = 708
    assert_eq!(app.titlebar_hit_test(708.0, 15.0), 3); // maximize
    assert_eq!(app.titlebar_hit_test(707.9, 15.0), 4); // minimize (just left of maximize)
    // Exact boundary: min_x = 662
    assert_eq!(app.titlebar_hit_test(662.0, 15.0), 4); // minimize
    assert_eq!(app.titlebar_hit_test(661.9, 15.0), 1); // drag (just left of minimize)
}

// ===================================================================
// titlebar_hit_test — y boundary at titlebar_height
// ===================================================================

#[test]
fn titlebar_y_boundary() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 30.0;
    // Just inside (y=29.9 < 30.0)
    assert_eq!(app.titlebar_hit_test(100.0, 29.9), 1);
    // Exactly at boundary (y=30.0 >= 30.0)
    assert_eq!(app.titlebar_hit_test(100.0, 30.0), 0);
}

// ===================================================================
// titlebar_hit_test — custom titlebar height
// ===================================================================

#[test]
fn titlebar_custom_height() {
    let mut app = make_test_app(800, 600, 1.0);
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .decorations_enabled = false;
    app.frame_windows
        .primary_window_mut()
        .expect("primary window state")
        .chrome_mut()
        .titlebar_height = 50.0;
    // y=49 is in the titlebar
    assert_eq!(app.titlebar_hit_test(100.0, 49.0), 1);
    // y=50 is below
    assert_eq!(app.titlebar_hit_test(100.0, 50.0), 0);
}

// ===================================================================
// TITLEBAR_BUTTON_WIDTH constant
// ===================================================================

#[test]
fn titlebar_button_width_constant() {
    assert_eq!(RenderApp::TITLEBAR_BUTTON_WIDTH, 46.0);
}
