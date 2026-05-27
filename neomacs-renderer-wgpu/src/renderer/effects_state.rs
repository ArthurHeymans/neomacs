//! Effects State methods for WgpuRenderer.

#[cfg(test)]
use super::ModeLineFadeEntry;
use super::{
    ClickHaloEntry, EdgeGlowEntry, EdgeSnapEntry, LineAnimEntry, ScrollMomentumEntry,
    ScrollSpacingEntry, ScrollVelocityFadeEntry, SonarPingEntry, TextFadeEntry, WindowFadeEntry,
};
use super::{RendererFrameEffects, WgpuRenderer};
use neomacs_display_protocol::types::{Color, Rect};

impl WgpuRenderer {
    /// Update inactive window dim config
    pub fn set_inactive_dim_config(&mut self, enabled: bool, opacity: f32) {
        self.effects.inactive_dim.enabled = enabled;
        self.effects.inactive_dim.opacity = opacity;
    }

    /// Start a line animation for a window
    pub fn start_line_animation(
        &mut self,
        window_bounds: Rect,
        edit_y: f32,
        offset: f32,
        duration_ms: u32,
    ) {
        // Remove any existing animation for this window region
        self.active_line_anims.retain(|a| {
            (a.window_bounds.x - window_bounds.x).abs() > 1.0
                || (a.window_bounds.y - window_bounds.y).abs() > 1.0
        });
        self.active_line_anims.push(LineAnimEntry {
            window_bounds,
            edit_y,
            initial_offset: offset,
            started: std::time::Instant::now(),
            duration: std::time::Duration::from_millis(duration_ms as u64),
        });
        self.needs_continuous_redraw = true;
    }

    /// Compute Y offset for a glyph due to active line animations
    pub(super) fn line_y_offset(&self, gx: f32, gy: f32) -> f32 {
        let mut offset = 0.0;
        for anim in &self.active_line_anims {
            let b = &anim.window_bounds;
            // Check if glyph is in this window and below the edit point
            if gx >= b.x
                && gx < b.x + b.width
                && gy >= b.y
                && gy < b.y + b.height
                && gy >= anim.edit_y
            {
                let elapsed = anim.started.elapsed();
                let t = (elapsed.as_secs_f32() / anim.duration.as_secs_f32()).min(1.0);
                // Ease-out quadratic: t * (2 - t)
                let eased = t * (2.0 - t);
                offset += anim.initial_offset * (1.0 - eased);
            }
        }
        // Scroll line spacing accordion effect
        let now = std::time::Instant::now();
        for entry in &self.active_scroll_spacings {
            let b = &entry.bounds;
            if gx >= b.x && gx < b.x + b.width && gy >= b.y && gy < b.y + b.height {
                let elapsed = now.duration_since(entry.started).as_secs_f32();
                let total = entry.duration.as_secs_f32();
                if elapsed < total {
                    let progress = elapsed / total;
                    let decay = 1.0 - progress;
                    let decay = decay * decay;
                    let norm = ((gy - b.y) / b.height).clamp(0.0, 1.0);
                    let edge_factor = if entry.direction > 0 {
                        1.0 - norm
                    } else {
                        norm
                    };
                    offset += self.effects.scroll_line_spacing.max * decay * edge_factor;
                }
            }
        }
        offset
    }

    /// Trigger a cursor wake animation
    pub fn trigger_cursor_wake(&mut self, now: std::time::Instant) {
        self.primary_frame_effects_mut().trigger_cursor_wake(now);
    }

    /// Trigger edge snap indicator
    pub fn trigger_edge_snap(
        &mut self,
        bounds: Rect,
        mode_line_height: f32,
        at_top: bool,
        at_bottom: bool,
        now: std::time::Instant,
    ) {
        let duration_ms = self.effects.edge_snap.duration_ms;
        self.primary_frame_effects_mut().trigger_edge_snap(
            bounds,
            mode_line_height,
            at_top,
            at_bottom,
            now,
            duration_ms,
        );
    }

    /// Update typing heat map config
    pub fn set_typing_heatmap(
        &mut self,
        enabled: bool,
        color: (f32, f32, f32),
        fade_ms: u32,
        opacity: f32,
    ) {
        self.effects.typing_heatmap.enabled = enabled;
        self.effects.typing_heatmap.color = color;
        self.effects.typing_heatmap.fade_ms = fade_ms;
        self.effects.typing_heatmap.opacity = opacity;
        if !enabled {
            self.typing_heatmap_entries.clear();
            self.typing_heatmap_prev_cursor = None;
        }
    }

