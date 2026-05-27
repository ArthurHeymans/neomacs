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
        self.apply_extra_spacing_if_needed();
        self.apply_visual_cursor_animations();
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
        if let Some(frame) = self.primary_current_frame() {
            self.faces = frame.faces.clone();
        }
        let primary_child_faces: Vec<_> = self
            .primary_child_frames()
            .frames
            .values()
            .flat_map(|entry| {
                entry
                    .frame
                    .faces
                    .iter()
                    .map(|(id, face)| (*id, face.clone()))
            })
            .collect();
        for (face_id, face) in primary_child_faces {
            self.faces.entry(face_id).or_insert(face);
        }
        for window_state in self.frame_windows.windows.values() {
            if let Some(frame) = window_state.render.current_frame.as_ref() {
                for (face_id, face) in &frame.faces {
                    self.faces.entry(*face_id).or_insert_with(|| face.clone());
                }
            }
            for entry in window_state.render.child_frames.frames.values() {
                for (face_id, face) in &entry.frame.faces {
                    self.faces.entry(*face_id).or_insert_with(|| face.clone());
                }
            }
        }
        let has_new_faces = self.faces.keys().any(|id| !old_face_ids.contains(id));
        if has_new_faces {
            if let Some(primary_frame) = self.primary_frame.as_mut() {
                tracing::info!(
                    "New face_ids detected (old={}, new={}), clearing primary glyph cache",
                    old_face_ids.len(),
                    self.faces.len()
                );
                primary_frame.glyph_atlas.clear();
            }
            for window_state in self.frame_windows.windows.values_mut() {
                window_state.render.glyph_atlas.clear();
            }
        }
    }

    fn apply_visual_cursor_animations(&mut self) {
        if self
            .primary_frame
            .as_ref()
            .is_none_or(|frame| frame.visual_cursors.is_empty())
        {
            return;
        }
        let visual_cursor_rects: HashMap<i64, (f32, f32, f32, f32)> = self
            .primary_frame
            .as_ref()
            .map(|frame| frame.visual_cursors.iter())
            .into_iter()
            .flatten()
            .map(|(id, state)| {
                (
                    *id,
                    (
                        state.current_x,
                        state.current_y,
                        state.current_w,
                        state.current_h,
                    ),
                )
            })
            .collect();
        if let Some(frame) = self.primary_current_frame_mut() {
            for cursor in &mut frame.window_cursors {
                let Some((x, y, width, height)) = visual_cursor_rects.get(&cursor.window_id) else {
                    continue;
                };
                cursor.x = *x;
                cursor.y = *y;
                cursor.width = *width;
                cursor.height = *height;
            }
        }
    }

    fn apply_extra_spacing_if_needed(&mut self) {
        if self.extra_line_spacing != 0.0 || self.extra_letter_spacing != 0.0 {
            let extra_line_spacing = self.extra_line_spacing;
            let extra_letter_spacing = self.extra_letter_spacing;
            if let Some(frame) = self.primary_current_frame_mut() {
                Self::apply_extra_spacing(
                    &mut frame.glyphs,
                    &mut frame.window_cursors,
                    &mut frame.phys_cursor,
                    extra_line_spacing,
                    extra_letter_spacing,
                );
            }
        }
    }

    fn apply_extra_spacing(
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
