//! Buffer-owned indexes for live overlays.
//!
//! `OverlayIndex` is the single mutation boundary for overlay membership,
//! endpoint lookup, and interval queries.  Lisp object state remains owned by
//! `OverlayList`; this module owns the structural invariants needed to find
//! those objects efficiently.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Unbounded};

use crate::emacs_core::value::Value;

use super::position::{EmacsBytePos, EmacsByteRange};

#[derive(Clone, Debug)]
pub(super) struct OverlayIndex {
    live: BTreeSet<Value>,
    by_start: BTreeMap<EmacsBytePos, BTreeSet<Value>>,
    by_end: BTreeMap<EmacsBytePos, BTreeSet<Value>>,
    intervals: IntervalTree,
}

impl OverlayIndex {
    pub(super) fn new() -> Self {
        Self {
            live: BTreeSet::new(),
            by_start: BTreeMap::new(),
            by_end: BTreeMap::new(),
            intervals: IntervalTree::new(),
        }
    }

    /// Attach a live overlay at `range`.
    ///
    /// Returns `false` without changing the index when the overlay is already
    /// attached.  Keeping all three writes here prevents membership and query
    /// indexes from drifting apart.
    pub(super) fn attach(&mut self, overlay: Value, range: EmacsByteRange) -> bool {
        if !self.live.insert(overlay) {
            return false;
        }
        insert_endpoint(&mut self.by_start, range.start(), overlay);
        insert_endpoint(&mut self.by_end, range.end(), overlay);
        let inserted = self.intervals.insert(overlay, range);
        debug_assert!(inserted, "new live overlay already had an interval node");
        true
    }

    /// Detach an overlay and return its indexed range.
    pub(super) fn detach(&mut self, overlay: Value) -> Option<EmacsByteRange> {
        if !self.live.remove(&overlay) {
            return None;
        }
        let range = self
            .intervals
            .remove(overlay)
            .expect("live overlay must have an interval node");
        remove_endpoint(&mut self.by_start, range.start(), overlay);
        remove_endpoint(&mut self.by_end, range.end(), overlay);
        Some(range)
    }

    /// Move an attached overlay, returning its old range.
    pub(super) fn move_to(
        &mut self,
        overlay: Value,
        new_range: EmacsByteRange,
    ) -> Option<EmacsByteRange> {
        if !self.live.contains(&overlay) {
            return None;
        }
        let old_range = self
            .intervals
            .remove(overlay)
            .expect("live overlay must have an interval node");
        remove_endpoint(&mut self.by_start, old_range.start(), overlay);
        remove_endpoint(&mut self.by_end, old_range.end(), overlay);

        insert_endpoint(&mut self.by_start, new_range.start(), overlay);
        insert_endpoint(&mut self.by_end, new_range.end(), overlay);
        let inserted = self.intervals.insert(overlay, new_range);
        debug_assert!(inserted, "removed overlay retained an interval node");
        Some(old_range)
    }

    /// Relocate an overlay because buffer text moved around it.
    ///
    /// Unlike `move_to`, this preserves the node's attachment-order tie-break:
    /// GNU shifts interval nodes in place during edits, so equal-start overlays
    /// must not be reordered merely because text was inserted or deleted.
    pub(super) fn relocate_for_text_edit(
        &mut self,
        overlay: Value,
        new_range: EmacsByteRange,
    ) -> Option<EmacsByteRange> {
        let old_range = self
            .intervals
            .relocate_preserving_order(overlay, new_range)?;
        if old_range == new_range {
            return Some(old_range);
        }
        remove_endpoint(&mut self.by_start, old_range.start(), overlay);
        remove_endpoint(&mut self.by_end, old_range.end(), overlay);
        insert_endpoint(&mut self.by_start, new_range.start(), overlay);
        insert_endpoint(&mut self.by_end, new_range.end(), overlay);
        Some(old_range)
    }

    /// A conservative, deduplicated set of overlays whose endpoints can move
    /// when text is inserted at `pos`.
    pub(super) fn insertion_candidates(&self, pos: EmacsBytePos) -> Vec<Value> {
        let mut candidates = BTreeSet::new();
        for overlays in self.by_start.range(pos..).map(|(_, overlays)| overlays) {
            candidates.extend(overlays.iter().copied());
        }
        for overlays in self.by_end.range(pos..).map(|(_, overlays)| overlays) {
            candidates.extend(overlays.iter().copied());
        }
        candidates.into_iter().collect()
    }