    /// Trigger click halo at position
    pub fn trigger_click_halo(&mut self, x: f32, y: f32, now: std::time::Instant) {
        let duration_ms = self.effects.click_halo.duration_ms;
        self.primary_frame_effects_mut()
            .trigger_click_halo(x, y, now, duration_ms);
    }

    /// Trigger scroll velocity fade for a window
    pub fn trigger_scroll_velocity_fade(
        &mut self,
        window_id: i64,
        bounds: Rect,
        delta: f32,
        now: std::time::Instant,
    ) {
        // Replace existing entry for this window
        self.scroll_velocity_fades
            .retain(|e| e.window_id != window_id);
        self.scroll_velocity_fades.push(ScrollVelocityFadeEntry {
            window_id,
            bounds,
            velocity: delta,
            started: now,
            duration: std::time::Duration::from_millis(self.effects.scroll_velocity_fade.ms as u64),
        });
    }

    /// Trigger resize padding animation
    pub fn trigger_resize_padding(&mut self, now: std::time::Instant) {
        self.primary_frame_effects_mut().trigger_resize_padding(now);
    }

    /// Get current resize padding amount (eases from max to 0)
    pub(super) fn resize_padding_amount(&self) -> f32 {
        if let Some(started) = self.resize_padding_started {
            let elapsed = started.elapsed().as_millis() as f32;
            let duration = self.effects.resize_padding.duration_ms as f32;
            if elapsed >= duration {
                return 0.0;
            }
            let t = elapsed / duration;
            let ease = t * (2.0 - t); // quadratic ease-out
            self.effects.resize_padding.max * (1.0 - ease)
        } else {
            0.0
        }
    }

    /// Trigger a cursor error pulse
    pub fn trigger_cursor_error_pulse(&mut self, now: std::time::Instant) {
        self.primary_frame_effects_mut()
            .trigger_cursor_error_pulse(now);
    }

    pub fn trigger_transient_click_halo(
        &self,
        effects: &mut RendererFrameEffects,
        x: f32,
        y: f32,
        now: std::time::Instant,
    ) {
        effects.trigger_click_halo(x, y, now, self.effects.click_halo.duration_ms);
    }

    pub fn trigger_transient_edge_snap(
        &self,
        effects: &mut RendererFrameEffects,
        bounds: Rect,
        mode_line_height: f32,
        at_top: bool,
        at_bottom: bool,
        now: std::time::Instant,
    ) {
        effects.trigger_edge_snap(
            bounds,
            mode_line_height,
            at_top,
            at_bottom,
            now,
            self.effects.edge_snap.duration_ms,
        );
    }

    pub fn trigger_transient_cursor_error_pulse(
        &self,
        effects: &mut RendererFrameEffects,
        now: std::time::Instant,
    ) {
        effects.trigger_cursor_error_pulse(now);
    }

    pub fn trigger_transient_cursor_wake(
        &self,
        effects: &mut RendererFrameEffects,
        now: std::time::Instant,
    ) {
        effects.trigger_cursor_wake(now);
    }

    pub fn trigger_transient_resize_padding(
        &self,
        effects: &mut RendererFrameEffects,
        now: std::time::Instant,
    ) {
        effects.trigger_resize_padding(now);
    }

    pub fn spawn_transient_ripple(&self, effects: &mut RendererFrameEffects, cx: f32, cy: f32) {
        if self.effects.typing_ripple.enabled {
            effects.spawn_ripple(cx, cy);
        }
    }

    pub fn record_transient_cursor_trail(
        &self,
        effects: &mut RendererFrameEffects,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        if self.effects.cursor_trail_fade.enabled {
            effects.record_cursor_trail(x, y, w, h, self.effects.cursor_trail_fade.length);
        }
    }

    pub fn with_frame_effects<R>(
        &mut self,
        effects: &mut RendererFrameEffects,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let primary = self.take_frame_effects();
        self.apply_frame_effects(std::mem::take(effects));
        let result = f(self);
        *effects = self.take_frame_effects();
        self.apply_frame_effects(primary);
        result
    }

