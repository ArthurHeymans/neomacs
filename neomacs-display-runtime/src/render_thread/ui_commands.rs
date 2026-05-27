//! UI overlay, animation, and effect render commands.

use super::{PopupMenuState, RenderApp, TooltipState};
use crate::thread_comm::{RenderCommand, ToolBarItem};
use neomacs_display_protocol::glyph_matrix::{GuiMenuBarState, GuiToolBarState};
use winit::dpi::{PhysicalPosition, PhysicalSize};

const GNU_TOOL_BAR_BASE_HEIGHT: f32 = 34.0;
const GNU_TOOL_BAR_BASE_PADDING: f32 = 5.0;

pub(super) fn toolbar_visual_config_for_height(height: f32) -> (u32, u32) {
    let height_px = if height.is_finite() && height > 0.0 {
        height.round().max(1.0) as u32
    } else {
        GNU_TOOL_BAR_BASE_HEIGHT as u32
    };
    let scale = (height_px as f32 / GNU_TOOL_BAR_BASE_HEIGHT).max(0.1);
    let max_padding = height_px.saturating_sub(1) / 2;
    let padding = ((GNU_TOOL_BAR_BASE_PADDING * scale).round() as u32).min(max_padding);
    let icon_size = height_px.saturating_sub(padding.saturating_mul(2)).max(1);

    (icon_size, padding)
}

impl RenderApp {
    fn mark_frame_windows_dirty(&mut self) {
        for window_state in self.frame_windows.windows.values_mut() {
            window_state.render.frame_dirty = true;
        }
    }

    pub(super) fn set_toolbar_visual_config(&mut self, icon_size: u32, padding: u32) {
        if self.toolbar_icon_size == icon_size && self.toolbar_padding == padding {
            return;
        }
        self.toolbar_icon_size = icon_size;
        self.toolbar_padding = padding;
        for (_name, id) in self.toolbar_icon_textures.drain() {
            if let Some(renderer) = self.renderer.as_mut() {
                renderer.free_image(id);
            }
        }
    }

    pub(super) fn sync_toolbar_visual_config_from_height(&mut self, height: f32) {
        let (icon_size, padding) = toolbar_visual_config_for_height(height);
        self.set_toolbar_visual_config(icon_size, padding);
    }

    pub(super) fn ensure_toolbar_icon_textures(&mut self, items: &[ToolBarItem]) {
        for item in items {
            if !item.is_separator
                && !item.icon_name.is_empty()
                && !self.toolbar_icon_textures.contains_key(&item.icon_name)
                && let Some(svg_data) =
                    crate::backend::wgpu::toolbar_icons::get_icon_svg(&item.icon_name)
                && let Some(renderer) = self.renderer.as_mut()
            {
                let icon_size = self.toolbar_icon_size;
                let id = renderer.load_image_data(svg_data, icon_size, icon_size, 0, 0);
                self.toolbar_icon_textures
                    .insert(item.icon_name.clone(), id);
                tracing::debug!(
                    "Loaded toolbar icon '{}' as image_id={}",
                    item.icon_name,
                    id
                );
            }
        }
    }

