//! UI overlay, animation, and effect render commands.

use super::{PopupMenuState, RenderApp, TooltipState};
use crate::thread_comm::{RenderCommand, ToolBarItem};
use neomacs_display_protocol::glyph_matrix::{GuiMenuBarState, GuiToolBarState};

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
    pub(super) fn set_toolbar_visual_config(&mut self, icon_size: u32, padding: u32) {
        if self.toolbar.icon_size == icon_size && self.toolbar.padding == padding {
            return;
        }
        self.toolbar.icon_size = icon_size;
        self.toolbar.padding = padding;
        for (_name, id) in self.toolbar.icon_textures.drain() {
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
                && !self.toolbar.icon_textures.contains_key(&item.icon_name)
                && let Some(svg_data) =
                    crate::backend::wgpu::toolbar_icons::get_icon_svg(&item.icon_name)
                && let Some(renderer) = self.renderer.as_mut()
            {
                let icon_size = self.toolbar.icon_size;
                let id = renderer.load_image_data(svg_data, icon_size, icon_size, 0, 0);
                self.toolbar
                    .icon_textures
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
                    if let Some(primary_frame) = self.primary_render_state_mut() {
                        primary_frame.force_cursor_blink_on();
                    }
                }
                self.sync_top_level_cursor_config_from_defaults_without_dirty();
                if !enabled {
                    self.frame_windows.force_top_level_cursor_blink_on();
                }
                Ok(())
            }
            RenderCommand::SetCursorAnimation { enabled, speed } => {
                tracing::debug!("Cursor animation: enabled={}, speed={}", enabled, speed);
                self.cursor_defaults.anim_enabled = enabled;
                self.cursor_defaults.anim_speed = speed;
                if !enabled {
                    self.cursor_defaults.animating = false;
                }
                self.sync_top_level_cursor_config_from_defaults();
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
                self.mark_top_level_frame_windows_dirty();
                if !cursor_enabled {
                    self.cursor_defaults.animating = false;
                }
                self.sync_top_level_cursor_config_from_defaults();
                self.frame_windows
                    .sync_top_level_transition_policy(self.transition_policy);
                if !self.transition_policy.crossfade_enabled {
                    self.clear_top_level_crossfade_transitions();
                }
                if !self.transition_policy.scroll_enabled {
                    self.clear_top_level_scroll_transitions();
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
                let (fs, lh, cw) = self
                    .frame_windows
                    .get(emacs_frame_id)
                    .map(|window_state| window_state.render.font_metrics())
                    .or_else(|| {
                        self.primary_render_state()
                            .map(|primary_frame| primary_frame.font_metrics())
                            .filter(|_| self.frame_windows.is_primary_frame_id(emacs_frame_id))
                    })
                    .unwrap_or((13.0, 17.0, 13.0 * 0.6));
                let mut menu = PopupMenuState::new(x, y, items, title, fs, lh, cw);
                menu.face_fg = fg;
                menu.face_bg = bg;
                if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                    window_state.render.set_popup_menu(Some(menu));
                } else if self.frame_windows.is_primary_frame_id(emacs_frame_id) {
                    self.set_primary_popup_menu(Some(menu));
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
                self.hide_top_level_popup_menus();
                self.with_primary_chrome_interaction_mut(|chrome| chrome.menu_bar_active = None);
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
                let (fs, lh, cw, screen_w, screen_h) = self
                    .frame_windows
                    .get(emacs_frame_id)
                    .map(|window_state| {
                        let (fs, lh, cw) = window_state.render.font_metrics();
                        let (screen_w, screen_h) = window_state.native_size();
                        let scale = window_state.scale_factor() as f32;
                        (fs, lh, cw, screen_w as f32 / scale, screen_h as f32 / scale)
                    })
                    .or_else(|| {
                        if !self.frame_windows.is_primary_frame_id(emacs_frame_id) {
                            return None;
                        }
                        let (fs, lh, cw) = self
                            .primary_render_state()
                            .map(|primary_frame| primary_frame.font_metrics())
                            .unwrap_or((13.0, 17.0, 13.0 * 0.6));
                        let (screen_w, screen_h) = self.primary_logical_size();
                        Some((fs, lh, cw, screen_w, screen_h))
                    })
                    .unwrap_or_else(|| {
                        let (screen_w, screen_h) = self.primary_logical_size();
                        (13.0, 17.0, 13.0 * 0.6, screen_w, screen_h)
                    });
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
                if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                    window_state.render.set_tooltip(Some(tooltip));
                } else if self.frame_windows.is_primary_frame_id(emacs_frame_id) {
                    self.set_primary_tooltip(Some(tooltip));
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
                self.hide_top_level_tooltips();
                Ok(())
            }
            RenderCommand::VisualBell { emacs_frame_id } => {
                let now = std::time::Instant::now();
                let cursor_error_pulse_enabled = self.effects.cursor_error_pulse.enabled;
                let edge_snap_enabled = self.effects.edge_snap.enabled;
                let edge_snap_duration_ms = self.effects.edge_snap.duration_ms;
                if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                    window_state.render.trigger_visual_bell(
                        cursor_error_pulse_enabled,
                        edge_snap_enabled,
                        edge_snap_duration_ms,
                        now,
                    );
                } else if self.frame_windows.is_primary_frame_id(emacs_frame_id) {
                    self.set_primary_visual_bell_start(Some(now));
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
                self.mark_top_level_frame_windows_dirty();
                Ok(())
            }
            RenderCommand::SetCursorEffect(command) => {
                command.apply_to(&mut self.effects);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.effects = self.effects.clone();
                }
                self.mark_top_level_frame_windows_dirty();
                Ok(())
            }
            RenderCommand::SetScrollIndicators { enabled } => {
                self.scroll_indicators_enabled = enabled;
                self.mark_top_level_frame_windows_dirty();
                Ok(())
            }
            RenderCommand::SetTitlebarHeight { height } => {
                self.set_top_level_titlebar_height(height);
                Ok(())
            }
            RenderCommand::SetShowFps { enabled } => {
                self.set_top_level_fps_enabled(enabled);
                Ok(())
            }
            RenderCommand::SetCornerRadius { radius } => {
                self.set_top_level_corner_radius(radius);
                Ok(())
            }
            RenderCommand::SetExtraSpacing {
                line_spacing,
                letter_spacing,
            } => {
                self.extra_line_spacing = line_spacing;
                self.extra_letter_spacing = letter_spacing;
                self.mark_top_level_frame_windows_dirty();
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
                self.mark_top_level_frame_windows_dirty();
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
                }
                self.sync_top_level_cursor_config_from_defaults();
                self.mark_top_level_frame_windows_dirty();
                Ok(())
            }
            RenderCommand::SetLigaturesEnabled { enabled } => {
                tracing::info!("Ligatures enabled: {}", enabled);
                Ok(())
            }
            RenderCommand::RemoveChildFrame { frame_id } => {
                tracing::info!("Removing child frame 0x{:x}", frame_id);
                self.frame_windows
                    .remove_child_frame_from_top_level_windows(frame_id);
                self.remove_primary_child_frame(frame_id);
                Ok(())
            }
            RenderCommand::SetChildFrameStyle {
                corner_radius,
                shadow_enabled,
                shadow_layers,
                shadow_offset,
                shadow_opacity,
            } => {
                self.child_frame_style.corner_radius = corner_radius;
                self.child_frame_style.shadow_enabled = shadow_enabled;
                self.child_frame_style.shadow_layers = shadow_layers;
                self.child_frame_style.shadow_offset = shadow_offset;
                self.child_frame_style.shadow_opacity = shadow_opacity;
                self.mark_top_level_frame_windows_dirty();
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
                self.set_primary_tool_bar(Some(tool_bar));
                Ok(())
            }
            RenderCommand::SetToolBarConfig { icon_size, padding } => {
                self.set_toolbar_visual_config(icon_size, padding);
                self.mark_top_level_frame_windows_dirty();
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
                self.set_primary_menu_bar(Some(menu_bar));
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
