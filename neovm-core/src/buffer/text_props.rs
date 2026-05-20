//! GNU-style text interval storage for buffers and strings.
//!
//! GNU Emacs represents text properties as an interval tree rooted from the
//! owning string or buffer.  Neomacs keeps the existing Rust API name
//! (`TextPropertyTable`) for callers, and follows GNU's mutation shape: split
//! at the edit range, change the affected interval plists, and preserve raw
//! interval boundaries.  Higher-level property-change queries decide whether
//! adjacent interval plists are semantically equal.

use std::collections::BTreeMap;

use crate::emacs_core::eval::{
    push_scratch_gc_root, restore_scratch_gc_roots, save_scratch_gc_roots,
};
use crate::emacs_core::value::{Value, eq_value, equal_value};
use crate::gc_trace::GcTrace;

// ---------------------------------------------------------------------------
// PropertyInterval
// ---------------------------------------------------------------------------

/// Public snapshot of one text-property interval.
///
/// Runtime storage uses the same start/end/plist shape.  This type remains the
/// serialization and inspection shape used by pdump/tests.  Bounds are character
/// positions, matching GNU intervals; buffer owners convert byte positions at
/// the boundary.
#[derive(Clone, Debug)]
pub struct PropertyInterval {
    /// Character position where this interval starts (inclusive).
    pub start: usize,
    /// Character position where this interval ends (exclusive).
    pub end: usize,
    /// Snapshot map for the interval plist.
    pub properties: HashMap<Value, Value>,
    /// Property keys in GNU plist order, newest first.
    pub(crate) key_order: Vec<Value>,
}

impl PropertyInterval {
    fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            properties: HashMap::new(),
            key_order: Vec::new(),
        }
    }

    pub fn with_properties(start: usize, end: usize, properties: HashMap<Value, Value>) -> Self {
        let key_order: Vec<Value> = properties.keys().copied().collect();
        Self {
            start,
            end,
            properties,
            key_order,
        }
    }

    fn from_plist(start: usize, end: usize, plist: &[(Value, Value)]) -> Self {
        let mut properties = HashMap::new();
        for (key, value) in plist.iter().rev() {
            properties.insert(*key, *value);
        }
        let mut key_order = Vec::new();
        for (key, _) in plist {
            if !key_order.iter().any(|seen| eq_value(seen, key)) {
                key_order.push(*key);
            }
        }
        Self {
            start,
            end,
            properties,
            key_order,
        }
    }

    fn into_plist(self) -> Vec<(Value, Value)> {
        let mut plist = Vec::new();
        for key in &self.key_order {
            if let Some(value) = self.properties.get(key)
                && !plist.iter().any(|(seen, _)| eq_value(seen, key))
            {
                plist.push((*key, *value));
            }
        }
        for (key, value) in self.properties {
            if !plist.iter().any(|(seen, _)| eq_value(seen, &key)) {
                plist.push((key, value));
            }
        }
        plist
    }

    /// Iterate properties in GNU plist order.
    pub fn ordered_properties(&self) -> impl Iterator<Item = (Value, &Value)> {
        self.key_order
            .iter()
            .filter_map(move |key| self.properties.get(key).map(|value| (*key, value)))
    }
}

use std::collections::HashMap;

type IntervalPlist = Value;

fn plist_value_from_pairs(plist: &[(Value, Value)]) -> Value {
    let mut items = Vec::with_capacity(plist.len() * 2);
    for (key, value) in plist {
        items.push(*key);
        items.push(*value);
    }
    Value::list(items)
}

fn plist_pairs(plist: Value) -> Vec<(Value, Value)> {
    let mut pairs = Vec::new();
    let mut tail = plist;
    while tail.is_cons() {
        let key = tail.cons_car();
        let rest = tail.cons_cdr();
        if !rest.is_cons() {
            break;
        }
        pairs.push((key, rest.cons_car()));
        tail = rest.cons_cdr();
    }
    pairs
}

