use super::{FpsCounter, RenderApp};
use crate::core::face::Face;
use crate::core::frame_glyphs::{DisplaySlotId, FrameGlyph, WindowCursor};
use crate::core::types::FaceId;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Instant;

const FACE_DIFF_SAMPLE_LIMIT: usize = 10;

/// Next unique frame-install stamp for the face aggregation signature.
pub(super) fn next_faces_ingest_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct FaceChangeSummary {
    pub(super) added: usize,
    pub(super) modified: usize,
    pub(super) removed: usize,
}

impl FaceChangeSummary {
    pub(super) fn invalidates_glyph_atlas(self) -> bool {
        self.added != 0 || self.modified != 0
    }
}

#[derive(Debug, PartialEq, Eq)]
struct FaceLabel {
    id: FaceId,
    name: String,
}

#[derive(Debug, PartialEq, Eq)]
struct ModifiedFaceDetail {
    id: FaceId,
    name: String,
    fields: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct FaceDiffDetails {
    added: Vec<FaceLabel>,
    modified: Vec<ModifiedFaceDetail>,
    removed: Vec<FaceLabel>,
    added_omitted: usize,
    modified_omitted: usize,
    removed_omitted: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct FaceSourceChange {
    frame_id: u64,
    old_ingest_sequences: Vec<u64>,
    new_ingest_sequences: Vec<u64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct FaceSourceDiffDetails {
    changes: Vec<FaceSourceChange>,
    omitted: usize,
}

#[derive(Debug)]
struct FaceOccurrence {
    face_id: FaceId,
    frame_id: u64,
    face: Face,
    sort_key: String,
}

#[derive(Debug, PartialEq, Eq)]
struct FaceIdConflict {
    face_id: FaceId,
    first_frame_id: u64,
    conflicting_frame_id: u64,
    first_name: String,
    conflicting_name: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct FaceConflictDetails {
    conflicts: Vec<FaceIdConflict>,
    omitted: usize,
}

pub(super) fn summarize_face_changes(
    old: &HashMap<FaceId, Face>,
    new: &HashMap<FaceId, Face>,
) -> FaceChangeSummary {
    let mut summary = FaceChangeSummary::default();
    for (face_id, face) in new {
        match old.get(face_id) {
            None => summary.added += 1,
            Some(old_face) if old_face != face => summary.modified += 1,
            Some(_) => {}
        }
    }
    summary.removed = old
        .keys()
        .filter(|face_id| !new.contains_key(face_id))
        .count();
    summary
}

fn face_name(face: &Face) -> String {
    face.lisp_name
        .clone()
        .unwrap_or_else(|| "<anonymous>".to_owned())
}

fn compact_json(value: &serde_json::Value) -> String {
    const VALUE_LIMIT: usize = 160;
    let rendered = value.to_string();
    if rendered.chars().count() <= VALUE_LIMIT {
        return rendered;
    }
    let prefix: String = rendered.chars().take(VALUE_LIMIT).collect();
    format!("{prefix}…")
}

fn changed_face_fields(old: &Face, new: &Face) -> Vec<String> {
    let Ok(serde_json::Value::Object(old_fields)) = serde_json::to_value(old) else {
        return vec!["<unable to serialize old face>".to_owned()];
    };
    let Ok(serde_json::Value::Object(new_fields)) = serde_json::to_value(new) else {
        return vec!["<unable to serialize new face>".to_owned()];
    };
    let field_names: BTreeSet<_> = old_fields.keys().chain(new_fields.keys()).collect();

    field_names
        .into_iter()
        .filter_map(|name| {
            let old_value = old_fields.get(name).unwrap_or(&serde_json::Value::Null);
            let new_value = new_fields.get(name).unwrap_or(&serde_json::Value::Null);
            (old_value != new_value).then(|| {
                format!(
                    "{name}={}->{}",
                    compact_json(old_value),
                    compact_json(new_value)
                )
            })
        })
        .collect()
}

pub(super) fn build_face_diff_details(
    old: &HashMap<FaceId, Face>,
    new: &HashMap<FaceId, Face>,
    limit: usize,
) -> FaceDiffDetails {
    let mut added: Vec<_> = new
        .iter()
        .filter(|(face_id, _)| !old.contains_key(face_id))
        .collect();
    added.sort_unstable_by_key(|(face_id, _)| face_id.get());

    let mut modified: Vec<_> = new
        .iter()
        .filter_map(|(face_id, face)| {
            old.get(face_id)
                .filter(|old_face| *old_face != face)
                .map(|old_face| (face_id, old_face, face))
        })
        .collect();
    modified.sort_unstable_by_key(|(face_id, _, _)| face_id.get());

    let mut removed: Vec<_> = old
        .iter()
        .filter(|(face_id, _)| !new.contains_key(face_id))
        .collect();
    removed.sort_unstable_by_key(|(face_id, _)| face_id.get());

    FaceDiffDetails {
        added_omitted: added.len().saturating_sub(limit),
        modified_omitted: modified.len().saturating_sub(limit),
        removed_omitted: removed.len().saturating_sub(limit),
        added: added
            .into_iter()
            .take(limit)
            .map(|(id, face)| FaceLabel {
                id: *id,
                name: face_name(face),
            })
            .collect(),
        modified: modified
            .into_iter()
            .take(limit)
            .map(|(id, old_face, new_face)| ModifiedFaceDetail {
                id: *id,
                name: face_name(new_face),
                fields: changed_face_fields(old_face, new_face),
            })
            .collect(),
        removed: removed
            .into_iter()
            .take(limit)
            .map(|(id, face)| FaceLabel {
                id: *id,
                name: face_name(face),
            })
            .collect(),
    }
}

pub(super) fn changed_face_sources(
    old: &[(u64, u64)],
    new: &[(u64, u64)],
    limit: usize,
) -> FaceSourceDiffDetails {
    fn by_frame(signature: &[(u64, u64)]) -> BTreeMap<u64, Vec<u64>> {
        let mut result = BTreeMap::<u64, Vec<u64>>::new();
        for &(frame_id, ingest_seq) in signature {
            result.entry(frame_id).or_default().push(ingest_seq);
        }
        result
    }

    let old_by_frame = by_frame(old);
    let new_by_frame = by_frame(new);
    let frame_ids: BTreeSet<_> = old_by_frame
        .keys()
        .chain(new_by_frame.keys())
        .copied()
        .collect();

    let changes: Vec<_> = frame_ids
        .into_iter()
        .filter_map(|frame_id| {
            let old_ingest_sequences = old_by_frame.get(&frame_id).cloned().unwrap_or_default();
            let new_ingest_sequences = new_by_frame.get(&frame_id).cloned().unwrap_or_default();
            (old_ingest_sequences != new_ingest_sequences).then_some(FaceSourceChange {
                frame_id,
                old_ingest_sequences,
                new_ingest_sequences,
            })
        })
        .collect();

    FaceSourceDiffDetails {
        omitted: changes.len().saturating_sub(limit),
        changes: changes.into_iter().take(limit).collect(),
    }
}

fn build_face_conflict_details(
    mut occurrences: Vec<FaceOccurrence>,
    limit: usize,
) -> FaceConflictDetails {
    occurrences.sort_unstable_by(|left, right| {
        (left.face_id.get(), left.frame_id, &left.sort_key).cmp(&(
            right.face_id.get(),
            right.frame_id,
            &right.sort_key,
        ))
    });

    let mut details = FaceConflictDetails::default();
    let mut group_start = 0;
    while group_start < occurrences.len() {
        let face_id = occurrences[group_start].face_id;
        let group_end = occurrences[group_start..]
            .iter()
            .position(|occurrence| occurrence.face_id != face_id)
            .map_or(occurrences.len(), |offset| group_start + offset);
        let first = &occurrences[group_start];
        for conflicting in &occurrences[group_start + 1..group_end] {
            if first.face != conflicting.face {
                if details.conflicts.len() < limit {
                    details.conflicts.push(FaceIdConflict {
                        face_id,
                        first_frame_id: first.frame_id,
                        conflicting_frame_id: conflicting.frame_id,
                        first_name: face_name(&first.face),
                        conflicting_name: face_name(&conflicting.face),
                    });
                } else {
                    details.omitted += 1;
                }
            }
        }
        group_start = group_end;
    }
    details
}

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

    fn for_each_face_source(&self, mut visit: impl FnMut(u64, u64, &HashMap<FaceId, Face>)) {
        self.frame_windows
            .for_each_top_level_window(|window_state| {
                let compositor = &window_state.render.compositor;
                if let Some(frame) = compositor.current_frame.as_ref() {
                    visit(
                        frame.frame_placement.frame().get(),
                        compositor.current_frame_ingest_seq,
                        &frame.faces,
                    );
                }
                for entry in compositor.child_frames.frames.values() {
                    visit(entry.frame_id, entry.ingest_seq, &entry.frame.faces);
                }
            });
    }

    fn collect_face_id_conflicts(&self, limit: usize) -> FaceConflictDetails {
        let mut occurrences = Vec::new();
        self.for_each_face_source(|frame_id, _ingest_seq, frame_faces| {
            for (face_id, face) in frame_faces {
                occurrences.push(FaceOccurrence {
                    face_id: *face_id,
                    frame_id,
                    face: face.clone(),
                    sort_key: serde_json::to_string(face).unwrap_or_else(|_| format!("{face:?}")),
                });
            }
        });
        build_face_conflict_details(occurrences, limit)
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
        self.for_each_face_source(|frame_id, ingest_seq, _faces| {
            signature.push((frame_id, ingest_seq));
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
        self.for_each_face_source(|_frame_id, _ingest_seq, frame_faces| {
            for (face_id, face) in frame_faces {
                faces.entry(*face_id).or_insert_with(|| face.clone());
            }
        });

        // Glyph atlas entries go stale when a face id appears OR an existing
        // id changes attributes; the old id-set check missed the latter and
        // left stale glyphs rendered until some new face happened to appear.
        // A pure removal leaves no live glyphs behind and keeps the atlas.
        let summary = summarize_face_changes(&self.faces, &faces);

        if !summary.invalidates_glyph_atlas() {
            if tracing::enabled!(tracing::Level::DEBUG) {
                let details = build_face_diff_details(&self.faces, &faces, FACE_DIFF_SAMPLE_LIMIT);
                let source_changes =
                    changed_face_sources(&self.faces_signature, &signature, FACE_DIFF_SAMPLE_LIMIT);
                let conflicts = self.collect_face_id_conflicts(FACE_DIFF_SAMPLE_LIMIT);
                tracing::debug!(
                    event = "face_table_update_without_invalidation",
                    faces_old = self.faces.len(),
                    faces_new = faces.len(),
                    faces_removed = summary.removed,
                    removed = ?details.removed,
                    removed_omitted = details.removed_omitted,
                    source_changes = ?source_changes.changes,
                    source_changes_omitted = source_changes.omitted,
                    face_id_conflicts = ?conflicts.conflicts,
                    face_id_conflicts_omitted = conflicts.omitted,
                    "face-table source update did not require glyph-atlas invalidation"
                );
            }
            self.faces = faces;
            self.faces_signature = signature;
            return;
        }

        let debug_diagnostics = tracing::enabled!(tracing::Level::DEBUG).then(|| {
            (
                build_face_diff_details(&self.faces, &faces, FACE_DIFF_SAMPLE_LIMIT),
                changed_face_sources(&self.faces_signature, &signature, FACE_DIFF_SAMPLE_LIMIT),
                self.collect_face_id_conflicts(FACE_DIFF_SAMPLE_LIMIT),
            )
        });

        let old_face_count = self.faces.len();
        let old_source_count = self.faces_signature.len();
        let new_source_count = signature.len();
        self.faces = faces;
        self.faces_signature = signature;
        let clear_stats = self.frame_windows.clear_top_level_glyph_atlases();

        let now = Instant::now();
        let since_previous_ms = self.last_face_cache_invalidation.map(|previous| {
            now.saturating_duration_since(previous)
                .as_millis()
                .min(u64::MAX as u128) as u64
        });
        let has_previous_invalidation = since_previous_ms.is_some();
        let since_previous_ms = since_previous_ms.unwrap_or_default();
        self.last_face_cache_invalidation = Some(now);
        self.face_cache_invalidation_seq = self.face_cache_invalidation_seq.wrapping_add(1);

        tracing::info!(
            event = "glyph_atlas_invalidated",
            invalidation_seq = self.face_cache_invalidation_seq,
            faces_old = old_face_count,
            faces_new = self.faces.len(),
            faces_added = summary.added,
            faces_modified = summary.modified,
            faces_removed = summary.removed,
            face_sources_old = old_source_count,
            face_sources_new = new_source_count,
            windows_visited = clear_stats.windows_visited,
            atlases_cleared = clear_stats.atlases_cleared,
            glyphs_evicted = clear_stats.glyphs_evicted,
            alpha_pages_cleared = clear_stats.alpha_pages_cleared,
            subpixel_pages_cleared = clear_stats.subpixel_pages_cleared,
            color_pages_cleared = clear_stats.color_pages_cleared,
            has_previous_invalidation,
            since_previous_ms,
            "glyph atlases invalidated after face-table update"
        );

        if let Some((details, source_changes, conflicts)) = debug_diagnostics {
            tracing::debug!(
                event = "face_table_diff",
                added = ?details.added,
                added_omitted = details.added_omitted,
                modified = ?details.modified,
                modified_omitted = details.modified_omitted,
                removed = ?details.removed,
                removed_omitted = details.removed_omitted,
                source_changes = ?source_changes.changes,
                source_changes_omitted = source_changes.omitted,
                face_id_conflicts = ?conflicts.conflicts,
                face_id_conflicts_omitted = conflicts.omitted,
                "face-table changes that invalidated glyph atlases"
            );
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