    /// A conservative, deduplicated set of overlays whose endpoints can move
    /// when `range` is deleted.
    pub(super) fn deletion_candidates(&self, range: EmacsByteRange) -> Vec<Value> {
        let mut candidates = BTreeSet::new();
        for overlays in self
            .by_start
            .range((Excluded(range.start()), Unbounded))
            .map(|(_, overlays)| overlays)
        {
            candidates.extend(overlays.iter().copied());
        }
        for overlays in self
            .by_end
            .range((Excluded(range.start()), Unbounded))
            .map(|(_, overlays)| overlays)
        {
            candidates.extend(overlays.iter().copied());
        }
        candidates.into_iter().collect()
    }

    pub(super) fn clear(&mut self) {
        self.live.clear();
        self.by_start.clear();
        self.by_end.clear();
        self.intervals = IntervalTree::new();
    }

    pub(super) fn contains(&self, overlay: Value) -> bool {
        self.live.contains(&overlay)
    }

    pub(super) fn len(&self) -> usize {
        self.live.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.live.is_empty()
    }

    pub(super) fn values(&self) -> impl DoubleEndedIterator<Item = Value> + '_ {
        self.live.iter().copied()
    }

    pub(super) fn overlays_at(&self, pos: EmacsBytePos) -> Vec<Value> {
        let mut overlays = Vec::new();
        self.intervals.overlays_at(pos, &mut overlays);
        overlays
    }

    /// Return intervals whose closed bounds touch `pos`.
    ///
    /// Character-property lookup uses half-open coverage, while GNU's
    /// `get-pos-property` additionally considers overlays beginning or ending
    /// exactly at the insertion position before applying advance flags.
    pub(super) fn overlays_touching(&self, pos: EmacsBytePos) -> Vec<Value> {
        let mut overlays = Vec::new();
        self.intervals.overlays_touching(pos, &mut overlays);
        overlays
    }

    pub(super) fn overlays_in_region(
        &self,
        range: EmacsByteRange,
        accessible_end: EmacsBytePos,
    ) -> Vec<Value> {
        let mut overlays = Vec::new();
        self.intervals
            .overlays_in_region(range, accessible_end, &mut overlays);
        overlays
    }

    pub(super) fn all_ascending(&self) -> Vec<Value> {
        let mut overlays = Vec::with_capacity(self.len());
        self.intervals.all_ascending(&mut overlays);
        overlays
    }

    pub(super) fn starts_at(&self, boundary: EmacsBytePos) -> Option<&BTreeSet<Value>> {
        self.by_start.get(&boundary)
    }

    pub(super) fn ends_at(&self, boundary: EmacsBytePos) -> Option<&BTreeSet<Value>> {
        self.by_end.get(&boundary)
    }

    pub(super) fn next_boundary_after(
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
        min_option(next_start, next_end).filter(|boundary| *boundary <= limit)
    }

    pub(super) fn previous_boundary_before(
        &self,
        pos: EmacsBytePos,
        limit: EmacsBytePos,
    ) -> Option<EmacsBytePos> {
        if pos <= limit {
            return None;
        }
        let previous_start = self
            .by_start
            .range(..pos)
            .next_back()
            .map(|(boundary, _)| *boundary);
        let previous_end = self
            .by_end
            .range(..pos)
            .next_back()
            .map(|(boundary, _)| *boundary);
        max_option(previous_start, previous_end).filter(|boundary| *boundary >= limit)
    }

    #[cfg(test)]
    pub(super) fn interval_height(&self) -> usize {
        self.intervals.height()
    }
}

fn insert_endpoint(
    index: &mut BTreeMap<EmacsBytePos, BTreeSet<Value>>,
    boundary: EmacsBytePos,
    overlay: Value,
) {
    index.entry(boundary).or_default().insert(overlay);
}

fn remove_endpoint(
    index: &mut BTreeMap<EmacsBytePos, BTreeSet<Value>>,
    boundary: EmacsBytePos,
    overlay: Value,
) {
    let remove_boundary = index.get_mut(&boundary).is_some_and(|overlays| {
        overlays.remove(&overlay);
        overlays.is_empty()
    });
    if remove_boundary {
        index.remove(&boundary);
    }
}

fn min_option(left: Option<EmacsBytePos>, right: Option<EmacsBytePos>) -> Option<EmacsBytePos> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn max_option(left: Option<EmacsBytePos>, right: Option<EmacsBytePos>) -> Option<EmacsBytePos> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

/// An index-internal handle.  It cannot be confused with Lisp overlay identity
/// or a byte position at compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NodeId(u32);

impl NodeId {
    fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).expect("overlay interval arena exceeds u32::MAX nodes"))
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}

