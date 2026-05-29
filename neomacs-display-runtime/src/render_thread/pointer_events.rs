//! Pointer, wheel, and hover handling for winit window events.

use super::RenderApp;
use super::frame_windows::GuiFrameWindowState;
use super::input::{
    compact_bar_menu_width, menu_bar_hit_test_items, tab_bar_hit_test_items, toolbar_hit_test_items,
};
use crate::backend::wgpu::NEOMACS_SUPER_MASK;
use crate::core::frame_glyphs::FrameGlyph;
use crate::thread_comm::InputEvent;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::window::WindowId;

/// Search a glyph buffer for a WebKit view at the given local coordinates.
/// Returns `(webkit_id, relative_x, relative_y)` if found.
fn webkit_glyph_hit_test(glyphs: &[FrameGlyph], x: f32, y: f32) -> Option<(u32, i32, i32)> {
    for glyph in glyphs.iter().rev() {
        if let FrameGlyph::WebKit {
            webkit_id,
            x: wx,
            y: wy,
            width,
            height,
            ..
        } = glyph
        {
            if x >= *wx && x < *wx + *width && y >= *wy && y < *wy + *height {
                return Some((*webkit_id, (x - *wx) as i32, (y - *wy) as i32));
            }
        }
    }
    None
}

impl RenderApp {
    #[cfg(feature = "wpe-webkit")]
    fn floating_webkit_hit_test(
        floating_webkits: &[crate::core::scene::FloatingWebKit],
        x: f32,
        y: f32,
    ) -> Option<(u32, i32, i32)> {
        floating_webkits.iter().rev().find_map(|wk| {
            if x >= wk.x && x < wk.x + wk.width && y >= wk.y && y < wk.y + wk.height {
                Some((wk.webkit_id, (x - wk.x) as i32, (y - wk.y) as i32))
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
        if let Some((fid, local_x, local_y)) = window_state.render.child_frames.hit_test(x, y) {
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
                .current_frame
                .as_ref()
                .map(|frame| frame.glyphs.as_slice())
        } else {
            window_state
                .render
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

    fn frame_window_char_width(window_state: &GuiFrameWindowState) -> f32 {
        window_state.render.glyph_atlas.as_ref().map_or(8.0, |atlas| atlas.default_char_width())
    }

    fn frame_window_menu_bar_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<u32> {
        let menu_bar = window_state.render.menu_bar.as_ref()?;
        menu_bar_hit_test_items(
            &menu_bar.items,
            menu_bar.height,
            Self::frame_window_char_width(window_state),
            x,
            y,
        )
    }

    fn frame_window_compact_bar_menu_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<u32> {
        let compact_bar = window_state.render.compact_bar.as_ref()?;
        menu_bar_hit_test_items(
            &compact_bar.menu_items,
            compact_bar.height,
            Self::frame_window_char_width(window_state),
            x,
            y,
        )
    }

    fn frame_window_compact_bar_tool_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
        toolbar_padding: u32,
        toolbar_icon_size: u32,
    ) -> Option<u32> {
        let compact_bar = window_state.render.compact_bar.as_ref()?;
        let x = x - compact_bar_menu_width(
            &compact_bar.menu_items,
            Self::frame_window_char_width(window_state),
        );
        if x < 0.0 {
            return None;
        }
        toolbar_hit_test_items(
            &compact_bar.tool_items,
            compact_bar.height,
            toolbar_padding,
            toolbar_icon_size,
            x,
            y,
        )
    }

    fn frame_window_toolbar_y_origin(window_state: &GuiFrameWindowState) -> f32 {
        if let Some(frame) = window_state.render.current_frame.as_ref()
            && let Some(tab_bar) = frame.tab_bar.as_ref()
            && tab_bar.height > 0.0
        {
            return tab_bar.y + tab_bar.height;
        }
        window_state
            .render
            .menu_bar
            .as_ref()
            .map_or(0.0, |menu_bar| menu_bar.height)
            + window_state
                .render
                .compact_bar
                .as_ref()
                .map_or(0.0, |compact_bar| compact_bar.height)
    }

    fn frame_window_toolbar_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
        toolbar_padding: u32,
        toolbar_icon_size: u32,
    ) -> Option<u32> {
        let tool_bar = window_state.render.tool_bar.as_ref()?;
        let toolbar_y = Self::frame_window_toolbar_y_origin(window_state);
        if y < toolbar_y || y >= toolbar_y + tool_bar.height {
            return None;
        }
        toolbar_hit_test_items(
            &tool_bar.items,
            tool_bar.height,
            toolbar_padding,
            toolbar_icon_size,
            x,
            y - toolbar_y,
        )
    }

    fn frame_window_tab_bar_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> Option<u32> {
        let tab_bar = window_state
            .render
            .current_frame
            .as_ref()
            .and_then(|frame| frame.tab_bar.as_ref())?;
        if y < tab_bar.y || y >= tab_bar.y + tab_bar.height {
            return None;
        }
        tab_bar_hit_test_items(
            &tab_bar.items,
            tab_bar.height,
            Self::frame_window_char_width(window_state),
            x,
            y - tab_bar.y,
        )
    }

    fn glyphs_for_pointer_target(&self, target_fid: u64) -> Option<&[FrameGlyph]> {
        if let Some(primary_state) = self.primary_window_state() {
            Self::glyphs_for_frame_window_pointer_target(primary_state, target_fid)
        } else if self.frame_windows.is_primary_frame_id(target_fid) {
            self.primary_current_frame()
                .map(|frame| frame.glyphs.as_slice())
        } else {
            self.primary_child_frames()
                .frames
                .get(&target_fid)
                .map(|entry| entry.frame.glyphs.as_slice())
        }
    }

    pub(super) fn pointer_target_at(&self, x: f32, y: f32) -> (f32, f32, u64) {
        if let Some(primary_state) = self.primary_window_state() {
            return Self::pointer_target_for_frame_window(primary_state, x, y);
        }
        let primary_frame_id = self.frame_windows.primary_event_frame_id();
        #[cfg(feature = "wpe-webkit")]
        if let Some(primary_frame) = self.primary_render_state() {
            if Self::floating_webkit_hit_test(&primary_frame.floating_webkits, x, y).is_some() {
                return (x, y, primary_frame_id);
            }
        }
        if let Some((fid, local_x, local_y)) = self.primary_child_frames().hit_test(x, y) {
            (local_x, local_y, fid)
        } else {
            (x, y, primary_frame_id)
        }
    }

    fn webkit_target_at(&self, target_fid: u64, ev_x: f32, ev_y: f32) -> (u32, i32, i32) {
        if let Some(primary_state) = self.primary_window_state() {
            return Self::webkit_target_for_frame_window(primary_state, target_fid, ev_x, ev_y);
        }
        let mut wk_id = 0u32;
        let mut wk_rx = 0i32;
        let mut wk_ry = 0i32;

        #[cfg(feature = "wpe-webkit")]
        if self.frame_windows.is_primary_frame_id(target_fid) {
            if let Some(primary_frame) = self.primary_render_state() {
                if let Some((id, rx, ry)) =
                    Self::floating_webkit_hit_test(&primary_frame.floating_webkits, ev_x, ev_y)
                {
                    wk_id = id;
                    wk_rx = rx;
                    wk_ry = ry;
                }
            }
        }

        if wk_id == 0 {
            if let Some(glyphs) = self.glyphs_for_pointer_target(target_fid) {
                if let Some((id, rx, ry)) = webkit_glyph_hit_test(glyphs, ev_x, ev_y) {
                    wk_id = id;
                    wk_rx = rx;
                    wk_ry = ry;
                }
            }
        }

        (wk_id, wk_rx, wk_ry)
    }

    pub(super) fn handle_mouse_input(
        &mut self,
        window_id: WindowId,
        state: ElementState,
        button: MouseButton,
    ) {
        self.record_idle_dim_activity(window_id);
        let primary_event_frame_id = self.frame_windows.primary_event_frame_id();
        if self.frame_windows.get_by_winit(window_id).is_some() {
            let mut event = None;
            let mut handled_chrome = false;
            let mut delivered_mouse_button = false;
            if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
                let x = window_state.render.mouse_pos.0;
                let y = window_state.render.mouse_pos.1;
                let popup_was_open = window_state.render.popup_menu.is_some();
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
                            && let Some(compact_bar) = window_state.render.compact_bar.as_ref()
                            && compact_bar.height > 0.0
                            && y < compact_bar.height
                        {
                            if let Some(idx) =
                                Self::frame_window_compact_bar_menu_hit_test(window_state, x, y)
                            {
                                self.comms
                                    .send_input(InputEvent::MenuSelection { index: -1 });
                                window_state.render.set_popup_menu(None);
                                window_state
                                    .render
                                    .chrome_interaction
                                    .compact_bar_menu_active = Some(idx);
                                event = Some(InputEvent::MenuBarClick {
                                    index: idx as i32,
                                    emacs_frame_id: window_state.render.emacs_frame_id,
                                });
                            } else {
                                event = Some(InputEvent::MenuSelection { index: -1 });
                                window_state.render.set_popup_menu(None);
                                window_state
                                    .render
                                    .chrome_interaction
                                    .compact_bar_menu_active = None;
                            }
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        } else if let Some(menu_bar) = window_state.render.menu_bar.as_ref()
                            && menu_bar.height > 0.0
                            && y < menu_bar.height
                        {
                            if let Some(idx) =
                                Self::frame_window_menu_bar_hit_test(window_state, x, y)
                            {
                                self.comms
                                    .send_input(InputEvent::MenuSelection { index: -1 });
                                window_state.render.set_popup_menu(None);
                                window_state.render.chrome_interaction.menu_bar_active = Some(idx);
                                event = Some(InputEvent::MenuBarClick {
                                    index: idx as i32,
                                    emacs_frame_id: window_state.render.emacs_frame_id,
                                });
                            } else {
                                event = Some(InputEvent::MenuSelection { index: -1 });
                                window_state.render.set_popup_menu(None);
                                window_state.render.chrome_interaction.menu_bar_active = None;
                            }
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        } else if let Some(tab_bar) = window_state
                            .render
                            .current_frame
                            .as_ref()
                            .and_then(|frame| frame.tab_bar.as_ref())
                            && y >= tab_bar.y
                            && y < tab_bar.y + tab_bar.height
                        {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            window_state.render.set_popup_menu(None);
                            window_state.render.chrome_interaction.menu_bar_active = None;
                            window_state
                                .render
                                .chrome_interaction
                                .compact_bar_menu_active = None;
                            window_state
                                .render
                                .chrome_interaction
                                .tab_bar_press_captured = true;
                            if let Some(idx) =
                                Self::frame_window_tab_bar_hit_test(window_state, x, y)
                            {
                                window_state.render.chrome_interaction.tab_bar_pressed = Some(idx);
                                event = Some(InputEvent::TabBarClick {
                                    index: idx as i32,
                                    emacs_frame_id: window_state.render.emacs_frame_id,
                                });
                            }
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        } else if let Some(tool_bar) = window_state.render.tool_bar.as_ref()
                            && y >= Self::frame_window_toolbar_y_origin(window_state)
                            && y < Self::frame_window_toolbar_y_origin(window_state)
                                + tool_bar.height
                        {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            window_state.render.set_popup_menu(None);
                            window_state.render.chrome_interaction.menu_bar_active = None;
                            window_state
                                .render
                                .chrome_interaction
                                .compact_bar_menu_active = None;
                            window_state
                                .render
                                .chrome_interaction
                                .toolbar_press_captured = true;
                            if let Some(idx) = Self::frame_window_toolbar_hit_test(
                                window_state,
                                x,
                                y,
                                self.toolbar_padding,
                                self.toolbar_icon_size,
                            ) {
                                window_state.render.chrome_interaction.toolbar_pressed = Some(idx);
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
                                .popup_menu
                                .as_ref()
                                .map_or(-1, |menu| menu.hit_test(x, y));
                            if idx >= 0 {
                                event = Some(InputEvent::MenuSelection { index: idx });
                                window_state.render.dismiss_all_chrome_menus();
                            } else {
                                let (depth, local_idx) = window_state
                                    .render
                                    .popup_menu
                                    .as_ref()
                                    .map_or((-1, -1), |menu| menu.hit_test_all(x, y));
                                if depth >= 0 && local_idx >= 0 {
                                    let is_submenu = window_state
                                        .render
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
                    if !handled_chrome
                        && let Some(compact_bar) = window_state.render.compact_bar.as_ref()
                        && compact_bar.height > 0.0
                        && y < compact_bar.height
                    {
                        if let Some(idx) =
                            Self::frame_window_compact_bar_menu_hit_test(window_state, x, y)
                        {
                            if window_state
                                .render
                                .chrome_interaction
                                .compact_bar_menu_active
                                == Some(idx)
                            {
                                window_state
                                    .render
                                    .chrome_interaction
                                    .compact_bar_menu_active = None;
                            } else {
                                window_state
                                    .render
                                    .chrome_interaction
                                    .compact_bar_menu_active = Some(idx);
                                event = Some(InputEvent::MenuBarClick {
                                    index: idx as i32,
                                    emacs_frame_id: window_state.render.emacs_frame_id,
                                });
                            }
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        }
                        if !handled_chrome
                            && let Some(idx) = Self::frame_window_compact_bar_tool_hit_test(
                                window_state,
                                x,
                                y,
                                self.toolbar_padding,
                                self.toolbar_icon_size,
                            )
                        {
                            window_state
                                .render
                                .chrome_interaction
                                .compact_bar_tool_pressed = Some(idx);
                            event = Some(InputEvent::ToolBarClick {
                                index: idx as i32,
                                emacs_frame_id: window_state.render.emacs_frame_id,
                            });
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        }
                    } else if !handled_chrome
                        && let Some(menu_bar) = window_state.render.menu_bar.as_ref()
                        && menu_bar.height > 0.0
                        && y < menu_bar.height
                    {
                        if let Some(idx) = Self::frame_window_menu_bar_hit_test(window_state, x, y)
                        {
                            window_state.render.chrome_interaction.menu_bar_active = Some(idx);
                            event = Some(InputEvent::MenuBarClick {
                                index: idx as i32,
                                emacs_frame_id: window_state.render.emacs_frame_id,
                            });
                            window_state.render.mark_dirty();
                            handled_chrome = true;
                        }
                    } else if !handled_chrome
                        && let Some(tab_bar) = window_state
                            .render
                            .current_frame
                            .as_ref()
                            .and_then(|frame| frame.tab_bar.as_ref())
                        && y >= tab_bar.y
                        && y < tab_bar.y + tab_bar.height
                    {
                        window_state
                            .render
                            .chrome_interaction
                            .tab_bar_press_captured = true;
                        if let Some(idx) = Self::frame_window_tab_bar_hit_test(window_state, x, y) {
                            window_state.render.chrome_interaction.tab_bar_pressed = Some(idx);
                            event = Some(InputEvent::TabBarClick {
                                index: idx as i32,
                                emacs_frame_id: window_state.render.emacs_frame_id,
                            });
                            window_state.render.mark_dirty();
                        }
                        handled_chrome = true;
                    } else if !handled_chrome
                        && let Some(tool_bar) = window_state.render.tool_bar.as_ref()
                        && y >= Self::frame_window_toolbar_y_origin(window_state)
                        && y < Self::frame_window_toolbar_y_origin(window_state) + tool_bar.height
                    {
                        window_state
                            .render
                            .chrome_interaction
                            .toolbar_press_captured = true;
                        if let Some(idx) = Self::frame_window_toolbar_hit_test(
                            window_state,
                            x,
                            y,
                            self.toolbar_padding,
                            self.toolbar_icon_size,
                        ) {
                            window_state.render.chrome_interaction.toolbar_pressed = Some(idx);
                            event = Some(InputEvent::ToolBarClick {
                                index: idx as i32,
                                emacs_frame_id: window_state.render.emacs_frame_id,
                            });
                            window_state.render.mark_dirty();
                        }
                        handled_chrome = true;
                    }
                } else if state == ElementState::Released && button == MouseButton::Left {
                    if window_state
                        .render
                        .chrome_interaction
                        .tab_bar_press_captured
                        || window_state
                            .render
                            .chrome_interaction
                            .toolbar_press_captured
                        || window_state
                            .render
                            .chrome_interaction
                            .tab_bar_pressed
                            .is_some()
                        || window_state
                            .render
                            .chrome_interaction
                            .compact_bar_tool_pressed
                            .is_some()
                        || window_state
                            .render
                            .chrome_interaction
                            .toolbar_pressed
                            .is_some()
                    {
                        window_state.render.clear_all_chrome_pressed();
                        handled_chrome = true;
                    }
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
                    let (ev_x, ev_y, target_fid) = Self::pointer_target_for_frame_window(
                        window_state,
                        window_state.render.mouse_pos.0,
                        window_state.render.mouse_pos.1,
                    );
                    let (wk_id, wk_rx, wk_ry) = if state == ElementState::Pressed {
                        Self::webkit_target_for_frame_window(window_state, target_fid, ev_x, ev_y)
                    } else {
                        (0, 0, 0)
                    };
                    event = Some(InputEvent::MouseButton {
                        button: btn,
                        x: ev_x,
                        y: ev_y,
                        pressed: state == ElementState::Pressed,
                        modifiers: self.modifiers,
                        target_frame_id: target_fid,
                        webkit_id: wk_id,
                        webkit_rel_x: wk_rx,
                        webkit_rel_y: wk_ry,
                    });
                    if state == ElementState::Pressed {
                        window_state
                            .render
                            .chrome_interaction
                            .tab_bar_press_captured = false;
                        window_state
                            .render
                            .chrome_interaction
                            .toolbar_press_captured = false;
                    }
                    delivered_mouse_button = true;
                }
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
            return;
        }

        if !self.frame_windows.is_primary_winit(window_id) {
            return;
        }

        let menu_bar_height = self.menu_bar_height();
        let compact_bar_height = self.compact_bar_height();
        let tool_bar_height = self.tool_bar_height();
        let tab_bar_y = self.tab_bar_y();
        let tab_bar_height = self.tab_bar_height();

        if state == ElementState::Pressed {
            tracing::debug!(
                "MouseInput: {:?} at ({:.1}, {:.1}), menu_bar_h={}, popup={}",
                button,
                self.primary_mouse_pos().0,
                self.primary_mouse_pos().1,
                menu_bar_height,
                self.primary_popup_menu().is_some()
            );
        }

        if self.primary_popup_menu().is_some() {
            if state == ElementState::Pressed && button == MouseButton::Left {
                if compact_bar_height > 0.0 && self.primary_mouse_pos().1 < compact_bar_height {
                    if let Some(idx) = self.compact_bar_menu_hit_test(
                        self.primary_mouse_pos().0,
                        self.primary_mouse_pos().1,
                    ) {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: -1 });
                        self.set_primary_popup_menu(None);
                        self.with_primary_chrome_interaction_mut(|chrome| {
                            chrome.compact_bar_menu_active = Some(idx);
                        });
                        self.comms.send_input(InputEvent::MenuBarClick {
                            index: idx as i32,
                            emacs_frame_id: primary_event_frame_id,
                        });
                    } else {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: -1 });
                        self.set_primary_popup_menu(None);
                        self.with_primary_chrome_interaction_mut(|chrome| {
                            chrome.compact_bar_menu_active = None;
                        });
                    }
                } else if menu_bar_height > 0.0 && self.primary_mouse_pos().1 < menu_bar_height {
                    if let Some(idx) = self
                        .menu_bar_hit_test(self.primary_mouse_pos().0, self.primary_mouse_pos().1)
                    {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: -1 });
                        self.set_primary_popup_menu(None);
                        self.with_primary_chrome_interaction_mut(|chrome| {
                            chrome.menu_bar_active = Some(idx);
                        });
                        self.comms.send_input(InputEvent::MenuBarClick {
                            index: idx as i32,
                            emacs_frame_id: primary_event_frame_id,
                        });
                    } else {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: -1 });
                        self.set_primary_popup_menu(None);
                        self.with_primary_chrome_interaction_mut(|chrome| {
                            chrome.menu_bar_active = None;
                        });
                    }
                } else if tab_bar_height > 0.0
                    && self.primary_mouse_pos().1 >= tab_bar_y
                    && self.primary_mouse_pos().1 < tab_bar_y + tab_bar_height
                {
                    let idx = self
                        .tab_bar_hit_test(self.primary_mouse_pos().0, self.primary_mouse_pos().1);
                    self.comms
                        .send_input(InputEvent::MenuSelection { index: -1 });
                    self.set_primary_popup_menu(None);
                    self.with_primary_chrome_interaction_mut(|chrome| {
                        chrome.menu_bar_active = None;
                        chrome.compact_bar_menu_active = None;
                        chrome.tab_bar_press_captured = true;
                        if let Some(idx) = idx {
                            chrome.tab_bar_pressed = Some(idx);
                        }
                    });
                    if let Some(idx) = idx {
                        self.comms.send_input(InputEvent::TabBarClick {
                            index: idx as i32,
                            emacs_frame_id: primary_event_frame_id,
                        });
                    }
                } else if tool_bar_height > 0.0
                    && self.primary_mouse_pos().1 < self.toolbar_y_origin() + tool_bar_height
                    && self.primary_mouse_pos().1 >= self.toolbar_y_origin()
                {
                    let idx = self.toolbar_hit_test(
                        self.primary_mouse_pos().0,
                        self.primary_mouse_pos().1 - self.toolbar_y_origin(),
                    );
                    self.comms
                        .send_input(InputEvent::MenuSelection { index: -1 });
                    self.set_primary_popup_menu(None);
                    self.with_primary_chrome_interaction_mut(|chrome| {
                        chrome.menu_bar_active = None;
                        chrome.compact_bar_menu_active = None;
                        chrome.toolbar_press_captured = true;
                        if let Some(idx) = idx {
                            chrome.toolbar_pressed = Some(idx);
                        }
                    });
                    if let Some(idx) = idx {
                        self.comms.send_input(InputEvent::ToolBarClick {
                            index: idx as i32,
                            emacs_frame_id: primary_event_frame_id,
                        });
                    }
                } else {
                    let idx = self.primary_popup_menu().map_or(-1, |menu| {
                        menu.hit_test(self.primary_mouse_pos().0, self.primary_mouse_pos().1)
                    });
                    if idx >= 0 {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: idx });
                        self.set_primary_popup_menu(None);
                        self.with_primary_chrome_interaction_mut(|chrome| {
                            chrome.menu_bar_active = None;
                            chrome.compact_bar_menu_active = None;
                        });
                    } else {
                        let (depth, local_idx) =
                            self.primary_popup_menu().map_or((-1, -1), |menu| {
                                menu.hit_test_all(
                                    self.primary_mouse_pos().0,
                                    self.primary_mouse_pos().1,
                                )
                            });
                        if depth >= 0 && local_idx >= 0 {
                            let is_submenu = self.primary_popup_menu().is_some_and(|menu| {
                                let panel = if depth == 0 {
                                    &menu.root_panel
                                } else {
                                    &menu.submenu_panels[(depth - 1) as usize]
                                };
                                let global_idx = panel.item_indices[local_idx as usize];
                                menu.all_items[global_idx].submenu
                            });
                            if is_submenu {
                                self.mark_primary_dirty();
                            } else {
                                self.comms
                                    .send_input(InputEvent::MenuSelection { index: -1 });
                                self.set_primary_popup_menu(None);
                                self.with_primary_chrome_interaction_mut(|chrome| {
                                    chrome.menu_bar_active = None;
                                    chrome.compact_bar_menu_active = None;
                                });
                            }
                        } else {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            self.set_primary_popup_menu(None);
                            self.with_primary_chrome_interaction_mut(|chrome| {
                                chrome.menu_bar_active = None;
                                chrome.compact_bar_menu_active = None;
                            });
                        }
                    }
                }
            } else if state == ElementState::Pressed {
                self.comms
                    .send_input(InputEvent::MenuSelection { index: -1 });
                self.set_primary_popup_menu(None);
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.menu_bar_active = None;
                    chrome.compact_bar_menu_active = None;
                });
            }
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && self.primary_chrome().resize_edge.is_some()
        {
            if let Some(primary_state) = self.primary_window_state() {
                primary_state.drag_resize_for_current_edge();
            }
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && self.titlebar_hit_test(self.primary_mouse_pos().0, self.primary_mouse_pos().1) > 0
        {
            match self.titlebar_hit_test(self.primary_mouse_pos().0, self.primary_mouse_pos().1) {
                2 => {
                    self.comms.send_input(InputEvent::WindowClose {
                        emacs_frame_id: primary_event_frame_id,
                    });
                }
                action => {
                    if let Some(primary_state) = self.primary_window_state_mut() {
                        primary_state.handle_titlebar_action(action);
                    }
                }
            }
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && !self.primary_chrome().decorations_enabled
            && (self.modifiers & NEOMACS_SUPER_MASK) != 0
        {
            if let Some(primary_state) = self.primary_window_state() {
                primary_state.drag_window();
            }
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && compact_bar_height > 0.0
            && self.primary_mouse_pos().1 < compact_bar_height
        {
            if let Some(idx) = self
                .compact_bar_menu_hit_test(self.primary_mouse_pos().0, self.primary_mouse_pos().1)
            {
                if self.primary_chrome_interaction().compact_bar_menu_active == Some(idx) {
                    self.with_primary_chrome_interaction_mut(|chrome| {
                        chrome.compact_bar_menu_active = None;
                    });
                } else {
                    self.with_primary_chrome_interaction_mut(|chrome| {
                        chrome.compact_bar_menu_active = Some(idx);
                    });
                    self.comms.send_input(InputEvent::MenuBarClick {
                        index: idx as i32,
                        emacs_frame_id: primary_event_frame_id,
                    });
                }
                return;
            }
            if let Some(idx) = self
                .compact_bar_tool_hit_test(self.primary_mouse_pos().0, self.primary_mouse_pos().1)
            {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.compact_bar_tool_pressed = Some(idx);
                });
                self.comms.send_input(InputEvent::ToolBarClick {
                    index: idx as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
                return;
            }
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && menu_bar_height > 0.0
            && self.primary_mouse_pos().1 < menu_bar_height
        {
            tracing::debug!(
                "Menu bar click at ({:.1}, {:.1}), menu_bar_height={}",
                self.primary_mouse_pos().0,
                self.primary_mouse_pos().1,
                menu_bar_height
            );
            if let Some(idx) =
                self.menu_bar_hit_test(self.primary_mouse_pos().0, self.primary_mouse_pos().1)
            {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.menu_bar_active = Some(idx);
                });
                self.comms.send_input(InputEvent::MenuBarClick {
                    index: idx as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
                return;
            }
        }

        // Tab bar click (between menu bar and toolbar)
        if state == ElementState::Pressed
            && button == MouseButton::Left
            && tab_bar_height > 0.0
            && self.primary_mouse_pos().1 >= tab_bar_y
            && self.primary_mouse_pos().1 < tab_bar_y + tab_bar_height
        {
            if let Some(idx) =
                self.tab_bar_hit_test(self.primary_mouse_pos().0, self.primary_mouse_pos().1)
            {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.tab_bar_pressed = Some(idx);
                    chrome.tab_bar_press_captured = true;
                });
                self.comms.send_input(InputEvent::TabBarClick {
                    index: idx as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
            } else {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.tab_bar_press_captured = true;
                });
            }
            return;
        }

        if state == ElementState::Released
            && button == MouseButton::Left
            && (self.primary_chrome_interaction().tab_bar_pressed.is_some()
                || self.primary_chrome_interaction().tab_bar_press_captured)
        {
            self.with_primary_chrome_interaction_mut(|chrome| {
                chrome.tab_bar_pressed = None;
                chrome.tab_bar_press_captured = false;
            });
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && tool_bar_height > 0.0
            && self.primary_mouse_pos().1 < self.toolbar_y_origin() + tool_bar_height
            && self.primary_mouse_pos().1 >= self.toolbar_y_origin()
        {
            if let Some(idx) = self.toolbar_hit_test(
                self.primary_mouse_pos().0,
                self.primary_mouse_pos().1 - self.toolbar_y_origin(),
            ) {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.toolbar_pressed = Some(idx);
                    chrome.toolbar_press_captured = true;
                });
                self.comms.send_input(InputEvent::ToolBarClick {
                    index: idx as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
            } else {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.toolbar_press_captured = true;
                });
            }
            return;
        }

        if state == ElementState::Released
            && button == MouseButton::Left
            && self
                .primary_chrome_interaction()
                .compact_bar_tool_pressed
                .is_some()
        {
            self.with_primary_chrome_interaction_mut(|chrome| {
                chrome.compact_bar_tool_pressed = None;
            });
            return;
        }

        if state == ElementState::Released
            && button == MouseButton::Left
            && (self.primary_chrome_interaction().toolbar_pressed.is_some()
                || self.primary_chrome_interaction().toolbar_press_captured)
        {
            self.with_primary_chrome_interaction_mut(|chrome| {
                chrome.toolbar_pressed = None;
                chrome.toolbar_press_captured = false;
            });
            return;
        }

        if state == ElementState::Pressed && button == MouseButton::Left {
            tracing::trace!(
                "Left click at ({:.1}, {:.1}) NOT in menu bar (h={}) or toolbar (h={})",
                self.primary_mouse_pos().0,
                self.primary_mouse_pos().1,
                menu_bar_height,
                tool_bar_height
            );
        }

        let btn = match button {
            MouseButton::Left => 1,
            MouseButton::Middle => 2,
            MouseButton::Right => 3,
            MouseButton::Back => 4,
            MouseButton::Forward => 5,
            MouseButton::Other(n) => n as u32,
        };

        let (ev_x, ev_y, target_fid) =
            self.pointer_target_at(self.primary_mouse_pos().0, self.primary_mouse_pos().1);
        if target_fid != 0 {
            if let Some(entry) = self.primary_child_frames().frames.get(&target_fid) {
                tracing::trace!(
                    "Child frame hit: fid={} abs=({:.1},{:.1}) size=({:.1}x{:.1}) mouse=({:.1},{:.1}) local=({:.1},{:.1})",
                    target_fid,
                    entry.abs_x,
                    entry.abs_y,
                    entry.frame.width,
                    entry.frame.height,
                    self.primary_mouse_pos().0,
                    self.primary_mouse_pos().1,
                    ev_x,
                    ev_y
                );
            }
        }

        let (wk_id, wk_rx, wk_ry) = if state == ElementState::Pressed {
            let (id, rx, ry) = self.webkit_target_at(target_fid, ev_x, ev_y);
            if id != 0 {
                tracing::trace!("WebKit hit: id={} rel=({},{})", id, rx, ry);
            }
            (id, rx, ry)
        } else {
            (0, 0, 0)
        };

        if state == ElementState::Pressed {
            tracing::trace!(
                "MouseButton: btn={} ev=({:.1},{:.1}) target_fid={} wk_id={} wk_rel=({},{})",
                btn,
                ev_x,
                ev_y,
                target_fid,
                wk_id,
                wk_rx,
                wk_ry
            );
        }

        self.comms.send_input(InputEvent::MouseButton {
            button: btn,
            x: ev_x,
            y: ev_y,
            pressed: state == ElementState::Pressed,
            modifiers: self.modifiers,
            target_frame_id: target_fid,
            webkit_id: wk_id,
            webkit_rel_x: wk_rx,
            webkit_rel_y: wk_ry,
        });
        if state == ElementState::Pressed {
            self.with_primary_chrome_interaction_mut(|chrome| {
                chrome.tab_bar_press_captured = false;
                chrome.toolbar_press_captured = false;
            });
        }

        if state == ElementState::Pressed && self.effects.click_halo.enabled {
            let now = std::time::Instant::now();
            let mouse_pos = self.primary_mouse_pos();
            let duration_ms = self.effects.click_halo.duration_ms;
            if let Some(primary_frame) = self.primary_render_state_mut() {
                primary_frame.trigger_click_halo(mouse_pos.0, mouse_pos.1, now, duration_ms);
            }
        }
    }

    pub(super) fn handle_cursor_moved(
        &mut self,
        window_id: WindowId,
        position: PhysicalPosition<f64>,
    ) {
        self.record_idle_dim_activity(window_id);
        let toolbar_padding = self.toolbar_padding;
        let toolbar_icon_size = self.toolbar_icon_size;
        let modifiers = self.modifiers;
        if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
            let mut event = None;
            let scale = window_state.scale_factor();
            let (native_w, native_h) = window_state.native_size();
            let lx = (position.x / scale) as f32;
            let ly = (position.y / scale) as f32;
            window_state.render.set_mouse_pos((lx, ly));
            let mut dirty = false;

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
                if !window_state.chrome().decorations_enabled {
                    if let Some(window) = window_state.window() {
                        window.set_cursor(icon);
                    }
                }
            }

            if !window_state.chrome().decorations_enabled {
                let new_hover = Self::frame_window_titlebar_hit_test(window_state, lx, ly);
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

            if let Some(menu_bar) = window_state.render.menu_bar.as_ref()
                && menu_bar.height > 0.0
            {
                let old_hover = window_state.render.chrome_interaction.menu_bar_hovered;
                if ly < menu_bar.height {
                    let new_hover = Self::frame_window_menu_bar_hit_test(window_state, lx, ly);
                    window_state.render.chrome_interaction.menu_bar_hovered = new_hover;
                    if let (Some(active), Some(hov)) = (
                        window_state.render.chrome_interaction.menu_bar_active,
                        new_hover,
                    ) && hov != active
                    {
                        window_state.render.chrome_interaction.menu_bar_active = Some(hov);
                        event = Some(InputEvent::MenuBarClick {
                            index: hov as i32,
                            emacs_frame_id: window_state.render.emacs_frame_id,
                        });
                    }
                } else {
                    window_state.render.chrome_interaction.menu_bar_hovered = None;
                }
                dirty |= window_state.render.chrome_interaction.menu_bar_hovered != old_hover;
            }

            if let Some(compact_bar) = window_state.render.compact_bar.as_ref()
                && compact_bar.height > 0.0
            {
                let old_menu_hover = window_state
                    .render
                    .chrome_interaction
                    .compact_bar_menu_hovered;
                let old_tool_hover = window_state
                    .render
                    .chrome_interaction
                    .compact_bar_tool_hovered;
                if ly < compact_bar.height {
                    let new_menu_hover =
                        Self::frame_window_compact_bar_menu_hit_test(window_state, lx, ly);
                    window_state
                        .render
                        .chrome_interaction
                        .compact_bar_menu_hovered = new_menu_hover;
                    window_state
                        .render
                        .chrome_interaction
                        .compact_bar_tool_hovered = if new_menu_hover.is_none() {
                        Self::frame_window_compact_bar_tool_hit_test(
                            window_state,
                            lx,
                            ly,
                            toolbar_padding,
                            toolbar_icon_size,
                        )
                    } else {
                        None
                    };
                    if let (Some(active), Some(hov)) = (
                        window_state
                            .render
                            .chrome_interaction
                            .compact_bar_menu_active,
                        new_menu_hover,
                    ) && hov != active
                    {
                        window_state
                            .render
                            .chrome_interaction
                            .compact_bar_menu_active = Some(hov);
                        event = Some(InputEvent::MenuBarClick {
                            index: hov as i32,
                            emacs_frame_id: window_state.render.emacs_frame_id,
                        });
                    }
                } else {
                    window_state
                        .render
                        .chrome_interaction
                        .compact_bar_menu_hovered = None;
                    window_state
                        .render
                        .chrome_interaction
                        .compact_bar_tool_hovered = None;
                }
                dirty |= window_state
                    .render
                    .chrome_interaction
                    .compact_bar_menu_hovered
                    != old_menu_hover
                    || window_state
                        .render
                        .chrome_interaction
                        .compact_bar_tool_hovered
                        != old_tool_hover;
            }

            let old_tab_hover = window_state.render.chrome_interaction.tab_bar_hovered;
            window_state.render.chrome_interaction.tab_bar_hovered =
                Self::frame_window_tab_bar_hit_test(window_state, lx, ly);
            dirty |= window_state.render.chrome_interaction.tab_bar_hovered != old_tab_hover;

            let old_toolbar_hover = window_state.render.chrome_interaction.toolbar_hovered;
            window_state.render.chrome_interaction.toolbar_hovered =
                Self::frame_window_toolbar_hit_test(
                    window_state,
                    lx,
                    ly,
                    toolbar_padding,
                    toolbar_icon_size,
                );
            dirty |= window_state.render.chrome_interaction.toolbar_hovered != old_toolbar_hover;

            dirty |= window_state.render.update_popup_hover(lx, ly);

            if dirty {
                window_state.render.mark_dirty();
            }

            window_state.set_mouse_hidden_for_typing(false);

            let (ev_x, ev_y, target_fid) =
                Self::pointer_target_for_frame_window(window_state, lx, ly);
            if event.is_none() {
                event = Some(InputEvent::MouseMove {
                    x: ev_x,
                    y: ev_y,
                    modifiers,
                    target_frame_id: target_fid,
                });
            }
            if let Some(event) = event {
                self.comms.send_input(event);
            }
            return;
        }

        if !self.frame_windows.is_primary_winit(window_id) {
            return;
        }

        let lx = (position.x / self.primary_scale_factor()) as f32;
        let ly = (position.y / self.primary_scale_factor()) as f32;
        self.set_primary_mouse_pos((lx, ly));
        let primary_event_frame_id = self.frame_windows.primary_event_frame_id();

        self.clear_primary_mouse_hidden_for_typing();

        let edge = self.detect_resize_edge(lx, ly);
        if edge != self.primary_chrome().resize_edge {
            self.primary_chrome_mut().resize_edge = edge;
            if let Some(window) = self.primary_window() {
                use winit::window::CursorIcon;
                let icon = match edge {
                    Some(dir) => CursorIcon::from(dir),
                    None => CursorIcon::Default,
                };
                window.set_cursor(icon);
            }
        }

        if !self.primary_chrome().decorations_enabled {
            let new_hover = self.titlebar_hit_test(lx, ly);
            if new_hover != self.primary_chrome().titlebar_hover {
                self.primary_chrome_mut().titlebar_hover = new_hover;
                self.mark_primary_dirty();
                if self.primary_chrome().resize_edge.is_none() {
                    if let Some(window) = self.primary_window() {
                        use winit::window::CursorIcon;
                        let icon = match new_hover {
                            2 | 3 | 4 => CursorIcon::Pointer,
                            _ => CursorIcon::Default,
                        };
                        window.set_cursor(icon);
                    }
                }
            }
        }

        if self.menu_bar_height() > 0.0 {
            let mut send_menu_bar_click = None;
            if ly < self.menu_bar_height() {
                let new_hover = self.menu_bar_hit_test(lx, ly);
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.menu_bar_hovered = new_hover;
                    if let (Some(active), Some(hov)) = (chrome.menu_bar_active, new_hover)
                        && hov != active
                    {
                        chrome.menu_bar_active = Some(hov);
                        send_menu_bar_click = Some(hov);
                    }
                });
            } else {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.menu_bar_hovered = None;
                });
            }
            if let Some(index) = send_menu_bar_click {
                self.comms.send_input(InputEvent::MenuBarClick {
                    index: index as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
            }
        }

        if self.compact_bar_height() > 0.0 {
            let mut send_menu_bar_click = None;
            if ly < self.compact_bar_height() {
                let new_menu_hover = self.compact_bar_menu_hit_test(lx, ly);
                let new_tool_hover = if new_menu_hover.is_none() {
                    self.compact_bar_tool_hit_test(lx, ly)
                } else {
                    None
                };
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.compact_bar_menu_hovered = new_menu_hover;
                    chrome.compact_bar_tool_hovered = new_tool_hover;
                    if let (Some(active), Some(hov)) =
                        (chrome.compact_bar_menu_active, new_menu_hover)
                        && hov != active
                    {
                        chrome.compact_bar_menu_active = Some(hov);
                        send_menu_bar_click = Some(hov);
                    }
                });
            } else {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.compact_bar_menu_hovered = None;
                    chrome.compact_bar_tool_hovered = None;
                });
            }
            if let Some(index) = send_menu_bar_click {
                self.comms.send_input(InputEvent::MenuBarClick {
                    index: index as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
            }
        }

        if self.tab_bar_height() > 0.0 {
            if ly >= self.tab_bar_y() && ly < self.tab_bar_y() + self.tab_bar_height() {
                let new_hover = self.tab_bar_hit_test(lx, ly);
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.tab_bar_hovered = new_hover;
                });
            } else {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.tab_bar_hovered = None;
                });
            }
        }

        if self.tool_bar_height() > 0.0 {
            let toolbar_y = self.toolbar_y_origin();
            if ly < toolbar_y + self.tool_bar_height() && ly >= toolbar_y {
                let new_hover = self.toolbar_hit_test(lx, ly - toolbar_y);
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.toolbar_hovered = new_hover;
                });
            } else {
                self.with_primary_chrome_interaction_mut(|chrome| {
                    chrome.toolbar_hovered = None;
                });
            }
        }

        let handled_primary_popup = self.update_primary_popup_hover(lx, ly);
        if handled_primary_popup {
            return;
        }

        let (ev_x, ev_y, target_fid) = self.pointer_target_at(lx, ly);
        self.comms.send_input(InputEvent::MouseMove {
            x: ev_x,
            y: ev_y,
            modifiers: self.modifiers,
            target_frame_id: target_fid,
        });
    }

    pub(super) fn handle_mouse_wheel(&mut self, window_id: WindowId, delta: MouseScrollDelta) {
        if let Some(window_state) = self.frame_windows.get_by_winit(window_id) {
            let scale = window_state.scale_factor();
            let (dx, dy, pixel_precise) = match delta {
                MouseScrollDelta::LineDelta(x, y) => (x, y, false),
                MouseScrollDelta::PixelDelta(pos) => (
                    (pos.x / scale) as f32,
                    (pos.y / scale) as f32,
                    true,
                ),
            };
            let (ev_x, ev_y, target_fid) = Self::pointer_target_for_frame_window(
                window_state,
                window_state.render.mouse_pos.0,
                window_state.render.mouse_pos.1,
            );
            let (wk_id, wk_rx, wk_ry) =
                Self::webkit_target_for_frame_window(window_state, target_fid, ev_x, ev_y);
            self.comms.send_input(InputEvent::MouseScroll {
                delta_x: dx,
                delta_y: dy,
                x: ev_x,
                y: ev_y,
                modifiers: self.modifiers,
                pixel_precise,
                target_frame_id: target_fid,
                webkit_id: wk_id,
                webkit_rel_x: wk_rx,
                webkit_rel_y: wk_ry,
            });
            self.record_idle_dim_activity(window_id);
            return;
        }

        if !self.frame_windows.is_primary_winit(window_id) {
            return;
        }

        let (dx, dy, pixel_precise) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (x, y, false),
            MouseScrollDelta::PixelDelta(pos) => (
                (pos.x / self.primary_scale_factor()) as f32,
                (pos.y / self.primary_scale_factor()) as f32,
                true,
            ),
        };

        let (ev_x, ev_y, target_fid) =
            self.pointer_target_at(self.primary_mouse_pos().0, self.primary_mouse_pos().1);
        let (wk_id, wk_rx, wk_ry) = self.webkit_target_at(target_fid, ev_x, ev_y);

        self.comms.send_input(InputEvent::MouseScroll {
            delta_x: dx,
            delta_y: dy,
            x: ev_x,
            y: ev_y,
            modifiers: self.modifiers,
            pixel_precise,
            target_frame_id: target_fid,
            webkit_id: wk_id,
            webkit_rel_x: wk_rx,
            webkit_rel_y: wk_ry,
        });
        self.record_idle_dim_activity(window_id);
    }
}
