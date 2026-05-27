use super::frame_windows::GuiFrameWindowState;
use super::{ImeCursorArea, RenderApp};
use crate::render_thread::cursor::CursorTarget;
use winit::dpi::{PhysicalPosition, PhysicalSize};

impl RenderApp {
    fn ime_cursor_area_for_window_target(
        window_state: &GuiFrameWindowState,
        target: &CursorTarget,
    ) -> ImeCursorArea {
        let (ime_off_x, ime_off_y) = if target.frame_id != window_state.render.emacs_frame_id {
            window_state
                .render
                .child_frames
                .frames
                .get(&target.frame_id)
                .map(|e| (e.abs_x as f64, e.abs_y as f64))
                .unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        };

        ImeCursorArea {
            x: ((target.x as f64 + ime_off_x) * window_state.native.scale_factor).round() as i32,
            y: ((target.y as f64 + target.height as f64 + ime_off_y)
                * window_state.native.scale_factor)
                .round() as i32,
            width: ((target.width as f64 * window_state.native.scale_factor).max(1.0)).round()
                as u32,
            height: ((target.height as f64 * window_state.native.scale_factor).max(1.0)).round()
                as u32,
        }
    }

    /// Compute physical IME cursor rectangle for the current cursor target.
    pub(super) fn ime_cursor_area_for_target(&self, target: &CursorTarget) -> ImeCursorArea {
        // If cursor is in a child frame, offset by the child's absolute position.
        let (ime_off_x, ime_off_y) = if target.frame_id != 0 {
            self.primary_child_frames()
                .frames
                .get(&target.frame_id)
                .map(|e| (e.abs_x as f64, e.abs_y as f64))
                .unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        };
        let scale_factor = self.primary_scale_factor();

        ImeCursorArea {
            x: ((target.x as f64 + ime_off_x) * scale_factor).round() as i32,
            y: ((target.y as f64 + target.height as f64 + ime_off_y) * scale_factor).round() as i32,
            width: ((target.width as f64 * scale_factor).max(1.0)).round() as u32,
            height: ((target.height as f64 * scale_factor).max(1.0)).round() as u32,
        }
    }

    /// Update IME cursor area only when IME is active and the rectangle changed.
    pub(super) fn update_ime_cursor_area_if_needed(&mut self, target: &CursorTarget) {
        if !self
            .primary_window_state()
            .map_or(self.ime_enabled, |window_state| {
                window_state.native.ime_enabled
            })
            && !self.primary_ime_preedit_active()
        {
            return;
        }
        let area = self.ime_cursor_area_for_target(target);
        let Some(window_state) = self.primary_window_state_mut() else {
            return;
        };
        let native = &mut window_state.native;
        if native.last_ime_cursor_area == Some(area) {
            return;
        }

        native.window.set_ime_cursor_area(
            PhysicalPosition::new(area.x as f64, area.y as f64),
            PhysicalSize::new(area.width as f64, area.height as f64),
        );
        native.last_ime_cursor_area = Some(area);
    }

    /// Update a secondary frame window's IME cursor area when composition is active.
    pub(super) fn update_frame_window_ime_cursor_area_if_needed(
        window_state: &mut GuiFrameWindowState,
        target: &CursorTarget,
    ) {
        if !window_state.native.ime_enabled && !window_state.render.ime_preedit_active {
            return;
        }

        let area = Self::ime_cursor_area_for_window_target(window_state, target);
        if window_state.native.last_ime_cursor_area == Some(area) {
            return;
        }

        window_state.native.window.set_ime_cursor_area(
            PhysicalPosition::new(area.x as f64, area.y as f64),
            PhysicalSize::new(area.width as f64, area.height as f64),
        );
        window_state.native.last_ime_cursor_area = Some(area);
    }

    pub(super) fn tick_frame_window_cursor_blink(
        window_state: &mut GuiFrameWindowState,
        now: std::time::Instant,
    ) -> bool {
        if !window_state.render.cursor.blink_enabled
            || window_state.render.cursor.target_cloned().is_none()
        {
            return false;
        }
        if now.duration_since(window_state.render.cursor.last_blink_toggle)
            < window_state.render.cursor.blink_interval
        {
            return false;
        }
        window_state.render.cursor.blink_on = !window_state.render.cursor.blink_on;
        window_state.render.cursor.last_blink_toggle = now;
        true
    }

    /// Update cursor blink state, returns true if blink toggled.
    pub(super) fn tick_cursor_blink(&mut self) -> bool {
        let cursor_wake_enabled = self.effects.cursor_wake.enabled;
        let renderer = self.renderer.as_ref();
        let primary_frame = if let Some(window_state) = self.primary_window_state.as_mut() {
            &mut window_state.render
        } else {
            #[cfg(test)]
            {
                let Some(primary_frame) = self.primary_render_state_for_tests.as_mut() else {
                    return false;
                };
                primary_frame
            }
            #[cfg(not(test))]
            {
                return false;
            }
        };
        let cursor = &mut primary_frame.cursor;
        if !cursor.blink_enabled || cursor.target_cloned().is_none() {
            return false;
        }
        let now = std::time::Instant::now();
        if now.duration_since(cursor.last_blink_toggle) >= cursor.blink_interval {
            let was_off = !cursor.blink_on;
            cursor.blink_on = !cursor.blink_on;
            cursor.last_blink_toggle = now;
            if was_off && cursor.blink_on && cursor_wake_enabled {
                if let Some(renderer) = renderer {
                    renderer
                        .trigger_transient_cursor_wake(&mut primary_frame.renderer_effects, now);
                }
            }
            true
        } else {
            false
        }
    }

    pub(super) fn next_cursor_blink_deadline(&self) -> Option<std::time::Instant> {
        let mut next = self.primary_cursor().next_blink_deadline();
        for window_state in self.frame_windows.windows.values() {
            if let Some(deadline) = window_state.render.cursor.next_blink_deadline() {
                next = Some(next.map_or(deadline, |current| current.min(deadline)));
            }
        }
        next
    }
}
