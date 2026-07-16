use super::{FpsCounter, RenderApp};

/// Next unique frame-install stamp for the face aggregation signature.
pub(super) fn next_faces_ingest_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}
use crate::core::frame_glyphs::{DisplaySlotId, FrameGlyph, WindowCursor};
use std::collections::HashMap;

impl RenderApp {
    pub(super) fn prepare_frame_state_for_render(&mut self) {
        #[cfg(feature = "neo-term")]
        self.update_terminals();

        self.process_webkit_frames();
        self.process_video_frames();
        self.process_shader_surfaces();
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
        // Cheap change detection first: every frame install stamps a unique
        // ingest sequence, so the sorted (frame_id, seq) signature of the
        // contributing frames identifies the exact face-source set. An
        // unchanged signature means the aggregate face map cannot have
        // changed - the common case for cursor-blink and animation renders,
        // which used to rebuild and clone the whole map on every rendered
        // window.
        let mut signature: Vec<(u64, u64)> = Vec::new();
        self.frame_windows
            .for_each_top_level_window(|window_state| {
                let compositor = &window_state.render.compositor;
                if let Some(frame) = compositor.current_frame.as_ref() {
                    signature.push((
                        frame.frame_placement.frame().get(),
                        compositor.current_frame_ingest_seq,
                    ));
                }
                for entry in compositor.child_frames.frames.values() {
                    signature.push((entry.frame_id, entry.ingest_seq));
                }
            });
        signature.sort_unstable();
        if signature == self.faces_signature {
            return;
        }

        // One traversal covers every top-level window including the primary
        // (for_each_top_level_window iterates the full window map), so the
        // former second primary-window pass was pure duplication - and
        // panicked when no primary window existed.
        let mut faces = std::collections::HashMap::new();
        self.frame_windows
            .for_each_top_level_window(|window_state| {
                let compositor = &window_state.render.compositor;
                if let Some(frame) = compositor.current_frame.as_ref() {
                    for (face_id, face) in &frame.faces {
                        faces.entry(*face_id).or_insert_with(|| face.clone());
                    }
                }
                for entry in compositor.child_frames.frames.values() {
                    for (face_id, face) in &entry.frame.faces {
                        faces.entry(*face_id).or_insert_with(|| face.clone());
                    }
                }
            });

        // Glyph atlas entries go stale when a face id appears OR an existing
        // id changes attributes; the old id-set check missed the latter and
        // left stale glyphs rendered until some new face happened to appear.
        // A pure removal leaves no live glyphs behind and keeps the atlas.
        let faces_changed = faces
            .iter()
            .any(|(face_id, face)| self.faces.get(face_id) != Some(face));
        let old_face_count = self.faces.len();
        self.faces = faces;
        self.faces_signature = signature;
        if faces_changed {
            let face_count = self.faces.len();
            if let Some(primary_frame) = self
                .frame_windows
                .primary_window_mut()
                .map(|ws| &mut ws.render)
            {
                tracing::info!(
                    "Face definitions changed (old={}, new={}), clearing primary glyph cache",
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
                }
                | FrameGlyph::Surface {
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
