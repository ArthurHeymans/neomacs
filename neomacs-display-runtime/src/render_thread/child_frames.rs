//! Child frame management for the render thread.
//!
//! Manages child frames (posframe, which-key-posframe, etc.) as floating
//! overlays composited on top of the parent frame within a single winit window.

use std::collections::HashMap;

use crate::core::frame_glyphs::FrameGlyphBuffer;
use neomacs_display_protocol::{
    DisplayFrameId, FrameRect, PlaceChildQuery, PresentedFramePlacement, PresentedFrameScene,
};

/// State for one child frame.
pub(crate) struct ChildFrameEntry {
    pub frame_id: u64,
    pub frame: FrameGlyphBuffer,
    /// Computed absolute position on screen (from parent_x/parent_y)
    pub abs_x: f32,
    pub abs_y: f32,
    pub clip_in_root: Option<FrameRect>,
    pub z_path: Vec<i32>,
    /// Frame counter when this entry was last updated
    #[allow(dead_code)] // read by the test-exercised prune_stale
    pub last_updated: u64,
    /// Unique per-install stamp; the face aggregation signature uses it to
    /// detect that this entry's frame payload was replaced.
    pub ingest_seq: u64,
}

/// Manages all child frames for the render thread.
pub(crate) struct ChildFrameManager {
    pub frames: HashMap<u64, ChildFrameEntry>,
    /// Frame IDs sorted by z_order for rendering (lowest first = back-most)
    render_order: Vec<u64>,
    /// Monotonic counter incremented each poll_frame cycle
    frame_counter: u64,
    root: Option<PresentedFramePlacement>,
}

impl ChildFrameManager {
    pub fn new() -> Self {
        Self {
            frames: HashMap::new(),
            render_order: Vec::new(),
            frame_counter: 0,
            root: None,
        }
    }

    pub fn set_root_frame(&mut self, root: Option<&FrameGlyphBuffer>) {
        self.root = root.map(|root| root.frame_placement);
        self.rebuild_presented_scene();
    }

    /// Increment the frame counter. Call once per poll_frame cycle.
    pub fn tick(&mut self) {
        self.frame_counter += 1;
    }

    /// Insert or update a child frame, recompute absolute position, rebuild render order.
    ///
    /// Returns true only when the rendered payload changed. Repeated delivery of
    /// an identical child-frame snapshot still refreshes liveness, but it must
    /// not look like a new frame install to face-cache and dirty-redraw logic.
    pub fn update_frame(&mut self, buf: FrameGlyphBuffer) -> bool {
        let frame_id = buf.frame_placement.frame();
        let outer = buf.frame_placement.outer_in_parent();
        let abs_x = outer.x();
        let abs_y = outer.y();
        let z_order = buf.frame_placement.z_order();
        let existing = self.frames.get_mut(&frame_id.get());

        let glyph_count = buf.glyphs.len();
        if let Some(entry) = existing
            && entry.frame == buf
        {
            entry.last_updated = self.frame_counter;
            tracing::debug!(
                frame_id = frame_id.get(),
                abs_x,
                abs_y,
                width = buf.width,
                height = buf.height,
                z_order,
                glyphs = glyph_count,
                "child_frame_lifecycle: render_thread_child_buffer_unchanged"
            );
            return false;
        }

        let existed = self.frames.contains_key(&frame_id.get());
        tracing::debug!(
            frame_id = frame_id.get(),
            abs_x,
            abs_y,
            width = buf.width,
            height = buf.height,
            z_order,
            glyphs = glyph_count,
            existed,
            "child_frame_lifecycle: render_thread_child_buffer"
        );

        self.frames.insert(
            frame_id.get(),
            ChildFrameEntry {
                frame_id: frame_id.get(),
                frame: buf,
                abs_x,
                abs_y,
                clip_in_root: None,
                z_path: vec![z_order],
                last_updated: self.frame_counter,
                ingest_seq: super::frame_state::next_faces_ingest_seq(),
            },
        );

        self.rebuild_presented_scene();
        true
    }

