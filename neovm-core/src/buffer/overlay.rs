//! Overlay system for buffers.
//!
//! GNU Emacs exposes overlays as first-class Lisp objects whose identity
//! outlives deletion. The buffer owns the interval index, while the overlay
//! object owns plist, buffer membership, and endpoint state. NeoVM models that
//! split by keeping overlay objects on the GC heap and storing only live object
//! ids in each buffer's overlay index.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Unbounded};

use crate::buffer::BufferId;
use crate::emacs_core::error::Flow;
use crate::emacs_core::plist;
use crate::emacs_core::value::{Value, ValueKind, eq_value};
use crate::gc_trace::GcTrace;
use crate::heap_types::OverlayData;

use super::position::{EmacsByteLen, EmacsBytePos, EmacsByteRange};
use super::text::{TextEditRange, TextInsertion, TextReplacement};

pub type Overlay = OverlayData;

#[cfg(test)]
static OVERLAYS_AT_NODE_VISITS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn reset_overlays_at_node_visit_count() {
    OVERLAYS_AT_NODE_VISITS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn overlays_at_node_visit_count() -> usize {
    OVERLAYS_AT_NODE_VISITS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Augmented interval tree node for O(log n + k) overlay queries.
#[derive(Clone, Debug)]
struct ItreeNode {
    range: EmacsByteRange,
    /// Maximum `end` value in this subtree (augmented).
    max_end: EmacsBytePos,
    /// The overlay object.
    overlay: Value,
    left: Option<Box<ItreeNode>>,
    right: Option<Box<ItreeNode>>,
}

impl ItreeNode {
    fn new(range: EmacsByteRange, overlay: Value) -> Self {
        Self {
            range,
            max_end: range.end(),
            overlay,
            left: None,
            right: None,
        }
    }

    fn start(&self) -> EmacsBytePos {
        self.range.start()
    }

    fn end(&self) -> EmacsBytePos {
        self.range.end()
    }

    fn set_range(&mut self, range: EmacsByteRange) {
        self.range = range;
        self.update_max_end();
    }

    fn update_max_end(&mut self) {
        self.max_end = self.end();
        if let Some(ref left) = self.left {
            self.max_end = self.max_end.max(left.max_end);
        }
        if let Some(ref right) = self.right {
            self.max_end = self.max_end.max(right.max_end);
        }
    }
}

/// BST-based augmented interval tree.  Not self-balancing, but overlay
/// counts are typically small (< 1000), so depth is acceptable.
#[derive(Clone, Debug)]
struct Itree {
    root: Option<Box<ItreeNode>>,
}

impl Itree {
    fn new() -> Self {
        Self { root: None }
    }

    /// Collect all overlays that cover `pos` into `out`.
    fn overlays_at(&self, pos: EmacsBytePos, out: &mut Vec<Value>) {
        Self::overlays_at_node(&self.root, pos, out);
    }

    /// Collect every overlay in the tree in ascending `begin` (start) order.
    ///
    /// GNU's `Foverlay_lists` walks `current_buffer->overlays` with
    /// `ITREE_FOREACH (node, ..., BEG, Z, DESCENDING)` and conses each node
    /// onto the result, which reverses the descending walk back into ascending
    /// `begin` order. An in-order (left, self, right) traversal of this
    /// `begin`-keyed BST yields the same ascending sequence directly.
    fn all_overlays_ascending(&self, out: &mut Vec<Value>) {
        Self::all_overlays_ascending_node(&self.root, out);
    }

    fn all_overlays_ascending_node(node: &Option<Box<ItreeNode>>, out: &mut Vec<Value>) {
        let Some(n) = node.as_ref() else { return };
        Self::all_overlays_ascending_node(&n.left, out);
        out.push(n.overlay);
        Self::all_overlays_ascending_node(&n.right, out);
    }

    /// Collect all overlays overlapping `start..end` in GNU's interval-tree
    /// ascending traversal order.
    fn overlays_in_region(
        &self,
        range: EmacsByteRange,
        accessible_end: EmacsBytePos,
        out: &mut Vec<Value>,
    ) {
        Self::overlays_in_region_node(&self.root, range, accessible_end, out);
    }

    fn overlays_at_node(node: &Option<Box<ItreeNode>>, pos: EmacsBytePos, out: &mut Vec<Value>) {
        let Some(n) = node.as_ref() else { return };
        #[cfg(test)]
        OVERLAYS_AT_NODE_VISITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // If left child's max_end > pos, it might contain covering intervals
        if let Some(left) = n.left.as_ref() {
            if left.max_end > pos {
                Self::overlays_at_node(&n.left, pos, out);
            }
        }
        // Check current node
        if n.start() <= pos && pos < n.end() {
            out.push(n.overlay);
        }
        if let Some(right) = n.right.as_ref() {
            if n.start() <= pos && right.max_end > pos {
                Self::overlays_at_node(&n.right, pos, out);
            }
        }
    }

    fn overlays_in_region_node(
        node: &Option<Box<ItreeNode>>,
        range: EmacsByteRange,
        accessible_end: EmacsBytePos,
        out: &mut Vec<Value>,
    ) {
        let Some(n) = node.as_ref() else { return };

        if let Some(left) = n.left.as_ref()
            && left.max_end >= range.start()
        {
            Self::overlays_in_region_node(&n.left, range, accessible_end, out);
        }

        if n.start() > range.end() {
            return;
        }

        if overlay_overlaps_region(n.overlay, range, accessible_end) {
            out.push(n.overlay);
        }

        Self::overlays_in_region_node(&n.right, range, accessible_end, out);
    }

    /// Insert an interval.  Returns false if already present.
    fn insert(&mut self, range: EmacsByteRange, overlay: Value) -> bool {
        Self::insert_node(&mut self.root, range, overlay)
    }

    fn insert_node(
        node: &mut Option<Box<ItreeNode>>,
        range: EmacsByteRange,
        overlay: Value,
    ) -> bool {
        match node {
            None => {
                *node = Some(Box::new(ItreeNode::new(range, overlay)));
                true
            }
            Some(n) => {
                if eq_value(&n.overlay, &overlay) {
                    n.set_range(range);
                    return false;
                }
                // GNU's `itree_insert_node` descends left for
                // `node->begin <= child->begin`, so same-begin overlays are
                // visited newest-first by the ascending iterator used by
                // `overlays-at`.
                if range.start() <= n.start() {
                    let inserted = Self::insert_node(&mut n.left, range, overlay);
                    if inserted {
                        n.update_max_end();
                    }
                    inserted
                } else {
                    let inserted = Self::insert_node(&mut n.right, range, overlay);
                    if inserted {
                        n.update_max_end();
                    }
                    inserted
                }
            }
        }
    }

    /// Remove an overlay from the tree.
    fn remove(&mut self, overlay: Value) -> bool {
        let Some(range) = overlay_range(overlay) else {
            return false;
        };
        self.remove_at(range.start(), overlay)
    }

    fn remove_at(&mut self, start: EmacsBytePos, overlay: Value) -> bool {
        Self::remove_node(&mut self.root, start, overlay)
    }

    fn remove_node(node: &mut Option<Box<ItreeNode>>, start: EmacsBytePos, overlay: Value) -> bool {
        // Take ownership of the node, operate on it, put back
        let Some(mut boxed) = node.take() else {
            return false;
        };

        if boxed.start() == start && eq_value(&boxed.overlay, &overlay) {
            // Remove this node, replace with merged children.
            let left = boxed.left.take();
            let right = boxed.right.take();
            *node = match (left, right) {
                (None, None) => None,
                (Some(child), None) | (None, Some(child)) => Some(child),
                (Some(left), Some(right)) => {
                    let mut merged = left;
                    Self::attach_rightmost(&mut merged, right);
                    Some(merged)
                }
            };
            return true;
        }

        let found = if start < boxed.start() {
            Self::remove_node(&mut boxed.left, start, overlay)
        } else if start > boxed.start() {
            Self::remove_node(&mut boxed.right, start, overlay)
        } else {
            Self::remove_node(&mut boxed.left, start, overlay)
                || Self::remove_node(&mut boxed.right, start, overlay)
        };
        if found {
            boxed.update_max_end();
        }
        *node = Some(boxed);
        found
    }

    fn attach_rightmost(node: &mut Box<ItreeNode>, child: Box<ItreeNode>) {
        if node.right.is_none() {
            node.right = Some(child);
        } else {
            Self::attach_rightmost(node.right.as_mut().unwrap(), child);
        }
        node.update_max_end();
    }
}

#[derive(Clone)]
pub struct OverlayList {
    overlays: BTreeSet<Value>,
    by_start: BTreeMap<EmacsBytePos, BTreeSet<Value>>,
    by_end: BTreeMap<EmacsBytePos, BTreeSet<Value>>,
    /// Augmented interval tree for O(log n + k) overlays_at queries.
    itree: Itree,
}

impl OverlayList {
    pub fn new() -> Self {
        Self {
            overlays: BTreeSet::new(),
            by_start: BTreeMap::new(),
            by_end: BTreeMap::new(),
            itree: Itree::new(),
        }
    }

    pub fn insert_overlay(&mut self, overlay: Value) {
        let data = overlay.as_overlay_data().unwrap();
        let range = overlay_data_range(data);
        if !self.overlays.insert(overlay) {
            return;
        }
        Self::insert_index_entry(&mut self.by_start, range.start(), overlay);
        Self::insert_index_entry(&mut self.by_end, range.end(), overlay);
        self.itree.insert(range, overlay);
    }

    pub fn detach_overlay(&mut self, overlay: Value) -> bool {
        if !self.overlays.remove(&overlay) {
            return false;
        }
        if let Some(range) = overlay_range(overlay) {
            Self::remove_index_entry(&mut self.by_start, range.start(), overlay);
            Self::remove_index_entry(&mut self.by_end, range.end(), overlay);
        }
        self.itree.remove(overlay);
        true
    }

    pub fn delete_overlay(&mut self, overlay: Value) -> bool {
        if !self.detach_overlay(overlay) {
            return false;
        }
        let _ = overlay.with_overlay_data_mut(|data| {
            data.buffer = None;
        });
        true
    }

    pub fn delete_all_overlays(&mut self) {
        let live: Vec<Value> = self.overlays.iter().copied().collect();
        for overlay in live {
            let _ = overlay.with_overlay_data_mut(|data| {
                data.buffer = None;
            });
        }
        self.overlays.clear();
        self.by_start.clear();
        self.by_end.clear();
        self.itree = Itree::new();
    }

    pub(crate) fn retarget_buffer(&mut self, from: BufferId, to: BufferId) {
        for overlay in self.overlays.iter().copied() {
            let _ = overlay.with_overlay_data_mut(|data| {
                if data.buffer == Some(from) {
                    data.buffer = Some(to);
                }
            });
        }
    }

    pub fn overlay_put(&mut self, overlay: Value, prop: Value, value: Value) -> Result<bool, Flow> {
        overlay
            .with_overlay_data_mut(|data| {
                let (plist, changed) = overlay_plist_put(data.plist, prop, value);
                data.plist = plist;
                Ok::<bool, Flow>(changed)
            })
            .unwrap()
    }

    pub fn overlay_get(&self, overlay: Value, prop: &Value) -> Option<Value> {
        plist::plist_get(overlay.as_overlay_data().unwrap().plist, prop)
    }

    pub fn overlay_get_named(&self, overlay: Value, prop_name: Value) -> Option<Value> {
        overlay_property_named(overlay, prop_name)
    }

    pub fn overlay_plist(&self, overlay: Value) -> Option<Value> {
        if self.overlays.contains(&overlay) || overlay_live_buffer(overlay).is_none() {
            return Some(overlay.as_overlay_data().unwrap().plist);
        }
        None
    }

    pub fn overlay_start_emacs_byte_pos(&self, overlay: Value) -> Option<EmacsBytePos> {
        if overlay_live_buffer(overlay).is_none() {
            return None;
        }
        overlay_range(overlay).map(EmacsByteRange::start)
    }

    pub fn overlay_end_emacs_byte_pos(&self, overlay: Value) -> Option<EmacsBytePos> {
        if overlay_live_buffer(overlay).is_none() {
            return None;
        }
        overlay_range(overlay).map(EmacsByteRange::end)
    }

    pub fn move_overlay_to_emacs_byte_range(&mut self, overlay: Value, range: EmacsByteRange) {
        let Some(old_range) = overlay_range(overlay) else {
            return;
        };
        Self::remove_index_entry(&mut self.by_start, old_range.start(), overlay);
        Self::remove_index_entry(&mut self.by_end, old_range.end(), overlay);
        Self::insert_index_entry(&mut self.by_start, range.start(), overlay);
        Self::insert_index_entry(&mut self.by_end, range.end(), overlay);
        self.itree.remove_at(old_range.start(), overlay);
        let _ = overlay.with_overlay_data_mut(|data| {
            data.start = range.start().get();
            data.end = range.end().get();
        });
        self.itree.insert(range, overlay);
        // GNU Emacs drops empty overlays created by move-overlay when
        // `evaporate' is non-nil. Minibuffer shadow overlays depend on this
        // to avoid leaking stale before/after-strings into later prompts.
        if range.is_empty()
            && self
                .overlay_get_named(overlay, Value::symbol("evaporate"))
                .is_some_and(|value| value.is_truthy())
        {
            let _ = self.delete_overlay(overlay);
        }
    }

    pub fn overlays_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Vec<Value> {
        let mut overlays = Vec::new();
        self.itree.overlays_at(pos, &mut overlays);
        overlays
    }

    pub fn overlays_in_emacs_byte_range(&self, range: EmacsByteRange) -> Vec<Value> {
        self.overlays_in_accessible_emacs_byte_range(range, range.end())
    }

    /// Return every live overlay of this buffer in GNU's `overlay-lists` order.
    ///
    /// Mirrors `Foverlay_lists` (buffer.c): the buffer's interval tree is
    /// walked `BEG..Z` descending and consed, producing all overlays in
    /// ascending `begin` order. Used to build the `(BEFORE . AFTER)` pair that
    /// `overlay-lists` returns; since Emacs 29.1 the "overlay center" is gone,
    /// so every overlay lands in the `BEFORE` (car) list and the `AFTER` (cdr)
    /// list is always empty.
    pub fn overlays_in_gnu_lists_order(&self) -> Vec<Value> {
        let mut overlays = Vec::with_capacity(self.overlays.len());
        self.itree.all_overlays_ascending(&mut overlays);
        overlays
    }

    pub fn overlays_in_accessible_emacs_byte_range(
        &self,
        range: EmacsByteRange,
        accessible_end: EmacsBytePos,
    ) -> Vec<Value> {
        let mut overlays = Vec::new();
        self.itree
            .overlays_in_region(range, accessible_end, &mut overlays);
        overlays
    }

    pub fn highest_priority_overlay_at_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        property: Value,
    ) -> Option<Value> {
        self.best_overlay_for(property, |overlay| overlay_covers_pos(overlay, pos))
    }

    pub fn highest_priority_overlay_for_inserted_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        property: &Value,
    ) -> Option<Value> {
        self.best_overlay_for(*property, |overlay| {
            let Some(data) = overlay.as_overlay_data() else {
                return false;
            };
            if data.buffer.is_none() {
                return false;
            }
            let range = overlay_data_range(data);
            !(range.start() == pos && data.front_advance)
                && !(range.end() == pos && !data.rear_advance)
                && range.start() <= pos
                && pos <= range.end()
        })
    }

    pub fn sort_overlay_ids_by_priority_desc(&self, overlay_ids: &mut [Value]) {
        overlay_ids.sort_by(|left, right| compare_overlay_precedence(*right, *left));
    }

    pub fn adjust_for_insert_at_emacs_byte_pos(
        &mut self,
        pos: EmacsBytePos,
        len: EmacsByteLen,
        before_markers: bool,
    ) {
        if len.is_empty() {
            return;
        }
        let live: Vec<Value> = self.overlays.iter().copied().collect();
        for overlay in &live {
            let _ = overlay.with_overlay_data_mut(|object| {
                let start = EmacsBytePos::new(object.start);
                let end = EmacsBytePos::new(object.end);
                let empty = start == end;

                if before_markers {
                    if start >= pos {
                        object.start += len.get();
                    }
                    if end >= pos {
                        object.end += len.get();
                    }
                    return;
                }

                if start > pos
                    || (start == pos && object.front_advance && (!empty || object.rear_advance))
                {
                    object.start += len.get();
                }

                if end > pos || (end == pos && object.rear_advance) {
                    object.end += len.get();
                }
            });
        }
        self.rebuild_indexes();
    }

    pub fn adjust_for_inserted_text(&mut self, insertion: TextInsertion, before_markers: bool) {
        self.adjust_for_insert_at_emacs_byte_pos(
            insertion.byte_pos(),
            insertion.extent().emacs_bytes(),
            before_markers,
        );
    }

    pub fn adjust_for_delete_emacs_byte_range(&mut self, range: EmacsByteRange) {
        if range.is_empty() {
            return;
        }
        let start = range.start();
        let end = range.end();
        let len = range.len();
        let live: Vec<Value> = self.overlays.iter().copied().collect();
        let mut evaporated = Vec::new();
        for overlay in &live {
            let should_evaporate = overlay
                .with_overlay_data_mut(|object| {
                    let object_start = EmacsBytePos::new(object.start);
                    let object_end = EmacsBytePos::new(object.end);
                    if object_start >= end {
                        object.start -= len.get();
                    } else if object_start > start {
                        object.start = start.get();
                    }

                    if object_end >= end {
                        object.end -= len.get();
                    } else if object_end > start {
                        object.end = start.get();
                    }

                    if object.start == object.end
                        && plist::plist_get(object.plist, &Value::symbol("evaporate"))
                            .is_some_and(|v| v.is_truthy())
                    {
                        object.buffer = None;
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false);
            if should_evaporate {
                evaporated.push(*overlay);
            }
        }

        for overlay in evaporated {
            self.overlays.remove(&overlay);
        }
        self.rebuild_indexes();
    }

    pub fn adjust_for_deleted_text(&mut self, range: TextEditRange) {
        self.adjust_for_delete_emacs_byte_range(range.byte_range());
    }

    pub fn adjust_for_replace_at_emacs_byte_pos(
        &mut self,
        start: EmacsBytePos,
        old_len: EmacsByteLen,
        new_len: EmacsByteLen,
    ) {
        if old_len.is_empty() {
            self.adjust_for_insert_at_emacs_byte_pos(start, new_len, false);
            return;
        }

        self.adjust_for_insert_at_emacs_byte_pos(start.add_len(old_len), new_len, true);
        self.adjust_for_delete_emacs_byte_range(EmacsByteRange::from_start_len(start, old_len));
    }

    pub fn adjust_for_replaced_text(&mut self, replacement: TextReplacement) {
        self.adjust_for_replace_at_emacs_byte_pos(
            replacement.byte_start(),
            replacement.old_byte_len(),
            replacement.new_byte_len(),
        );
    }

    pub fn set_front_advance(&mut self, overlay: Value, advance: bool) {
        let _ = overlay.with_overlay_data_mut(|data| {
            data.front_advance = advance;
        });
    }

    pub fn set_rear_advance(&mut self, overlay: Value, advance: bool) {
        let _ = overlay.with_overlay_data_mut(|data| {
            data.rear_advance = advance;
        });
    }

    pub fn get(&self, overlay: Value) -> Option<Overlay> {
        self.overlays
            .contains(&overlay)
            .then(|| overlay.as_overlay_data().unwrap().clone())
    }

    pub fn len(&self) -> usize {
        self.overlays.len()
    }

    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    pub fn next_boundary_after_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<EmacsBytePos> {
        self.next_boundary_after_until_emacs_byte_pos(pos, EmacsBytePos::new(usize::MAX))
    }

    pub fn next_boundary_after_until_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        limit: EmacsBytePos,
    ) -> Option<EmacsBytePos> {
        if pos >= limit {
            return None;
        }
        let next_start = self
            .by_start
            .range((Excluded(pos), Unbounded))
            .next()
            .map(|(boundary, _)| *boundary);
        let next_end = self
            .by_end
            .range((Excluded(pos), Unbounded))
            .next()
            .map(|(boundary, _)| *boundary);
        let boundary = match (next_start, next_end) {
            (Some(start), Some(end)) => Some(start.min(end)),
            (Some(start), None) => Some(start),
            (None, Some(end)) => Some(end),
            (None, None) => None,
        };
        boundary.filter(|boundary| *boundary <= limit)
    }

    pub fn previous_boundary_before_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
    ) -> Option<EmacsBytePos> {
        self.previous_boundary_before_since_emacs_byte_pos(pos, EmacsBytePos::ZERO)
    }

    pub fn previous_boundary_before_since_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        limit: EmacsBytePos,
    ) -> Option<EmacsBytePos> {
        if pos <= limit {
            return None;
        }
        let prev_start = self
            .by_start
            .range(..pos)
            .next_back()
            .map(|(boundary, _)| *boundary);
        let prev_end = self
            .by_end
            .range(..pos)
            .next_back()
            .map(|(boundary, _)| *boundary);
        let boundary = match (prev_start, prev_end) {
            (Some(start), Some(end)) => Some(start.max(end)),
            (Some(start), None) => Some(start),
            (None, Some(end)) => Some(end),
            (None, None) => None,
        };
        boundary.filter(|boundary| *boundary >= limit)
    }

    pub(crate) fn dump_overlays(&self) -> Vec<Value> {
        self.overlays.iter().copied().collect()
    }

    pub(crate) fn from_dump(overlays: Vec<Value>) -> Self {
        let mut list = Self::new();
        for overlay in overlays {
            if overlay_live_buffer(overlay).is_some() {
                list.insert_overlay(overlay);
            }
        }
        list
    }

    fn best_overlay_for<F>(&self, property: Value, predicate: F) -> Option<Value>
    where
        F: Fn(Value) -> bool,
    {
        let mut best: Option<Value> = None;
        for overlay in &self.overlays {
            if !predicate(*overlay) {
                continue;
            }
            let Some(value) = overlay_property_named(*overlay, property) else {
                continue;
            };
            if value.is_nil() {
                continue;
            }
            match best {
                None => best = Some(*overlay),
                Some(current)
                    if compare_overlay_precedence(current, *overlay) == Ordering::Less =>
                {
                    best = Some(*overlay);
                }
                _ => {}
            }
        }
        best
    }

    fn insert_index_entry(
        index: &mut BTreeMap<EmacsBytePos, BTreeSet<Value>>,
        boundary: EmacsBytePos,
        overlay: Value,
    ) {
        index.entry(boundary).or_default().insert(overlay);
    }

    fn remove_index_entry(
        index: &mut BTreeMap<EmacsBytePos, BTreeSet<Value>>,
        boundary: EmacsBytePos,
        overlay: Value,
    ) {
        if let Some(ids) = index.get_mut(&boundary) {
            ids.remove(&overlay);
            if ids.is_empty() {
                index.remove(&boundary);
            }
        }
    }

    fn rebuild_indexes(&mut self) {
        self.by_start.clear();
        self.by_end.clear();
        self.itree = Itree::new();
        let live: Vec<Value> = self.overlays.iter().copied().collect();
        for overlay in live {
            if overlay_live_buffer(overlay).is_none() {
                self.overlays.remove(&overlay);
                continue;
            }
            if let Some(range) = overlay_range(overlay) {
                Self::insert_index_entry(&mut self.by_start, range.start(), overlay);
                Self::insert_index_entry(&mut self.by_end, range.end(), overlay);
                self.itree.insert(range, overlay);
            }
        }
    }
}

