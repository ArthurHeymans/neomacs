//! UI overlay, animation, and effect render commands.

use super::{PopupMenuState, RenderApp, TooltipState};
use crate::thread_comm::{ConfigCommand, ToolBarItem, UiCommand};
use neomacs_display_protocol::ToolBarImageSource;

impl RenderApp {
    pub(super) fn ensure_toolbar_icon_textures(&mut self, items: &[ToolBarItem], icon_size: u32) {
        for item in items {
            if item.is_separator() {
                continue;
            }
            let Some(image) = item.image.as_ref() else {
                continue;
            };
            let key = (image.clone(), icon_size);
            if self.toolbar.icon_textures.contains_key(&key) {
                continue;
            }
            let Some(renderer) = self.renderer.as_mut() else {
                continue;
            };

            let id = match image {
                ToolBarImageSource::File { path } => {
                    renderer.load_image_file(path, icon_size, icon_size, 0, 0)
                }
            };
            self.toolbar.icon_textures.insert(key, id);
            tracing::debug!(
                "Loaded toolbar image '{}' as image_id={}",
                image.cache_key(),
                id
            );
        }
    }

    pub(super) fn handle_ui(&mut self, cmd: UiCommand) {
        match cmd {
            UiCommand::ShowPopupMenu {
                frame,
                x,
                y,
                items,
                title,
                fg,
                bg,
            } => {
                let emacs_frame_id = frame.raw_id();
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
                        self.frame_windows
                            .primary_window()
                            .map(|ws| &ws.render)
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
                    if let Some(ws) = self.frame_windows.primary_window_mut() {
                        ws.render.set_popup_menu(Some(menu))
                    };
                } else {
                    tracing::warn!(
                        "ShowPopupMenu requested for unknown frame_id=0x{:x}",
                        emacs_frame_id
                    );
                }
            }
            UiCommand::HidePopupMenu => {
                tracing::info!("HidePopupMenu");
                self.frame_windows.hide_top_level_popup_menus();
                if let Some(ws) = self.frame_windows.primary_window_mut() {
                    ws.render
                        .with_chrome_interaction_mut(|chrome| chrome.menu_bar_active = None)
                } else {
                    false
                };
            }
            UiCommand::ShowTooltip {
                frame,
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
                let emacs_frame_id = frame.raw_id();
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
                            .frame_windows
                            .primary_window()
                            .map(|ws| &ws.render)
                            .map(|primary_frame| primary_frame.font_metrics())
                            .unwrap_or((13.0, 17.0, 13.0 * 0.6));
                        let (screen_w, screen_h) =
                            self.frame_windows
                                .primary_window()
                                .map_or((0.0, 0.0), |ws| {
                                    let (w, h) = ws.native_size();
                                    let s = ws.scale_factor() as f32;
                                    (w as f32 / s, h as f32 / s)
                                });
                        Some((fs, lh, cw, screen_w, screen_h))
                    })
                    .unwrap_or_else(|| {
                        let (screen_w, screen_h) =
                            self.frame_windows
                                .primary_window()
                                .map_or((0.0, 0.0), |ws| {
                                    let (w, h) = ws.native_size();
                                    let s = ws.scale_factor() as f32;
                                    (w as f32 / s, h as f32 / s)
                                });
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
                    if let Some(ws) = self.frame_windows.primary_window_mut() {
                        ws.render.set_tooltip(Some(tooltip))
                    };
                } else {
                    tracing::warn!(
                        "ShowTooltip requested for unknown frame_id=0x{:x}",
                        emacs_frame_id
                    );
                }
            }
            UiCommand::HideTooltip => {
                tracing::debug!("HideTooltip");
                self.frame_windows.hide_top_level_tooltips();
            }
            UiCommand::VisualBell { frame } => {
                let emacs_frame_id = frame.raw_id();
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
                    if let Some(ws) = self.frame_windows.primary_window_mut() {
                        ws.render.set_visual_bell_start(Some(now))
                    };
                } else {
                    tracing::warn!(
                        "VisualBell requested for unknown frame_id=0x{:x}",
                        emacs_frame_id
                    );
                }
            }
        }
    }

    pub(super) fn handle_config(&mut self, cmd: ConfigCommand) {
        match cmd {
            ConfigCommand::SetCursorBlink {
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
                    if let Some(primary_frame) = self
                        .frame_windows
                        .primary_window_mut()
                        .map(|ws| &mut ws.render)
                    {
                        primary_frame.force_cursor_blink_on();
                    }
                }
                self.frame_windows
                    .sync_top_level_cursor_config(&self.cursor_defaults, false);
                if !enabled {
                    self.frame_windows.force_top_level_cursor_blink_on();
                }
            }
            ConfigCommand::SetCursorAnimation { enabled, speed } => {
                tracing::debug!("Cursor animation: enabled={}, speed={}", enabled, speed);
                self.cursor_defaults.anim_enabled = enabled;
                self.cursor_defaults.anim_speed = speed;
                if !enabled {
                    self.cursor_defaults.animating = false;
                }
                self.frame_windows
                    .sync_top_level_cursor_config(&self.cursor_defaults, true);
            }
            ConfigCommand::SetAnimationConfig {
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
                self.frame_windows.mark_top_level_dirty();
                if !cursor_enabled {
                    self.cursor_defaults.animating = false;
                }
                self.frame_windows
                    .sync_top_level_cursor_config(&self.cursor_defaults, true);
                self.frame_windows
                    .sync_top_level_transition_policy(self.transition_policy);
                if !self.transition_policy.crossfade_enabled {
                    self.frame_windows.clear_top_level_crossfade_transitions();
                }
                if !self.transition_policy.scroll_enabled {
                    self.frame_windows.clear_top_level_scroll_transitions();
                }
            }
            ConfigCommand::SetCursorSizeTransition {
                enabled,
                duration_ms,
            } => {
                self.cursor_defaults.size_transition_enabled = enabled;
                self.cursor_defaults.size_transition_duration = duration_ms as f32 / 1000.0;
                if !enabled {
                    self.cursor_defaults.size_animating = false;
                }
                self.frame_windows
                    .sync_top_level_cursor_config(&self.cursor_defaults, true);
                self.frame_windows.mark_top_level_dirty();
            }
            ConfigCommand::SetLigaturesEnabled { enabled } => {
                tracing::info!("Ligatures enabled: {}", enabled);
            }
            ConfigCommand::UpdateEffect(updater) => {
                (updater.0)(&mut self.effects);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.effects = self.effects.clone();
                }
                self.frame_windows.mark_top_level_dirty();
            }
            ConfigCommand::SetCursorEffect(command) => {
                command.apply_to(&mut self.effects);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.effects = self.effects.clone();
                }
                self.frame_windows.mark_top_level_dirty();
            }
            ConfigCommand::SetScrollIndicators { enabled } => {
                self.scroll_indicators_enabled = enabled;
                self.frame_windows.mark_top_level_dirty();
            }
            ConfigCommand::SetTitlebarHeight { height } => {
                self.frame_windows.set_top_level_titlebar_height(height);
            }
            ConfigCommand::SetShowFps { enabled } => {
                self.frame_windows.set_top_level_fps_enabled(enabled);
            }
            ConfigCommand::SetCornerRadius { radius } => {
                self.frame_windows.set_top_level_corner_radius(radius);
            }
            ConfigCommand::SetExtraSpacing {
                line_spacing,
                letter_spacing,
            } => {
                self.extra_line_spacing = line_spacing;
                self.extra_letter_spacing = letter_spacing;
                self.frame_windows.mark_top_level_dirty();
            }
            ConfigCommand::SetIndentGuideRainbow { enabled, colors } => {
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
                self.frame_windows.mark_top_level_dirty();
            }
            ConfigCommand::SetChildFrameStyle {
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
                self.frame_windows.mark_top_level_dirty();
            }
        }
    }
}