    /// Remove a child frame by ID.
    pub fn remove_frame(&mut self, frame_id: u64) -> bool {
        if self.frames.remove(&frame_id).is_some() {
            self.rebuild_presented_scene();
            tracing::info!(
                frame_id,
                "child_frame_lifecycle: render_thread_child_removed"
            );
            true
        } else {
            tracing::debug!(
                frame_id,
                "child_frame_lifecycle: render_thread_child_remove_missing"
            );
            false
        }
    }

    /// Remove child frames not updated in the last `max_age` poll cycles.
    #[allow(dead_code)] // child-frame staleness API, exercised by the child_frames tests
    pub fn prune_stale(&mut self, max_age: u64) {
        let threshold = self.frame_counter.saturating_sub(max_age);
        let before = self.frames.len();
        self.frames
            .retain(|_, entry| entry.last_updated >= threshold);
        if self.frames.len() != before {
            self.rebuild_presented_scene();
        }
    }

    /// Get the z_order-sorted list of frame IDs for rendering.
    pub fn sorted_for_rendering(&self) -> &[u64] {
        &self.render_order
    }

    /// Hit test: find the topmost child frame at the given point.
    /// Returns (frame_id: frame_id.get(), local_x, local_y) if hit, None otherwise.
    /// Iterates in reverse render order (topmost first).
    pub fn hit_test(&self, x: f32, y: f32) -> Option<(u64, f32, f32)> {
        for &frame_id in self.render_order.iter().rev() {
            if let Some(entry) = self.frames.get(&frame_id) {
                let local_x = x - entry.abs_x;
                let local_y = y - entry.abs_y;
                if local_x >= 0.0
                    && local_y >= 0.0
                    && local_x < entry.frame.width
                    && local_y < entry.frame.height
                    && entry.clip_in_root.is_none_or(|clip| {
                        x >= clip.x()
                            && y >= clip.y()
                            && x < clip.x() + clip.width()
                            && y < clip.y() + clip.height()
                    })
                {
                    return Some((frame_id, local_x, local_y));
                }
            }
        }
        None
    }

    /// Whether there are any child frames.
    #[allow(dead_code)] // exercised by the child_frames tests
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Rebuild the render order from z_order values.
    fn rebuild_render_order(&mut self) {
        self.render_order.clear();
        self.render_order.extend(self.frames.keys());
        // Sort by z_order ascending (lowest z = rendered first = behind)
        self.render_order.sort_by(|a, b| {
            let za = self
                .frames
                .get(a)
                .map(|e| e.z_path.as_slice())
                .unwrap_or(&[]);
            let zb = self
                .frames
                .get(b)
                .map(|e| e.z_path.as_slice())
                .unwrap_or(&[]);
            za.cmp(zb).then(a.cmp(b))
        });
    }

    fn rebuild_presented_scene(&mut self) {
        let child_ids = self
            .frames
            .keys()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let root_id = self.root.map(|root| root.frame().get());
        let mut placements = self.root.into_iter().collect::<Vec<_>>();
        placements.extend(self.frames.values().map(|entry| {
            let placement = entry.frame.frame_placement;
            let raw_parent = placement.parent().map(DisplayFrameId::get);
            if raw_parent
                .is_some_and(|parent| Some(parent) != root_id && !child_ids.contains(&parent))
            {
                PresentedFramePlacement::new(
                    placement.frame(),
                    placement.presentation(),
                    None,
                    placement.outer_in_parent(),
                    placement.z_order(),
                )
            } else {
                placement
            }
        }));
        let Ok(scene) = PresentedFrameScene::from_placements(placements) else {
            tracing::error!("rejecting incoherent child-frame ancestry");
            return;
        };
        for entry in self.frames.values_mut() {
            let Ok(placed) = scene.place(PlaceChildQuery::new(
                entry.frame.frame_placement.frame(),
                entry.frame.frame_placement.presentation(),
            )) else {
                continue;
            };
            entry.abs_x = placed.root_relative().x();
            entry.abs_y = placed.root_relative().y();
            entry.clip_in_root = placed.clip_in_root();
            entry.z_path = placed.z_path().to_vec();
        }
        self.rebuild_render_order();
    }
}

#[cfg(test)]
#[path = "child_frames_test.rs"]
mod tests;