    fn primary_frame_effects_mut(&mut self) -> RendererFrameEffectsRef<'_> {
        RendererFrameEffectsRef { renderer: self }
    }

    fn take_frame_effects(&mut self) -> RendererFrameEffects {
        RendererFrameEffects {
            needs_continuous_redraw: self.needs_continuous_redraw,
            has_animated_borders: self.has_animated_borders,
            per_window_dim: std::mem::take(&mut self.per_window_dim),
            last_dim_tick: Some(self.last_dim_tick),
            active_ripples: std::mem::take(&mut self.active_ripples),
            active_line_anims: std::mem::take(&mut self.active_line_anims),
            active_window_fades: std::mem::take(&mut self.active_window_fades),
            active_title_fades: std::mem::take(&mut self.active_title_fades),
            prev_breadcrumb_text: std::mem::take(&mut self.prev_breadcrumb_text),
            border_transitions: std::mem::take(&mut self.border_transitions),
            prev_border_selected: self.prev_border_selected,
            active_mode_line_fades: std::mem::take(&mut self.active_mode_line_fades),
            prev_mode_line_hashes: std::mem::take(&mut self.prev_mode_line_hashes),
            active_text_fades: std::mem::take(&mut self.active_text_fades),
            active_scroll_spacings: std::mem::take(&mut self.active_scroll_spacings),
            cursor_trail_positions: std::mem::take(&mut self.cursor_trail_positions),
            cursor_trail_last_pos: self.cursor_trail_last_pos,
            idle_dim_alpha: self.idle_dim_alpha,
            noise_grain_frame: self.noise_grain_frame,
            cursor_pulse_start: Some(self.cursor_pulse_start),
            search_pulse_start: Some(self.search_pulse_start),
            cursor_color_cycle_start: Some(self.cursor_color_cycle_start),
            focus_ring_start: Some(self.focus_ring_start),
            cursor_wake_started: self.cursor_wake_started.take(),
            cursor_magnetism_entries: std::mem::take(&mut self.cursor_magnetism_entries),
            cursor_comet_positions: std::mem::take(&mut self.cursor_comet_positions),
            cursor_particles: std::mem::take(&mut self.cursor_particles),
            cursor_particles_prev_pos: self.cursor_particles_prev_pos.take(),
            typing_heatmap_entries: std::mem::take(&mut self.typing_heatmap_entries),
            typing_heatmap_prev_cursor: self.typing_heatmap_prev_cursor.take(),
            scroll_velocity_fades: std::mem::take(&mut self.scroll_velocity_fades),
            resize_padding_started: self.resize_padding_started.take(),
            active_scroll_momentums: std::mem::take(&mut self.active_scroll_momentums),
            matrix_rain_columns: std::mem::take(&mut self.matrix_rain_columns),
            cursor_ghost_entries: std::mem::take(&mut self.cursor_ghost_entries),
            cursor_sonar_ping_entries: std::mem::take(&mut self.cursor_sonar_ping_entries),
            lightning_bolt_last: Some(self.lightning_bolt_last),
            lightning_bolt_segments: std::mem::take(&mut self.lightning_bolt_segments),
            lightning_bolt_age: self.lightning_bolt_age,
            cursor_pendulum_last_x: self.cursor_pendulum_last_x,
            cursor_pendulum_last_y: self.cursor_pendulum_last_y,
            cursor_pendulum_swing_start: self.cursor_pendulum_swing_start.take(),
            cursor_sparkle_burst_entries: std::mem::take(&mut self.cursor_sparkle_burst_entries),
            cursor_metronome_last_x: self.cursor_metronome_last_x,
            cursor_metronome_last_y: self.cursor_metronome_last_y,
            cursor_metronome_tick_start: self.cursor_metronome_tick_start.take(),
            cursor_ripple_ring_start: self.cursor_ripple_ring_start.take(),
            cursor_ripple_ring_last_x: self.cursor_ripple_ring_last_x,
            cursor_ripple_ring_last_y: self.cursor_ripple_ring_last_y,
            cursor_shockwave_start: self.cursor_shockwave_start.take(),
            cursor_shockwave_last_x: self.cursor_shockwave_last_x,
            cursor_shockwave_last_y: self.cursor_shockwave_last_y,
            cursor_bubble_spawn_time: self.cursor_bubble_spawn_time.take(),
            cursor_bubble_last_x: self.cursor_bubble_last_x,
            cursor_bubble_last_y: self.cursor_bubble_last_y,
            cursor_firework_start: self.cursor_firework_start.take(),
            cursor_firework_last_x: self.cursor_firework_last_x,
            cursor_firework_last_y: self.cursor_firework_last_y,
            cursor_lightning_start: self.cursor_lightning_start.take(),
            cursor_lightning_last_x: self.cursor_lightning_last_x,
            cursor_lightning_last_y: self.cursor_lightning_last_y,
            cursor_snowflake_start: self.cursor_snowflake_start.take(),
            cursor_snowflake_last_x: self.cursor_snowflake_last_x,
            cursor_snowflake_last_y: self.cursor_snowflake_last_y,
            edge_glow_entries: std::mem::take(&mut self.edge_glow_entries),
            rain_drops: std::mem::take(&mut self.rain_drops),
            rain_last_spawn: Some(self.rain_last_spawn),
            cursor_ripple_waves: std::mem::take(&mut self.cursor_ripple_waves),
            click_halos: std::mem::take(&mut self.click_halos),
            edge_snaps: std::mem::take(&mut self.edge_snaps),
            cursor_error_pulse_started: self.cursor_error_pulse_started.take(),
        }
    }

    fn apply_frame_effects(&mut self, effects: RendererFrameEffects) {
        self.needs_continuous_redraw = effects.needs_continuous_redraw;
        self.has_animated_borders = effects.has_animated_borders;
        self.per_window_dim = effects.per_window_dim;
        if let Some(last_dim_tick) = effects.last_dim_tick {
            self.last_dim_tick = last_dim_tick;
        }
        self.active_ripples = effects.active_ripples;
        self.active_line_anims = effects.active_line_anims;
        self.active_window_fades = effects.active_window_fades;
        self.active_title_fades = effects.active_title_fades;
        self.prev_breadcrumb_text = effects.prev_breadcrumb_text;
        self.border_transitions = effects.border_transitions;
        self.prev_border_selected = effects.prev_border_selected;
        self.active_mode_line_fades = effects.active_mode_line_fades;
        self.prev_mode_line_hashes = effects.prev_mode_line_hashes;
        self.active_text_fades = effects.active_text_fades;
        self.active_scroll_spacings = effects.active_scroll_spacings;
        self.cursor_trail_positions = effects.cursor_trail_positions;
        self.cursor_trail_last_pos = effects.cursor_trail_last_pos;
        self.idle_dim_alpha = effects.idle_dim_alpha;
        self.noise_grain_frame = effects.noise_grain_frame;
        if let Some(start) = effects.cursor_pulse_start {
            self.cursor_pulse_start = start;
        }
        if let Some(start) = effects.search_pulse_start {
            self.search_pulse_start = start;
        }
        if let Some(start) = effects.cursor_color_cycle_start {
            self.cursor_color_cycle_start = start;
        }
        if let Some(start) = effects.focus_ring_start {
            self.focus_ring_start = start;
        }
        self.cursor_wake_started = effects.cursor_wake_started;
        self.cursor_magnetism_entries = effects.cursor_magnetism_entries;
        self.cursor_comet_positions = effects.cursor_comet_positions;
        self.cursor_particles = effects.cursor_particles;
        self.cursor_particles_prev_pos = effects.cursor_particles_prev_pos;
        self.typing_heatmap_entries = effects.typing_heatmap_entries;
        self.typing_heatmap_prev_cursor = effects.typing_heatmap_prev_cursor;
        self.scroll_velocity_fades = effects.scroll_velocity_fades;
        self.resize_padding_started = effects.resize_padding_started;
        self.active_scroll_momentums = effects.active_scroll_momentums;
        self.matrix_rain_columns = effects.matrix_rain_columns;
        self.cursor_ghost_entries = effects.cursor_ghost_entries;
        self.cursor_sonar_ping_entries = effects.cursor_sonar_ping_entries;
        if let Some(last) = effects.lightning_bolt_last {
            self.lightning_bolt_last = last;
        }
        self.lightning_bolt_segments = effects.lightning_bolt_segments;
        self.lightning_bolt_age = effects.lightning_bolt_age;
        self.cursor_pendulum_last_x = effects.cursor_pendulum_last_x;
        self.cursor_pendulum_last_y = effects.cursor_pendulum_last_y;
        self.cursor_pendulum_swing_start = effects.cursor_pendulum_swing_start;
        self.cursor_sparkle_burst_entries = effects.cursor_sparkle_burst_entries;
        self.cursor_metronome_last_x = effects.cursor_metronome_last_x;
        self.cursor_metronome_last_y = effects.cursor_metronome_last_y;
        self.cursor_metronome_tick_start = effects.cursor_metronome_tick_start;
        self.cursor_ripple_ring_start = effects.cursor_ripple_ring_start;
        self.cursor_ripple_ring_last_x = effects.cursor_ripple_ring_last_x;
        self.cursor_ripple_ring_last_y = effects.cursor_ripple_ring_last_y;
        self.cursor_shockwave_start = effects.cursor_shockwave_start;
        self.cursor_shockwave_last_x = effects.cursor_shockwave_last_x;
        self.cursor_shockwave_last_y = effects.cursor_shockwave_last_y;
        self.cursor_bubble_spawn_time = effects.cursor_bubble_spawn_time;
        self.cursor_bubble_last_x = effects.cursor_bubble_last_x;
        self.cursor_bubble_last_y = effects.cursor_bubble_last_y;
        self.cursor_firework_start = effects.cursor_firework_start;
        self.cursor_firework_last_x = effects.cursor_firework_last_x;
        self.cursor_firework_last_y = effects.cursor_firework_last_y;
        self.cursor_lightning_start = effects.cursor_lightning_start;
        self.cursor_lightning_last_x = effects.cursor_lightning_last_x;
        self.cursor_lightning_last_y = effects.cursor_lightning_last_y;
        self.cursor_snowflake_start = effects.cursor_snowflake_start;
        self.cursor_snowflake_last_x = effects.cursor_snowflake_last_x;
        self.cursor_snowflake_last_y = effects.cursor_snowflake_last_y;
        self.edge_glow_entries = effects.edge_glow_entries;
        self.rain_drops = effects.rain_drops;
        if let Some(last_spawn) = effects.rain_last_spawn {
            self.rain_last_spawn = last_spawn;
        }
        self.cursor_ripple_waves = effects.cursor_ripple_waves;
        self.click_halos = effects.click_halos;
        self.edge_snaps = effects.edge_snaps;
        self.cursor_error_pulse_started = effects.cursor_error_pulse_started;
    }

    /// Get the cursor error pulse color override, if active
    pub(super) fn cursor_error_pulse_override(&self) -> Option<Color> {
        if !self.effects.cursor_error_pulse.enabled {
            return None;
        }
        if let Some(started) = self.cursor_error_pulse_started {
            let elapsed = started.elapsed().as_millis() as f32;
            let duration = self.effects.cursor_error_pulse.duration_ms as f32;
            if elapsed >= duration {
                return None;
            }
            let t = elapsed / duration;
            // Flash: bright at start, fade out
            let alpha = (1.0 - t) * (1.0 - t);
            let (r, g, b) = self.effects.cursor_error_pulse.color;
            Some(Color::new(r, g, b, alpha))
        } else {
            None
        }
    }

    /// Trigger a scroll momentum indicator for a window
    pub fn trigger_scroll_momentum(
        &mut self,
        window_id: i64,
        bounds: Rect,
        direction: i32,
        now: std::time::Instant,
    ) {
        self.active_scroll_momentums
            .retain(|e| e.window_id != window_id);
        self.active_scroll_momentums.push(ScrollMomentumEntry {
            window_id,
            bounds,
            direction,
            started: now,
            duration: std::time::Duration::from_millis(self.effects.scroll_momentum.fade_ms as u64),
        });
    }

    /// Update matrix rain config
    pub fn set_matrix_rain(
        &mut self,
        enabled: bool,
        color: (f32, f32, f32),
        column_count: u32,
        speed: f32,
        opacity: f32,
    ) {
        self.effects.matrix_rain.enabled = enabled;
        self.effects.matrix_rain.color = color;
        self.effects.matrix_rain.column_count = column_count;
        self.effects.matrix_rain.speed = speed;
        self.effects.matrix_rain.opacity = opacity;
        if !enabled {
            self.matrix_rain_columns.clear();
        }
    }

    /// Update frost border config
    pub fn set_frost_border_effect(
        &mut self,
        enabled: bool,
        color: (f32, f32, f32),
        width: f32,
        opacity: f32,
    ) {
        self.effects.frost_border.enabled = enabled;
        self.effects.frost_border.color = color;
        self.effects.frost_border.width = width;
        self.effects.frost_border.opacity = opacity;
    }

    /// Trigger edge glow for a window (at_top = beginning-of-buffer)
    pub fn trigger_edge_glow(
        &mut self,
        window_id: i64,
        bounds: Rect,
        at_top: bool,
        now: std::time::Instant,
    ) {
        self.edge_glow_entries
            .retain(|e| e.window_id != window_id || e.at_top != at_top);
        self.edge_glow_entries.push(EdgeGlowEntry {
            window_id,
            bounds,
            at_top,
            started: now,
            duration: std::time::Duration::from_millis(self.effects.edge_glow.fade_ms as u64),
        });
    }

    /// Trigger a sonar ping at cursor position
    pub fn trigger_sonar_ping(&mut self, cx: f32, cy: f32, now: std::time::Instant) {
        self.cursor_sonar_ping_entries.push(SonarPingEntry {
            cx,
            cy,
            started: now,
            duration: std::time::Duration::from_millis(
                self.effects.cursor_sonar_ping.duration_ms as u64,
            ),
        });
    }

    /// Get the mode-line transition alpha for a glyph at (x, y)
    pub(super) fn mode_line_fade_alpha(&self, gx: f32, gy: f32) -> f32 {
        if !self.effects.mode_line_transition.enabled || self.active_mode_line_fades.is_empty() {
            return 1.0;
        }
        let now = std::time::Instant::now();
        for entry in &self.active_mode_line_fades {
            if gx >= entry.bounds_x
                && gx < entry.bounds_x + entry.bounds_w
                && gy >= entry.mode_line_y
                && gy < entry.mode_line_y + entry.mode_line_h
            {
                let elapsed = now.duration_since(entry.started).as_secs_f32();
                let total = entry.duration.as_secs_f32();
                if elapsed < total {
                    let t = elapsed / total;
                    return t; // linear fade-in
                }
            }
        }
        1.0
    }

    /// Trigger a text fade-in animation for a window
    pub fn trigger_text_fade_in(&mut self, window_id: i64, bounds: Rect, now: std::time::Instant) {
        // Replace existing animation for this window
        self.active_text_fades.retain(|e| e.window_id != window_id);
        self.active_text_fades.push(TextFadeEntry {
            window_id,
            bounds,
            started: now,
            duration: std::time::Duration::from_millis(
                self.effects.text_fade_in.duration_ms as u64,
            ),
        });
        self.needs_continuous_redraw = true;
    }

    /// Get the text fade-in alpha multiplier for a glyph at (x, y).
    /// Returns 1.0 if no fade is active, or 0.0-1.0 during fade-in.
    pub(super) fn text_fade_alpha(&self, gx: f32, gy: f32) -> f32 {
        if !self.effects.text_fade_in.enabled || self.active_text_fades.is_empty() {
            return 1.0;
        }
        let now = std::time::Instant::now();
        for entry in &self.active_text_fades {
            let b = &entry.bounds;
            if gx >= b.x && gx < b.x + b.width && gy >= b.y && gy < b.y + b.height {
                let elapsed = now.duration_since(entry.started).as_secs_f32();
                let total = entry.duration.as_secs_f32();
                if elapsed < total {
                    // Ease-in: start at 0, end at 1
                    let t = elapsed / total;
                    return t * t; // quadratic ease-in for smooth appearance
                }
            }
        }
        1.0
    }

    /// Trigger a scroll line spacing animation for a window
    pub fn trigger_scroll_line_spacing(
        &mut self,
        window_id: i64,
        bounds: Rect,
        direction: i32,
        now: std::time::Instant,
    ) {
        // Replace existing animation for this window
        self.active_scroll_spacings
            .retain(|e| e.window_id != window_id);
        self.active_scroll_spacings.push(ScrollSpacingEntry {
            window_id,
            bounds,
            direction,
            started: now,
            duration: std::time::Duration::from_millis(self.scroll_line_spacing_duration_ms as u64),
        });
        self.needs_continuous_redraw = true;
    }

    /// Record a new cursor position for the trail
    pub fn record_cursor_trail(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if !self.effects.cursor_trail_fade.enabled {
            return;
        }
        let length = self.effects.cursor_trail_fade.length;
        self.primary_frame_effects_mut()
            .record_cursor_trail(x, y, w, h, length);
    }

    /// Update idle dim alpha
    pub fn set_idle_dim_alpha(&mut self, alpha: f32) {
        self.idle_dim_alpha = alpha;
    }

    /// Start a window switch fade for a specific window
    pub fn start_window_fade(&mut self, window_id: i64, bounds: Rect) {
        // Remove any existing fade for this window
        self.active_window_fades
            .retain(|f| f.window_id != window_id);
        self.active_window_fades.push(WindowFadeEntry {
            window_id,
            bounds,
            started: std::time::Instant::now(),
            duration: std::time::Duration::from_millis(
                self.effects.window_switch_fade.duration_ms as u64,
            ),
            intensity: self.effects.window_switch_fade.intensity,
        });
    }

    /// Convert HSL to sRGB Color
    /// Scale a rectangle from its center by a given factor
    pub(super) fn scale_rect(x: f32, y: f32, w: f32, h: f32, scale: f32) -> (f32, f32, f32, f32) {
        let cx = x + w * 0.5;
        let cy = y + h * 0.5;
        let nw = w * scale;
        let nh = h * scale;
        (cx - nw * 0.5, cy - nh * 0.5, nw, nh)
    }

    pub(super) fn hsl_to_color(h: f32, s: f32, l: f32) -> Color {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;
        let (r, g, b) = match (h * 6.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Color {
            r: r + m,
            g: g + m,
            b: b + m,
            a: 1.0,
        }
    }

    /// Spawn a new ripple at the given position
    pub fn spawn_ripple(&mut self, cx: f32, cy: f32) {
        if self.effects.typing_ripple.enabled {
            self.primary_frame_effects_mut().spawn_ripple(cx, cy);
        }
    }

    /// Update visible whitespace config
    pub fn set_show_whitespace_config(&mut self, enabled: bool, color: (f32, f32, f32, f32)) {
        self.effects.show_whitespace.enabled = enabled;
        self.effects.show_whitespace.color = color;
    }

    /// Update line highlight config
    pub fn set_line_highlight_config(&mut self, enabled: bool, color: (f32, f32, f32, f32)) {
        self.effects.line_highlight.enabled = enabled;
        self.effects.line_highlight.color = color;
    }

    /// Update rainbow indent guide config
    pub fn set_indent_guide_rainbow(&mut self, enabled: bool, colors: Vec<(f32, f32, f32, f32)>) {
        self.effects.indent_guides.rainbow_enabled = enabled;
        self.effects.indent_guides.rainbow_colors = colors;
    }
}

