//! Pointer, wheel, and hover handling for winit window events.

use super::RenderApp;
use super::frame_windows::{ChromePress, GuiFrameWindowState};
use super::input::{MenuBarHit, frame_chrome_hit, frame_chrome_owns_pointer};
use super::state::PresentedInteractionKey;
use crate::backend::wgpu::NEOMACS_SUPER_MASK;
use crate::core::frame_glyphs::FrameGlyph;
use crate::thread_comm::{
    InputEvent, PointerAction, PointerPosition, PointerTarget, PositionedPointerInput, ScrollDelta,
    WebKitPointerTarget,
};
use neomacs_display_protocol::frame_chrome::{ChromeAction, FrameChromeKind};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::window::WindowId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PointerOwner {
    Popup,
    Child {
        frame_id: u64,
        x: f32,
        y: f32,
    },
    Root {
        frame_id: u64,
        x: f32,
        y: f32,
    },
    /// Live surface area not covered by the immutable root presentation.
    Expose {
        frame_id: u64,
        x: f32,
        y: f32,
    },
}

impl PointerOwner {
    pub(super) fn target(self) -> Option<(f32, f32, u64)> {
        match self {
            Self::Popup | Self::Expose { .. } => None,
            Self::Child { frame_id, x, y } | Self::Root { frame_id, x, y } => {
                Some((x, y, frame_id))
            }
        }
    }

    /// Raw evaluator input still names the live frame in expose area, while
    /// presentation-qualified hit testing must not.
    pub(super) fn raw_target(self) -> Option<(f32, f32, u64)> {
        match self {
            Self::Popup => None,
            Self::Child { frame_id, x, y }
            | Self::Root { frame_id, x, y }
            | Self::Expose { frame_id, x, y } => Some((x, y, frame_id)),
        }
    }

    /// Popup handling deliberately delegates menu/chrome clicks back to the
    /// existing popup branch, but never exposes an underlying presented target.
    pub(super) fn permits_root_chrome(self) -> bool {
        !matches!(self, Self::Child { .. })
    }

    pub(super) fn owns_root_hover(self) -> bool {
        matches!(self, Self::Root { .. })
    }

    pub(super) fn permits_native_chrome(self) -> bool {
        matches!(self, Self::Root { .. } | Self::Expose { .. })
    }
}

/// Search a glyph buffer for an inline xwidget at the given local coordinates.
/// Returns `(xwidget_id, relative_x, relative_y)` if found.
fn webkit_glyph_hit_test(glyphs: &[FrameGlyph], x: f32, y: f32) -> Option<(u32, i32, i32)> {
    for glyph in glyphs.iter().rev() {
        if let FrameGlyph::Xwidget {
            xwidget_id,
            x: wx,
            y: wy,
            width,
            height,
            ..
        } = glyph
            && x >= *wx
            && x < *wx + *width
            && y >= *wy
            && y < *wy + *height
        {
            return Some((xwidget_id.get(), (x - *wx) as i32, (y - *wy) as i32));
        }
    }
    None
}

/// Search a glyph buffer for an inline shader surface at the given local
/// coordinates (the `webkit_glyph_hit_test` mirror for `iMouse` click state).
/// Returns `(surface_id, u, v)` — the pointer's normalized position inside
/// the glyph rect (top-left origin) — if found.
fn surface_glyph_hit_test(glyphs: &[FrameGlyph], x: f32, y: f32) -> Option<(u32, f32, f32)> {
    for glyph in glyphs.iter().rev() {
        if let FrameGlyph::Surface {
            surface_id,
            x: sx,
            y: sy,
            width,
            height,
            ..
        } = glyph
            && x >= *sx
            && x < *sx + *width
            && y >= *sy
            && y < *sy + *height
        {
            return Some((surface_id.get(), (x - *sx) / *width, (y - *sy) / *height));
        }
    }
    None
}

impl RenderApp {
    fn positioned_pointer_input_event(
        render: &super::frame_windows::GuiFrameRenderState,
        owner: PointerOwner,
        position: PointerPosition,
        action: PointerAction,
    ) -> Result<InputEvent, neomacs_display_protocol::PresentedHitError> {
        let target = if owner.target().is_some() {
            match render.presented_region_observation(
                position.target_frame_id,
                position.x,
                position.y,
            )? {
                Some((presentation, hit)) => PointerTarget::Presented {
                    presentation: presentation.get(),
                    hit,
                },
                None => PointerTarget::Unpresented,
            }
        } else {
            PointerTarget::Unpresented
        };
        Ok(InputEvent::PositionedPointer(PositionedPointerInput {
            position,
            target,
            action,
        }))
    }

    fn webkit_pointer_target(
        id: u32,
        relative_x: i32,
        relative_y: i32,
    ) -> Option<WebKitPointerTarget> {
        (id != 0).then_some(WebKitPointerTarget {
            id,
            relative_x,
            relative_y,
        })
    }