/// Total ordering used by the balanced tree.
///
/// GNU descends left when inserting another node at the same begin position,
/// so an ascending traversal observes the newest attachment first.  Encoding
/// that tie-break explicitly prevents rotations from changing Lisp-visible
/// traversal order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IntervalKey {
    start: EmacsBytePos,
    attachment_order: u64,
}

impl Ord for IntervalKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start
            .cmp(&other.start)
            .then_with(|| other.attachment_order.cmp(&self.attachment_order))
    }
}

impl PartialOrd for IntervalKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
struct IntervalNode {
    key: IntervalKey,
    range: EmacsByteRange,
    min_start: EmacsBytePos,
    max_end: EmacsBytePos,
    height: u32,
    overlay: Value,
    left: Option<NodeId>,
    right: Option<NodeId>,
}

impl IntervalNode {
    fn new(key: IntervalKey, range: EmacsByteRange, overlay: Value) -> Self {
        Self {
            key,
            range,
            min_start: range.start(),
            max_end: range.end(),
            height: 1,
            overlay,
            left: None,
            right: None,
        }
    }
}

/// Arena-backed augmented AVL tree.
///
/// Rotations update stable node handles rather than moving boxed subtrees.
/// The arena also gives removal an O(1) overlay-to-node lookup before the
/// O(log n) structural update.
#[derive(Clone, Debug)]
struct IntervalTree {
    root: Option<NodeId>,
    slots: Vec<Option<IntervalNode>>,
    free: Vec<NodeId>,
    by_overlay: BTreeMap<Value, NodeId>,
    next_attachment_order: u64,
}

impl IntervalTree {
    fn new() -> Self {
        Self {
            root: None,
            slots: Vec::new(),
            free: Vec::new(),
            by_overlay: BTreeMap::new(),
            next_attachment_order: 0,
        }
    }

    fn insert(&mut self, overlay: Value, range: EmacsByteRange) -> bool {
        if self.by_overlay.contains_key(&overlay) {
            return false;
        }
        let attachment_order = self.next_attachment_order;
        self.next_attachment_order = self
            .next_attachment_order
            .checked_add(1)
            .expect("overlay attachment order exhausted");
        self.insert_with_order(overlay, range, attachment_order)
    }

    fn insert_with_order(
        &mut self,
        overlay: Value,
        range: EmacsByteRange,
        attachment_order: u64,
    ) -> bool {
        if self.by_overlay.contains_key(&overlay) {
            return false;
        }
        let id = self.allocate(IntervalNode::new(
            IntervalKey {
                start: range.start(),
                attachment_order,
            },
            range,
            overlay,
        ));
        self.root = Some(self.insert_node(self.root, id));
        self.by_overlay.insert(overlay, id);
        true
    }

    fn remove(&mut self, overlay: Value) -> Option<EmacsByteRange> {
        self.remove_with_order(overlay).map(|(range, _)| range)
    }

    fn remove_with_order(&mut self, overlay: Value) -> Option<(EmacsByteRange, u64)> {
        let id = self.by_overlay.remove(&overlay)?;
        let key = self.node(id).key;
        let range = self.node(id).range;
        self.root = self.remove_node(self.root, key, id);
        Some((range, key.attachment_order))
    }

    fn relocate_preserving_order(
        &mut self,
        overlay: Value,
        new_range: EmacsByteRange,
    ) -> Option<EmacsByteRange> {
        let (old_range, attachment_order) = self.remove_with_order(overlay)?;
        let inserted = self.insert_with_order(overlay, new_range, attachment_order);
        debug_assert!(inserted, "removed overlay retained an interval node");
        Some(old_range)
    }

    fn overlays_at(&self, pos: EmacsBytePos, out: &mut Vec<Value>) {
        self.overlays_at_node(self.root, pos, out);
    }

    fn overlays_at_node(&self, id: Option<NodeId>, pos: EmacsBytePos, out: &mut Vec<Value>) {
        let Some(id) = id else { return };
        let node = self.node(id);
        #[cfg(test)]
        super::overlay::record_overlays_at_node_visit();

        if node.min_start > pos {
            return;
        }
        if node.left.is_some_and(|left| self.node(left).max_end > pos) {
            self.overlays_at_node(node.left, pos, out);
        }
        if node.range.start() <= pos && pos < node.range.end() {
            out.push(node.overlay);
        }
        if node.range.start() <= pos
            && node
                .right
                .is_some_and(|right| self.node(right).max_end > pos)
        {
            self.overlays_at_node(node.right, pos, out);
        }
    }