fn plist_value_prepend_pair(plist: Value, key: Value, value: Value) -> Value {
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(plist);
    push_scratch_gc_root(key);
    push_scratch_gc_root(value);

    let value_cell = Value::cons(value, plist);
    push_scratch_gc_root(value_cell);
    let result = Value::cons(key, value_cell);

    restore_scratch_gc_roots(saved);
    result
}

fn plist_value_get(plist: Value, key: Value) -> Option<Value> {
    let mut tail = plist;
    while tail.is_cons() {
        let name = tail.cons_car();
        let rest = tail.cons_cdr();
        if !rest.is_cons() {
            return None;
        }
        if eq_value(&name, &key) {
            return Some(rest.cons_car());
        }
        tail = rest.cons_cdr();
    }
    None
}

fn plist_value_put_replace(plist: &mut Value, key: Value, value: Value) -> bool {
    let mut tail = *plist;
    while tail.is_cons() {
        let name = tail.cons_car();
        let rest = tail.cons_cdr();
        if !rest.is_cons() {
            break;
        }
        if eq_value(&name, &key) {
            let existing = rest.cons_car();
            if eq_value(&existing, &value) {
                return false;
            }
            rest.set_car(value);
            return true;
        }
        tail = rest.cons_cdr();
    }

    let mut pairs = plist_pairs(*plist);
    pairs.insert(0, (key, value));
    *plist = plist_value_from_pairs(&pairs);
    true
}

fn plist_value_remove(plist: &mut Value, key: Value) -> bool {
    let mut pairs = plist_pairs(*plist);
    let before = pairs.len();
    pairs.retain(|(name, _)| !eq_value(name, &key));
    if pairs.len() == before {
        return false;
    }
    *plist = plist_value_from_pairs(&pairs);
    true
}

// ---------------------------------------------------------------------------
// IntervalNode — internal tree node matching GNU struct interval
// ---------------------------------------------------------------------------

/// Internal interval node stored in the BTreeMap.
///
/// Mirrors GNU's `struct interval` cached fields (`front_sticky`,
/// `rear_sticky`, `write_protect`, `visible`, `plist`).  The BTreeMap
/// key is the start position; `end` is stored in the value.
#[derive(Clone, Debug)]
struct IntervalNode {
    end: usize,
    front_sticky: bool,
    rear_sticky: bool,
    write_protect: bool,
    visible: bool,
    plist: IntervalPlist,
}

impl IntervalNode {
    fn new(end: usize, plist: IntervalPlist) -> Self {
        let (front_sticky, rear_sticky, write_protect, visible) = Self::extract_cached(plist);
        Self {
            end,
            front_sticky,
            rear_sticky,
            write_protect,
            visible,
            plist,
        }
    }

    fn default(start: usize, end: usize) -> Self {
        Self::new(end, Value::NIL)
    }

    fn with_cached(
        end: usize,
        front_sticky: bool,
        rear_sticky: bool,
        write_protect: bool,
        visible: bool,
        plist: IntervalPlist,
    ) -> Self {
        Self {
            end,
            front_sticky,
            rear_sticky,
            write_protect,
            visible,
            plist,
        }
    }

    fn is_empty_plist(&self) -> bool {
        self.plist.is_nil()
    }

    /// Extract cached booleans from a plist (mirrors GNU's cache bits).
    fn extract_cached(plist: Value) -> (bool, bool, bool, bool) {
        let mut front_sticky = false;
        let mut rear_sticky = false;
        let mut write_protect = false;
        let mut visible = true;
        for (key, value) in plist_pairs(plist) {
            if key.is_symbol() {
                let name = key.as_symbol_name().unwrap_or("");
                match name {
                    "front-sticky" => front_sticky = value.is_truthy(),
                    "rear-nonsticky" => rear_sticky = !value.is_truthy(),
                    "read-only" => write_protect = value.is_truthy(),
                    "invisible" => visible = !value.is_truthy(),
                    _ => {}
                }
            }
        }
        (front_sticky, rear_sticky, write_protect, visible)
    }