fn overlay_live_buffer(overlay: Value) -> Option<crate::buffer::BufferId> {
    overlay.as_overlay_data().and_then(|d| d.buffer)
}

fn overlay_data_range(data: &OverlayData) -> EmacsByteRange {
    EmacsByteRange::new(EmacsBytePos::new(data.start), EmacsBytePos::new(data.end))
}

fn overlay_range(overlay: Value) -> Option<EmacsByteRange> {
    let data = overlay.as_overlay_data()?;
    data.buffer.map(|_| overlay_data_range(data))
}

fn overlay_covers_pos(overlay: Value, pos: EmacsBytePos) -> bool {
    let Some(data) = overlay.as_overlay_data() else {
        return false;
    };
    if data.buffer.is_none() {
        return false;
    }
    let range = overlay_data_range(data);
    range.start() <= pos && pos < range.end()
}

fn overlay_overlaps_region(
    overlay: Value,
    range: EmacsByteRange,
    accessible_end: EmacsBytePos,
) -> bool {
    let Some(data) = overlay.as_overlay_data() else {
        return false;
    };
    if data.buffer.is_none() {
        return false;
    }
    let overlay_range = overlay_data_range(data);
    if overlay_range.is_empty() {
        return overlay_range.start() == range.start()
            || (range.start() < overlay_range.start() && overlay_range.start() < range.end())
            || (overlay_range.start() == range.end() && range.end() == accessible_end);
    }
    if range.is_empty() {
        return overlay_range.start() < range.start() && range.start() < overlay_range.end();
    }
    overlay_range.start() < range.end() && overlay_range.end() > range.start()
}