    fn overlays_touching(&self, pos: EmacsBytePos, out: &mut Vec<Value>) {
        self.overlays_touching_node(self.root, pos, out);
    }

    fn overlays_touching_node(&self, id: Option<NodeId>, pos: EmacsBytePos, out: &mut Vec<Value>) {
        let Some(id) = id else { return };
        let node = self.node(id);
        if node.min_start > pos || node.max_end < pos {
            return;
        }
        if node.left.is_some_and(|left| self.node(left).max_end >= pos) {
            self.overlays_touching_node(node.left, pos, out);
        }
        if node.range.start() <= pos && pos <= node.range.end() {
            out.push(node.overlay);
        }
        if node.range.start() <= pos
            && node
                .right
                .is_some_and(|right| self.node(right).max_end >= pos)
        {
            self.overlays_touching_node(node.right, pos, out);
        }
    }

    fn overlays_in_region(
        &self,
        range: EmacsByteRange,
        accessible_end: EmacsBytePos,
        out: &mut Vec<Value>,
    ) {
        self.overlays_in_region_node(self.root, range, accessible_end, out);
    }

    fn overlays_in_region_node(
        &self,
        id: Option<NodeId>,
        range: EmacsByteRange,
        accessible_end: EmacsBytePos,
        out: &mut Vec<Value>,
    ) {
        let Some(id) = id else { return };
        let node = self.node(id);

        if node.min_start > range.end() {
            return;
        }
        if node
            .left
            .is_some_and(|left| self.node(left).max_end >= range.start())
        {
            self.overlays_in_region_node(node.left, range, accessible_end, out);
        }
        if node.range.start() > range.end() {
            return;
        }
        if ranges_overlap_region(node.range, range, accessible_end) {
            out.push(node.overlay);
        }
        self.overlays_in_region_node(node.right, range, accessible_end, out);
    }

    fn all_ascending(&self, out: &mut Vec<Value>) {
        self.all_ascending_node(self.root, out);
    }

    fn all_ascending_node(&self, id: Option<NodeId>, out: &mut Vec<Value>) {
        let Some(id) = id else { return };
        let node = self.node(id);
        self.all_ascending_node(node.left, out);
        out.push(node.overlay);
        self.all_ascending_node(node.right, out);
    }

    fn allocate(&mut self, node: IntervalNode) -> NodeId {
        if let Some(id) = self.free.pop() {
            debug_assert!(self.slots[id.index()].is_none());
            self.slots[id.index()] = Some(node);
            id
        } else {
            let id = NodeId::from_index(self.slots.len());
            self.slots.push(Some(node));
            id
        }
    }

    fn release(&mut self, id: NodeId) {
        let removed = self.slots[id.index()].take();
        debug_assert!(removed.is_some());
        self.free.push(id);
    }

    fn node(&self, id: NodeId) -> &IntervalNode {
        self.slots[id.index()]
            .as_ref()
            .expect("interval node handle points to a free slot")
    }

    fn node_mut(&mut self, id: NodeId) -> &mut IntervalNode {
        self.slots[id.index()]
            .as_mut()
            .expect("interval node handle points to a free slot")
    }

    fn insert_node(&mut self, root: Option<NodeId>, inserted: NodeId) -> NodeId {
        let Some(root) = root else { return inserted };
        let ordering = self.node(inserted).key.cmp(&self.node(root).key);
        match ordering {
            Ordering::Less => {
                let left = self.insert_node(self.node(root).left, inserted);
                self.node_mut(root).left = Some(left);
            }
            Ordering::Greater => {
                let right = self.insert_node(self.node(root).right, inserted);
                self.node_mut(root).right = Some(right);
            }
            Ordering::Equal => unreachable!("attachment order makes interval keys unique"),
        }
        self.refresh(root);
        self.rebalance(root)
    }

    fn remove_node(
        &mut self,
        root: Option<NodeId>,
        key: IntervalKey,
        removed: NodeId,
    ) -> Option<NodeId> {
        let root = root?;
        match key.cmp(&self.node(root).key) {
            Ordering::Less => {
                let left = self.remove_node(self.node(root).left, key, removed);
                self.node_mut(root).left = left;
            }
            Ordering::Greater => {
                let right = self.remove_node(self.node(root).right, key, removed);
                self.node_mut(root).right = right;
            }
            Ordering::Equal => {
                debug_assert_eq!(root, removed);
                let left = self.node(root).left;
                let right = self.node(root).right;
                self.release(root);
                return self.join(left, right);
            }
        }
        self.refresh(root);
        Some(self.rebalance(root))
    }

