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

                    if self.clipboard.is_err() {
                        self.clipboard = crate::clipboard::ClipboardService::for_display(
                            event_loop.owned_display_handle(),
                        );
                        if let Err(err) = &self.clipboard {
                            tracing::error!("Failed to initialize clipboard service: {err}");
                        }
                    }

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
        // Device-loss recovery (SHADER_SURFACES.md: user shader hang → TDR):
        // latched by the wgpu device-lost callback, by a streak of
        // consecutive surface-Lost acquisitions, or by the debug simulation
        // command. Rebuild the whole GPU stack before doing anything else
        // with it.
        if self.device_lost.take() {
            self.recover_from_device_loss(event_loop);
        }
        self.refresh_monitor_snapshot(event_loop, true);
        if self.process_commands() {
            event_loop.exit();
            return;
        }

        // Decoder workers cannot wake winit directly. Poll their result channel
        // while work is pending so decoded image metadata and pixels become visible.
        self.process_pending_images();

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

        #[cfg(feature = "wpe-webkit")]
        if self.has_webkit_needing_redraw() {
            self.frame_windows
                .for_each_top_level_window_mut(|window_state| {
                    if !window_state.render.floating_webkits.is_empty() {
                        window_state.render.mark_dirty();
                    }
                });
        }

        // Stage 2 of the frame scheduling plan: legacy activity latches are
        // reconciled into the persistent frame coordinator, which owns
        // one-shot redraw requests (coalesced per window) and the loop's
        // wake deadline. Continuous activity is paced at the estimated
        // display cadence instead of a 4 ms poll; new-content demand fires
        // immediately on its first frame after idle.
        let now = std::time::Instant::now();
        self.declare_frame_demands(now);
        let mut deadline = self.frame_coordinator.next_wake_deadline();

        // GLib service wake (frame scheduling plan, invariant 1 carve-out):
        // WPE WebKit needs its thread-default GMainContext pumped for IPC,
        // networking, and JS timers even when no frame is needed. While any
        // WebKit view is alive, cap the wake at a bounded service interval so
        // pump_glib runs regularly; this is a wake, not frame demand — it
        // renders nothing unless separate demand exists. With no WebKit view
        // there is no service wake and the loop may Wait indefinitely.
        if self.has_live_webkit_views() {
            const WPE_SERVICE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(16);
            let service = now + WPE_SERVICE_INTERVAL;
            deadline = Some(deadline.map_or(service, |d| d.min(service)));
        }
        if self.has_pending_images() {
            const IMAGE_DECODE_POLL_INTERVAL: std::time::Duration =
                std::time::Duration::from_millis(16);
            let image_poll = now + IMAGE_DECODE_POLL_INTERVAL;
            deadline = Some(deadline.map_or(image_poll, |d| d.min(image_poll)));
        }

        match deadline {
            Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }

    /// Whether any live WPE WebKit view exists, requiring its GMainContext to
    /// be serviced. Always false in builds without the `wpe-webkit` feature,
    /// where `pump_glib` is a no-op and the loop can Wait indefinitely.
    fn has_live_webkit_views(&self) -> bool {
        #[cfg(feature = "wpe-webkit")]
        {
            if !self.webkit_views.is_empty() {
                return true;
            }
            let mut any = false;
            self.frame_windows
                .for_each_top_level_window(|window_state| {
                    any |= !window_state.render.floating_webkits.is_empty();
                });
            any
        }
        #[cfg(not(feature = "wpe-webkit"))]
        {
            false
        }
    }

    /// Reconcile the legacy activity latches into the persistent frame
    /// coordinator: declare demand for active signals, retract reasons whose
    /// signals ceased, and execute the coordinator's one-shot redraw
    /// requests. Continuous demand is paced at the estimated display cadence
    /// (the plan's bounded synthetic clock); this replaced the legacy 4 ms
    /// active poll.
    fn declare_frame_demands(&mut self, now: std::time::Instant) {
        use super::frame_sched::{
            Cadence, Damage, DemandReason, FrameDemand, Invalidation, LayerMask, NativeWindowId,
            PacingAction,
        };
        const LEGACY_IDLE_POLL: std::time::Duration = std::time::Duration::from_millis(16);
        /// Loop-global demands (idle poll) with no specific frame window.
        /// Compat-only; deleted when poll_when_idle's owners migrate.
        const LOOP_WINDOW: NativeWindowId = NativeWindowId(u64::MAX);

        // The coordinator is keyed by the event frame id: 0 for the primary
        // window before Emacs adopts it, the Emacs frame id afterwards
        // (matching RedrawRequested dispatch).
        let native_window_id = |key: &super::frame_windows::FrameKey| match key {
            super::frame_windows::FrameKey::Pending => NativeWindowId(0),
            super::frame_windows::FrameKey::Adopted(id) => NativeWindowId(*id),
        };
        let legacy_repaint = Invalidation::RepaintLayers {
            layers: LayerMask::all(),
            damage: Damage::FullLayer,
        };

        // Media signals are process-wide; they demand frames on every
        // top-level window (matching the legacy request targeting).
        let webkit_active = self.has_webkit_needing_redraw();
        let videos_active = self.has_playing_videos();
        let surfaces_active = self.has_active_shader_surfaces();
        // Stage 3 tracer bullet: the cursor color cycle is explicit infinite
        // compositor-only demand, not a render-time latch.
        let cursor_cycle_enabled = self.effects.cursor_color_cycle.enabled;

        // Destroyed windows must not keep waking the loop.
        let live: std::collections::HashSet<NativeWindowId> = self
            .frame_windows
            .windows
            .keys()
            .map(&native_window_id)
            .collect();
        self.frame_coordinator
            .prune_windows(|id| id == LOOP_WINDOW || live.contains(&id));

        for (key, window_state) in &self.frame_windows.windows {
            let id = native_window_id(key);
            if window_state.window().is_none() {
                // A window with no native surface cannot render; drop any
                // scheduling state so a stale outstanding-request token or
                // deadline cannot survive to its activation.
                self.frame_coordinator.remove_window(id);
                continue;
            }
            let max_rate = Self::window_max_rate(window_state);

            // Stage 6: each active render-effect family (and dirty content,
            // cursor animation, and transitions) submits its own typed demand
            // so diagnostics can attribute the demand, rather than one opaque
            // catch-all. All use the same repaint invalidation and cadence, so
            // aggregate behavior is unchanged; only the reason differs.
            let fx = &window_state.render.compositor.renderer_effects;
            let effect_demands = [
                (
                    window_state.has_presentable_dirty_content(),
                    DemandReason::Redisplay,
                ),
                (fx.cursor_effects_active(), DemandReason::CursorEffect),
                (fx.window_effects_active(), DemandReason::WindowEffect),
                (fx.text_effects_active(), DemandReason::TextEffect),
                (fx.scroll_effects_active(), DemandReason::ScrollEffect),
                (
                    fx.decorative_effects_active(),
                    DemandReason::DecorativeEffect,
                ),
                (fx.transient_effects_active(), DemandReason::TransientEffect),
                (
                    window_state.render.cursor.is_animating(),
                    DemandReason::CursorAnimation,
                ),
                (
                    window_state.render.compositor.transitions.has_active(),
                    DemandReason::Transition,
                ),
            ];
            let mut action = PacingAction::Sleep;
            for (active, reason) in effect_demands {
                if active {
                    let a = self.frame_coordinator.submit_demand(
                        id,
                        FrameDemand {
                            invalidation: legacy_repaint,
                            cadence: Cadence::MaxRate(max_rate),
                            reason,
                        },
                        now,
                    );
                    if a == PacingAction::RequestRedraw {
                        action = PacingAction::RequestRedraw;
                    }
                } else {
                    self.frame_coordinator.retract(id, reason);
                }
            }

            // Shader surfaces may cap their animation rate (`:fps`): when they
            // are the demand, throttle the compositor cadence to the max of
            // their caps so an ambient background shader lets the frame idle
            // instead of pinning it at display refresh (battery). WebKit and
            // video have no such cap and stay at full rate.
            let surface_rate = if surfaces_active {
                let capped = self.shader_surface_demand_rate(u32::from(max_rate.get()));
                std::num::NonZeroU16::new(capped.min(u32::from(max_rate.get())) as u16)
                    .unwrap_or(max_rate)
            } else {
                max_rate
            };

            for (active, reason, rate) in [
                (webkit_active, DemandReason::WebKit, max_rate),
                (videos_active, DemandReason::Video, max_rate),
                (surfaces_active, DemandReason::ShaderSurface, surface_rate),
            ] {
                if active {
                    let media_action = self.frame_coordinator.submit_demand(
                        id,
                        FrameDemand {
                            invalidation: legacy_repaint,
                            cadence: Cadence::MaxRate(rate),
                            reason,
                        },
                        now,
                    );
                    if media_action == PacingAction::RequestRedraw {
                        action = PacingAction::RequestRedraw;
                    }
                } else {
                    self.frame_coordinator.retract(id, reason);
                }
            }

            // Infinite ambient effect: cursor color cycle animates whenever a
            // cursor exists in a committed frame. Compositor-only demand at
            // display cadence; the draw path no longer latches continuation.
            // Policy (Stage 7): an unfocused window pauses the ambient cycle —
            // there is no reason to keep cycling the cursor color at display
            // cadence on a window the user is not looking at.
            let cursor_cycle_active = cursor_cycle_enabled
                && self.frame_coordinator.is_focused(id)
                && window_state.render.cursor.target.is_some()
                && window_state.render.compositor.current_frame.is_some();
            if cursor_cycle_active {
                let cycle_action = self.frame_coordinator.submit_demand(
                    id,
                    FrameDemand {
                        invalidation: Invalidation::CompositeOnly {
                            layers: LayerMask::CURSOR_EFFECTS,
                        },
                        cadence: Cadence::MaxRate(max_rate),
                        reason: DemandReason::CursorColorCycle,
                    },
                    now,
                );
                if cycle_action == PacingAction::RequestRedraw {
                    action = PacingAction::RequestRedraw;
                }
            } else {
                self.frame_coordinator
                    .retract(id, DemandReason::CursorColorCycle);
            }

            // A blink toggle that already happened needs its frame now. It
            // changes only the cursor layer, so it asks for a composite of the
            // retained scene; a content repaint owed on the same pass has
            // already submitted the stronger Redisplay demand above and wins
            // the strongest-invalidation merge.
            if window_state.has_presentable_cursor_change() {
                let cursor_action = self.frame_coordinator.submit_demand(
                    id,
                    FrameDemand {
                        invalidation: Invalidation::CompositeOnly {
                            layers: LayerMask::CURSOR_EFFECTS,
                        },
                        cadence: Cadence::NextPresentation,
                        reason: DemandReason::CursorAnimation,
                    },
                    now,
                );
                if cursor_action == PacingAction::RequestRedraw {
                    action = PacingAction::RequestRedraw;
                }
            }

            match window_state.render.cursor.next_blink_deadline() {
                Some(blink) => {
                    self.frame_coordinator.submit_demand(
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
                None => {
                    self.frame_coordinator
                        .retract(id, DemandReason::CursorAnimation);
                }
            }

            if action == PacingAction::RequestRedraw {
                window_state.request_redraw();
            }
        }

        if self.lifecycle_flags.poll_when_idle {
            self.frame_coordinator.submit_demand(
                LOOP_WINDOW,
                FrameDemand {
                    invalidation: legacy_repaint,
                    cadence: Cadence::At(now + LEGACY_IDLE_POLL),
                    reason: DemandReason::Redisplay,
                },
                now,
            );
        }
    }

    /// Estimated display cadence for a window: the monitor-reported refresh
    /// rate is an initial estimate (refined by measurement in later stages),
    /// clamped to a sane range, defaulting to 60 Hz.
    pub(super) fn window_max_rate(
        window_state: &super::frame_windows::GuiFrameWindowState,
    ) -> std::num::NonZeroU16 {
        let hz = window_state
            .window()
            .and_then(|window| window.current_monitor())
            .and_then(|monitor| monitor.refresh_rate_millihertz())
            .map(|mhz| ((mhz as f64) / 1000.0).round() as u16)
            .unwrap_or(60)
            .clamp(30, 240);
        std::num::NonZeroU16::new(hz).unwrap_or(std::num::NonZeroU16::new(60).unwrap())
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

        // The Wayland clipboard borrows Winit's wl_display. Stop its worker
        // before dropping any native windows or the event-loop connection.
        drop(std::mem::replace(
            &mut self.clipboard,
            Err("display is shutting down".to_owned()),
        ));

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
