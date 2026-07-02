use super::{FpsCounter, RenderApp};
use crate::core::face::Face;
use crate::core::frame_glyphs::FrameGlyphBuffer;
use crate::core::frame_glyphs::{DisplaySlotId, FrameGlyph, WindowCursor};
use std::collections::{HashMap, HashSet};

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
        fn merge_frame_faces(
            frame: &FrameGlyphBuffer,
            faces: &mut HashMap<u32, Face>,
            observed_face_ids: &mut HashSet<u32>,
        ) -> bool {
            let mut changed = false;
            for (face_id, face) in &frame.faces {
                observed_face_ids.insert(*face_id);
                match faces.get_mut(face_id) {
                    Some(cached) if cached == face => {}
                    Some(cached) => {
                        *cached = face.clone();
                        changed = true;
                    }
                    None => {
                        faces.insert(*face_id, face.clone());
                        changed = true;
                    }
                }
            }
            changed
        }

        let old_face_count = self.faces.len();
        let frame_windows = &self.frame_windows;
        let faces = &mut self.faces;
        let mut observed_face_ids = HashSet::with_capacity(faces.len());
        let mut faces_changed = false;

        frame_windows.for_each_top_level_window(|window_state| {
            if let Some(frame) = window_state.render.compositor.current_frame.as_ref() {
                faces_changed |= merge_frame_faces(frame, faces, &mut observed_face_ids);
            }
            for entry in window_state.render.compositor.child_frames.frames.values() {
                faces_changed |= merge_frame_faces(&entry.frame, faces, &mut observed_face_ids);
            }
        });

        if let Some(frame) = frame_windows
            .primary_window()
            .and_then(|ws| ws.render.compositor.current_frame.as_ref())
        {
            faces_changed |= merge_frame_faces(frame, faces, &mut observed_face_ids);
        }
        for entry in frame_windows
            .primary_window()
            .expect("primary child frames")
            .render
            .compositor
            .child_frames
            .frames
            .values()
        {
            faces_changed |= merge_frame_faces(&entry.frame, faces, &mut observed_face_ids);
        }

        faces.retain(|face_id, _| observed_face_ids.contains(face_id));
        if faces_changed {
            let face_count = faces.len();
            if let Some(primary_frame) = self
                .frame_windows
                .primary_window_mut()
                .map(|ws| &mut ws.render)
            {
                tracing::info!(
                    "Face cache changed (old={}, new={}), clearing primary glyph cache",
                    old_face_count,
                    face_count
                );
                if let Some(atlas) = primary_frame.compositor.glyph_atlas.as_mut() {
                    atlas.clear();
                }
            }
            self.frame_windows.clear_top_level_glyph_atlases();
        }
    }

    fn apply_primary_fallback_visual_cursor_animations(&mut self) {
        if let Some(primary_frame) = self
            .frame_windows
            .primary_window_mut()
            .map(|ws| &mut ws.render)
        {
            primary_frame.apply_visual_cursor_animations();
        }
    }
}

impl RenderApp {
    pub(super) fn apply_extra_spacing(
        glyphs: &mut [FrameGlyph],
        cursors: &mut [WindowCursor],
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
                | FrameGlyph::Xwidget {
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

        // The active (selected window's) cursor is now in this list, so the
        // single loop adjusts it alongside the decorative cursors.
        for cursor in cursors.iter_mut() {
            if let Some((x, y)) = slot_positions.get(&cursor.slot_id).copied() {
                cursor.x = x;
                cursor.y = y;
            }
        }
    }
}

#[cfg(test)]
#[path = "frame_state_test.rs"]
mod tests;
