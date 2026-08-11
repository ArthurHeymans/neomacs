//! Overlay system for buffers.
//!
//! GNU Emacs exposes overlays as first-class Lisp objects whose identity
//! outlives deletion. The buffer owns the interval index, while the overlay
//! object owns plist, buffer membership, and endpoint state. NeoVM models that
//! split by keeping overlay objects on the GC heap and storing only live object
//! ids in each buffer's overlay index.

use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};

use crate::buffer::BufferId;
use crate::emacs_core::error::Flow;
use crate::emacs_core::eval::{
    push_scratch_gc_root, restore_scratch_gc_roots, save_scratch_gc_roots,
};
use crate::emacs_core::plist;
use crate::emacs_core::value::{Value, ValueKind, eq_value};
use crate::gc_trace::GcTrace;
use crate::heap_types::OverlayData;

use super::overlay_index::{OverlayBatchOrder, OverlayEditEffect, OverlayIndex, OverlayTextEdit};
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

#[cfg(test)]
pub(super) fn record_overlays_at_node_visit() {
    OVERLAYS_AT_NODE_VISITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
static OVERLAY_PROPERTY_EXTENT_INSPECTIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn reset_overlay_property_extent_inspection_count() {
    OVERLAY_PROPERTY_EXTENT_INSPECTIONS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn overlay_property_extent_inspection_count() -> usize {
    OVERLAY_PROPERTY_EXTENT_INSPECTIONS.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
static BEST_OVERLAY_CANDIDATE_INSPECTIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn reset_best_overlay_candidate_inspection_count() {
    BEST_OVERLAY_CANDIDATE_INSPECTIONS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn best_overlay_candidate_inspection_count() -> usize {
    BEST_OVERLAY_CANDIDATE_INSPECTIONS.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
static OVERLAY_EDIT_CANDIDATE_INSPECTIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn reset_overlay_edit_candidate_inspection_count() {
    OVERLAY_EDIT_CANDIDATE_INSPECTIONS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn overlay_edit_candidate_inspection_count() -> usize {
    OVERLAY_EDIT_CANDIDATE_INSPECTIONS.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
pub(super) fn record_overlay_edit_candidate_inspection() {
    OVERLAY_EDIT_CANDIDATE_INSPECTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
static OVERLAY_SHIFT_NODE_VISITS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn reset_overlay_shift_node_visit_count() {
    OVERLAY_SHIFT_NODE_VISITS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn overlay_shift_node_visit_count() -> usize {
    OVERLAY_SHIFT_NODE_VISITS.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
pub(super) fn record_overlay_shift_node_visit() {
    OVERLAY_SHIFT_NODE_VISITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
static ENDPOINT_SEARCH_NODE_VISITS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn reset_endpoint_search_node_visit_count() {
    ENDPOINT_SEARCH_NODE_VISITS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn endpoint_search_node_visit_count() -> usize {
    ENDPOINT_SEARCH_NODE_VISITS.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
pub(super) fn record_endpoint_search_node_visit() {
    ENDPOINT_SEARCH_NODE_VISITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
static OVERLAY_FULL_ENUMERATION_VISITS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn reset_overlay_full_enumeration_visit_count() {
    OVERLAY_FULL_ENUMERATION_VISITS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn overlay_full_enumeration_visit_count() -> usize {
    OVERLAY_FULL_ENUMERATION_VISITS.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
pub(super) fn record_overlay_full_enumeration_visit() {
    OVERLAY_FULL_ENUMERATION_VISITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub struct OverlayList {
    index: OverlayIndex,
}

#[derive(Clone, Copy)]
enum OverlayCloneIdentity {
    /// GNU `copy_overlays`: the destination owns distinct Lisp objects.
    Fresh,
    /// A read snapshot owns distinct objects and position handles, but keeps
    /// the precedence identity observed at snapshot time.
    PreservePrecedence,
}

impl Clone for OverlayList {
    /// GNU `copy_overlays`: clone each Lisp overlay object and publish a fresh
    /// buffer-owned index. Sharing either the object or a partially cloned
    /// index would let a move in one indirect buffer corrupt the other.
    fn clone(&self) -> Self {
        self.clone_with_identity(OverlayCloneIdentity::Fresh)
    }
}

impl OverlayList {
    /// Copy the list for an immutable observer such as redisplay.
    ///
    /// Snapshot overlays require independent position handles, so they cannot
    /// share the live Lisp objects. They do preserve each source object's
    /// precedence serial: changing that serial would change GNU's final
    /// `compare_overlays` tiebreak merely because redisplay took a snapshot.
    pub fn snapshot_clone(&self) -> Self {
        self.clone_with_identity(OverlayCloneIdentity::PreservePrecedence)
    }

    fn clone_with_identity(&self, identity: OverlayCloneIdentity) -> Self {
        let saved_roots = save_scratch_gc_roots();
        let entries: Vec<_> = self
            .index
            .all_ascending()
            .into_iter()
            .map(|overlay| {
                let data = overlay
                    .as_overlay_data()
                    .expect("overlay index contains a non-overlay value");
                let range = overlay_data_range(data);
                let plist = crate::emacs_core::builtins::builtin_copy_sequence(vec![data.plist])
                    .expect("a live overlay must carry a copyable plist");
                push_scratch_gc_root(plist);
                let copy = Value::make_overlay(OverlayData {
                    serial: match identity {
                        OverlayCloneIdentity::Fresh => 0,
                        OverlayCloneIdentity::PreservePrecedence => data.serial,
                    },
                    plist,
                    buffer: data.buffer,
                    start: range.start().get(),
                    end: range.end().get(),
                    position_handle: None,
                    front_advance: data.front_advance,
                    rear_advance: data.rear_advance,
                });
                push_scratch_gc_root(copy);
                (copy, range)
            })
            .collect();
        let mut cloned = Self::new();
        let batch_order = match identity {
            // GNU `copy_overlays` attaches copies in the source tree's
            // ascending traversal order, intentionally reversing equal-start
            // nodes in the destination tree.
            OverlayCloneIdentity::Fresh => OverlayBatchOrder::AttachmentSequence,
            // Redisplay snapshots are observers, so their raw query order must
            // remain identical to the live buffer.
            OverlayCloneIdentity::PreservePrecedence => OverlayBatchOrder::AscendingQueryOrder,
        };
        let attached = cloned.index.attach_batch(&entries, batch_order);
        debug_assert!(attached, "fresh overlay clone index rejected its batch");
        restore_scratch_gc_roots(saved_roots);
        cloned
    }
}

/// The effective contiguous range of an overlay-supplied property winner.
///
/// `overlay` and `value` are absent/nil when no live overlay supplies a
/// non-nil value in `range`.  That negative extent is useful to callers which
/// fall back to text properties without rescanning unrelated overlays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayPropertyExtent {
    overlay: Option<Value>,
    value: Value,
    range: EmacsByteRange,
}

impl OverlayPropertyExtent {
    pub fn overlay(self) -> Option<Value> {
        self.overlay
    }

    pub fn value(self) -> Value {
        self.value
    }

    pub fn range(self) -> EmacsByteRange {
        self.range
    }
}

#[derive(Clone, Copy, Debug)]
struct OverlayByPrecedence(Value);

impl PartialEq for OverlayByPrecedence {
    fn eq(&self, other: &Self) -> bool {
        eq_value(&self.0, &other.0)
    }
}

impl Eq for OverlayByPrecedence {}

impl PartialOrd for OverlayByPrecedence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OverlayByPrecedence {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_overlay_precedence(self.0, other.0)
    }
}

#[derive(Clone)]
struct ActivePropertyOverlays {
    property: Value,
    /// Window currently being laid out, so a windowed overlay (one carrying a
    /// `window` property, e.g. hl-line non-sticky) is considered only in its own
    /// window. `None` = no window context ⇒ unrestricted (matches GNU).
    window_id: Option<u64>,
    active: BTreeSet<Value>,
    by_precedence: BinaryHeap<OverlayByPrecedence>,
}

impl ActivePropertyOverlays {
    fn new(property: Value, window_id: Option<u64>) -> Self {
        Self {
            property,
            window_id,
            active: BTreeSet::new(),
            by_precedence: BinaryHeap::new(),
        }
    }

    fn inspect_and_insert(&mut self, overlay: Value) {
        #[cfg(test)]
        OVERLAY_PROPERTY_EXTENT_INSPECTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if overlay_applies_to_window(overlay, self.window_id)
            && overlay_property_named(overlay, self.property).is_some_and(|value| !value.is_nil())
            && self.active.insert(overlay)
        {
            self.by_precedence.push(OverlayByPrecedence(overlay));
        }
    }

    fn remove(&mut self, overlay: Value) {
        self.active.remove(&overlay);
    }

    fn winner(&mut self) -> Option<Value> {
        while self
            .by_precedence
            .peek()
            .is_some_and(|candidate| !self.active.contains(&candidate.0))
        {
            self.by_precedence.pop();
        }
        self.by_precedence.peek().map(|candidate| candidate.0)
    }
}

impl OverlayList {
    pub fn new() -> Self {
        Self {
            index: OverlayIndex::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn interval_index_height(&self) -> usize {
        self.index.interval_height()
    }

    pub fn insert_overlay(&mut self, overlay: Value) {
        let data = overlay.as_overlay_data().unwrap();
        // Attach consumes the object's materialized coordinates. A live
        // overlay already owned by this index is rejected, while cross-buffer
        // moves materialize and rewrite the coordinates before reattaching.
        let range = EmacsByteRange::from_usize(data.start, data.end);
        self.index.attach(overlay, range);
    }

    pub fn detach_overlay(&mut self, overlay: Value) -> bool {
        self.index.detach(overlay).is_some()
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
        let live: Vec<Value> = self.index.values().collect();
        for overlay in live {
            let _ = overlay.with_overlay_data_mut(|data| {
                data.buffer = None;
            });
        }
        self.index.clear();
    }

    pub(crate) fn retarget_buffer(&mut self, from: BufferId, to: BufferId) {
        for overlay in self.index.values() {
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

    /// Whether `overlay`'s contributions apply in the window identified by
    /// `window_id` -- GNU `overlay_matches_window` (src/window.h):
    /// `! WINDOWP (window) || XWINDOW (window) == w`.
    ///
    /// Exposed because the rule must be the SAME one the property resolvers above
    /// apply: a path that collects overlays itself (overlay strings, which order
    /// by GNU `compare_overlay_entries` rather than by property precedence) still
    /// has to filter windowed overlays identically, and a second copy of the rule
    /// is how hl-line's per-window highlight leaked into every window before.
    pub fn overlay_applies_to_window(&self, overlay: Value, window_id: Option<u64>) -> bool {
        overlay_applies_to_window(overlay, window_id)
    }

    pub fn overlay_plist(&self, overlay: Value) -> Option<Value> {
        if self.index.contains(overlay) || overlay_live_buffer(overlay).is_none() {
            return Some(overlay.as_overlay_data().unwrap().plist);
        }
        None
    }

    pub fn overlay_start_emacs_byte_pos(&self, overlay: Value) -> Option<EmacsBytePos> {
        overlay_live_buffer(overlay)?;
        self.index.range(overlay).map(EmacsByteRange::start)
    }

    pub fn overlay_end_emacs_byte_pos(&self, overlay: Value) -> Option<EmacsBytePos> {
        overlay_live_buffer(overlay)?;
        self.index.range(overlay).map(EmacsByteRange::end)
    }

    pub fn move_overlay_to_emacs_byte_range(&mut self, overlay: Value, range: EmacsByteRange) {
        if self.index.move_to(overlay, range).is_none() {
            return;
        }
        let _ = overlay.with_overlay_data_mut(|data| {
            data.start = range.start().get();
            data.end = range.end().get();
        });
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
        self.iter_overlays_at_emacs_byte_pos(pos).collect()
    }

    /// Borrow matching overlays without allocating an intermediate vector.
    pub fn iter_overlays_at_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
    ) -> impl Iterator<Item = Value> + '_ {
        self.index.overlays_at_iter(pos)
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
        self.index.all_ascending()
    }

    pub fn overlays_in_accessible_emacs_byte_range(
        &self,
        range: EmacsByteRange,
        accessible_end: EmacsBytePos,
    ) -> Vec<Value> {
        self.iter_overlays_in_accessible_emacs_byte_range(range, accessible_end)
            .collect()
    }

    /// Borrow region matches without allocating an intermediate vector.
    pub fn iter_overlays_in_accessible_emacs_byte_range(
        &self,
        range: EmacsByteRange,
        accessible_end: EmacsBytePos,
    ) -> impl Iterator<Item = Value> + '_ {
        self.index.overlays_in_region_iter(range, accessible_end)
    }

    pub fn highest_priority_overlay_at_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        property: Value,
    ) -> Option<Value> {
        self.best_overlay_among(property, self.index.overlays_at_iter(pos), |overlay| {
            overlay_covers_pos(overlay, pos)
        })
    }

    /// GNU `get_char_property_and_overlay` (src/textprop.c): PROPERTY's value from
    /// the highest-precedence overlay at POS that carries it, with
    /// window-specific overlays filtered against `window_id`.
    ///
    /// `None` means *no* overlay carries the property, and only then may the
    /// caller fall back to the **text** property. An overlay that carries it
    /// SHADOWS the text property outright: the value never merges with the
    /// text-property value, and no lower-precedence overlay gets a say -- not even
    /// when the winner's value happens to mean "inactive" (an `invisible` value
    /// absent from `buffer-invisibility-spec`, say). That is the policy for
    /// `display`, `invisible`, `fontified` and `mouse-face`. `face` is the sole
    /// exception and uses
    /// [`Self::overlay_property_values_ascending_at_emacs_byte_pos`].
    ///
    /// This is the value-only form of
    /// [`Self::highest_priority_overlay_property_extent_at_emacs_byte_pos`], for
    /// callers whose runs are already bounded at every overlay boundary and so do
    /// not need the extent scan.
    pub fn highest_priority_overlay_property_value_at_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        property: Value,
        window_id: Option<u64>,
    ) -> Option<Value> {
        let mut active = ActivePropertyOverlays::new(property, window_id);
        for overlay in self.iter_overlays_at_emacs_byte_pos(pos) {
            active.inspect_and_insert(overlay);
        }
        active
            .winner()
            .and_then(|overlay| overlay_property_named(overlay, property))
    }

    /// Single-winner overlay lookup with GNU `overlay-get` alias semantics.
    /// The canonical property is first in `property_lookup_order`, followed by
    /// its `char-property-alias-alist` fallbacks. Property lookup happens per
    /// overlay before the highest-precedence carrier is selected.
    pub fn highest_priority_overlay_effective_property_value_at_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        property_lookup_order: &[Value],
        window_id: Option<u64>,
    ) -> Option<Value> {
        self.iter_overlays_at_emacs_byte_pos(pos)
            .filter(|overlay| {
                overlay_applies_to_window(*overlay, window_id)
                    && overlay_property_in_lookup_order(*overlay, property_lookup_order)
                        .is_some_and(|value| !value.is_nil())
            })
            .max_by(|left, right| compare_overlay_precedence(*left, *right))
            .and_then(|overlay| overlay_property_in_lookup_order(overlay, property_lookup_order))
    }

    /// The overlay half of GNU `face_at_buffer_position` (src/xfaces.c): every
    /// window-visible overlay's PROPERTY value at POS in ASCENDING precedence
    /// (`sort_overlays` order), for the one policy where overlay values MERGE
    /// instead of shadowing -- `face`. Higher precedence merges last and so wins.
    ///
    /// Ordering is GNU `compare_overlays`: priority, then containment weighed
    /// against the secondary priority of a `(PRIMARY . SECONDARY)` `priority`
    /// value, then a stable tiebreak. A bare `priority`-integer comparison is not
    /// equivalent -- it silently reads a cons `priority` as 0 and drops the
    /// containment rule.
    pub fn overlay_property_values_ascending_at_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        property: Value,
        window_id: Option<u64>,
    ) -> Vec<Value> {
        self.overlay_effective_property_values_ascending_at_emacs_byte_pos(
            pos,
            std::slice::from_ref(&property),
            window_id,
        )
    }

    /// GNU `overlay-get` property lookup composed with the ascending overlay
    /// merge order used by `face_at_buffer_position`.
    ///
    /// `property_lookup_order` contains the canonical property first followed
    /// by its `char-property-alias-alist` fallbacks.  Lookup happens WITHIN
    /// each overlay before overlay precedence is considered: a high-priority
    /// overlay carrying an alias must still merge after a lower-priority
    /// overlay carrying the canonical name.  Querying each name across all
    /// overlays separately would reverse that GNU ordering.
    pub fn overlay_effective_property_values_ascending_at_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        property_lookup_order: &[Value],
        window_id: Option<u64>,
    ) -> Vec<Value> {
        let mut carriers: Vec<Value> = self
            .iter_overlays_at_emacs_byte_pos(pos)
            .filter(|overlay| {
                overlay_applies_to_window(*overlay, window_id)
                    && overlay_property_in_lookup_order(*overlay, property_lookup_order)
                        .is_some_and(|value| !value.is_nil())
            })
            .collect();
        carriers.sort_by(|left, right| compare_overlay_precedence(*left, *right));
        carriers
            .into_iter()
            .filter_map(|overlay| overlay_property_in_lookup_order(overlay, property_lookup_order))
            .collect()
    }

    /// The same single-winner policy as
    /// [`Self::highest_priority_overlay_property_value_at_emacs_byte_pos`], plus
    /// the maximal contiguous extent over which that winner is unchanged -- GNU's
    /// `endptr` narrowing, for callers that cache a resolved run.
    ///
    /// The interval tree initializes the overlays active at `pos`.  From there
    /// the start/end indexes are swept outward, updating a precedence heap only
    /// for overlays crossing each boundary.  Thus a sweep inspects an entering
    /// overlay at most once instead of rescanning the complete overlay list at
    /// every boundary.
    pub fn highest_priority_overlay_property_extent_at_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        property: Value,
        bounds: EmacsByteRange,
        window_id: Option<u64>,
    ) -> Option<OverlayPropertyExtent> {
        if bounds.is_empty() || pos < bounds.start() || pos >= bounds.end() {
            return None;
        }

        let mut active = ActivePropertyOverlays::new(property, window_id);
        for overlay in self.iter_overlays_at_emacs_byte_pos(pos) {
            active.inspect_and_insert(overlay);
        }
        let winner = active.winner();

        let mut start = bounds.start();
        let mut backward = active.clone();
        let mut cursor = EmacsBytePos::new(pos.get().saturating_add(1));
        while let Some(boundary) =
            self.previous_boundary_before_since_emacs_byte_pos(cursor, bounds.start())
        {
            if boundary <= bounds.start() {
                break;
            }
            for overlay in self.index.starts_at(boundary) {
                backward.remove(overlay);
            }
            for overlay in self.index.ends_at(boundary) {
                if overlay_range(overlay).is_some_and(|range| !range.is_empty()) {
                    backward.inspect_and_insert(overlay);
                }
            }
            if !same_overlay_identity(backward.winner(), winner) {
                start = boundary;
                break;
            }
            cursor = boundary;
        }

        let mut end = bounds.end();
        let mut forward = active;
        let mut cursor = pos;
        while let Some(boundary) =
            self.next_boundary_after_until_emacs_byte_pos(cursor, bounds.end())
        {
            if boundary >= bounds.end() {
                break;
            }
            for overlay in self.index.ends_at(boundary) {
                forward.remove(overlay);
            }
            for overlay in self.index.starts_at(boundary) {
                if overlay_range(overlay).is_some_and(|range| !range.is_empty()) {
                    forward.inspect_and_insert(overlay);
                }
            }
            if !same_overlay_identity(forward.winner(), winner) {
                end = boundary;
                break;
            }
            cursor = boundary;
        }

        let value = winner
            .and_then(|overlay| overlay_property_named(overlay, property))
            .unwrap_or(Value::NIL);
        Some(OverlayPropertyExtent {
            overlay: winner,
            value,
            range: EmacsByteRange::new(start, end),
        })
    }

    pub fn highest_priority_overlay_for_inserted_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        property: &Value,
    ) -> Option<Value> {
        self.best_overlay_among(*property, self.index.overlays_touching(pos), |overlay| {
            let Some(data) = overlay.as_overlay_data() else {
                return false;
            };
            if data.buffer.is_none() {
                return false;
            }
            let range = overlay_data_range(data);
            !(range.start() == pos && data.front_advance
                || range.end() == pos && !data.rear_advance)
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
        let effects = self.index.adjust_for_text_edit(OverlayTextEdit::Insert {
            position: pos,
            length: len,
            before_markers,
        });
        self.apply_edit_effects(effects);
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
        let effects = self
            .index
            .adjust_for_text_edit(OverlayTextEdit::Delete { range });
        self.apply_edit_effects(effects);
    }

    fn apply_edit_effects(&mut self, effects: Vec<OverlayEditEffect>) {
        for effect in effects {
            match effect {
                OverlayEditEffect::Resized { overlay, range } => {
                    let _ = overlay.with_overlay_data_mut(|object| {
                        object.start = range.start().get();
                        object.end = range.end().get();
                    });
                }
                OverlayEditEffect::Evaporated {
                    overlay,
                    collapsed_at,
                } => {
                    let _ = overlay.with_overlay_data_mut(|object| {
                        object.start = collapsed_at.get();
                        object.end = collapsed_at.get();
                        object.buffer = None;
                    });
                }
            }
        }
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
        self.index
            .contains(overlay)
            .then(|| overlay.as_overlay_data().unwrap().clone())
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn next_boundary_after_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<EmacsBytePos> {
        self.next_boundary_after_until_emacs_byte_pos(pos, EmacsBytePos::new(usize::MAX))
    }

    pub fn next_boundary_after_until_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        limit: EmacsBytePos,
    ) -> Option<EmacsBytePos> {
        self.index.next_boundary_after(pos, limit)
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
        self.index.previous_boundary_before(pos, limit)
    }

    pub(crate) fn dump_overlays(&self) -> Vec<Value> {
        self.index.values().collect()
    }

    pub(crate) fn from_dump(overlays: Vec<Value>) -> Self {
        let mut list = Self::new();
        let entries: Vec<_> = overlays
            .into_iter()
            .filter_map(|overlay| {
                let data = overlay.as_overlay_data()?;
                data.buffer?;
                Some((
                    overlay,
                    EmacsByteRange::new(EmacsBytePos::new(data.start), EmacsBytePos::new(data.end)),
                ))
            })
            .collect();
        let attached = list
            .index
            .attach_batch(&entries, OverlayBatchOrder::AscendingQueryOrder);
        debug_assert!(attached, "fresh overlay list rejected its dump batch");
        list
    }

    fn best_overlay_among<I, F>(
        &self,
        property: Value,
        candidates: I,
        predicate: F,
    ) -> Option<Value>
    where
        I: IntoIterator<Item = Value>,
        F: Fn(Value) -> bool,
    {
        let mut best: Option<Value> = None;
        for overlay in candidates {
            #[cfg(test)]
            BEST_OVERLAY_CANDIDATE_INSPECTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if !predicate(overlay) {
                continue;
            }
            let Some(value) = overlay_property_named(overlay, property) else {
                continue;
            };
            if value.is_nil() {
                continue;
            }
            match best {
                None => best = Some(overlay),
                Some(current) if compare_overlay_precedence(current, overlay) == Ordering::Less => {
                    best = Some(overlay);
                }
                _ => {}
            }
        }
        best
    }
}

fn overlay_live_buffer(overlay: Value) -> Option<crate::buffer::BufferId> {
    overlay.as_overlay_data().and_then(|d| d.buffer)
}

fn overlay_data_range(data: &OverlayData) -> EmacsByteRange {
    let (start, end) = data.current_range();
    EmacsByteRange::new(EmacsBytePos::new(start), EmacsBytePos::new(end))
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

fn same_overlay_identity(left: Option<Value>, right: Option<Value>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => eq_value(&left, &right),
        (None, None) => true,
        _ => false,
    }
}

fn overlay_property_named(overlay: Value, prop_name: Value) -> Option<Value> {
    let plist = overlay.as_overlay_data()?.plist;
    plist::plist_get(plist, &prop_name)
}

fn overlay_property_in_lookup_order(
    overlay: Value,
    property_lookup_order: &[Value],
) -> Option<Value> {
    let (canonical, aliases) = property_lookup_order.split_first()?;

    // GNU `lookup_char_property` returns a directly present canonical value
    // immediately, even when that value is nil.  A canonical nil therefore
    // blocks alias fallback; the caller subsequently decides that this
    // overlay is not a carrier and may continue to a lower overlay.
    if let Some(value) = overlay_property_named(overlay, *canonical) {
        return Some(value);
    }

    // Aliases are fallback candidates: GNU keeps scanning while their values
    // are nil and returns the first non-nil one.
    aliases.iter().find_map(|property| {
        overlay_property_named(overlay, *property).filter(|value| !value.is_nil())
    })
}

/// Whether `overlay`'s contributions apply in the window being laid out. GNU
/// restricts an overlay carrying a `window` property to that window only (e.g.
/// hl-line with a non-sticky flag). A missing or non-window `window` property is
/// unrestricted, and `window_id == None` (no window context) applies every
/// overlay. Mirrors the layout engine's same-named check, one abstraction level
/// down (raw overlay `Value` rather than a buffer view).
fn overlay_applies_to_window(overlay: Value, window_id: Option<u64>) -> bool {
    let Some(window_prop) = overlay_property_named(overlay, Value::symbol("window")) else {
        return true;
    };
    let Some(target) = window_prop.as_window_id() else {
        return true;
    };
    window_id.is_none_or(|current| current == target)
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
        for overlay in self.index.values() {
            roots.push(overlay);
        }
    }
}

#[cfg(test)]
#[path = "overlay_test.rs"]
mod tests;
