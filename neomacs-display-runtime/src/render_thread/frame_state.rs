use super::{FpsCounter, RenderApp};
use crate::core::frame_glyphs::{DisplaySlotId, FrameGlyph, PhysCursor, WindowCursorVisual};
use std::collections::HashMap;

impl RenderApp {
    pub(super) fn prepare_frame_state_for_render(&mut self) {
        #[cfg(feature = "neo-term")]
        self.update_terminals();

        self.process_webkit_frames();
        self.process_video_frames();
        self.process_pending_images();
        self.refresh_faces_from_frames();
        self.apply_primary_fallback_visual_cursor_animations();
        self.frame_windows
            .apply_top_level_visual_cursor_animations();
    }

    pub(super) fn update_fps_counter(fps: &mut FpsCounter) {
        if fps.enabled {
            fps.render_start = std::time::Instant::now();
            fps.frame_count += 1;
            let elapsed = fps.last_instant.elapsed();
            if elapsed.as_secs_f32() >= 1.0 {
                fps.display_value = fps.frame_count as f32 / elapsed.as_secs_f32();
                fps.frame_count = 0;
                fps.last_instant = std::time::Instant::now();
            }
        }
    }

    fn refresh_faces_from_frames(&mut self) {
        let old_face_ids: std::collections::HashSet<u32> = self.faces.keys().copied().collect();

        let mut faces = std::collections::HashMap::new();
        self.frame_windows
            .for_each_top_level_window(|window_state| {
                if let Some(frame) = window_state.render.current_frame.as_ref() {
                    for (face_id, face) in &frame.faces {
                        faces.entry(*face_id).or_insert_with(|| face.clone());
                    }
                }
                for entry in window_state.render.child_frames.frames.values() {
                    for (face_id, face) in &entry.frame.faces {
                        faces.entry(*face_id).or_insert_with(|| face.clone());
                    }
                }
            });

        if self.primary_window_state().is_none() {
            if let Some(frame) = self.primary_current_frame() {
                for (face_id, face) in &frame.faces {
                    faces.entry(*face_id).or_insert_with(|| face.clone());
                }
            }
            for entry in self.primary_child_frames().frames.values() {
                for (face_id, face) in &entry.frame.faces {
                    faces.entry(*face_id).or_insert_with(|| face.clone());
                }
            }
        }

        self.faces = faces;
        let has_new_faces = self.faces.keys().any(|id| !old_face_ids.contains(id));
        if has_new_faces {
            let face_count = self.faces.len();
            if self.primary_window_state().is_none()
                && let Some(primary_frame) = self.primary_render_state_mut()
            {
                tracing::info!(
                    "New face_ids detected (old={}, new={}), clearing primary glyph cache",
                    old_face_ids.len(),
                    face_count
                );
                primary_frame.glyph_atlas.clear();
            }
            self.frame_windows.clear_top_level_glyph_atlases();
        }
    }

    fn apply_primary_fallback_visual_cursor_animations(&mut self) {
        if self.primary_window_state().is_none()
            && let Some(primary_frame) = self.primary_render_state_mut()
        {
            primary_frame.apply_visual_cursor_animations();
        }
    }
}

impl RenderApp {
    pub(super) fn apply_extra_spacing(
        glyphs: &mut [FrameGlyph],
        window_cursors: &mut [WindowCursorVisual],
        phys_cursor: &mut Option<PhysCursor>,
        line_spacing: f32,
        letter_spacing: f32,
    ) {
        let mut last_y: f32 = f32::NEG_INFINITY;
        let mut row_index: i32 = -1;
        let mut char_in_row: i32 = 0;
        let mut last_window_y: f32 = f32::NEG_INFINITY;
        let mut slot_positions: HashMap<DisplaySlotId, (f32, f32)> = HashMap::new();

        for glyph in glyphs.iter_mut() {
            match glyph {
                FrameGlyph::Char {
                    x,
                    y,
                    row_role,
                    slot_id,
                    ..
                } => {
                    if row_role.is_chrome() {
                        continue;
                    }
                    if *y < last_window_y - 1.0 {
                        row_index = -1;
                        last_y = f32::NEG_INFINITY;
                    }
                    last_window_y = *y;

                    if (*y - last_y).abs() > 0.5 {
                        row_index += 1;
                        char_in_row = 0;
                        last_y = *y;
                    } else {
                        char_in_row += 1;
                    }
                    *y += row_index as f32 * line_spacing;
                    *x += char_in_row as f32 * letter_spacing;
                    slot_positions.insert(*slot_id, (*x, *y));
                }
                FrameGlyph::Stretch {
                    x,
                    y,
                    row_role,
                    slot_id,
                    ..
                } => {
                    if row_role.is_chrome() {
                        continue;
                    }
                    if *y < last_window_y - 1.0 {
                        row_index = -1;
                        last_y = f32::NEG_INFINITY;
                    }
                    last_window_y = *y;

                    if (*y - last_y).abs() > 0.5 {
                        row_index += 1;
                        char_in_row = 0;
                        last_y = *y;
                    } else {
                        char_in_row += 1;
                    }
                    *y += row_index as f32 * line_spacing;
                    *x += char_in_row as f32 * letter_spacing;
                    slot_positions.insert(*slot_id, (*x, *y));
                }
                FrameGlyph::Image {
                    x,
                    y,
                    row_role,
                    slot_id,
                    ..
                }
                | FrameGlyph::Video {
                    x,
                    y,
                    row_role,
                    slot_id,
                    ..
                }
                | FrameGlyph::WebKit {
                    x,
                    y,
                    row_role,
                    slot_id,
                    ..
                } => {
                    if row_role.is_chrome() {
                        continue;
                    }
                    let Some(slot_id) = *slot_id else {
                        continue;
                    };
                    if *y < last_window_y - 1.0 {
                        row_index = -1;
                        last_y = f32::NEG_INFINITY;
                    }
                    last_window_y = *y;

                    if (*y - last_y).abs() > 0.5 {
                        row_index += 1;
                        char_in_row = 0;
                        last_y = *y;
                    } else {
                        char_in_row += 1;
                    }
                    *y += row_index as f32 * line_spacing;
                    *x += char_in_row as f32 * letter_spacing;
                    slot_positions.insert(slot_id, (*x, *y));
                }
                _ => {}
            }
        }

        for cursor in window_cursors.iter_mut() {
            if let Some((x, y)) = slot_positions.get(&cursor.slot_id).copied() {
                cursor.x = x;
                cursor.y = y;
            }
        }

        if let Some(cursor) = phys_cursor.as_mut()
            && let Some((x, y)) = slot_positions.get(&cursor.slot_id).copied()
        {
            cursor.x = x;
            cursor.y = y;
        }
    }
}

#[cfg(test)]
#[path = "frame_state_test.rs"]
mod tests;