fn overlay_property_named(overlay: Value, prop_name: Value) -> Option<Value> {
    let plist = overlay.as_overlay_data()?.plist;
    plist::plist_get(plist, &prop_name)
}

fn compare_overlay_precedence(left: Value, right: Value) -> Ordering {
    let left_data = left.as_overlay_data();
    let right_data = right.as_overlay_data();
    let Some(left_overlay) = left_data.filter(|d| d.buffer.is_some()) else {
        return Ordering::Less;
    };
    let Some(right_overlay) = right_data.filter(|d| d.buffer.is_some()) else {
        return Ordering::Greater;
    };
    let (left_priority, left_subpriority) = overlay_priority(left_overlay);
    let (right_priority, right_subpriority) = overlay_priority(right_overlay);
    let left_range = overlay_data_range(left_overlay);
    let right_range = overlay_data_range(right_overlay);

    if left_priority != right_priority {
        return left_priority.cmp(&right_priority);
    }
    if left_range.start() < right_range.start() {
        if left_range.end() < right_range.end() && left_subpriority > right_subpriority {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    } else if left_range.start() > right_range.start() {
        if left_range.end() > right_range.end() && left_subpriority < right_subpriority {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    } else if left_range.end() != right_range.end() {
        if right_range.end() < left_range.end() {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    } else if left_subpriority != right_subpriority {
        left_subpriority.cmp(&right_subpriority)
    } else if eq_value(&left, &right) {
        Ordering::Equal
    } else if overlay_identity_key(left) < overlay_identity_key(right) {
        // GNU `compare_overlays` uses raw Lisp object identity as the final
        // stable tiebreaker for otherwise equal overlays.  Neomacs stores an
        // overlay allocation serial because Rust heap addresses are not
        // monotonic like GNU's Lisp object representation in this path.
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

fn overlay_identity_key(overlay: Value) -> u64 {
    overlay
        .as_overlay_data()
        .map(|data| data.serial)
        .filter(|serial| *serial != 0)
        .unwrap_or(overlay.bits() as u64)
}

fn overlay_priority(overlay: &Overlay) -> (i64, i64) {
    match plist_get_named(overlay.plist, "priority") {
        None => (0, 0),
        Some(value) => match value.kind() {
            ValueKind::Fixnum(n) => (n, 0),
            ValueKind::Cons => (
                priority_component(value.cons_car()),
                priority_component(value.cons_cdr()),
            ),
            _ => (0, 0),
        },
    }
}

fn priority_component(value: Value) -> i64 {
    match value.kind() {
        ValueKind::Fixnum(n) => n,
        _ => 0,
    }
}

fn plist_get_named(plist: Value, prop_name: &str) -> Option<Value> {
    let mut tail = plist;
    loop {
        if !tail.is_cons() {
            return None;
        };
        let pair_car = tail.cons_car();
        let pair_cdr = tail.cons_cdr();
        if !pair_cdr.is_cons() {
            return None;
        };
        if pair_car.as_symbol_name() == Some(prop_name) {
            return Some(pair_cdr.cons_car());
        }
        tail = pair_cdr.cons_cdr();
    }
}

fn overlay_plist_put(plist: Value, prop: Value, value: Value) -> (Value, bool) {
    let mut tail = plist;
    while tail.is_cons() {
        let rest = tail.cons_cdr();
        if !rest.is_cons() {
            break;
        }
        if eq_value(&tail.cons_car(), &prop) {
            let changed = !eq_value(&rest.cons_car(), &value);
            rest.set_car(value);
            return (plist, changed);
        }
        tail = rest.cons_cdr();
    }
    (
        Value::cons(prop, Value::cons(value, plist)),
        !value.is_nil(),
    )
}

impl Default for OverlayList {
    fn default() -> Self {
        Self::new()
    }
}

impl GcTrace for OverlayList {
    fn trace_roots(&self, roots: &mut Vec<Value>) {
        for overlay in &self.overlays {
            roots.push(*overlay);
        }
    }
}

#[cfg(test)]
#[path = "overlay_test.rs"]
mod tests;