struct RendererFrameEffectsRef<'a> {
    renderer: &'a mut WgpuRenderer,
}

impl RendererFrameEffectsRef<'_> {
    fn trigger_click_halo(&mut self, x: f32, y: f32, now: std::time::Instant, duration_ms: u32) {
        self.renderer.click_halos.push(ClickHaloEntry {
            x,
            y,
            started: now,
            duration: std::time::Duration::from_millis(duration_ms as u64),
        });
    }

    fn trigger_edge_snap(
        &mut self,
        bounds: Rect,
        mode_line_height: f32,
        at_top: bool,
        at_bottom: bool,
        now: std::time::Instant,
        duration_ms: u32,
    ) {
        self.renderer.edge_snaps.push(EdgeSnapEntry {
            bounds,
            mode_line_height,
            at_top,
            at_bottom,
            started: now,
            duration: std::time::Duration::from_millis(duration_ms as u64),
        });
    }

    fn trigger_cursor_error_pulse(&mut self, now: std::time::Instant) {
        self.renderer.cursor_error_pulse_started = Some(now);
    }

    fn trigger_cursor_wake(&mut self, now: std::time::Instant) {
        self.renderer.cursor_wake_started = Some(now);
    }

    fn trigger_resize_padding(&mut self, now: std::time::Instant) {
        self.renderer.resize_padding_started = Some(now);
    }

    fn spawn_ripple(&mut self, cx: f32, cy: f32) {
        self.renderer
            .active_ripples
            .push((cx, cy, std::time::Instant::now()));
    }

    fn record_cursor_trail(&mut self, x: f32, y: f32, w: f32, h: f32, length: usize) {
        let dist = ((x - self.renderer.cursor_trail_last_pos.0).powi(2)
            + (y - self.renderer.cursor_trail_last_pos.1).powi(2))
        .sqrt();
        if dist < 2.0 {
            return;
        }
        self.renderer
            .cursor_trail_positions
            .push((x, y, w, h, std::time::Instant::now()));
        self.renderer.cursor_trail_last_pos = (x, y);
        while self.renderer.cursor_trail_positions.len() > length {
            self.renderer.cursor_trail_positions.remove(0);
        }
    }
}

#[cfg(test)]
#[path = "effects_state_test.rs"]
mod tests;
