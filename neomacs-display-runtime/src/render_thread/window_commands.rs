//! Window and chrome render commands.

use super::RenderApp;
use crate::thread_comm::RenderCommand;
use winit::dpi::PhysicalPosition;
use winit::window::{CursorIcon, UserAttentionType};

impl RenderApp {
    pub(super) fn handle_window_command(
        &mut self,
        cmd: RenderCommand,
    ) -> Result<(), RenderCommand> {
        match cmd {
            RenderCommand::SetMouseCursor { cursor_type } => {
                if let Some(window) = self.primary_window() {
                    if cursor_type == 0 {
                        window.set_cursor_visible(false);
                    } else {
                        window.set_cursor_visible(true);
                        let icon = match cursor_type {
                            2 => CursorIcon::Text,
                            3 => CursorIcon::Pointer,
                            4 => CursorIcon::Crosshair,
                            5 => CursorIcon::EwResize,
                            6 => CursorIcon::NsResize,
                            7 => CursorIcon::Wait,
                            8 => CursorIcon::NwseResize,
                            9 => CursorIcon::NeswResize,
                            10 => CursorIcon::NeswResize,
                            11 => CursorIcon::NwseResize,
                            _ => CursorIcon::Default,
                        };
                        window.set_cursor(icon);
                    }
                }
                Ok(())
            }
            RenderCommand::WarpMouse { x, y } => {
                if let Some(window) = self.primary_window() {
                    let pos = PhysicalPosition::new(x as f64, y as f64);
                    let _ = window.set_cursor_position(pos);
                }
                Ok(())
            }
            RenderCommand::SetWindowTitle { title } => {
                self.primary_chrome_mut().title = title.clone();
                if let Some(primary_state) = self.primary_window_state_mut() {
                    primary_state.set_title(title);
                    if !primary_state.chrome().decorations_enabled {
                        primary_state.render.mark_dirty();
                    }
                }
                Ok(())
            }
            RenderCommand::SetFrameWindowTitle {
                emacs_frame_id,
                title,
            } => {
                if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                    window_state.set_title(title);
                } else if self.frame_windows.is_primary_frame_id(emacs_frame_id) {
                    self.primary_chrome_mut().title = title;
                } else {
                    tracing::warn!(
                        "SetFrameWindowTitle requested for unknown frame_id=0x{:x}",
                        emacs_frame_id
                    );
                }
                Ok(())
            }
            RenderCommand::SetWindowFullscreen { mode } => {
                if let Some(primary_state) = self.primary_window_state_mut() {
                    primary_state.set_fullscreen_mode(mode);
                }
                Ok(())
            }
            RenderCommand::SetWindowMinimized { minimized } => {
                if let Some(window) = self.primary_window() {
                    window.set_minimized(minimized);
                }
                Ok(())
            }
            RenderCommand::SetWindowPosition { x, y } => {
                if let Some(window) = self.primary_window() {
                    window.set_outer_position(PhysicalPosition::new(x, y));
                }
                Ok(())
            }
            RenderCommand::SetWindowSize { width, height } => {
                tracing::debug!("RenderCommand::SetWindowSize {}x{}", width, height);
                if let Some(primary_state) = self.primary_window_state_mut() {
                    primary_state.request_inner_size(width, height);
                }
                Ok(())
            }
            RenderCommand::ResizeWindow {
                emacs_frame_id,
                width,
                height,
                geometry_hints,
            } => {
                tracing::debug!(
                    "RenderCommand::ResizeWindow frame_id=0x{:x} {}x{}",
                    emacs_frame_id,
                    width,
                    height
                );
                if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                    window_state.apply_geometry_hints(geometry_hints);
                    window_state.request_inner_size(width, height);
                } else {
                    tracing::warn!(
                        "ResizeWindow requested for unknown frame_id=0x{:x}",
                        emacs_frame_id
                    );
                }
                Ok(())
            }
            RenderCommand::SetFrameGeometryHints {
                emacs_frame_id,
                geometry_hints,
            } => {
                tracing::debug!(
                    "RenderCommand::SetFrameGeometryHints frame_id=0x{:x} base={}x{} inc={}x{}",
                    emacs_frame_id,
                    geometry_hints.base_width,
                    geometry_hints.base_height,
                    geometry_hints.width_inc,
                    geometry_hints.height_inc
                );
                if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                    window_state.apply_geometry_hints(geometry_hints);
                } else {
                    tracing::warn!(
                        "SetFrameGeometryHints requested for unknown frame_id=0x{:x}",
                        emacs_frame_id
                    );
                }
                Ok(())
            }
            RenderCommand::SetWindowDecorated { decorated } => {
                self.primary_chrome_mut().decorations_enabled = decorated;
                self.frame_windows.set_top_level_decorations(decorated);
                Ok(())
            }
            RenderCommand::RequestAttention { urgent } => {
                if let Some(window) = self.primary_window() {
                    let attention = if urgent {
                        Some(UserAttentionType::Critical)
                    } else {
                        Some(UserAttentionType::Informational)
                    };
                    window.request_user_attention(attention);
                }
                Ok(())
            }
            RenderCommand::CreateWindow {
                emacs_frame_id,
                width,
                height,
                title,
                geometry_hints,
            } => {
                tracing::info!(
                    "CreateWindow request: frame_id=0x{:x} {}x{} \"{}\"",
                    emacs_frame_id,
                    width,
                    height,
                    title
                );
                self.frame_windows.request_create(
                    emacs_frame_id,
                    width,
                    height,
                    title,
                    geometry_hints,
                );
                Ok(())
            }
            RenderCommand::DestroyWindow { emacs_frame_id } => {
                tracing::info!("DestroyWindow request: frame_id=0x{:x}", emacs_frame_id);
                if self.frame_windows.is_primary_frame_id(emacs_frame_id) {
                    self.frame_windows.take_primary_window();
                    self.frame_windows.clear_primary_mapping();
                    #[cfg(test)]
                    {
                        self.primary_render_state_for_tests = None;
                    }
                    self.primary_window_destroyed = true;
                } else {
                    self.frame_windows.request_destroy(emacs_frame_id);
                }
                Ok(())
            }
            RenderCommand::AdoptPrimaryFrame { emacs_frame_id } => {
                tracing::info!("AdoptPrimaryFrame request: frame_id=0x{:x}", emacs_frame_id);
                self.frame_windows.adopt_primary_frame_id(emacs_frame_id);
                if let Some(primary_frame) = self.primary_render_state_mut() {
                    primary_frame.set_emacs_frame_id(emacs_frame_id);
                }
                Ok(())
            }
            other => Err(other),
        }
    }
}
