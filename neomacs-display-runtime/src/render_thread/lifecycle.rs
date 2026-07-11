use super::RenderApp;
use super::frame_windows::{FrameLifecycle, NativeTextInputPolicy};
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
        if !self.lifecycle_flags.resumed_seen {
            tracing::info!(
                "Render thread resumed: primary_window_exists={} size={}x{} title={:?}",
                self.frame_windows
                    .primary_window()
                    .and_then(|ws| ws.window())
                    .is_some(),
                self.frame_windows
                    .primary_window()
                    .map_or((0, 0), |ws| ws.native_size())
                    .0,
                self.frame_windows
                    .primary_window()
                    .map_or((0, 0), |ws| ws.native_size())
                    .1,
                self.frame_windows
                    .primary_window()
                    .expect("primary window state")
                    .chrome()
                    .title
            );
            self.lifecycle_flags.resumed_seen = true;
        }
        let needs_native = self
            .frame_windows
            .primary_window()
            .is_some_and(|ws| !ws.lifecycle.is_active());
        if needs_native {
            let (width, height, title, decorations_enabled) = {
                let primary = self.frame_windows.primary_window().unwrap();
                let (w, h) = primary.lifecycle.native_size();
                let chrome = primary.lifecycle.chrome();
                (w, h, chrome.title.clone(), chrome.decorations_enabled)
            };
            let attrs = Window::default_attributes()
                .with_title(&title)
                .with_inner_size(window_size_from_emacs_pixels(width, height))
                .with_decorations(decorations_enabled)
                .with_transparent(true);
            let attrs = crate::window_identity::apply_platform_window_identity(attrs);

            tracing::info!(
                "Render thread creating primary window: emacs_pixels={}x{} title={:?}",
                width,
                height,
                title
            );
            match event_loop.create_window(attrs) {
                Ok(window) => {
                    let window = Arc::new(window);
                    NativeTextInputPolicy::for_gui_frame().apply_to_window(&window);

                    let raw_scale_factor = window.scale_factor();
                    let effective_scale = effective_window_scale_factor(raw_scale_factor);
                    {
                        let primary = self.frame_windows.primary_window_mut().unwrap();
                        if let FrameLifecycle::Pending { scale_factor, .. } = &mut primary.lifecycle
                        {
                            *scale_factor = effective_scale;
                        }
                    }
                    tracing::info!(
                        "Display scale factor: raw={} effective={}",
                        raw_scale_factor,
                        effective_scale
                    );

                    let phys = window.inner_size();
                    {
                        let primary = self.frame_windows.primary_window_mut().unwrap();
                        if let FrameLifecycle::Pending {
                            width: pw,
                            height: ph,
                            ..
                        } = &mut primary.lifecycle
                        {
                            *pw = phys.width;
                            *ph = phys.height;
                        }
                    }
                    tracing::info!(
                        "Render thread: window created (physical {}x{})",
                        phys.width,
                        phys.height
                    );

                    self.init_wgpu(event_loop, window.clone());

                    if let Some(geometry_hints) = self
                        .frame_windows
                        .primary_window()
                        .unwrap()
                        .lifecycle
                        .geometry_hints()
                    {
                        apply_window_geometry_hints(&window, geometry_hints);
                    }

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
        super::frame_stats::count(&super::frame_stats::EVENT_LOOP_WAKEUPS);
        super::frame_stats::maybe_log_snapshot(std::time::Instant::now());
        if !self.lifecycle_flags.about_to_wait_seen {
            tracing::info!(
                "Render thread entered about_to_wait: primary_window_exists={} frame_windows={}",
                self.frame_windows
                    .primary_window()
                    .and_then(|ws| ws.window())
                    .is_some(),
                self.frame_windows.count()
            );
            self.lifecycle_flags.about_to_wait_seen = true;
        }
        if self.lifecycle_flags.shutdown_requested {
            event_loop.exit();
            return;
        }
        self.refresh_monitor_snapshot(event_loop, true);
        if self.process_commands() {
            event_loop.exit();
            return;
        }

        if let Some(gpu) = &self.gpu {
            self.frame_windows.process_creates(
                event_loop,
                &gpu.instance,
                &gpu.device,
                &gpu.adapter,
            );
        }
        self.frame_windows.process_destroys();

        self.poll_frame();

        self.pump_glib();

        let now = std::time::Instant::now();
        self.frame_windows.tick_top_level_cursor_blinks(
            now,
            self.effects.cursor_wake.enabled,
            self.renderer.as_ref(),
        );

        self.frame_windows.tick_top_level_cursor_animations();

        self.frame_windows.tick_top_level_cursor_size_animations();

        if self.effects.idle_dim.enabled {
            let idle_dim_config = self.effects.idle_dim.clone();
            self.frame_windows.tick_top_level_idle_dim(&idle_dim_config);
        } else {
            self.frame_windows.clear_top_level_idle_dim();
        }

        if self.effects.cursor_pulse.enabled && self.effects.cursor_glow.enabled {
            self.frame_windows.mark_top_level_dirty();
        }

        self.frame_windows.mark_active_top_level_visuals_dirty();

        if self.has_terminal_activity() {
            self.frame_windows.mark_top_level_dirty();
        }

        let has_active_content = self.has_webkit_needing_redraw() || self.has_playing_videos();

        #[cfg(feature = "wpe-webkit")]
        if self.has_webkit_needing_redraw() {
            self.frame_windows
                .for_each_top_level_window_mut(|window_state| {
                    if !window_state.render.floating_webkits.is_empty() {
                        window_state.render.mark_dirty();
                    }
                });
        }

        self.frame_windows
            .request_redraw_for_dirty_top_level_windows();

        let top_level_dirty = self.frame_windows.any_redrawable_top_level_dirty();
        if top_level_dirty || has_active_content {
            tracing::debug!(
                top_level_dirty,
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
            if has_active_content
                && let Some(window) = self
                    .frame_windows
                    .primary_window()
                    .and_then(|ws| ws.window())
            {
                super::frame_stats::count(&super::frame_stats::REDRAW_REQUESTS);
                window.request_redraw();
            }
        }

        // Stage 1 of the frame scheduling plan: the wake decision is
        // expressed as typed frame demand and the coordinator chooses the
        // deadline. Demands are declared afresh each pass from current state,
        // so signal cessation needs no retraction bookkeeping. Stage 2 makes
        // the coordinator persistent and requests one-shot.
        let now = std::time::Instant::now();
        match self.compat_frame_demand_deadline(now, has_active_content) {
            Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }

    /// Stage 1 compatibility adapter: translate the legacy activity signals
    /// into typed [`super::frame_sched::FrameDemand`] and let the frame
    /// coordinator pick the earliest wake deadline. This is the only place
    /// the legacy 4 ms active cadence and 16 ms idle-poll cadence are
    /// selected.
    fn compat_frame_demand_deadline(
        &self,
        now: std::time::Instant,
        has_active_content: bool,
    ) -> Option<std::time::Instant> {
        use super::frame_sched::{
            Cadence, Damage, DemandReason, FrameCoordinator, FrameDemand, Invalidation, LayerMask,
            NativeWindowId,
        };
        const LEGACY_ACTIVE_WAKE: std::time::Duration = std::time::Duration::from_millis(4);
        const LEGACY_IDLE_POLL: std::time::Duration = std::time::Duration::from_millis(16);
        /// Loop-global demands (media polling, idle poll) before/beyond any
        /// specific frame window. Compat-only; deleted with this adapter.
        const LOOP_WINDOW: NativeWindowId = NativeWindowId(u64::MAX);

        let native_window_id = |key: &super::frame_windows::FrameKey| match key {
            super::frame_windows::FrameKey::Pending => NativeWindowId(u64::MAX - 1),
            super::frame_windows::FrameKey::Adopted(id) => NativeWindowId(*id),
        };
        let legacy_repaint = Invalidation::RepaintLayers {
            layers: LayerMask::all(),
            damage: Damage::FullLayer,
        };

        let mut sched = FrameCoordinator::new();
        for (key, window_state) in &self.frame_windows.windows {
            let id = native_window_id(key);
            let legacy_active = window_state.has_presentable_dirty_content()
                || window_state
                    .render
                    .compositor
                    .renderer_effects
                    .needs_redraw()
                || window_state.render.cursor.is_animating()
                || window_state.render.compositor.transitions.has_active();
            if legacy_active {
                sched.submit_demand(
                    id,
                    FrameDemand {
                        invalidation: legacy_repaint,
                        cadence: Cadence::At(now + LEGACY_ACTIVE_WAKE),
                        reason: DemandReason::LegacyActivity,
                    },
                    now,
                );
            }
            if let Some(blink) = window_state.render.cursor.next_blink_deadline() {
                sched.submit_demand(
                    id,
                    FrameDemand {
                        invalidation: Invalidation::CompositeOnly {
                            layers: LayerMask::CURSOR_EFFECTS,
                        },
                        cadence: Cadence::At(blink),
                        reason: DemandReason::CursorAnimation,
                    },
                    now,
                );
            }
        }
        if has_active_content {
            sched.submit_demand(
                LOOP_WINDOW,
                FrameDemand {
                    invalidation: legacy_repaint,
                    cadence: Cadence::At(now + LEGACY_ACTIVE_WAKE),
                    reason: DemandReason::Video,
                },
                now,
            );
        }
        if self.lifecycle_flags.poll_when_idle {
            sched.submit_demand(
                LOOP_WINDOW,
                FrameDemand {
                    invalidation: legacy_repaint,
                    cadence: Cadence::At(now + LEGACY_IDLE_POLL),
                    reason: DemandReason::LegacyActivity,
                },
                now,
            );
        }
        sched.next_wake_deadline()
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