    pub(super) fn suppress_root_chrome_hover(
        render: &mut super::frame_windows::GuiFrameRenderState,
    ) -> bool {
        let interaction = &mut render.chrome.interaction;
        let changed = interaction.menu_bar_hovered.is_some()
            || interaction.compact_bar_menu_hovered.is_some()
            || interaction.compact_bar_tool_hovered.is_some()
            || interaction.toolbar_hovered.is_some();
        interaction.menu_bar_hovered = None;
        interaction.compact_bar_menu_hovered = None;
        interaction.compact_bar_tool_hovered = None;
        interaction.toolbar_hovered = None;
        changed
    }

    pub(super) fn pointer_owner(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> PointerOwner {
        if window_state.render.overlays.popup_menu.is_some() {
            return PointerOwner::Popup;
        }
        let Some((frame_x, frame_y)) = window_state.render.root_frame_point_from_surface(x, y)
        else {
            return PointerOwner::Expose {
                frame_id: window_state.render.emacs_frame_id,
                x,
                y,
            };
        };
        if let Some((frame_id, local_x, local_y)) = window_state
            .render
            .compositor
            .child_frames
            .hit_test(frame_x, frame_y)
        {
            PointerOwner::Child {
                frame_id,
                x: local_x,
                y: local_y,
            }
        } else {
            PointerOwner::Root {
                frame_id: window_state.render.emacs_frame_id,
                x: frame_x,
                y: frame_y,
            }
        }
    }
    #[cfg(feature = "wpe-webkit")]
    fn floating_webkit_hit_test(
        floating_webkits: &[crate::core::scene::FloatingWebKit],
        x: f32,
        y: f32,
    ) -> Option<(u32, i32, i32)> {
        floating_webkits.iter().rev().find_map(|wk| {
            if x >= wk.x && x < wk.x + wk.width && y >= wk.y && y < wk.y + wk.height {
                Some((wk.webkit_id.get(), (x - wk.x) as i32, (y - wk.y) as i32))
            } else {
                None
            }
        })
    }

    fn pointer_target_for_frame_window(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> (f32, f32, u64) {
        #[cfg(feature = "wpe-webkit")]
        if Self::floating_webkit_hit_test(&window_state.render.floating_webkits, x, y).is_some() {
            return (x, y, window_state.render.emacs_frame_id);
        }
        if let Some((fid, local_x, local_y)) =
            window_state.render.compositor.child_frames.hit_test(x, y)
        {
            (local_x, local_y, fid)
        } else {
            (x, y, window_state.render.emacs_frame_id)
        }
    }

    fn glyphs_for_frame_window_pointer_target(
        window_state: &GuiFrameWindowState,
        target_fid: u64,
    ) -> Option<&[FrameGlyph]> {
        if target_fid == window_state.render.emacs_frame_id {
            window_state
                .render
                .compositor
                .current_frame
                .as_ref()
                .map(|frame| frame.glyphs.as_slice())
        } else {
            window_state
                .render
                .compositor
                .child_frames
                .frames
                .get(&target_fid)
                .map(|entry| entry.frame.glyphs.as_slice())
        }
    }

    fn webkit_target_for_frame_window(
        window_state: &GuiFrameWindowState,
        target_fid: u64,
        ev_x: f32,
        ev_y: f32,
    ) -> (u32, i32, i32) {
        #[cfg(feature = "wpe-webkit")]
        if target_fid == window_state.render.emacs_frame_id
            && let Some(target) =
                Self::floating_webkit_hit_test(&window_state.render.floating_webkits, ev_x, ev_y)
        {
            return target;
        }

        Self::glyphs_for_frame_window_pointer_target(window_state, target_fid)
            .and_then(|glyphs| webkit_glyph_hit_test(glyphs, ev_x, ev_y))
            .unwrap_or((0, 0, 0))
    }

    fn frame_window_menu_bar_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<MenuBarHit> {
        Self::frame_window_menu_hit_test(window_state, x, y)
    }

    fn frame_window_compact_bar_menu_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<MenuBarHit> {
        Self::frame_window_menu_hit_test(window_state, x, y)
    }

    fn menu_bar_click_event(hit: MenuBarHit, emacs_frame_id: u64) -> InputEvent {
        InputEvent::MenuBarClick {
            index: hit.index as i32,
            key: hit.key,
            menu_x: hit.menu_x,
            anchor: hit.anchor,
            emacs_frame_id,
        }
    }

    fn frame_window_compact_bar_tool_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<u32> {
        Self::frame_window_tool_hit_test(window_state, x, y)
    }

    fn frame_window_band_bounds(
        window_state: &GuiFrameWindowState,
        kind: FrameChromeKind,
    ) -> Option<neomacs_display_protocol::frame_chrome::FrameRect> {
        window_state
            .render
            .compositor
            .current_frame
            .as_ref()?
            .frame_chrome
            .band(kind)
            .map(|band| band.bounds())
    }

    fn frame_window_point_in_band(
        window_state: &GuiFrameWindowState,
        kind: FrameChromeKind,
        x: f32,
        y: f32,
    ) -> bool {
        Self::frame_window_band_bounds(window_state, kind).is_some_and(|bounds| {
            x >= bounds.x()
                && x < bounds.x() + bounds.width()
                && y >= bounds.y()
                && y < bounds.y() + bounds.height()
        })
    }

    fn frame_window_toolbar_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<u32> {
        Self::frame_window_tool_hit_test(window_state, x, y)
    }

    pub(super) fn frame_window_tab_bar_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<PresentedInteractionKey> {
        if let Ok(Some(hit)) =
            window_state
                .render
                .presented_pointer_hit(window_state.render.emacs_frame_id, x, y)
            && let Some(interaction) = hit.interaction()
        {
            return Some(PresentedInteractionKey::new(
                hit.presentation(),
                interaction,
            ));
        }
        None
    }

    fn presented_pointer_input_event(
        render: &super::frame_windows::GuiFrameRenderState,
        target: PresentedInteractionKey,
        pressed: bool,
        x: f32,
        y: f32,
    ) -> InputEvent {
        InputEvent::PresentedPointer {
            presentation: target.presentation().get(),
            interaction: target.interaction().get(),
            pressed,
            button: 1,
            x,
            y,
            emacs_frame_id: if target.frame_id() == 0 {
                render.emacs_frame_id
            } else {
                target.frame_id()
            },
        }
    }

    fn presented_interaction_for_owner(
        window_state: &GuiFrameWindowState,
        owner: PointerOwner,
    ) -> Result<Option<PresentedInteractionKey>, neomacs_display_protocol::PresentedHitError> {
        let Some((x, y, frame_id)) = owner.target() else {
            return Ok(None);
        };
        let Some((presentation, semantic)) = window_state
            .render
            .presented_region_observation(frame_id, x, y)?
        else {
            return Ok(None);
        };
        if semantic.is_none() {
            return Ok(None);
        }
        let Some(hit) = window_state.render.presented_pointer_hit(frame_id, x, y)? else {
            return Ok(None);
        };
        if hit.presentation() != presentation {
            return Err(
                neomacs_display_protocol::PresentedHitError::StalePresentation {
                    expected: presentation,
                    requested: hit.presentation(),
                },
            );
        }
        let Some(interaction) = hit.interaction() else {
            return Ok(None);
        };
        Ok(Some(PresentedInteractionKey::for_frame(
            frame_id,
            hit.presentation(),
            interaction,
        )))
    }

    pub(super) fn capture_presented_pointer_press(
        window_state: &mut GuiFrameWindowState,
        owner: PointerOwner,
        x: f32,
        y: f32,
    ) -> Result<Option<InputEvent>, neomacs_display_protocol::PresentedHitError> {
        let Some((local_x, local_y, _)) = owner.target() else {
            return Ok(None);
        };
        let Some(target) = Self::presented_interaction_for_owner(window_state, owner)? else {
            return Ok(None);
        };
        window_state
            .render
            .capture_presented_at(target, (x - local_x, y - local_y));
        Ok(Some(Self::presented_pointer_input_event(
            &window_state.render,
            target,
            true,
            local_x,
            local_y,
        )))
    }

    pub(super) fn capture_tab_band_press(
        window_state: &mut GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<InputEvent> {
        let target = Self::frame_window_tab_bar_hit_test(window_state, x, y);
        window_state.render.capture_presented(target);
        target.map(|target| {
            Self::presented_pointer_input_event(&window_state.render, target, true, x, y)
        })
    }

    pub(super) fn take_presented_release_events(
        render: &mut super::frame_windows::GuiFrameRenderState,
        x: f32,
        y: f32,
    ) -> Vec<InputEvent> {
        let release = render.take_presented_capture().and_then(|capture| {
            let target = capture.target()?;
            let (local_x, local_y) = capture.local_coordinates(x, y);
            Some(Self::presented_pointer_input_event(
                render, target, false, local_x, local_y,
            ))
        });
        release
            .into_iter()
            .chain(
                render
                    .take_deferred_pointer_retirements()
                    .into_iter()
                    .map(|presentation| InputEvent::PresentationRetired { presentation }),
            )
            .collect()
    }

    fn frame_window_tool_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<u32> {
        let frame = window_state.render.compositor.current_frame.as_ref()?;
        match frame_chrome_hit(frame, x, y)?.0 {
            ChromeAction::InvokeToolBarItem { index } => Some(*index),
            _ => None,
        }
    }

    fn frame_window_menu_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<MenuBarHit> {
        let frame = window_state.render.compositor.current_frame.as_ref()?;
        let (ChromeAction::OpenMenu { index, key }, bounds) = frame_chrome_hit(frame, x, y)? else {
            return None;
        };
        Some(MenuBarHit {
            index: *index,
            key: key.clone(),
            menu_x: bounds.x(),
            anchor: crate::thread_comm::PopupAnchorRect::new(
                bounds.x(),
                bounds.y(),
                bounds.width(),
                bounds.height(),
            ),
        })
    }

    #[cfg(test)]
    pub(super) fn pointer_target_at(&self, x: f32, y: f32) -> (f32, f32, u64) {
        if let Some(primary_state) = self.frame_windows.primary_window() {
            return Self::pointer_target_for_frame_window(primary_state, x, y);
        }
        let primary_frame_id = self.frame_windows.primary_event_frame_id();
        #[cfg(feature = "wpe-webkit")]
        if let Some(primary_frame) = self.frame_windows.primary_window().map(|ws| &ws.render)
            && Self::floating_webkit_hit_test(&primary_frame.floating_webkits, x, y).is_some()
        {
            return (x, y, primary_frame_id);
        }
        if let Some((fid, local_x, local_y)) = self
            .frame_windows
            .primary_window()
            .expect("primary child frames")
            .render
            .compositor
            .child_frames
            .hit_test(x, y)
        {
            (local_x, local_y, fid)
        } else {
            (x, y, primary_frame_id)
        }
    }

    pub(super) fn handle_mouse_input(
        &mut self,
        window_id: WindowId,
        state: ElementState,
        button: MouseButton,
    ) {
        self.record_idle_dim_activity(window_id);
        if state == ElementState::Released
            && let Some(renderer) = self.renderer.as_mut()
        {
            // iMouse click state: any button release ends the pressed-surface
            // click (negates its iMouse.zw). Render-thread internal, like
            // hover — no-op when no surface is pressed, so it runs before any
            // chrome capture can swallow the release.
            renderer.surface_mouse_release();
        }
        if self.frame_windows.get_by_winit(window_id).is_some() {
            let mut event = None;
            let mut captured_events = Vec::new();
            let mut handled_chrome = false;
            let mut delivered_mouse_button = false;
            if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
                let x = window_state.render.mouse_pos.0;
                let y = window_state.render.mouse_pos.1;
                let pointer_owner = Self::pointer_owner(window_state, x, y);
                if button == MouseButton::Left {
                    let target = if window_state.render.pointer_inside {
                        pointer_owner
                            .target()
                            .map(|(x, y, frame_id)| (frame_id, x, y))
                    } else {
                        None
                    };
                    window_state
                        .render
                        .update_presented_pointer_button(target, state == ElementState::Pressed);
                }
                let popup_was_open = window_state.render.overlays.popup_menu.is_some();
                if !popup_was_open && state == ElementState::Pressed && button == MouseButton::Left
                {
                    if window_state.drag_resize_for_current_edge() {
                        handled_chrome = true;
                    }

                    if !handled_chrome
                        && Self::frame_window_titlebar_hit_test(window_state, x, y) > 0
                    {
                        match Self::frame_window_titlebar_hit_test(window_state, x, y) {
                            2 => {
                                event = Some(InputEvent::WindowClose {
                                    emacs_frame_id: window_state.render.emacs_frame_id,
                                });
                            }
                            action => {
                                window_state.handle_titlebar_action(action);
                            }
                        }
                        handled_chrome = true;
                    }

                    if !handled_chrome
                        && !window_state.chrome().decorations_enabled
                        && (self.modifiers & NEOMACS_SUPER_MASK) != 0
                    {
                        window_state.drag_window();
                        handled_chrome = true;
                    }
                }

                if popup_was_open {
                    if state == ElementState::Pressed && button == MouseButton::Left {
                        if !handled_chrome
                            && Self::frame_window_point_in_band(
                                window_state,
                                FrameChromeKind::CompactBar,
                                x,
                                y,
                            )
                        {
                            if let Some(hit) =
                                Self::frame_window_compact_bar_menu_hit_test(window_state, x, y)
                            {
                                self.comms
                                    .send_input(InputEvent::MenuSelection { index: -1 });
                                window_state.render.set_popup_menu(None);
                                window_state
                                    .render
                                    .chrome
                                    .interaction
                                    .compact_bar_menu_active = Some(hit.index);
                                event = Some(Self::menu_bar_click_event(
                                    hit,
                                    window_state.render.emacs_frame_id,
                                ));
                            } else {
                                event = Some(InputEvent::MenuSelection { index: -1 });
                                window_state.render.set_popup_menu(None);
                                window_state
                                    .render
                                    .chrome
                                    .interaction
                                    .compact_bar_menu_active = None;
                            }
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        } else if Self::frame_window_point_in_band(
                            window_state,
                            FrameChromeKind::MenuBar,
                            x,
                            y,
                        ) {
                            if let Some(hit) =
                                Self::frame_window_menu_bar_hit_test(window_state, x, y)
                            {
                                self.comms
                                    .send_input(InputEvent::MenuSelection { index: -1 });
                                window_state.render.set_popup_menu(None);
                                window_state
                                    .render
                                    .chrome
                                    .press_with_popup(&ChromePress::MenuBar(hit.index));
                                event = Some(Self::menu_bar_click_event(
                                    hit,
                                    window_state.render.emacs_frame_id,
                                ));
                            } else {
                                event = Some(InputEvent::MenuSelection { index: -1 });
                                window_state.render.set_popup_menu(None);
                                window_state.render.chrome.dismiss_menus();
                            }
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        } else if Self::frame_window_point_in_band(
                            window_state,
                            FrameChromeKind::TabBar,
                            x,
                            y,
                        ) {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            window_state.render.set_popup_menu(None);
                            window_state.render.chrome.dismiss_menus();
                            window_state
                                .render
                                .chrome
                                .interaction
                                .compact_bar_menu_active = None;
                            event = Self::capture_tab_band_press(window_state, x, y);
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        } else if Self::frame_window_point_in_band(
                            window_state,
                            FrameChromeKind::ToolBar,
                            x,
                            y,
                        ) {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            window_state.render.set_popup_menu(None);
                            window_state.render.chrome.dismiss_menus();
                            window_state
                                .render
                                .chrome
                                .interaction
                                .compact_bar_menu_active = None;
                            window_state
                                .render
                                .chrome
                                .interaction
                                .toolbar_press_captured = true;
                            if let Some(idx) =
                                Self::frame_window_toolbar_hit_test(window_state, x, y)
                            {
                                window_state
                                    .render
                                    .chrome
                                    .press_with_popup(&ChromePress::ToolBar(idx));
                                event = Some(InputEvent::ToolBarClick {
                                    index: idx as i32,
                                    emacs_frame_id: window_state.render.emacs_frame_id,
                                });
                            }
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        } else {
                            let idx = window_state
                                .render
                                .overlays
                                .popup_menu
                                .as_ref()
                                .map_or(-1, |menu| menu.hit_test(x, y));
                            if idx >= 0 {
                                event = Some(InputEvent::MenuSelection { index: idx });
                                window_state.render.dismiss_all_chrome_menus();
                            } else {
                                let (depth, local_idx) = window_state
                                    .render
                                    .overlays
                                    .popup_menu
                                    .as_ref()
                                    .map_or((-1, -1), |menu| menu.hit_test_all(x, y));
                                if depth >= 0 && local_idx >= 0 {
                                    let is_submenu = window_state
                                        .render
                                        .overlays
                                        .popup_menu
                                        .as_ref()
                                        .is_some_and(|menu| {
                                            let panel = if depth == 0 {
                                                &menu.root_panel
                                            } else {
                                                &menu.submenu_panels[(depth - 1) as usize]
                                            };
                                            let global_idx = panel.item_indices[local_idx as usize];
                                            menu.all_items[global_idx].submenu
                                        });
                                    if is_submenu {
                                        window_state.render.mark_dirty();
                                    } else {
                                        event = Some(InputEvent::MenuSelection { index: -1 });
                                        window_state.render.dismiss_all_chrome_menus();
                                    }
                                } else {
                                    event = Some(InputEvent::MenuSelection { index: -1 });
                                    window_state.render.dismiss_all_chrome_menus();
                                }
                            }
                            handled_chrome = true;
                        }
                    } else if state == ElementState::Pressed {
                        event = Some(InputEvent::MenuSelection { index: -1 });
                        window_state.render.dismiss_all_chrome_menus();
                        handled_chrome = true;
                    }
                    if !handled_chrome {
                        handled_chrome = true;
                    }
                }

                if state == ElementState::Pressed && button == MouseButton::Left {
                    let x = window_state.render.mouse_pos.0;
                    let y = window_state.render.mouse_pos.1;
                    if !handled_chrome {
                        match Self::capture_presented_pointer_press(
                            window_state,
                            pointer_owner,
                            x,
                            y,
                        ) {
                            Ok(Some(presented)) => {
                                event = Some(presented);
                                handled_chrome = true;
                            }
                            Ok(None) => {}
                            Err(error) => {
                                tracing::error!(?error, "rejecting incoherent pointer press");
                                handled_chrome = true;
                            }
                        }
                    }
                    if !handled_chrome
                        && pointer_owner.permits_root_chrome()
                        && Self::frame_window_point_in_band(
                            window_state,
                            FrameChromeKind::CompactBar,
                            x,
                            y,
                        )
                    {
                        if let Some(hit) =
                            Self::frame_window_compact_bar_menu_hit_test(window_state, x, y)
                        {
                            if window_state
                                .render
                                .chrome
                                .interaction
                                .compact_bar_menu_active
                                == Some(hit.index)
                            {
                                window_state
                                    .render
                                    .chrome
                                    .interaction
                                    .compact_bar_menu_active = None;
                            } else {
                                window_state
                                    .render
                                    .chrome
                                    .interaction
                                    .compact_bar_menu_active = Some(hit.index);
                                event = Some(Self::menu_bar_click_event(
                                    hit,
                                    window_state.render.emacs_frame_id,
                                ));
                            }
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        }
                        if !handled_chrome
                            && let Some(idx) =
                                Self::frame_window_compact_bar_tool_hit_test(window_state, x, y)
                        {
                            window_state
                                .render
                                .chrome
                                .interaction
                                .compact_bar_tool_pressed = Some(idx);
                            event = Some(InputEvent::ToolBarClick {
                                index: idx as i32,
                                emacs_frame_id: window_state.render.emacs_frame_id,
                            });
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        }
                    } else if !handled_chrome
                        && pointer_owner.permits_root_chrome()
                        && Self::frame_window_point_in_band(
                            window_state,
                            FrameChromeKind::MenuBar,
                            x,
                            y,
                        )
                    {
                        if let Some(hit) = Self::frame_window_menu_bar_hit_test(window_state, x, y)
                        {
                            window_state
                                .render
                                .chrome
                                .press_with_popup(&ChromePress::MenuBar(hit.index));
                            event = Some(Self::menu_bar_click_event(
                                hit,
                                window_state.render.emacs_frame_id,
                            ));
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        }
                    } else if !handled_chrome
                        && pointer_owner.permits_root_chrome()
                        && Self::frame_window_point_in_band(
                            window_state,
                            FrameChromeKind::TabBar,
                            x,
                            y,
                        )
                    {
                        event = Self::capture_tab_band_press(window_state, x, y);
                        if event.is_some() {
                            window_state.render.mark_dirty();
                        }
                        handled_chrome = true;
                    } else if !handled_chrome
                        && pointer_owner.permits_root_chrome()
                        && Self::frame_window_point_in_band(
                            window_state,
                            FrameChromeKind::ToolBar,
                            x,
                            y,
                        )
                    {
                        window_state
                            .render
                            .chrome
                            .interaction
                            .toolbar_press_captured = true;
                        if let Some(idx) = Self::frame_window_toolbar_hit_test(window_state, x, y) {
                            window_state
                                .render
                                .chrome
                                .press_with_popup(&ChromePress::ToolBar(idx));
                            event = Some(InputEvent::ToolBarClick {
                                index: idx as i32,
                                emacs_frame_id: window_state.render.emacs_frame_id,
                            });
                            window_state.render.mark_dirty();
                        }
                        handled_chrome = true;
                    }
                } else if state == ElementState::Released
                    && button == MouseButton::Left
                    && (window_state.render.presented_capture().is_some()
                        || window_state
                            .render
                            .chrome
                            .interaction
                            .toolbar_press_captured
                        || window_state
                            .render
                            .chrome
                            .interaction
                            .compact_bar_tool_pressed
                            .is_some()
                        || window_state
                            .render
                            .chrome
                            .interaction
                            .toolbar_pressed
                            .is_some())
                {
                    if window_state.render.presented_capture().is_some() {
                        captured_events =
                            Self::take_presented_release_events(&mut window_state.render, x, y);
                    }
                    window_state.render.clear_all_chrome_pressed();
                    handled_chrome = true;
                }

                if !handled_chrome
                    && pointer_owner.permits_root_chrome()
                    && window_state
                        .render
                        .compositor
                        .current_frame
                        .as_ref()
                        .is_some_and(|frame| frame_chrome_owns_pointer(frame, x, y))
                {
                    handled_chrome = true;
                }

                if handled_chrome {
                    // Chrome consumed the click/release; do not also deliver it
                    // as a buffer mouse event.
                } else {
                    let btn = match button {
                        MouseButton::Left => 1,
                        MouseButton::Middle => 2,
                        MouseButton::Right => 3,
                        MouseButton::Back => 4,
                        MouseButton::Forward => 5,
                        MouseButton::Other(n) => n as u32,
                    };
                    let (ev_x, ev_y, target_fid) =
                        pointer_owner.raw_target().unwrap_or_else(|| {
                            Self::pointer_target_for_frame_window(
                                window_state,
                                window_state.render.mouse_pos.0,
                                window_state.render.mouse_pos.1,
                            )
                        });
                    let (wk_id, wk_rx, wk_ry) = if state == ElementState::Pressed {
                        Self::webkit_target_for_frame_window(window_state, target_fid, ev_x, ev_y)
                    } else {
                        (0, 0, 0)
                    };
                    // iMouse click state (doc/display-engine/SHADER_SURFACES.md):
                    // a press over a shader-surface glyph routes the press
                    // position into that surface's iMouse.zw — the Surface
                    // mirror of the Xwidget search above. Render-thread
                    // internal, like hover; the Lisp event below is unchanged.
                    if state == ElementState::Pressed
                        && let Some((surface_id, u, v)) =
                            Self::glyphs_for_frame_window_pointer_target(window_state, target_fid)
                                .and_then(|glyphs| surface_glyph_hit_test(glyphs, ev_x, ev_y))
                        && let Some(renderer) = self.renderer.as_mut()
                    {
                        renderer.surface_mouse_press(surface_id, u, v);
                    }
                    let position = PointerPosition {
                        x: ev_x,
                        y: ev_y,
                        target_frame_id: target_fid,
                    };
                    let action = PointerAction::Button {
                        button: btn,
                        pressed: state == ElementState::Pressed,
                        modifiers: self.modifiers,
                        webkit: Self::webkit_pointer_target(wk_id, wk_rx, wk_ry),
                    };
                    match Self::positioned_pointer_input_event(
                        &window_state.render,
                        pointer_owner,
                        position,
                        action,
                    ) {
                        Ok(input) => {
                            event = Some(input);
                            if state == ElementState::Pressed {
                                window_state.render.clear_presented_capture();
                                window_state
                                    .render
                                    .chrome
                                    .interaction
                                    .toolbar_press_captured = false;
                            }
                            delivered_mouse_button = true;
                        }
                        Err(error) => {
                            tracing::error!(
                                ?error,
                                target_fid,
                                ev_x,
                                ev_y,
                                "dropping incoherent mouse-button input"
                            );
                        }
                    }
                }
            }
            for event in captured_events {
                self.comms.send_input(event);
            }
            if let Some(event) = event {
                self.comms.send_input(event);
            }
            if state == ElementState::Pressed
                && self.effects.click_halo.enabled
                && delivered_mouse_button
                && let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id)
            {
                let (x, y) = window_state.render.mouse_pos;
                window_state.render.trigger_click_halo(
                    x,
                    y,
                    std::time::Instant::now(),
                    self.effects.click_halo.duration_ms,
                );
            }
        }
    }

    pub(super) fn handle_cursor_moved(
        &mut self,
        window_id: WindowId,
        position: PhysicalPosition<f64>,
    ) {
        self.record_idle_dim_activity(window_id);
        let modifiers = self.modifiers;
        if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
            let mut event = None;
            let scale = window_state.scale_factor();
            let (native_w, native_h) = window_state.native_size();
            let lx = (position.x / scale) as f32;
            let ly = (position.y / scale) as f32;
            window_state.render.set_mouse_pos((lx, ly));
            let mut dirty = false;
            let pointer_owner = Self::pointer_owner(window_state, lx, ly);
            if !pointer_owner.owns_root_hover() {
                dirty |= Self::suppress_root_chrome_hover(&mut window_state.render);
            }

            let edge = Self::detect_resize_edge_for_chrome(
                window_state.chrome(),
                native_w as f32 / scale as f32,
                native_h as f32 / scale as f32,
                lx,
                ly,
            );
            if edge != window_state.chrome().resize_edge {
                window_state.chrome_mut().resize_edge = edge;
                let icon = match edge {
                    Some(dir) => winit::window::CursorIcon::from(dir),
                    None => winit::window::CursorIcon::Default,
                };
                if !window_state.chrome().decorations_enabled
                    && let Some(window) = window_state.window()
                {
                    window.set_cursor(icon);
                }
            }

            if !window_state.chrome().decorations_enabled {
                let new_hover = if pointer_owner.permits_native_chrome() {
                    Self::frame_window_titlebar_hit_test(window_state, lx, ly)
                } else {
                    0
                };
                if new_hover != window_state.chrome().titlebar_hover {
                    window_state.chrome_mut().titlebar_hover = new_hover;
                    dirty = true;
                    if window_state.chrome().resize_edge.is_none() {
                        let icon = match new_hover {
                            2..=4 => winit::window::CursorIcon::Pointer,
                            _ => winit::window::CursorIcon::Default,
                        };
                        if let Some(window) = window_state.window() {
                            window.set_cursor(icon);
                        }
                    }
                }
            }

            if let Some(menu_bounds) =
                Self::frame_window_band_bounds(window_state, FrameChromeKind::MenuBar)
            {
                let old_hover = window_state.render.chrome.interaction.menu_bar_hovered;
                if pointer_owner.owns_root_hover()
                    && ly >= menu_bounds.y()
                    && ly < menu_bounds.y() + menu_bounds.height()
                {
                    let new_hover = Self::frame_window_menu_bar_hit_test(window_state, lx, ly);
                    window_state.render.chrome.interaction.menu_bar_hovered =
                        new_hover.as_ref().map(|hit| hit.index);
                    if let (Some(active), Some(hit)) = (
                        window_state.render.chrome.interaction.menu_bar_active,
                        new_hover,
                    ) && hit.index != active
                    {
                        window_state.render.chrome.interaction.menu_bar_active = Some(hit.index);
                        event = Some(Self::menu_bar_click_event(
                            hit,
                            window_state.render.emacs_frame_id,
                        ));
                    }
                } else {
                    window_state.render.chrome.interaction.menu_bar_hovered = None;
                }
                dirty |= window_state.render.chrome.interaction.menu_bar_hovered != old_hover;
            }

            if let Some(compact_bounds) =
                Self::frame_window_band_bounds(window_state, FrameChromeKind::CompactBar)
            {
                let old_menu_hover = window_state
                    .render
                    .chrome
                    .interaction
                    .compact_bar_menu_hovered;
                let old_tool_hover = window_state
                    .render
                    .chrome
                    .interaction
                    .compact_bar_tool_hovered;
                if pointer_owner.owns_root_hover()
                    && ly >= compact_bounds.y()
                    && ly < compact_bounds.y() + compact_bounds.height()
                {
                    let new_menu_hover =
                        Self::frame_window_compact_bar_menu_hit_test(window_state, lx, ly);
                    window_state
                        .render
                        .chrome
                        .interaction
                        .compact_bar_menu_hovered = new_menu_hover.as_ref().map(|hit| hit.index);
                    window_state
                        .render
                        .chrome
                        .interaction
                        .compact_bar_tool_hovered = if new_menu_hover.is_none() {
                        Self::frame_window_compact_bar_tool_hit_test(window_state, lx, ly)
                    } else {
                        None
                    };
                    if let (Some(active), Some(hit)) = (
                        window_state
                            .render
                            .chrome
                            .interaction
                            .compact_bar_menu_active,
                        new_menu_hover,
                    ) && hit.index != active
                    {
                        window_state
                            .render
                            .chrome
                            .interaction
                            .compact_bar_menu_active = Some(hit.index);
                        event = Some(Self::menu_bar_click_event(
                            hit,
                            window_state.render.emacs_frame_id,
                        ));
                    }
                } else {
                    window_state
                        .render
                        .chrome
                        .interaction
                        .compact_bar_menu_hovered = None;
                    window_state
                        .render
                        .chrome
                        .interaction
                        .compact_bar_tool_hovered = None;
                }
                dirty |= window_state
                    .render
                    .chrome
                    .interaction
                    .compact_bar_menu_hovered
                    != old_menu_hover
                    || window_state
                        .render
                        .chrome
                        .interaction
                        .compact_bar_tool_hovered
                        != old_tool_hover;
            }

            let old_toolbar_hover = window_state.render.chrome.interaction.toolbar_hovered;
            window_state.render.chrome.interaction.toolbar_hovered =
                if pointer_owner.owns_root_hover() {
                    Self::frame_window_toolbar_hit_test(window_state, lx, ly)
                } else {
                    None
                };
            dirty |= window_state.render.chrome.interaction.toolbar_hovered != old_toolbar_hover;

            dirty |= window_state.render.update_popup_hover(lx, ly);

            window_state.set_mouse_hidden_for_typing(false);

            let (ev_x, ev_y, target_fid) = pointer_owner
                .raw_target()
                .unwrap_or_else(|| Self::pointer_target_for_frame_window(window_state, lx, ly));
            let appearance_target = pointer_owner
                .target()
                .map(|(x, y, frame_id)| (frame_id, x, y));
            dirty |= window_state
                .render
                .update_presented_pointer_motion(appearance_target);
            if dirty {
                window_state.render.mark_dirty();
            }
            if event.is_none() {
                let position = PointerPosition {
                    x: ev_x,
                    y: ev_y,
                    target_frame_id: target_fid,
                };
                match Self::positioned_pointer_input_event(
                    &window_state.render,
                    pointer_owner,
                    position,
                    PointerAction::Move { modifiers },
                ) {
                    Ok(input) => {
                        event = Some(input);
                    }
                    Err(error) => {
                        tracing::error!(
                            ?error,
                            target_fid,
                            ev_x,
                            ev_y,
                            "dropping incoherent mouse-move input"
                        );
                    }
                }
            }
            if let Some(event) = event {
                self.comms.send_input(event);
            }
        }
    }

