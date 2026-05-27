//! Pointer, wheel, and hover handling for winit window events.

use super::RenderApp;
use super::frame_windows::GuiFrameWindowState;
use crate::backend::wgpu::NEOMACS_SUPER_MASK;
use crate::core::frame_glyphs::FrameGlyph;
use crate::thread_comm::{InputEvent, MenuBarItem, TabBarItem, ToolBarItem};
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

fn menu_bar_hit_test_items(
    items: &[MenuBarItem],
    height: f32,
    char_width: f32,
    x: f32,
    y: f32,
) -> Option<u32> {
    if height <= 0.0 || y >= height || items.is_empty() {
        return None;
    }
    let padding_x = 8.0_f32;
    let mut item_x = padding_x;
    for item in items {
        let label_width = item.label.len() as f32 * char_width + padding_x * 2.0;
        if x >= item_x && x < item_x + label_width {
            return Some(item.index);
        }
        item_x += label_width;
    }
    None
}

fn toolbar_hit_test_items(
    items: &[ToolBarItem],
    height: f32,
    padding: u32,
    icon_size: u32,
    x: f32,
    y: f32,
) -> Option<u32> {
    if height <= 0.0 || y >= height || items.is_empty() {
        return None;
    }
    let padding = padding as f32;
    let icon_size = icon_size as f32;
    let item_size = icon_size + padding * 2.0;
    let separator_width = 12.0_f32;
    let item_spacing = 2.0_f32;
    let mut item_x = padding;
    for item in items {
        if item.is_separator {
            item_x += separator_width;
            continue;
        }
        let right = item_x + item_size;
        if x >= item_x && x < right {
            return Some(item.index);
        }
        item_x = right + item_spacing;
    }
    None
}

fn compact_bar_menu_width(items: &[MenuBarItem], char_width: f32) -> f32 {
    let padding_x = 8.0_f32;
    let menu_width = items.iter().fold(padding_x, |x, item| {
        x + item.label.len() as f32 * char_width + padding_x * 2.0
    });
    menu_width + padding_x
}