    pub(super) fn handle_ui_command(&mut self, cmd: RenderCommand) -> Result<(), RenderCommand> {
        match cmd {
            RenderCommand::SetCursorBlink {
                enabled,
                interval_ms,
            } => {
                tracing::debug!(
                    "Cursor blink: enabled={}, interval={}ms",
                    enabled,
                    interval_ms
                );
                self.cursor_defaults.blink_enabled = enabled;
                self.cursor_defaults.blink_interval =
                    std::time::Duration::from_millis(interval_ms as u64);
                if !enabled {
                    self.cursor_defaults.blink_on = true;
                    if let Some(cursor) = self.primary_cursor_mut() {
                        cursor.blink_on = true;
                    }
                    self.mark_primary_dirty();
                }
                self.sync_primary_cursor_config_from_defaults();
                for window_state in self.frame_windows.windows.values_mut() {
                    window_state
                        .render
                        .cursor
                        .copy_config_from(&self.cursor_defaults);
                    if !enabled {
                        window_state.render.cursor.blink_on = true;
                        window_state.render.frame_dirty = true;
                    }
                }
                Ok(())
            }
            RenderCommand::SetCursorAnimation { enabled, speed } => {
                tracing::debug!("Cursor animation: enabled={}, speed={}", enabled, speed);
                self.cursor_defaults.anim_enabled = enabled;
                self.cursor_defaults.anim_speed = speed;
                if !enabled {
                    self.cursor_defaults.animating = false;
                    if let Some(cursor) = self.primary_cursor_mut() {
                        cursor.animating = false;
                    }
                }
                self.sync_primary_cursor_config_from_defaults();
                for window_state in self.frame_windows.windows.values_mut() {
                    window_state
                        .render
                        .cursor
                        .copy_config_from(&self.cursor_defaults);
                    window_state.render.frame_dirty = true;
                }
                Ok(())
            }
            RenderCommand::SetAnimationConfig {
                cursor_enabled,
                cursor_speed,
                cursor_style,
                cursor_duration_ms,
                transition_policy,
                trail_size,
            } => {
                tracing::debug!(
                    "Animation config: cursor={}/{}/style={:?}/{}ms/trail={}, crossfade={}/{}ms/effect={:?}/easing={:?}, scroll={}/{}ms/effect={:?}/easing={:?}",
                    cursor_enabled,
                    cursor_speed,
                    cursor_style,
                    cursor_duration_ms,
                    trail_size,
                    transition_policy.crossfade_enabled,
                    transition_policy.crossfade_duration_ms,
                    transition_policy.crossfade_effect,
                    transition_policy.crossfade_easing,
                    transition_policy.scroll_enabled,
                    transition_policy.scroll_duration_ms,
                    transition_policy.scroll_effect,
                    transition_policy.scroll_easing
                );
                self.cursor_defaults.anim_enabled = cursor_enabled;
                self.cursor_defaults.anim_speed = cursor_speed;
                self.cursor_defaults.anim_style = cursor_style;
                self.cursor_defaults.anim_duration = cursor_duration_ms as f32 / 1000.0;
                self.cursor_defaults.trail_size = trail_size.clamp(0.0, 1.0);
                self.transition_policy = transition_policy;
                self.sync_primary_transition_policy_from_default();
                self.mark_primary_dirty();
                if !cursor_enabled {
                    self.cursor_defaults.animating = false;
                    if let Some(cursor) = self.primary_cursor_mut() {
                        cursor.animating = false;
                    }
                }
                self.sync_primary_cursor_config_from_defaults();
                for window_state in self.frame_windows.windows.values_mut() {
                    window_state
                        .render
                        .cursor
                        .copy_config_from(&self.cursor_defaults);
                    window_state.render.transitions.policy = transition_policy;
                    window_state.render.frame_dirty = true;
                }
                if !self.transition_policy.crossfade_enabled {
                    if let Some(primary_frame) = self.primary_render_state_mut() {
                        primary_frame.transitions.crossfades.clear();
                    }
                    for window_state in self.frame_windows.windows.values_mut() {
                        window_state.render.transitions.crossfades.clear();
                    }
                }
                if !self.transition_policy.scroll_enabled {
                    if let Some(primary_frame) = self.primary_render_state_mut() {
                        primary_frame.transitions.scroll_slides.clear();
                    }
                    for window_state in self.frame_windows.windows.values_mut() {
                        window_state.render.transitions.scroll_slides.clear();
                    }
                }
                Ok(())
            }
            RenderCommand::ShowPopupMenu {
                emacs_frame_id,
                x,
                y,
                items,
                title,
                fg,
                bg,
            } => {
                tracing::info!(
                    "ShowPopupMenu frame=0x{:x} at ({}, {}) with {} items",
                    emacs_frame_id,
                    x,
                    y,
                    items.len()
                );
                let (fs, lh, cw) = if emacs_frame_id == 0 {
                    self.primary_render_state()
                        .map(|a| {
                            let atlas = &a.glyph_atlas;
                            (
                                atlas.default_font_size(),
                                atlas.default_line_height(),
                                atlas.default_char_width(),
                            )
                        })
                        .unwrap_or((13.0, 17.0, 13.0 * 0.6))
                } else {
                    self.frame_windows
                        .get(emacs_frame_id)
                        .map(|window_state| {
                            let atlas = &window_state.render.glyph_atlas;
                            (
                                atlas.default_font_size(),
                                atlas.default_line_height(),
                                atlas.default_char_width(),
                            )
                        })
                        .unwrap_or((13.0, 17.0, 13.0 * 0.6))
                };
                let mut menu = PopupMenuState::new(x, y, items, title, fs, lh, cw);
                menu.face_fg = fg;
                menu.face_bg = bg;
                if emacs_frame_id == 0 {
                    self.set_primary_popup_menu(Some(menu));
                    self.mark_primary_dirty();
                } else if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                    window_state.render.popup_menu = Some(menu);
                    window_state.render.frame_dirty = true;
                } else {
                    tracing::warn!(
                        "ShowPopupMenu requested for unknown frame_id=0x{:x}",
                        emacs_frame_id
                    );
                }
                Ok(())
            }
            RenderCommand::HidePopupMenu => {
                tracing::info!("HidePopupMenu");
                self.set_primary_popup_menu(None);
                self.with_primary_chrome_interaction_mut(|chrome| chrome.menu_bar_active = None);
                for window_state in self.frame_windows.windows.values_mut() {
                    if window_state.render.popup_menu.is_some() {
                        window_state.render.popup_menu = None;
                        window_state.render.frame_dirty = true;
                    }
                }
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::ShowTooltip {
                emacs_frame_id,
                x,
                y,
                text,
                fg_r,
                fg_g,
                fg_b,
                bg_r,
                bg_g,
                bg_b,
            } => {
                tracing::debug!("ShowTooltip frame=0x{:x} at ({}, {})", emacs_frame_id, x, y);
                let (fs, lh, cw, screen_w, screen_h) = if emacs_frame_id == 0 {
                    let (fs, lh, cw) = self
                        .primary_render_state()
                        .map(|primary_frame| {
                            let atlas = &primary_frame.glyph_atlas;
                            (
                                atlas.default_font_size(),
                                atlas.default_line_height(),
                                atlas.default_char_width(),
                            )
                        })
                        .unwrap_or((13.0, 17.0, 13.0 * 0.6));
                    let (screen_w, screen_h) = self.primary_logical_size();
                    (fs, lh, cw, screen_w, screen_h)
                } else {
                    self.frame_windows
                        .get(emacs_frame_id)
                        .map(|window_state| {
                            let atlas = &window_state.render.glyph_atlas;
                            (
                                atlas.default_font_size(),
                                atlas.default_line_height(),
                                atlas.default_char_width(),
                                window_state.native.width as f32
                                    / window_state.native.scale_factor as f32,
                                window_state.native.height as f32
                                    / window_state.native.scale_factor as f32,
                            )
                        })
                        .unwrap_or((
                            13.0,
                            17.0,
                            13.0 * 0.6,
                            self.width as f32,
                            self.height as f32,
                        ))
                };
                let tooltip = TooltipState::new(
                    x,
                    y,
                    &text,
                    (fg_r, fg_g, fg_b),
                    (bg_r, bg_g, bg_b),
                    screen_w,
                    screen_h,
                    fs,
                    lh,
                    cw,
                );
                if emacs_frame_id == 0 {
                    self.set_primary_tooltip(Some(tooltip));
                    self.mark_primary_dirty();
                } else if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                    window_state.render.tooltip = Some(tooltip);
                    window_state.render.frame_dirty = true;
                } else {
                    tracing::warn!(
                        "ShowTooltip requested for unknown frame_id=0x{:x}",
                        emacs_frame_id
                    );
                }
                Ok(())
            }
            RenderCommand::HideTooltip => {
                tracing::debug!("HideTooltip");
                self.set_primary_tooltip(None);
                for window_state in self.frame_windows.windows.values_mut() {
                    if window_state.render.tooltip.is_some() {
                        window_state.render.tooltip = None;
                        window_state.render.frame_dirty = true;
                    }
                }
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::VisualBell { emacs_frame_id } => {
                let now = std::time::Instant::now();
                if emacs_frame_id == 0
                    || self.frame_windows.primary_frame_id() == Some(emacs_frame_id)
                {
                    self.set_primary_visual_bell_start(Some(now));
                    if self.effects.cursor_error_pulse.enabled {
                        if let (Some(renderer), Some(primary_state)) =
                            (self.renderer.as_ref(), self.primary_window_state.as_mut())
                        {
                            renderer.trigger_transient_cursor_error_pulse(
                                &mut primary_state.render.renderer_effects,
                                now,
                            );
                        }
                    }
                    if self.effects.edge_snap.enabled {
                        let selected_info = self.primary_current_frame().and_then(|frame| {
                            frame
                                .window_infos
                                .iter()
                                .find(|info| info.selected && !info.is_minibuffer)
                                .cloned()
                        });
                        if let Some(info) = selected_info {
                            let at_top = info.window_start <= 1;
                            let at_bottom = info.window_end >= info.buffer_size;
                            if at_top || at_bottom {
                                if let (Some(renderer), Some(primary_state)) =
                                    (self.renderer.as_ref(), self.primary_window_state.as_mut())
                                {
                                    renderer.trigger_transient_edge_snap(
                                        &mut primary_state.render.renderer_effects,
                                        info.bounds,
                                        info.mode_line_height,
                                        at_top,
                                        at_bottom,
                                        now,
                                    );
                                }
                            }
                        }
                    }
                } else if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                    window_state.render.visual_bell_start = Some(now);
                    if self.effects.cursor_error_pulse.enabled {
                        window_state
                            .render
                            .renderer_effects
                            .trigger_cursor_error_pulse(now);
                    }
                    if self.effects.edge_snap.enabled {
                        let selected_info =
                            window_state
                                .render
                                .current_frame
                                .as_ref()
                                .and_then(|frame| {
                                    frame
                                        .window_infos
                                        .iter()
                                        .find(|info| info.selected && !info.is_minibuffer)
                                        .cloned()
                                });
                        if let Some(info) = selected_info {
                            {
                                let at_top = info.window_start <= 1;
                                let at_bottom = info.window_end >= info.buffer_size;
                                if at_top || at_bottom {
                                    window_state.render.renderer_effects.trigger_edge_snap(
                                        info.bounds,
                                        info.mode_line_height,
                                        at_top,
                                        at_bottom,
                                        now,
                                        self.effects.edge_snap.duration_ms,
                                    );
                                }
                            }
                        }
                    }
                    window_state.render.frame_dirty = true;
                } else {
                    tracing::warn!(
                        "VisualBell requested for unknown frame_id=0x{:x}",
                        emacs_frame_id
                    );
                }
                Ok(())
            }
            RenderCommand::UpdateEffect(updater) => {
                (updater.0)(&mut self.effects);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.effects = self.effects.clone();
                }
                self.mark_frame_windows_dirty();
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetCursorEffect(command) => {
                command.apply_to(&mut self.effects);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.effects = self.effects.clone();
                }
                self.mark_frame_windows_dirty();
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetScrollIndicators { enabled } => {
                self.scroll_indicators_enabled = enabled;
                self.mark_frame_windows_dirty();
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetTitlebarHeight { height } => {
                self.chrome.titlebar_height = height;
                self.primary_chrome_mut().titlebar_height = height;
                self.frame_windows.chrome_defaults.titlebar_height = height;
                for window_state in self.frame_windows.windows.values_mut() {
                    window_state.native.chrome.titlebar_height = height;
                    window_state.render.frame_dirty = true;
                }
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetShowFps { enabled } => {
                self.primary_fps_enabled = enabled;
                if let Some(primary_frame) = self.primary_render_state_mut() {
                    primary_frame.fps.enabled = enabled;
                    primary_frame.frame_dirty = true;
                }
                self.frame_windows.fps_enabled = enabled;
                for window_state in self.frame_windows.windows.values_mut() {
                    window_state.render.fps.enabled = enabled;
                    window_state.render.frame_dirty = true;
                }
                Ok(())
            }
            RenderCommand::SetCornerRadius { radius } => {
                self.chrome.corner_radius = radius;
                self.primary_chrome_mut().corner_radius = radius;
                self.frame_windows.chrome_defaults.corner_radius = radius;
                for window_state in self.frame_windows.windows.values_mut() {
                    window_state.native.chrome.corner_radius = radius;
                    window_state.render.frame_dirty = true;
                }
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetExtraSpacing {
                line_spacing,
                letter_spacing,
            } => {
                self.extra_line_spacing = line_spacing;
                self.extra_letter_spacing = letter_spacing;
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetIndentGuideRainbow { enabled, colors } => {
                let linear_colors: Vec<(f32, f32, f32, f32)> = colors
                    .iter()
                    .map(|(r, g, b, a)| {
                        let c = crate::core::types::Color::new(*r, *g, *b, *a).srgb_to_linear();
                        (c.r, c.g, c.b, c.a)
                    })
                    .collect();
                self.effects.indent_guides.rainbow_enabled = enabled;
                self.effects.indent_guides.rainbow_colors = linear_colors.clone();
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.set_indent_guide_rainbow(enabled, linear_colors);
                }
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetCursorSizeTransition {
                enabled,
                duration_ms,
            } => {
                self.cursor_defaults.size_transition_enabled = enabled;
                self.cursor_defaults.size_transition_duration = duration_ms as f32 / 1000.0;
                if !enabled {
                    self.cursor_defaults.size_animating = false;
                    if let Some(cursor) = self.primary_cursor_mut() {
                        cursor.size_animating = false;
                    }
                }
                self.sync_primary_cursor_config_from_defaults();
                for window_state in self.frame_windows.windows.values_mut() {
                    window_state
                        .render
                        .cursor
                        .copy_config_from(&self.cursor_defaults);
                    window_state.render.frame_dirty = true;
                }
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetLigaturesEnabled { enabled } => {
                tracing::info!("Ligatures enabled: {}", enabled);
                Ok(())
            }
            RenderCommand::RemoveChildFrame { frame_id } => {
                tracing::info!("Removing child frame 0x{:x}", frame_id);
                self.primary_child_frames_mut().remove_frame(frame_id);
                if self
                    .primary_cursor()
                    .target_cloned()
                    .is_some_and(|target| target.frame_id == frame_id)
                {
                    if let Some(cursor) = self.primary_cursor_mut() {
                        cursor.clear_target();
                    }
                    self.reset_primary_ime_cursor_area();
                    self.clear_primary_ime_preedit();
                    if let Some(window) = self.primary_window() {
                        window.set_ime_cursor_area(
                            PhysicalPosition::new(0.0, 0.0),
                            PhysicalSize::new(1.0, 1.0),
                        );
                    }
                }
                for window_state in self.frame_windows.windows.values_mut() {
                    let removed = window_state
                        .render
                        .child_frames
                        .frames
                        .contains_key(&frame_id);
                    window_state.render.child_frames.remove_frame(frame_id);
                    if removed {
                        window_state.render.frame_dirty = true;
                    }
                    if window_state
                        .render
                        .cursor
                        .target_cloned()
                        .is_some_and(|target| target.frame_id == frame_id)
                    {
                        window_state.render.cursor.clear_target();
                        window_state.native.last_ime_cursor_area = None;
                        window_state.render.ime_preedit_active = false;
                        window_state.render.ime_preedit_text.clear();
                        window_state.native.window.set_ime_cursor_area(
                            PhysicalPosition::new(0.0, 0.0),
                            PhysicalSize::new(1.0, 1.0),
                        );
                        window_state.render.frame_dirty = true;
                    }
                }
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetChildFrameStyle {
                corner_radius,
                shadow_enabled,
                shadow_layers,
                shadow_offset,
                shadow_opacity,
            } => {
                self.child_frame_corner_radius = corner_radius;
                self.child_frame_shadow_enabled = shadow_enabled;
                self.child_frame_shadow_layers = shadow_layers;
                self.child_frame_shadow_offset = shadow_offset;
                self.child_frame_shadow_opacity = shadow_opacity;
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetToolBar {
                items,
                height,
                fg_r,
                fg_g,
                fg_b,
                bg_r,
                bg_g,
                bg_b,
            } => {
                self.sync_toolbar_visual_config_from_height(height);
                self.ensure_toolbar_icon_textures(&items);
                let tool_bar = GuiToolBarState {
                    items,
                    height,
                    fg: (fg_r, fg_g, fg_b),
                    bg: (bg_r, bg_g, bg_b),
                };
                if let Some(primary_frame) = self.primary_render_state_mut() {
                    primary_frame.tool_bar = Some(tool_bar);
                } else {
                    self.pending_primary_tool_bar = Some(tool_bar);
                }
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetToolBarConfig { icon_size, padding } => {
                self.set_toolbar_visual_config(icon_size, padding);
                self.mark_primary_dirty();
                Ok(())
            }
            RenderCommand::SetMenuBar {
                items,
                height,
                fg_r,
                fg_g,
                fg_b,
                bg_r,
                bg_g,
                bg_b,
            } => {
                tracing::debug!(
                    "SetMenuBar: {} items, height={}, fg=({:.3},{:.3},{:.3}), bg=({:.3},{:.3},{:.3})",
                    items.len(),
                    height,
                    fg_r,
                    fg_g,
                    fg_b,
                    bg_r,
                    bg_g,
                    bg_b
                );
                let menu_bar = GuiMenuBarState {
                    items,
                    height,
                    fg: (fg_r, fg_g, fg_b),
                    bg: (bg_r, bg_g, bg_b),
                };
                if let Some(primary_frame) = self.primary_render_state_mut() {
                    primary_frame.menu_bar = Some(menu_bar);
                } else {
                    self.pending_primary_menu_bar = Some(menu_bar);
                }
                self.mark_primary_dirty();
                Ok(())
            }
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::toolbar_visual_config_for_height;

    #[test]
    fn toolbar_visual_config_matches_gnu_default_geometry() {
        assert_eq!(toolbar_visual_config_for_height(34.0), (24, 5));
    }

    #[test]
    fn toolbar_visual_config_scales_with_frame_pixel_height() {
        assert_eq!(toolbar_visual_config_for_height(68.0), (48, 10));
    }
}
