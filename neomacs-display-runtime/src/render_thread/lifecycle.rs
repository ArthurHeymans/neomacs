use super::RenderApp;
use super::state::{
    RenderGpuContext, effective_window_scale_factor, window_size_from_emacs_pixels,
};
use super::x11_hints::apply_window_geometry_hints;
use crate::thread_comm::InputEvent;
use std::sync::Arc;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::Window;

impl RenderApp {
    fn collect_monitor_snapshot(
        event_loop: &ActiveEventLoop,
    ) -> Vec<crate::thread_comm::MonitorInfo> {
        let mut monitors = Vec::new();
        for monitor in event_loop.available_monitors() {
            let pos = monitor.position();
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let name = monitor.name();
            let width_mm = if scale > 0.0 {
                (size.width as f64 * 25.4 / (96.0 * scale)) as i32
            } else {
                0
            };
            let height_mm = if scale > 0.0 {
                (size.height as f64 * 25.4 / (96.0 * scale)) as i32
            } else {
                0
            };
            monitors.push(crate::thread_comm::MonitorInfo {
                x: pos.x,
                y: pos.y,
                width: size.width as i32,
                height: size.height as i32,
                scale,
                width_mm,
                height_mm,
                name,
            });
        }
        monitors
    }

    fn refresh_monitor_snapshot(&mut self, event_loop: &ActiveEventLoop, emit_change_event: bool) {
        let snapshot = Self::collect_monitor_snapshot(event_loop);
        let had_snapshot = self.monitors_populated;
        let changed = !had_snapshot || self.last_monitor_snapshot != snapshot;

        if !changed {
            return;
        }

        self.last_monitor_snapshot = snapshot.clone();
        self.monitors_populated = true;

        for monitor in &snapshot {
            tracing::info!(
                "Monitor: {:?} pos=({},{}) size={}x{} scale={} mm={}x{}",
                monitor.name,
                monitor.x,
                monitor.y,
                monitor.width,
                monitor.height,
                monitor.scale,
                monitor.width_mm,
                monitor.height_mm
            );
        }

        if let Some(ref shared) = self.shared_monitors {
            let (ref lock, ref cvar) = **shared;
            if let Ok(mut shared) = lock.lock() {
                *shared = snapshot.clone();
                cvar.notify_all();
            }
        }

        if emit_change_event && had_snapshot {
            self.comms
                .send_input(InputEvent::MonitorsChanged { monitors: snapshot });
        }
    }

    pub(super) fn handle_resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.resumed_seen {
            tracing::info!(
                "Render thread resumed: primary_window_exists={} size={}x{} title={:?}",
                self.primary_window().is_some(),
                self.primary_native_fallback.width,
                self.primary_native_fallback.height,
                self.primary_native_fallback.chrome.title
            );
            self.resumed_seen = true;
        }
        if self.primary_window().is_none() && !self.primary_window_destroyed {
            let attrs = Window::default_attributes()
                .with_title(&self.primary_native_fallback.chrome.title)
                .with_inner_size(window_size_from_emacs_pixels(
                    self.primary_native_fallback.width,
                    self.primary_native_fallback.height,
                ))
                .with_decorations(self.primary_native_fallback.chrome.decorations_enabled)
                .with_transparent(true);

            tracing::info!(
                "Render thread creating primary window: emacs_pixels={}x{} title={:?}",
                self.primary_native_fallback.width,
                self.primary_native_fallback.height,
                self.primary_native_fallback.chrome.title
            );
            match event_loop.create_window(attrs) {
                Ok(window) => {
                    let window = Arc::new(window);

                    let raw_scale_factor = window.scale_factor();
                    self.primary_native_fallback.scale_factor =
                        effective_window_scale_factor(raw_scale_factor);
                    tracing::info!(
                        "Display scale factor: raw={} effective={}",
                        raw_scale_factor,
                        self.primary_native_fallback.scale_factor
                    );

                    let phys = window.inner_size();
                    self.primary_native_fallback.width = phys.width;
                    self.primary_native_fallback.height = phys.height;
                    tracing::info!(
                        "Render thread: window created (physical {}x{})",
                        self.primary_native_fallback.width,
                        self.primary_native_fallback.height
                    );

                    // Initialize wgpu with the window
                    self.init_wgpu(event_loop, window.clone());
                    self.frame_windows.adopt_primary_winit_id(window.id());

                    // Enable IME input for CJK and compose support
                    window.set_ime_allowed(true);
                    if let Some(geometry_hints) = self.primary_native_fallback.geometry_hints {
                        apply_window_geometry_hints(&window, geometry_hints);
                    }

                    // Set window icon from project SVG.
                    crate::window_icon::apply_window_icon(&window);
                }
                Err(e) => {
                    tracing::error!("Failed to create window: {:?}", e);
                }
            }
        }