    fn join(&mut self, left: Option<NodeId>, right: Option<NodeId>) -> Option<NodeId> {
        match (left, right) {
            (None, None) => None,
            (Some(root), None) | (None, Some(root)) => Some(root),
            (Some(left), Some(right)) => {
                let (new_right, successor) = self.remove_min(right);
                {
                    let successor_node = self.node_mut(successor);
                    successor_node.left = Some(left);
                    successor_node.right = new_right;
                }
                self.refresh(successor);
                Some(self.rebalance(successor))
            }
        }
    }

    /// Remove the smallest node from a subtree without freeing its arena slot.
    fn remove_min(&mut self, root: NodeId) -> (Option<NodeId>, NodeId) {
        let Some(left) = self.node(root).left else {
            let right = self.node(root).right;
            self.node_mut(root).right = None;
            return (right, root);
        };
        let (new_left, minimum) = self.remove_min(left);
        self.node_mut(root).left = new_left;
        self.refresh(root);
        (Some(self.rebalance(root)), minimum)
    }

    fn refresh(&mut self, id: NodeId) {
        let (left, right, own_start, own_end) = {
            let node = self.node(id);
            (node.left, node.right, node.range.start(), node.range.end())
        };
        let left_height = self.node_height(left);
        let right_height = self.node_height(right);
        let left_min = left.map(|child| self.node(child).min_start);
        let right_min = right.map(|child| self.node(child).min_start);
        let left_max = left.map(|child| self.node(child).max_end);
        let right_max = right.map(|child| self.node(child).max_end);
        let node = self.node_mut(id);
        node.height = 1 + left_height.max(right_height);
        node.min_start = own_start
            .min(left_min.unwrap_or(own_start))
            .min(right_min.unwrap_or(own_start));
        node.max_end = own_end
            .max(left_max.unwrap_or(own_end))
            .max(right_max.unwrap_or(own_end));
    }

    fn rebalance(&mut self, root: NodeId) -> NodeId {
        let balance = self.balance_factor(Some(root));
        if balance > 1 {
            let left = self
                .node(root)
                .left
                .expect("left-heavy node has a left child");
            if self.balance_factor(Some(left)) < 0 {
                let rotated = self.rotate_left(left);
                self.node_mut(root).left = Some(rotated);
            }
            return self.rotate_right(root);
        }
        if balance < -1 {
            let right = self
                .node(root)
                .right
                .expect("right-heavy node has a right child");
            if self.balance_factor(Some(right)) > 0 {
                let rotated = self.rotate_right(right);
                self.node_mut(root).right = Some(rotated);
            }
            return self.rotate_left(root);
        }
        root
    }

    fn rotate_left(&mut self, root: NodeId) -> NodeId {
        let pivot = self
            .node(root)
            .right
            .expect("left rotation requires a right child");
        let transferred = self.node(pivot).left;
        self.node_mut(root).right = transferred;
        self.refresh(root);
        self.node_mut(pivot).left = Some(root);
        self.refresh(pivot);
        pivot
    }

    fn rotate_right(&mut self, root: NodeId) -> NodeId {
        let pivot = self
            .node(root)
            .left
            .expect("right rotation requires a left child");
        let transferred = self.node(pivot).right;
        self.node_mut(root).left = transferred;
        self.refresh(root);
        self.node_mut(pivot).right = Some(root);
        self.refresh(pivot);
        pivot
    }

    fn balance_factor(&self, id: Option<NodeId>) -> i64 {
        let Some(id) = id else { return 0 };
        i64::from(self.node_height(self.node(id).left))
            - i64::from(self.node_height(self.node(id).right))
    }

    fn node_height(&self, id: Option<NodeId>) -> u32 {
        id.map_or(0, |id| self.node(id).height)
    }

    #[cfg(test)]
    fn height(&self) -> usize {
        self.node_height(self.root) as usize
    }
}

fn ranges_overlap_region(
    overlay: EmacsByteRange,
    range: EmacsByteRange,
    accessible_end: EmacsBytePos,
) -> bool {
    if overlay.is_empty() {
        return overlay.start() == range.start()
            || (range.start() < overlay.start() && overlay.start() < range.end())
            || (overlay.start() == range.end() && range.end() == accessible_end);
    }
    if range.is_empty() {
        return overlay.start() < range.start() && range.start() < overlay.end();
    }
    overlay.start() < range.end() && overlay.end() > range.start()
}

#[cfg(test)]
#[path = "overlay_index_test.rs"]
mod tests;