fn tab_bar_hit_test_items(
    items: &[TabBarItem],
    height: f32,
    char_width: f32,
    x: f32,
    y: f32,
) -> Option<u32> {
    if height <= 0.0 || y >= height || items.is_empty() {
        return None;
    }
    let padding_x = 8.0_f32;
    let tab_padding = 12.0_f32;
    let mut tab_x = padding_x;
    for item in items {
        if item.is_separator {
            tab_x += 12.0;
            continue;
        }
        let tab_width = item.label.len() as f32 * char_width + tab_padding * 2.0;
        if x >= tab_x && x < tab_x + tab_width {
            return Some(item.index);
        }
        tab_x += tab_width + 2.0;
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
        window_state.render.glyph_atlas.default_char_width()
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
    ) -> Option<u32> {
        let compact_bar = window_state.render.compact_bar.as_ref()?;
        let x = x - compact_bar_menu_width(
            &compact_bar.menu_items,
            Self::frame_window_char_width(window_state),
        );
        if x < 0.0 {
            return None;
        }
        toolbar_hit_test_items(&compact_bar.tool_items, compact_bar.height, 5, 24, x, y)
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
    ) -> Option<u32> {
        let tool_bar = window_state.render.tool_bar.as_ref()?;
        let toolbar_y = Self::frame_window_toolbar_y_origin(window_state);
        if y < toolbar_y || y >= toolbar_y + tool_bar.height {
            return None;
        }
        toolbar_hit_test_items(&tool_bar.items, tool_bar.height, 5, 24, x, y - toolbar_y)
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
        if target_fid != 0 {
            self.child_frames
                .frames
                .get(&target_fid)
                .map(|entry| entry.frame.glyphs.as_slice())
        } else {
            self.current_frame
                .as_ref()
                .map(|frame| frame.glyphs.as_slice())
        }
    }

    fn pointer_target_at(&self, x: f32, y: f32) -> (f32, f32, u64) {
        #[cfg(feature = "wpe-webkit")]
        if Self::floating_webkit_hit_test(&self.floating_webkits, x, y).is_some() {
            return (x, y, 0);
        }
        if let Some((fid, local_x, local_y)) = self.child_frames.hit_test(x, y) {
            (local_x, local_y, fid)
        } else {
            (x, y, 0)
        }
    }

    fn webkit_target_at(&self, target_fid: u64, ev_x: f32, ev_y: f32) -> (u32, i32, i32) {
        let mut wk_id = 0u32;
        let mut wk_rx = 0i32;
        let mut wk_ry = 0i32;

        #[cfg(feature = "wpe-webkit")]
        if target_fid == 0 {
            if let Some((id, rx, ry)) =
                Self::floating_webkit_hit_test(&self.floating_webkits, ev_x, ev_y)
            {
                wk_id = id;
                wk_rx = rx;
                wk_ry = ry;
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
        let primary_event_frame_id = self.frame_windows.primary_event_frame_id();
        if !self.frame_windows.is_primary_winit(window_id) {
            let mut event = None;
            let mut handled_chrome = false;
            let mut delivered_mouse_button = false;
            if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
                if window_state.render.popup_menu.is_some() {
                    if state == ElementState::Pressed && button == MouseButton::Left {
                        let x = window_state.render.mouse_pos.0;
                        let y = window_state.render.mouse_pos.1;
                        if let Some(compact_bar) = window_state.render.compact_bar.as_ref()
                            && compact_bar.height > 0.0
                            && y < compact_bar.height
                        {
                            if let Some(idx) =
                                Self::frame_window_compact_bar_menu_hit_test(window_state, x, y)
                            {
                                self.comms
                                    .send_input(InputEvent::MenuSelection { index: -1 });
                                window_state.render.popup_menu = None;
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
                                window_state.render.popup_menu = None;
                                window_state
                                    .render
                                    .chrome_interaction
                                    .compact_bar_menu_active = None;
                            }
                            window_state.render.frame_dirty = true;
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
                                window_state.render.popup_menu = None;
                                window_state.render.chrome_interaction.menu_bar_active = Some(idx);
                                event = Some(InputEvent::MenuBarClick {
                                    index: idx as i32,
                                    emacs_frame_id: window_state.render.emacs_frame_id,
                                });
                            } else {
                                event = Some(InputEvent::MenuSelection { index: -1 });
                                window_state.render.popup_menu = None;
                                window_state.render.chrome_interaction.menu_bar_active = None;
                            }
                            window_state.render.frame_dirty = true;
                            handled_chrome = true;
                        } else if let Some(idx) =
                            Self::frame_window_tab_bar_hit_test(window_state, x, y)
                        {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            window_state.render.popup_menu = None;
                            window_state.render.chrome_interaction.tab_bar_pressed = Some(idx);
                            event = Some(InputEvent::TabBarClick {
                                index: idx as i32,
                                emacs_frame_id: window_state.render.emacs_frame_id,
                            });
                            window_state.render.frame_dirty = true;
                            handled_chrome = true;
                        } else if let Some(idx) =
                            Self::frame_window_toolbar_hit_test(window_state, x, y)
                        {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            window_state.render.popup_menu = None;
                            window_state.render.chrome_interaction.toolbar_pressed = Some(idx);
                            event = Some(InputEvent::ToolBarClick {
                                index: idx as i32,
                                emacs_frame_id: window_state.render.emacs_frame_id,
                            });
                            window_state.render.frame_dirty = true;
                            handled_chrome = true;
                        } else {
                            let idx = window_state
                                .render
                                .popup_menu
                                .as_ref()
                                .map_or(-1, |menu| menu.hit_test(x, y));
                            if idx >= 0 {
                                event = Some(InputEvent::MenuSelection { index: idx });
                                window_state.render.popup_menu = None;
                                window_state.render.chrome_interaction.menu_bar_active = None;
                                window_state.render.frame_dirty = true;
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
                                        window_state.render.frame_dirty = true;
                                    } else {
                                        event = Some(InputEvent::MenuSelection { index: -1 });
                                        window_state.render.popup_menu = None;
                                        window_state.render.chrome_interaction.menu_bar_active =
                                            None;
                                        window_state.render.frame_dirty = true;
                                    }
                                } else {
                                    event = Some(InputEvent::MenuSelection { index: -1 });
                                    window_state.render.popup_menu = None;
                                    window_state.render.chrome_interaction.menu_bar_active = None;
                                    window_state.render.frame_dirty = true;
                                }
                            }
                            handled_chrome = true;
                        }
                    } else if state == ElementState::Pressed {
                        event = Some(InputEvent::MenuSelection { index: -1 });
                        window_state.render.popup_menu = None;
                        window_state.render.chrome_interaction.menu_bar_active = None;
                        window_state.render.frame_dirty = true;
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
                            window_state.render.frame_dirty = true;
                            handled_chrome = true;
                        }
                        if !handled_chrome
                            && let Some(idx) =
                                Self::frame_window_compact_bar_tool_hit_test(window_state, x, y)
                        {
                            window_state
                                .render
                                .chrome_interaction
                                .compact_bar_tool_pressed = Some(idx);
                            event = Some(InputEvent::ToolBarClick {
                                index: idx as i32,
                                emacs_frame_id: window_state.render.emacs_frame_id,
                            });
                            window_state.render.frame_dirty = true;
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
                            window_state.render.frame_dirty = true;
                            handled_chrome = true;
                        }
                    } else if !handled_chrome
                        && let Some(idx) = Self::frame_window_tab_bar_hit_test(window_state, x, y)
                    {
                        window_state.render.chrome_interaction.tab_bar_pressed = Some(idx);
                        event = Some(InputEvent::TabBarClick {
                            index: idx as i32,
                            emacs_frame_id: window_state.render.emacs_frame_id,
                        });
                        window_state.render.frame_dirty = true;
                        handled_chrome = true;
                    } else if !handled_chrome
                        && let Some(idx) = Self::frame_window_toolbar_hit_test(window_state, x, y)
                    {
                        window_state.render.chrome_interaction.toolbar_pressed = Some(idx);
                        event = Some(InputEvent::ToolBarClick {
                            index: idx as i32,
                            emacs_frame_id: window_state.render.emacs_frame_id,
                        });
                        window_state.render.frame_dirty = true;
                        handled_chrome = true;
                    }
                } else if state == ElementState::Released && button == MouseButton::Left {
                    if window_state
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
                        window_state.render.chrome_interaction.tab_bar_pressed = None;
                        window_state
                            .render
                            .chrome_interaction
                            .compact_bar_tool_pressed = None;
                        window_state.render.chrome_interaction.toolbar_pressed = None;
                        window_state.render.frame_dirty = true;
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
                window_state.render.transient_effects.trigger_click_halo(
                    x,
                    y,
                    std::time::Instant::now(),
                    self.effects.click_halo.duration_ms,
                );
                window_state.render.frame_dirty = true;
            }
            return;
        }

        if state == ElementState::Pressed {
            tracing::debug!(
                "MouseInput: {:?} at ({:.1}, {:.1}), menu_bar_h={}, popup={}",
                button,
                self.mouse_pos.0,
                self.mouse_pos.1,
                self.menu_bar_height,
                self.popup_menu.is_some()
            );
        }

        if let Some(ref mut menu) = self.popup_menu {
            if state == ElementState::Pressed && button == MouseButton::Left {
                if self.compact_bar_height > 0.0 && self.mouse_pos.1 < self.compact_bar_height {
                    if let Some(idx) =
                        self.compact_bar_menu_hit_test(self.mouse_pos.0, self.mouse_pos.1)
                    {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: -1 });
                        self.popup_menu = None;
                        self.chrome_interaction.compact_bar_menu_active = Some(idx);
                        self.comms.send_input(InputEvent::MenuBarClick {
                            index: idx as i32,
                            emacs_frame_id: primary_event_frame_id,
                        });
                        self.frame_dirty = true;
                    } else {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: -1 });
                        self.popup_menu = None;
                        self.chrome_interaction.compact_bar_menu_active = None;
                        self.frame_dirty = true;
                    }
                } else if self.menu_bar_height > 0.0 && self.mouse_pos.1 < self.menu_bar_height {
                    if let Some(idx) = self.menu_bar_hit_test(self.mouse_pos.0, self.mouse_pos.1) {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: -1 });
                        self.popup_menu = None;
                        self.chrome_interaction.menu_bar_active = Some(idx);
                        self.comms.send_input(InputEvent::MenuBarClick {
                            index: idx as i32,
                            emacs_frame_id: primary_event_frame_id,
                        });
                        self.frame_dirty = true;
                    } else {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: -1 });
                        self.popup_menu = None;
                        self.chrome_interaction.menu_bar_active = None;
                        self.frame_dirty = true;
                    }
                } else {
                    let idx = menu.hit_test(self.mouse_pos.0, self.mouse_pos.1);
                    if idx >= 0 {
                        self.comms
                            .send_input(InputEvent::MenuSelection { index: idx });
                        self.popup_menu = None;
                        self.chrome_interaction.menu_bar_active = None;
                        self.frame_dirty = true;
                    } else {
                        let (depth, local_idx) =
                            menu.hit_test_all(self.mouse_pos.0, self.mouse_pos.1);
                        if depth >= 0 && local_idx >= 0 {
                            let panel = if depth == 0 {
                                &menu.root_panel
                            } else {
                                &menu.submenu_panels[(depth - 1) as usize]
                            };
                            let global_idx = panel.item_indices[local_idx as usize];
                            if menu.all_items[global_idx].submenu {
                                self.frame_dirty = true;
                            } else {
                                self.comms
                                    .send_input(InputEvent::MenuSelection { index: -1 });
                                self.popup_menu = None;
                                self.chrome_interaction.menu_bar_active = None;
                                self.frame_dirty = true;
                            }
                        } else {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            self.popup_menu = None;
                            self.chrome_interaction.menu_bar_active = None;
                            self.frame_dirty = true;
                        }
                    }
                }
            } else if state == ElementState::Pressed {
                self.comms
                    .send_input(InputEvent::MenuSelection { index: -1 });
                self.popup_menu = None;
                self.chrome_interaction.menu_bar_active = None;
                self.frame_dirty = true;
            }
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && self.chrome.resize_edge.is_some()
        {
            if let (Some(dir), Some(ref window)) = (self.chrome.resize_edge, self.window.as_ref()) {
                let _ = window.drag_resize_window(dir);
            }
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && self.titlebar_hit_test(self.mouse_pos.0, self.mouse_pos.1) > 0
        {
            match self.titlebar_hit_test(self.mouse_pos.0, self.mouse_pos.1) {
                1 => {
                    let now = std::time::Instant::now();
                    if now
                        .duration_since(self.chrome.last_titlebar_click)
                        .as_millis()
                        < 400
                    {
                        if let Some(ref window) = self.window {
                            window.set_maximized(!window.is_maximized());
                        }
                    } else if let Some(ref window) = self.window {
                        let _ = window.drag_window();
                    }
                    self.chrome.last_titlebar_click = now;
                }
                2 => {
                    self.comms.send_input(InputEvent::WindowClose {
                        emacs_frame_id: primary_event_frame_id,
                    });
                }
                3 => {
                    if let Some(ref window) = self.window {
                        if window.is_maximized() {
                            window.set_maximized(false);
                        } else {
                            window.set_maximized(true);
                        }
                    }
                }
                4 => {
                    if let Some(ref window) = self.window {
                        window.set_minimized(true);
                    }
                }
                _ => {}
            }
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && !self.chrome.decorations_enabled
            && (self.modifiers & NEOMACS_SUPER_MASK) != 0
        {
            if let Some(ref window) = self.window {
                let _ = window.drag_window();
            }
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && self.compact_bar_height > 0.0
            && self.mouse_pos.1 < self.compact_bar_height
        {
            if let Some(idx) = self.compact_bar_menu_hit_test(self.mouse_pos.0, self.mouse_pos.1) {
                if self.chrome_interaction.compact_bar_menu_active == Some(idx) {
                    self.chrome_interaction.compact_bar_menu_active = None;
                } else {
                    self.chrome_interaction.compact_bar_menu_active = Some(idx);
                    self.comms.send_input(InputEvent::MenuBarClick {
                        index: idx as i32,
                        emacs_frame_id: primary_event_frame_id,
                    });
                }
                self.frame_dirty = true;
                return;
            }
            if let Some(idx) = self.compact_bar_tool_hit_test(self.mouse_pos.0, self.mouse_pos.1) {
                self.chrome_interaction.compact_bar_tool_pressed = Some(idx);
                self.comms.send_input(InputEvent::ToolBarClick {
                    index: idx as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
                self.frame_dirty = true;
                return;
            }
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && self.menu_bar_height > 0.0
            && self.mouse_pos.1 < self.menu_bar_height
        {
            tracing::debug!(
                "Menu bar click at ({:.1}, {:.1}), menu_bar_height={}",
                self.mouse_pos.0,
                self.mouse_pos.1,
                self.menu_bar_height
            );
            if let Some(idx) = self.menu_bar_hit_test(self.mouse_pos.0, self.mouse_pos.1) {
                self.chrome_interaction.menu_bar_active = Some(idx);
                self.comms.send_input(InputEvent::MenuBarClick {
                    index: idx as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
                self.frame_dirty = true;
                return;
            }
        }

        // Tab bar click (between menu bar and toolbar)
        if state == ElementState::Pressed
            && button == MouseButton::Left
            && self.tab_bar_height > 0.0
            && self.mouse_pos.1 >= self.tab_bar_y
            && self.mouse_pos.1 < self.tab_bar_y + self.tab_bar_height
        {
            if let Some(idx) = self.tab_bar_hit_test(self.mouse_pos.0, self.mouse_pos.1) {
                self.chrome_interaction.tab_bar_pressed = Some(idx);
                self.comms.send_input(InputEvent::TabBarClick {
                    index: idx as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
                self.frame_dirty = true;
            }
            return;
        }

        if state == ElementState::Released
            && button == MouseButton::Left
            && self.chrome_interaction.tab_bar_pressed.is_some()
        {
            self.chrome_interaction.tab_bar_pressed = None;
            self.frame_dirty = true;
            return;
        }

        if state == ElementState::Pressed
            && button == MouseButton::Left
            && self.toolbar_height > 0.0
            && self.mouse_pos.1 < self.toolbar_y_origin() + self.toolbar_height
            && self.mouse_pos.1 >= self.toolbar_y_origin()
        {
            if let Some(idx) =
                self.toolbar_hit_test(self.mouse_pos.0, self.mouse_pos.1 - self.toolbar_y_origin())
            {
                self.chrome_interaction.toolbar_pressed = Some(idx);
                self.comms.send_input(InputEvent::ToolBarClick {
                    index: idx as i32,
                    emacs_frame_id: primary_event_frame_id,
                });
                self.frame_dirty = true;
            }
            return;
        }

        if state == ElementState::Released
            && button == MouseButton::Left
            && self.chrome_interaction.compact_bar_tool_pressed.is_some()
        {
            self.chrome_interaction.compact_bar_tool_pressed = None;
            self.frame_dirty = true;
            return;
        }

        if state == ElementState::Released
            && button == MouseButton::Left
            && self.chrome_interaction.toolbar_pressed.is_some()
        {
            self.chrome_interaction.toolbar_pressed = None;
            self.frame_dirty = true;
            return;
        }

        if state == ElementState::Pressed && button == MouseButton::Left {
            tracing::trace!(
                "Left click at ({:.1}, {:.1}) NOT in menu bar (h={}) or toolbar (h={})",
                self.mouse_pos.0,
                self.mouse_pos.1,
                self.menu_bar_height,
                self.toolbar_height
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

        let (ev_x, ev_y, target_fid) = self.pointer_target_at(self.mouse_pos.0, self.mouse_pos.1);
        if target_fid != 0 {
            if let Some(entry) = self.child_frames.frames.get(&target_fid) {
                tracing::trace!(
                    "Child frame hit: fid={} abs=({:.1},{:.1}) size=({:.1}x{:.1}) mouse=({:.1},{:.1}) local=({:.1},{:.1})",
                    target_fid,
                    entry.abs_x,
                    entry.abs_y,
                    entry.frame.width,
                    entry.frame.height,
                    self.mouse_pos.0,
                    self.mouse_pos.1,
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

        if state == ElementState::Pressed && self.effects.click_halo.enabled {
            let now = std::time::Instant::now();
            if let Some(renderer) = self.renderer.as_mut() {
                renderer.trigger_click_halo(self.mouse_pos.0, self.mouse_pos.1, now);
            }
            self.frame_dirty = true;
        }
    }

    pub(super) fn handle_cursor_moved(
        &mut self,
        window_id: WindowId,
        position: PhysicalPosition<f64>,
    ) {
        if !self.frame_windows.is_primary_winit(window_id) {
            let mut event = None;
            if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
                let lx = (position.x / window_state.native.scale_factor) as f32;
                let ly = (position.y / window_state.native.scale_factor) as f32;
                window_state.render.mouse_pos = (lx, ly);
                let mut dirty = false;

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
                            Self::frame_window_compact_bar_tool_hit_test(window_state, lx, ly)
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
                    Self::frame_window_toolbar_hit_test(window_state, lx, ly);
                dirty |=
                    window_state.render.chrome_interaction.toolbar_hovered != old_toolbar_hover;

                if let Some(ref mut menu) = window_state.render.popup_menu {
                    let (hit_depth, hit_local) = menu.hit_test_all(lx, ly);
                    if hit_depth >= 0 {
                        let target_depth = hit_depth as usize;
                        while menu.submenu_panels.len() > target_depth {
                            menu.submenu_panels.pop();
                            dirty = true;
                        }
                        let panel = if target_depth == 0 {
                            &mut menu.root_panel
                        } else {
                            &mut menu.submenu_panels[target_depth - 1]
                        };
                        if hit_local != panel.hover_index {
                            panel.hover_index = hit_local;
                            dirty = true;
                            if hit_local >= 0 && (hit_local as usize) < panel.item_indices.len() {
                                let global_idx = panel.item_indices[hit_local as usize];
                                if menu.all_items[global_idx].submenu {
                                    menu.open_submenu();
                                }
                            }
                        }
                    }
                }

                if dirty {
                    window_state.render.frame_dirty = true;
                }

                if window_state.native.mouse_hidden_for_typing {
                    window_state.native.window.set_cursor_visible(true);
                    window_state.native.mouse_hidden_for_typing = false;
                }

                let (ev_x, ev_y, target_fid) =
                    Self::pointer_target_for_frame_window(window_state, lx, ly);
                if event.is_none() {
                    event = Some(InputEvent::MouseMove {
                        x: ev_x,
                        y: ev_y,
                        modifiers: self.modifiers,
                        target_frame_id: target_fid,
                    });
                }
            }
            if let Some(event) = event {
                self.comms.send_input(event);
            }
            return;
        }

        let lx = (position.x / self.scale_factor) as f32;
        let ly = (position.y / self.scale_factor) as f32;
        self.mouse_pos = (lx, ly);
        let primary_event_frame_id = self.frame_windows.primary_event_frame_id();

        if self.effects.idle_dim.enabled {
            self.last_activity_time = std::time::Instant::now();
        }

        if self.mouse_hidden_for_typing {
            if let Some(ref window) = self.window {
                window.set_cursor_visible(true);
            }
            self.mouse_hidden_for_typing = false;
        }

        let edge = self.detect_resize_edge(lx, ly);
        if edge != self.chrome.resize_edge {
            self.chrome.resize_edge = edge;
            if let Some(ref window) = self.window {
                use winit::window::CursorIcon;
                let icon = match edge {
                    Some(dir) => CursorIcon::from(dir),
                    None => CursorIcon::Default,
                };
                window.set_cursor(icon);
            }
        }

        if !self.chrome.decorations_enabled {
            let new_hover = self.titlebar_hit_test(lx, ly);
            if new_hover != self.chrome.titlebar_hover {
                self.chrome.titlebar_hover = new_hover;
                self.frame_dirty = true;
                if self.chrome.resize_edge.is_none() {
                    if let Some(ref window) = self.window {
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

        if self.menu_bar_height > 0.0 {
            let old_hover = self.chrome_interaction.menu_bar_hovered;
            if ly < self.menu_bar_height {
                let new_hover = self.menu_bar_hit_test(lx, ly);
                self.chrome_interaction.menu_bar_hovered = new_hover;
                if let (Some(active), Some(hov)) =
                    (self.chrome_interaction.menu_bar_active, new_hover)
                {
                    if hov != active {
                        self.chrome_interaction.menu_bar_active = Some(hov);
                        self.comms.send_input(InputEvent::MenuBarClick {
                            index: hov as i32,
                            emacs_frame_id: primary_event_frame_id,
                        });
                    }
                }
            } else {
                self.chrome_interaction.menu_bar_hovered = None;
            }
            if self.chrome_interaction.menu_bar_hovered != old_hover {
                self.frame_dirty = true;
            }
        }

        if self.compact_bar_height > 0.0 {
            let old_menu_hover = self.chrome_interaction.compact_bar_menu_hovered;
            let old_tool_hover = self.chrome_interaction.compact_bar_tool_hovered;
            if ly < self.compact_bar_height {
                let new_menu_hover = self.compact_bar_menu_hit_test(lx, ly);
                self.chrome_interaction.compact_bar_menu_hovered = new_menu_hover;
                self.chrome_interaction.compact_bar_tool_hovered = if new_menu_hover.is_none() {
                    self.compact_bar_tool_hit_test(lx, ly)
                } else {
                    None
                };
                if let (Some(active), Some(hov)) = (
                    self.chrome_interaction.compact_bar_menu_active,
                    new_menu_hover,
                ) && hov != active
                {
                    self.chrome_interaction.compact_bar_menu_active = Some(hov);
                    self.comms.send_input(InputEvent::MenuBarClick {
                        index: hov as i32,
                        emacs_frame_id: primary_event_frame_id,
                    });
                }
            } else {
                self.chrome_interaction.compact_bar_menu_hovered = None;
                self.chrome_interaction.compact_bar_tool_hovered = None;
            }
            if self.chrome_interaction.compact_bar_menu_hovered != old_menu_hover
                || self.chrome_interaction.compact_bar_tool_hovered != old_tool_hover
            {
                self.frame_dirty = true;
            }
        }

        if self.tab_bar_height > 0.0 {
            let old_hover = self.chrome_interaction.tab_bar_hovered;
            if ly >= self.tab_bar_y && ly < self.tab_bar_y + self.tab_bar_height {
                self.chrome_interaction.tab_bar_hovered = self.tab_bar_hit_test(lx, ly);
            } else {
                self.chrome_interaction.tab_bar_hovered = None;
            }
            if self.chrome_interaction.tab_bar_hovered != old_hover {
                self.frame_dirty = true;
            }
        }

        if self.toolbar_height > 0.0 {
            let old_hover = self.chrome_interaction.toolbar_hovered;
            let toolbar_y = self.toolbar_y_origin();
            if ly < toolbar_y + self.toolbar_height && ly >= toolbar_y {
                self.chrome_interaction.toolbar_hovered = self.toolbar_hit_test(lx, ly - toolbar_y);
            } else {
                self.chrome_interaction.toolbar_hovered = None;
            }
            if self.chrome_interaction.toolbar_hovered != old_hover {
                self.frame_dirty = true;
            }
        }

        if let Some(ref mut menu) = self.popup_menu {
            let (hit_depth, hit_local) = menu.hit_test_all(lx, ly);
            if hit_depth >= 0 {
                let target_depth = hit_depth as usize;
                while menu.submenu_panels.len() > target_depth {
                    menu.submenu_panels.pop();
                    self.frame_dirty = true;
                }
                let panel = if target_depth == 0 {
                    &mut menu.root_panel
                } else {
                    &mut menu.submenu_panels[target_depth - 1]
                };
                if hit_local != panel.hover_index {
                    panel.hover_index = hit_local;
                    self.frame_dirty = true;
                    if hit_local >= 0 && (hit_local as usize) < panel.item_indices.len() {
                        let global_idx = panel.item_indices[hit_local as usize];
                        if menu.all_items[global_idx].submenu {
                            menu.open_submenu();
                        }
                    }
                }
            }
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
        if !self.frame_windows.is_primary_winit(window_id) {
            if let Some(window_state) = self.frame_windows.get_by_winit(window_id) {
                let (dx, dy, pixel_precise) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x, y, false),
                    MouseScrollDelta::PixelDelta(pos) => (
                        (pos.x / window_state.native.scale_factor) as f32,
                        (pos.y / window_state.native.scale_factor) as f32,
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
            }
            return;
        }

        let (dx, dy, pixel_precise) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (x, y, false),
            MouseScrollDelta::PixelDelta(pos) => (
                (pos.x / self.scale_factor) as f32,
                (pos.y / self.scale_factor) as f32,
                true,
            ),
        };

        let (ev_x, ev_y, target_fid) = self.pointer_target_at(self.mouse_pos.0, self.mouse_pos.1);
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
    }
}