        self.refresh_monitor_snapshot(event_loop, false);
    }

    pub(super) fn handle_about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.about_to_wait_seen {
            tracing::info!(
                "Render thread entered about_to_wait: primary_window_exists={} frame_windows={}",
                self.primary_window().is_some(),
                self.frame_windows.count()
            );
            self.about_to_wait_seen = true;
        }
        self.refresh_monitor_snapshot(event_loop, true);
        // Check for shutdown
        if self.process_commands() {
            event_loop.exit();
            return;
        }

        // Process multi-window creates/destroys
        if let Some(gpu) = &self.gpu {
            self.frame_windows.process_creates(
                event_loop,
                &gpu.instance,
                &gpu.device,
                &gpu.adapter,
            );
        }
        self.frame_windows.process_destroys();

        // Get latest frame from Emacs
        self.poll_frame();

        // Pump GLib for WebKit
        self.pump_glib();

        // Update cursor blink state
        let now = std::time::Instant::now();
        self.frame_windows.tick_top_level_cursor_blinks(
            now,
            self.effects.cursor_wake.enabled,
            self.renderer.as_ref(),
        );
        if self.primary_window_state().is_none() {
            self.tick_cursor_blink();
        }

        // Tick cursor animation
        self.frame_windows.tick_top_level_cursor_animations();
        self.with_unmanaged_primary_render(|r| {
            r.tick_cursor_animation();
        });

        // Tick cursor size transition (runs after position animation, overrides w/h)
        self.frame_windows.tick_top_level_cursor_size_animations();
        self.with_unmanaged_primary_render(|r| {
            r.tick_cursor_size_animation();
        });

        if self.effects.idle_dim.enabled {
            let idle_dim_config = self.effects.idle_dim.clone();
            self.frame_windows.tick_top_level_idle_dim(&idle_dim_config);
            self.with_unmanaged_primary_render(|r| {
                r.tick_idle_dim(&idle_dim_config);
            });
        } else {
            self.frame_windows.clear_top_level_idle_dim();
            self.with_unmanaged_primary_render(|r| r.clear_idle_dim());
        }

        // Keep dirty if cursor pulse is active (needs continuous redraw)
        if self.effects.cursor_pulse.enabled && self.effects.cursor_glow.enabled {
            self.mark_top_level_frame_windows_dirty();
        }

        // Keep dirty if frame-owned renderer effects or transitions are active.
        self.frame_windows.mark_active_top_level_visuals_dirty();
        self.with_unmanaged_primary_render(|r| {
            r.mark_active_visuals_dirty();
        });

        // Check for terminal PTY activity
        if self.has_terminal_activity() {
            self.mark_top_level_frame_windows_dirty();
        }

        // Determine if continuous rendering is needed
        let has_active_content = self.has_webkit_needing_redraw() || self.has_playing_videos();

        #[cfg(feature = "wpe-webkit")]
        if self.has_webkit_needing_redraw() {
            self.frame_windows
                .for_each_top_level_window_mut(|window_state| {
                    if !window_state.render.floating_webkits.is_empty() {
                        window_state.render.frame_dirty = true;
                    }
                });
        }

        // Request redraw when we have new frame data, cursor blink toggled,
        // or webkit/video content changed.
        self.frame_windows
            .request_redraw_for_dirty_top_level_windows();

        let top_level_dirty = self.frame_windows.any_top_level_dirty();
        let primary_fallback_dirty = self.primary_window_state().is_none() && self.primary_dirty();
        if top_level_dirty || primary_fallback_dirty || has_active_content {
            tracing::debug!(
                top_level_dirty,
                primary_fallback_dirty,
                has_active_content,
                renderer_effects_need_redraw = self
                    .frame_windows
                    .any_top_level_renderer_effects_need_redraw(),
                cursor_animating = self.frame_windows.any_top_level_cursor_animating(),
                idle_dim_active = self.frame_windows.any_top_level_idle_dim_active(),
                transitions_active = self.frame_windows.any_top_level_transitions_active(),
                "requesting redraw"
            );
            if has_active_content {
                self.frame_windows.request_redraw_for_top_level_windows();
            }
            if primary_fallback_dirty || has_active_content {
                if let Some(window) = self.primary_window() {
                    window.request_redraw();
                }
            }
        }

        // Use WaitUntil with smart timeouts instead of Poll to save CPU.
        // Window events (key, mouse, resize) still wake immediately.
        let now = std::time::Instant::now();
        let top_level_active = self.frame_windows.any_top_level_dirty()
            || self.frame_windows.any_top_level_cursor_animating()
            || self
                .frame_windows
                .any_top_level_renderer_effects_need_redraw()
            || self.frame_windows.any_top_level_transitions_active();
        let primary_fallback_active = self.primary_window_state().is_none()
            && (self.primary_dirty()
                || self.primary_cursor().is_animating()
                || self.primary_renderer_effects_need_redraw()
                || self.primary_transitions_active());
        let next_wake = if top_level_active || primary_fallback_active || has_active_content {
            // Active rendering: cap at ~240fps to avoid spinning
            now + std::time::Duration::from_millis(4)
        } else if let Some(next_blink) = self.next_cursor_blink_deadline() {
            // Idle with cursor blink: wake at next toggle time
            next_blink
        } else if self.poll_when_idle {
            // Compatibility mode for legacy RenderThread callers that do not
            // own a winit EventLoopProxy and therefore cannot wake this loop
            // when they enqueue frames or commands.
            now + std::time::Duration::from_millis(16)
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };
        event_loop.set_control_flow(ControlFlow::WaitUntil(next_wake));
    }
    pub(super) fn handle_exiting(&mut self) {
        // Explicitly drop wgpu resources while the Wayland connection is still alive.
        // Without this, RenderApp's implicit drop happens AFTER the event loop's
        // Wayland display is torn down, causing SEGV in eglTerminate → dri2_teardown_wayland.
        //
        // wgpu uses internal Arc reference counting: the Adapter holds Arc<Instance>,
        // and Device/Surface/Texture objects hold indirect Arc references back to it.
        // Even after .take()'ing all Option fields, other RenderApp fields (transition
        // textures, child frames, etc.) may still hold transitive Arc references that
        // keep the EGL Instance alive until the final implicit drop of RenderApp —
        // at which point the Wayland connection is already torn down.
        //
        // Solution: leak the adapter to prevent eglTerminate from ever running.
        // The OS reclaims all GPU resources on process exit anyway.
        tracing::info!("Event loop exiting, cleaning up GPU resources");

        // Drop WebKit views and WPE backend (hold EGL contexts)
        #[cfg(feature = "wpe-webkit")]
        {
            self.webkit_views.clear();
            self.wpe_backend = None;
        }
        // Drop renderer (holds device/queue references, textures, pipelines)
        drop(self.renderer.take());
        // Drop adopted primary state (surface holds wl_surface proxy if on Wayland)
        drop(self.frame_windows.take_primary_window());
        #[cfg(test)]
        drop(self.primary_render_state_for_tests.take());
        // Drop multi-window state (secondary surfaces)
        self.frame_windows.destroy_all();
        // Leak the adapter to prevent eglTerminate crash on Wayland.
        // The adapter's Drop triggers eglTerminate → dri2_teardown_wayland which
        // SEGVs if the Wayland connection is already gone. Since we're exiting,
        // the OS will reclaim all GPU/EGL resources.
        if let Some(gpu) = self.gpu.take() {
            let RenderGpuContext {
                instance,
                adapter,
                device,
                queue,
            } = gpu;
            drop(device);
            drop(queue);
            drop(instance);
            std::mem::forget(adapter);
        }

        tracing::info!("GPU resources cleaned up");
    }
}