    /// Re-extract and update cached booleans from current plist.
    fn refresh_cache(&mut self) {
        let (fs, rs, wp, vis) = Self::extract_cached(self.plist);
        self.front_sticky = fs;
        self.rear_sticky = rs;
        self.write_protect = wp;
        self.visible = vis;
    }
}

// ---------------------------------------------------------------------------
// Plist helpers
// ---------------------------------------------------------------------------

fn plists_equal_eq(left: &[(Value, Value)], right: &[(Value, Value)]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter().all(|(left_key, left_value)| {
        right.iter().any(|(right_key, right_value)| {
            eq_value(left_key, right_key) && eq_value(left_value, right_value)
        })
    })
}

fn plists_equal_values_equal(left: &[(Value, Value)], right: &[(Value, Value)]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter().all(|(left_key, left_value)| {
        right.iter().any(|(right_key, right_value)| {
            eq_value(left_key, right_key) && equal_value(left_value, right_value, 0)
        })
    })
}

fn plist_is_empty(plist: &[(Value, Value)]) -> bool {
    plist.is_empty()
}

// ---------------------------------------------------------------------------
// TextPropertyTable
// ---------------------------------------------------------------------------

/// GNU-style text-property interval storage.
///
/// Internally uses `BTreeMap<usize, IntervalNode>` for O(log n) queries,
/// matching GNU Emacs' augmented interval tree performance.  The key is
/// the start character position; nodes store `end`, cached sticky/write/visible
/// flags, and the property list.
#[derive(Clone, Debug)]
pub struct TextPropertyTable {
    intervals: BTreeMap<usize, IntervalNode>,
}

impl TextPropertyTable {
    pub fn new() -> Self {
        Self {
            intervals: BTreeMap::new(),
        }
    }

    /// Copy interval nodes and plist cons spines, while preserving plist
    /// values by reference.  GNU `copy_intervals` copies intervals and
    /// `copy_properties` copies `interval->plist` with `copy-sequence`.
    pub fn copy_interval_plist_spines(&self) -> Self {
        let mut copy = self.clone();
        for node in copy.intervals.values_mut() {
            node.plist = plist_value_from_pairs(&plist_pairs(node.plist));
        }
        copy
    }

    // -- Query helpers -------------------------------------------------------

    /// Find the interval containing `pos`, returning its (start, node) pair.
    fn find_interval(&self, pos: usize) -> Option<(usize, &IntervalNode)> {
        self.intervals
            .range(..=pos)
            .next_back()
            .filter(|(_, node)| pos < node.end)
            .map(|(s, n)| (*s, n))
    }

    /// Find the interval containing `pos`, returning mutable (start, node).
    fn find_interval_mut(&mut self, pos: usize) -> Option<(usize, &mut IntervalNode)> {
        // BTreeMap doesn't support range_mut with ..= efficiently,
        // so split and re-approach.
        let Some((&start, _)) = self.intervals.range(..=pos).next_back() else {
            return None;
        };
        if self.intervals.get(&start)?.end <= pos {
            return None;
        }
        self.intervals.get_mut(&start).map(|node| (start, node))
    }

    // -- Split and merge helpers --------------------------------------------

    /// Ensure no interval straddles `pos`. If one does, split it into two:
    /// [start, pos) and [pos, original end).  Idempotent if already split.
    fn split_at(&mut self, pos: usize) {
        let Some((start, node)) = self.find_interval(pos) else {
            return;
        };
        let end = node.end;
        if start == pos || end == pos {
            return;
        }
        if start < pos && pos < end {
            let plist = node.plist;
            let right_plist = plist_value_from_pairs(&plist_pairs(plist));
            let (fs, rs, wp, vis) = (
                node.front_sticky,
                node.rear_sticky,
                node.write_protect,
                node.visible,
            );
            // Left part: [start, pos)
            self.intervals.get_mut(&start).unwrap().end = pos;
            self.intervals.get_mut(&start).unwrap().refresh_cache();
            // Right part: [pos, end)
            self.intervals.insert(
                pos,
                IntervalNode::with_cached(end, fs, rs, wp, vis, right_plist),
            );
        }
    }