    pub(super) fn handle_cursor_left(&mut self, window_id: WindowId) {
        if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
            window_state.render.clear_pointer_hover();
        }
    }

    pub(super) fn handle_mouse_wheel(&mut self, window_id: WindowId, delta: MouseScrollDelta) {
        if let Some(window_state) = self.frame_windows.get_by_winit(window_id) {
            let scale = window_state.scale_factor();
            let delta = match delta {
                MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines { x, y },
                MouseScrollDelta::PixelDelta(pos) => ScrollDelta::Pixels {
                    x: (pos.x / scale) as f32,
                    y: (pos.y / scale) as f32,
                },
            };
            let pointer_owner = Self::pointer_owner(
                window_state,
                window_state.render.mouse_pos.0,
                window_state.render.mouse_pos.1,
            );
            let (ev_x, ev_y, target_fid) = pointer_owner.raw_target().unwrap_or_else(|| {
                Self::pointer_target_for_frame_window(
                    window_state,
                    window_state.render.mouse_pos.0,
                    window_state.render.mouse_pos.1,
                )
            });
            let (wk_id, wk_rx, wk_ry) =
                Self::webkit_target_for_frame_window(window_state, target_fid, ev_x, ev_y);
            let position = PointerPosition {
                x: ev_x,
                y: ev_y,
                target_frame_id: target_fid,
            };
            let action = PointerAction::Scroll {
                delta,
                modifiers: self.modifiers,
                webkit: Self::webkit_pointer_target(wk_id, wk_rx, wk_ry),
            };
            match Self::positioned_pointer_input_event(
                &window_state.render,
                pointer_owner,
                position,
                action,
            ) {
                Ok(input) => {
                    self.comms.send_input(input);
                }
                Err(error) => {
                    tracing::error!(
                        ?error,
                        target_fid,
                        ev_x,
                        ev_y,
                        "dropping incoherent mouse-wheel input"
                    );
                }
            }
            self.record_idle_dim_activity(window_id);
        }
    }
}