    /// Prune empty-plist intervals that don't encode a real gap.
    ///
    /// GNU keeps interval boundaries even when adjacent plists are equal; for
    /// example, two adjacent `put-text-property' calls can produce two equal
    /// intervals, and `(next-property-change POS OBJ t)' must still expose the
    /// raw boundary.  Empty intervals remain only when they separate non-empty
    /// runs, which is enough for the sparse representation to model implicit
    /// nil-property text without losing change boundaries.
    fn prune_empty_intervals_after_mutation(&mut self) {
        // Remove leading and trailing empty-plist intervals
        while let Some((&first_key, first_node)) = self.intervals.first_key_value() {
            if first_node.is_empty_plist() {
                self.intervals.remove(&first_key);
            } else {
                break;
            }
        }
        while let Some((&last_key, last_node)) = self.intervals.last_key_value() {
            if last_node.is_empty_plist() {
                self.intervals.remove(&last_key);
            } else {
                break;
            }
        }

        // Also remove interior empty-plist intervals that are surrounded
        // by the same empty state (i.e., not needed to separate different
        // property runs)
        let keys: Vec<usize> = self.intervals.keys().copied().collect();
        for k in &keys {
            if !self.intervals.contains_key(k) {
                continue;
            }
            if self
                .intervals
                .get(k)
                .map(|n| n.is_empty_plist())
                .unwrap_or(false)
            {
                let prev = self.intervals.range(..*k).next_back().map(|(pk, _)| *pk);
                let next = self.intervals.range(k + 1..).next().map(|(nk, _)| *nk);
                let before_is_nonempty = prev
                    .map(|p| {
                        self.intervals
                            .get(&p)
                            .map(|n| !n.is_empty_plist())
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                let after_is_nonempty = next
                    .map(|n| {
                        self.intervals
                            .get(&n)
                            .map(|n| !n.is_empty_plist())
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                // Keep empty interval only if it separates two non-empty ones
                if !before_is_nonempty || !after_is_nonempty {
                    self.intervals.remove(k);
                }
            }
        }
    }

    // -- Public API ----------------------------------------------------------

    pub fn put_property(&mut self, start: usize, end: usize, name: Value, value: Value) -> bool {
        if start >= end {
            return false;
        }
        self.split_at(start);
        self.split_at(end);

        let affected: Vec<usize> = self.intervals.range(start..end).map(|(k, _)| *k).collect();
        let mut changed = false;
        let mut cursor = start;

        for key in affected {
            if cursor < key {
                let mut node = IntervalNode::default(cursor, key);
                plist_value_put_replace(&mut node.plist, name, value);
                node.refresh_cache();
                self.intervals.insert(cursor, node);
                changed = true;
            }
            if let Some(node) = self.intervals.get_mut(&key) {
                if plist_value_put_replace(&mut node.plist, name, value) {
                    node.refresh_cache();
                    changed = true;
                }
                cursor = node.end.min(end);
            }
        }

        if cursor < end {
            let mut node = IntervalNode::default(cursor, end);
            plist_value_put_replace(&mut node.plist, name, value);
            node.refresh_cache();
            self.intervals.insert(cursor, node);
            changed = true;
        }

        changed
    }

    pub(crate) fn from_plist_runs(runs: Vec<(usize, usize, Vec<(Value, Value)>)>) -> Self {
        let mut table = Self::new();
        for (start, end, plist) in runs {
            if start < end && !plist.is_empty() {
                table.intervals.insert(
                    start,
                    IntervalNode::new(end, plist_value_from_pairs(&plist)),
                );
            }
        }
        table.prune_empty_intervals_after_mutation();
        table
    }

    pub fn get_property(&self, pos: usize, name: Value) -> Option<Value> {
        let (_, node) = self.find_interval(pos)?;
        plist_value_get(node.plist, name)
    }

    pub fn get_properties(&self, pos: usize) -> HashMap<Value, Value> {
        let Some((start, node)) = self.find_interval(pos) else {
            return HashMap::new();
        };
        PropertyInterval::from_plist(start, node.end, &plist_pairs(node.plist)).properties
    }

    pub fn get_properties_ordered(&self, pos: usize) -> Vec<(Value, Value)> {
        let Some((_, node)) = self.find_interval(pos) else {
            return Vec::new();
        };
        plist_pairs(node.plist)
    }

    pub fn get_properties_plist_value(&self, pos: usize) -> Value {
        let Some((_, node)) = self.find_interval(pos) else {
            return Value::NIL;
        };
        node.plist
    }

    pub fn remove_property(&mut self, start: usize, end: usize, name: Value) -> bool {
        if start >= end {
            return false;
        }
        self.split_at(start);
        self.split_at(end);

        let affected: Vec<usize> = self.intervals.range(start..end).map(|(k, _)| *k).collect();

        let mut changed = false;
        for key in &affected {
            if let Some(node) = self.intervals.get_mut(key) {
                if plist_value_remove(&mut node.plist, name) {
                    node.refresh_cache();
                    changed = true;
                }
            }
        }

        changed
    }

    pub fn remove_all_properties(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        self.split_at(start);
        self.split_at(end);

        let affected: Vec<usize> = self.intervals.range(start..end).map(|(k, _)| *k).collect();

        for key in &affected {
            if let Some(node) = self.intervals.get_mut(key) {
                node.plist = Value::NIL;
                node.refresh_cache();
            }
        }
    }

    pub fn set_properties(&mut self, start: usize, end: usize, plist: Vec<(Value, Value)>) {
        if start >= end {
            return;
        }
        self.split_at(start);
        self.split_at(end);

        let affected: Vec<usize> = self.intervals.range(start..end).map(|(k, _)| *k).collect();
        for key in affected {
            self.intervals.remove(&key);
        }

        self.intervals.insert(
            start,
            IntervalNode::new(end, plist_value_from_pairs(&plist)),
        );
        self.prune_empty_intervals_after_mutation();
    }

    pub fn next_property_change(&self, pos: usize) -> Option<usize> {
        let current = self.plist_at(pos).unwrap_or_default();
        let mut cursor = pos;
        while let Some(next) = self.next_interval_boundary(cursor) {
            let next_plist = self.plist_at(next).unwrap_or_default();
            if !plists_equal_eq(&current, &next_plist) {
                return Some(next);
            }
            if next <= cursor {
                return None;
            }
            cursor = next;
        }
        None
    }

    pub fn previous_property_change(&self, pos: usize) -> Option<usize> {
        if pos == 0 {
            return None;
        }

        let current = self.plist_at(pos - 1).unwrap_or_default();
        let mut cursor = pos;
        while let Some(prev) = self.previous_interval_boundary(cursor) {
            if prev == 0 {
                return None;
            }
            let previous_plist = self.plist_at(prev - 1).unwrap_or_default();
            if !plists_equal_eq(&current, &previous_plist) {
                return Some(prev);
            }
            if prev >= cursor {
                return None;
            }
            cursor = prev;
        }

        None
    }

    /// Return the next raw interval boundary after `pos`, even when adjacent
    /// interval plists are equal.  This matches GNU's `next_interval` path used
    /// by `(next-property-change POS OBJECT t)`.
    pub fn next_interval_boundary(&self, pos: usize) -> Option<usize> {
        if let Some((_, node)) = self.find_interval(pos) {
            return Some(node.end);
        }

        self.intervals
            .range((pos + 1)..)
            .next()
            .map(|(start, _)| *start)
    }

    /// Return the previous raw interval boundary before `pos`.
    pub fn previous_interval_boundary(&self, pos: usize) -> Option<usize> {
        if pos == 0 {
            return None;
        }

        let scan_pos = pos - 1;
        if let Some((start, _)) = self.find_interval(scan_pos) {
            return Some(start);
        }

        self.intervals
            .range(..=scan_pos)
            .next_back()
            .map(|(_, node)| node.end)
            .filter(|end| *end < pos)
    }

    pub fn adjust_for_insert(&mut self, pos: usize, len: usize) {
        if len == 0 {
            return;
        }

        self.split_at(pos);

        // Pop intervals with start >= pos, shift by +len, re-insert
        let shifted_keys: Vec<usize> = self.intervals.range(pos..).map(|(k, _)| *k).collect();

        let mut shifted: Vec<(usize, IntervalNode)> = Vec::new();
        for key in &shifted_keys {
            if let Some(node) = self.intervals.remove(key) {
                shifted.push((*key, node));
            }
        }

        for (old_key, mut node) in shifted {
            node.end += len;
            self.intervals.insert(old_key + len, node);
        }

        // Insert a default gap interval at [pos, pos+len) if pos is not already
        // the start of an interval AND there wasn't one already
        if !self.intervals.contains_key(&pos) {
            // Find what interval covers pos (should be the left part from split)
            if let Some((_, node)) = self.find_interval(pos) {
                // pos is inside an existing interval, gap is covered
            } else if self
                .intervals
                .range(pos..)
                .next()
                .map(|(k, _)| *k)
                .unwrap_or(pos + len)
                > pos
            {
                // Insert default interval for the gap
                let node = IntervalNode::default(pos, pos + len);
                self.intervals.insert(pos, node);
            }
        }

        self.prune_empty_intervals_after_mutation();
    }

    pub fn adjust_for_delete(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }

        let len = end - start;
        let old_intervals = std::mem::take(&mut self.intervals);
        let mut adjusted = BTreeMap::new();

        for (old_start, mut node) in old_intervals {
            let old_end = node.end;

            let (new_start, new_end) = if old_end <= start {
                (old_start, old_end)
            } else if old_start >= end {
                (old_start - len, old_end - len)
            } else if old_start < start && old_end > end {
                (old_start, old_end - len)
            } else if old_start < start {
                (old_start, start)
            } else if old_end > end {
                (start, old_end - len)
            } else {
                continue;
            };

            if new_start < new_end {
                node.end = new_end;
                adjusted.insert(new_start, node);
            }
        }

        self.intervals = adjusted;

        self.prune_empty_intervals_after_mutation();
    }

    pub fn intervals_snapshot(&self) -> Vec<PropertyInterval> {
        self.intervals
            .iter()
            .filter(|(_, node)| !node.is_empty_plist())
            .map(|(start, node)| {
                PropertyInterval::from_plist(*start, node.end, &plist_pairs(node.plist))
            })
            .collect()
    }

    /// Return GNU `object-intervals' shaped runs for a string of `len' chars.
    ///
    /// Unlike `intervals_snapshot', this keeps nil-property gaps once an
    /// interval tree exists.  GNU reports those gaps so callers can observe the
    /// complete interval partition of the string.
    pub fn object_interval_runs(&self, len: usize) -> Vec<(usize, usize, Vec<(Value, Value)>)> {
        self.object_interval_plist_runs(len)
            .into_iter()
            .map(|(start, end, plist)| (start, end, plist_pairs(plist)))
            .collect()
    }

    /// Return GNU `object-intervals' shaped runs with the live interval plist.
    ///
    /// GNU `collect_interval' stores `interval->plist' directly in the
    /// returned structure, so later property replacement can be visible through
    /// that plist object.
    pub fn object_interval_plist_runs(&self, len: usize) -> Vec<(usize, usize, Value)> {
        if self.intervals.is_empty() {
            return Vec::new();
        }

        let mut runs = Vec::new();
        let mut cursor = 0;
        for (&start, node) in &self.intervals {
            let start = start.min(len);
            let end = node.end.min(len);
            if cursor < start {
                runs.push((cursor, start, Value::NIL));
            }
            if start < end {
                runs.push((start, end, node.plist));
                cursor = end;
            }
        }
        if cursor < len {
            runs.push((cursor, len, Value::NIL));
        }
        runs
    }

    pub fn first_interval_pos_with_property_eq(
        &self,
        start: usize,
        end: usize,
        name: Value,
        value: Value,
    ) -> Option<usize> {
        if start >= end {
            return None;
        }
        for (&interval_start, node) in self.intervals.range(..end) {
            if node.end <= start {
                continue;
            }
            if interval_start >= end {
                break;
            }
            if plist_value_get(node.plist, name).is_some_and(|found| eq_value(&found, &value)) {
                return Some(interval_start.max(start));
            }
        }

        None
    }

    pub(crate) fn try_for_each_interval_in_range<E>(
        &self,
        start: usize,
        end: usize,
        mut f: impl FnMut(usize, usize, &[(Value, Value)]) -> Result<(), E>,
    ) -> Result<(), E> {
        if start >= end {
            return Ok(());
        }
        for (interval_start, node) in self.intervals.range(..end) {
            if node.end <= start {
                continue;
            }
            if *interval_start >= end {
                break;
            }
            let pairs = plist_pairs(node.plist);
            f(*interval_start, node.end, &pairs)?;
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    fn plist_at(&self, pos: usize) -> Option<Vec<(Value, Value)>> {
        let (_, node) = self.find_interval(pos)?;
        Some(plist_pairs(node.plist))
    }

    fn next_interval_boundary_after(&self, pos: usize, end: usize) -> usize {
        let containing_end = self
            .find_interval(pos)
            .map(|(_, node)| node.end)
            .unwrap_or(end);
        let next_start = self
            .intervals
            .range((pos + 1)..)
            .next()
            .map(|(start, _)| *start)
            .unwrap_or(end);
        containing_end.min(next_start).min(end)
    }

    /// Compare string text-property intervals the way GNU
    /// `compare_string_intervals` does for `equal-including-properties`.
    ///
    /// Missing intervals are default intervals.  Property names compare by
    /// `eq`, while property values compare by ordinary `equal`.
    pub fn equal_including_property_values(
        left: Option<&TextPropertyTable>,
        right: Option<&TextPropertyTable>,
        len: usize,
    ) -> bool {
        let mut pos = 0;
        while pos < len {
            let left_plist = left
                .and_then(|table| table.plist_at(pos))
                .unwrap_or_default();
            let right_plist = right
                .and_then(|table| table.plist_at(pos))
                .unwrap_or_default();
            if !plists_equal_values_equal(&left_plist, &right_plist) {
                return false;
            }

            let left_next = left
                .map(|table| table.next_interval_boundary_after(pos, len))
                .unwrap_or(len);
            let right_next = right
                .map(|table| table.next_interval_boundary_after(pos, len))
                .unwrap_or(len);
            let next = left_next.min(right_next).max(pos + 1);
            pos = next;
        }
        true
    }

    pub fn slice(&self, start: usize, end: usize) -> TextPropertyTable {
        if start >= end {
            return TextPropertyTable::new();
        }

        let runs: Vec<(usize, usize, Vec<(Value, Value)>)> = self
            .intervals
            .range(..end)
            .rev()
            .take_while(|(_, node)| node.end > start)
            .map(|(interval_start, node)| {
                let new_start = (*interval_start).max(start) - start;
                let new_end = node.end.min(end) - start;
                (new_start, new_end, plist_pairs(node.plist))
            })
            .filter(|(new_start, new_end, _)| new_start < new_end)
            .collect();

        TextPropertyTable::from_plist_runs(runs)
    }

    pub fn slice_copy_text_properties(&self, start: usize, end: usize) -> TextPropertyTable {
        if start >= end {
            return TextPropertyTable::new();
        }

        let mut table = TextPropertyTable::new();
        for (interval_start, node) in self
            .intervals
            .range(..end)
            .filter(|(_, node)| node.end > start)
        {
            let new_start = (*interval_start).max(start) - start;
            let new_end = node.end.min(end) - start;
            if new_start >= new_end {
                continue;
            }
            for (name, value) in plist_pairs(node.plist) {
                table.put_property(new_start, new_end, name, value);
            }
        }
        table
    }

    pub fn append_shifted(&mut self, other: &TextPropertyTable, offset: usize) {
        for (&start, node) in &other.intervals {
            self.intervals.insert(
                start + offset,
                IntervalNode::new(node.end + offset, node.plist),
            );
        }
        self.prune_empty_intervals_after_mutation();
    }

    pub fn append_shifted_via_add_text_properties(
        &mut self,
        other: &TextPropertyTable,
        offset: usize,
    ) {
        for (&start, node) in &other.intervals {
            for (name, value) in plist_pairs(node.plist) {
                self.put_property(start + offset, node.end + offset, name, value);
            }
        }
    }

    pub fn merge_missing_shifted(&mut self, other: &TextPropertyTable, offset: usize) {
        for (&start, node) in &other.intervals {
            if node.is_empty_plist() {
                continue;
            }
            let shifted_start = start + offset;
            let shifted_end = node.end + offset;

            self.split_at(shifted_start);
            self.split_at(shifted_end);

            // Modify all intervals fully within [shifted_start, shifted_end)
            let affected: Vec<usize> = self
                .intervals
                .range(shifted_start..shifted_end)
                .map(|(k, _)| *k)
                .collect();

            for key in &affected {
                if let Some(target) = self.intervals.get_mut(key) {
                    for (name, value) in plist_pairs(node.plist) {
                        if plist_value_get(target.plist, name).is_none() {
                            target.plist = plist_value_prepend_pair(target.plist, name, value);
                        }
                    }
                    target.refresh_cache();
                }
            }

            // If no intervals in range, insert the source directly
            if affected.is_empty() {
                self.intervals
                    .insert(shifted_start, IntervalNode::new(shifted_end, node.plist));
            }
        }
        self.prune_empty_intervals_after_mutation();
    }

    pub fn merge_adjacent_equal_properties_around(&mut self, start: usize, end: usize) {
        if start > end || self.intervals.len() < 2 {
            return;
        }

        loop {
            let keys: Vec<usize> = self.intervals.keys().copied().collect();
            let mut merged = false;

            for pair in keys.windows(2) {
                let left_start = pair[0];
                let right_start = pair[1];

                let Some(left) = self.intervals.get(&left_start) else {
                    continue;
                };
                let Some(right) = self.intervals.get(&right_start) else {
                    continue;
                };
                if left.end != right_start
                    || left.plist.is_nil()
                    || !plists_equal_eq(&plist_pairs(left.plist), &plist_pairs(right.plist))
                    || left.end < start
                    || right_start > end
                {
                    continue;
                }

                let right_end = right.end;
                if let Some(left) = self.intervals.get_mut(&left_start) {
                    left.end = right_end;
                    left.refresh_cache();
                }
                self.intervals.remove(&right_start);
                merged = true;
                break;
            }

            if !merged {
                break;
            }
        }
    }

    pub(crate) fn dump_intervals(&self) -> Vec<PropertyInterval> {
        self.intervals_snapshot()
    }

    pub(crate) fn from_dump(intervals: Vec<PropertyInterval>) -> Self {
        let mut table = Self::new();
        for interval in intervals {
            if interval.start < interval.end {
                table.intervals.insert(
                    interval.start,
                    IntervalNode::new(interval.end, plist_value_from_pairs(&interval.into_plist())),
                );
            }
        }
        table.prune_empty_intervals_after_mutation();
        table
    }

    pub(crate) fn for_each_root(&self, mut f: impl FnMut(Value)) {
        for (_, node) in &self.intervals {
            f(node.plist);
        }
    }
}

impl Default for TextPropertyTable {
    fn default() -> Self {
        Self::new()
    }
}

impl GcTrace for TextPropertyTable {
    fn trace_roots(&self, roots: &mut Vec<Value>) {
        self.for_each_root(|value| roots.push(value));
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[path = "text_props_test.rs"]
mod tests;
