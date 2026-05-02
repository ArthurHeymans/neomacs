//! GNU-style text interval storage for buffers and strings.
//!
//! GNU Emacs represents text properties as an interval tree rooted from the
//! owning string or buffer.  Neomacs keeps the existing Rust API name
//! (`TextPropertyTable`) for callers, and follows GNU's mutation shape: split
//! at the edit range, change the affected interval plists, then merge adjacent
//! intervals with equal plists.

use std::collections::BTreeMap;

use crate::emacs_core::value::{Value, eq_value};
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

type IntervalPlist = Vec<(Value, Value)>;

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
        let (front_sticky, rear_sticky, write_protect, visible) = Self::extract_cached(&plist);
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
        Self::new(end, Vec::new())
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
        self.plist.is_empty()
    }

    /// Extract cached booleans from a plist (mirrors GNU's cache bits).
    fn extract_cached(plist: &[(Value, Value)]) -> (bool, bool, bool, bool) {
        let mut front_sticky = false;
        let mut rear_sticky = false;
        let mut write_protect = false;
        let mut visible = true;
        for (key, value) in plist {
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
        let (fs, rs, wp, vis) = Self::extract_cached(&self.plist);
        self.front_sticky = fs;
        self.rear_sticky = rs;
        self.write_protect = wp;
        self.visible = vis;
    }
}

// ---------------------------------------------------------------------------
// Plist helpers
// ---------------------------------------------------------------------------

fn plist_get(plist: &[(Value, Value)], key: Value) -> Option<&Value> {
    plist
        .iter()
        .find_map(|(name, value)| eq_value(name, &key).then_some(value))
}

fn plist_put_replace(plist: &mut IntervalPlist, key: Value, value: Value) -> bool {
    for (name, existing) in plist.iter_mut() {
        if eq_value(name, &key) {
            if eq_value(existing, &value) {
                return false;
            }
            *existing = value;
            return true;
        }
    }
    plist.insert(0, (key, value));
    true
}

fn plist_remove(plist: &mut IntervalPlist, key: Value) -> bool {
    let before = plist.len();
    plist.retain(|(name, _)| !eq_value(name, &key));
    before != plist.len()
}

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
            let plist = node.plist.clone();
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
            self.intervals
                .insert(pos, IntervalNode::with_cached(end, fs, rs, wp, vis, plist));
        }
    }

    /// After mutations in [range_start, range_end], merge adjacent intervals
    /// that have equal plists. Also removes empty-plist intervals unless they
    /// are needed to cover gaps between non-empty ones (we remove them).
    fn merge_adjacent_after_mutation(&mut self, range_start: usize, _range_end: usize) {
        // Iteratively merge adjacent equal-plist intervals until stable.
        // This handles cascading merges (A=B, B=C → A=C) correctly.
        loop {
            let sorted_keys: Vec<usize> = {
                let mut ks: Vec<usize> = self.intervals.keys().copied().collect();
                ks.sort_unstable();
                ks
            };

            let mut merged_any = false;
            for window in sorted_keys.windows(2) {
                let (k1, k2) = (window[0], window[1]);
                let should_merge = {
                    let n1 = &self.intervals[&k1];
                    let n2 = &self.intervals[&k2];
                    n1.end >= k2 && plists_equal_eq(&n1.plist, &n2.plist)
                };
                if should_merge {
                    let new_end = self.intervals[&k2].end;
                    if let Some(into) = self.intervals.get_mut(&k1) {
                        into.end = new_end;
                    }
                    self.intervals.remove(&k2);
                    merged_any = true;
                    break; // restart the loop since keys changed
                }
            }

            if !merged_any {
                break;
            }
        }

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

        // Collect affected keys
        let affected: Vec<usize> = self.intervals.range(start..end).map(|(k, _)| *k).collect();

        if affected.is_empty() {
            // No properties yet — insert a new interval
            let mut node = IntervalNode::default(start, end);
            plist_put_replace(&mut node.plist, name, value);
            node.refresh_cache();
            self.intervals.insert(start, node);
            self.merge_adjacent_after_mutation(start, end);
            return true;
        }

        let mut changed = false;
        for key in &affected {
            if let Some(node) = self.intervals.get_mut(key) {
                if plist_put_replace(&mut node.plist, name, value) {
                    node.refresh_cache();
                    changed = true;
                }
            }
        }

        // Extend coverage: if the range [start, end) extends beyond the
        // last affected interval, insert default intervals for the gap.
        // Then modify those defaults too.
        let last_end = affected
            .iter()
            .filter_map(|k| self.intervals.get(k).map(|n| n.end))
            .max();
        let covered_end = last_end.unwrap_or(start);
        if covered_end < end {
            let ext_node = IntervalNode::default(covered_end, end);
            self.intervals.insert(covered_end, ext_node);
            let ext_keys: Vec<usize> = self
                .intervals
                .range(covered_end..end)
                .map(|(k, _)| *k)
                .collect();
            for key in &ext_keys {
                if let Some(node) = self.intervals.get_mut(key) {
                    if plist_put_replace(&mut node.plist, name, value) {
                        node.refresh_cache();
                        changed = true;
                    }
                }
            }
        }

        if changed {
            self.merge_adjacent_after_mutation(start, end);
        }
        changed
    }

    pub(crate) fn from_plist_runs(runs: Vec<(usize, usize, Vec<(Value, Value)>)>) -> Self {
        let mut table = Self::new();
        for (start, end, plist) in runs {
            if start < end && !plist.is_empty() {
                table.intervals.insert(start, IntervalNode::new(end, plist));
            }
        }
        table.merge_adjacent_after_mutation(0, usize::MAX);
        table
    }

    pub fn get_property(&self, pos: usize, name: Value) -> Option<&Value> {
        let (_, node) = self.find_interval(pos)?;
        plist_get(&node.plist, name)
    }

    pub fn get_properties(&self, pos: usize) -> HashMap<Value, Value> {
        let Some((start, node)) = self.find_interval(pos) else {
            return HashMap::new();
        };
        PropertyInterval::from_plist(start, node.end, &node.plist).properties
    }

    pub fn get_properties_ordered(&self, pos: usize) -> Vec<(Value, Value)> {
        let Some((_, node)) = self.find_interval(pos) else {
            return Vec::new();
        };
        node.plist.clone()
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
                if plist_remove(&mut node.plist, name) {
                    node.refresh_cache();
                    changed = true;
                }
            }
        }

        if changed {
            self.merge_adjacent_after_mutation(start, end);
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
                node.plist.clear();
                node.refresh_cache();
            }
        }

        self.merge_adjacent_after_mutation(start, end);
    }

    pub fn next_property_change(&self, pos: usize) -> Option<usize> {
        if self.intervals.is_empty() {
            return None;
        }

        // Check if pos is inside a non-empty interval – return its end
        if let Some((_, node)) = self.find_interval(pos) {
            if !node.is_empty_plist() {
                return Some(node.end);
            }
        }

        // Find the next interval with non-empty plist
        for (start, node) in self.intervals.range(pos..) {
            if *start > pos && !node.is_empty_plist() {
                return Some(*start);
            }
        }

        // Also handle the case where pos is before any interval start
        // but covered by find_interval above
        for (start, node) in self.intervals.range(pos..) {
            if !node.is_empty_plist() {
                return Some(*start);
            }
        }

        None
    }

    pub fn previous_property_change(&self, pos: usize) -> Option<usize> {
        if pos == 0 || self.intervals.is_empty() {
            return None;
        }

        let scan_pos = pos.saturating_sub(1);

        // Check if scan_pos is inside a non-empty interval – return its start
        if let Some((start, node)) = self.find_interval(scan_pos) {
            if !node.is_empty_plist() {
                return Some(start);
            }
        }

        // Find the preceding non-empty interval
        for (start, node) in self.intervals.range(..pos).rev() {
            if node.end <= scan_pos && !node.is_empty_plist() {
                return Some(node.end);
            }
            if *start <= scan_pos && scan_pos < node.end && !node.is_empty_plist() {
                return Some(*start);
            }
        }

        None
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

        self.merge_adjacent_after_mutation(pos, pos + len);
    }

    pub fn adjust_for_delete(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }

        let len = end - start;
        self.split_at(start);
        self.split_at(end);

        // Remove all intervals fully within [start, end)
        let remove_keys: Vec<usize> = self.intervals.range(start..end).map(|(k, _)| *k).collect();
        for key in &remove_keys {
            self.intervals.remove(key);
        }

        // Shift intervals with start >= end by -len
        let shifted_keys: Vec<usize> = self.intervals.range(end..).map(|(k, _)| *k).collect();

        let mut shifted: Vec<(usize, IntervalNode)> = Vec::new();
        for key in &shifted_keys {
            if let Some(node) = self.intervals.remove(key) {
                shifted.push((*key, node));
            }
        }
        for (old_key, mut node) in shifted {
            node.end -= len;
            self.intervals.insert(old_key - len, node);
        }

        self.merge_adjacent_after_mutation(start, start);
    }

    pub fn intervals_snapshot(&self) -> Vec<PropertyInterval> {
        self.intervals
            .iter()
            .filter(|(_, node)| !node.is_empty_plist())
            .map(|(start, node)| PropertyInterval::from_plist(*start, node.end, &node.plist))
            .collect()
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
        for (interval_start, node) in self.intervals.range(start..) {
            if *interval_start >= end {
                break;
            }
            f(*interval_start, node.end, &node.plist)?;
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    pub fn slice(&self, start: usize, end: usize) -> TextPropertyTable {
        if start >= end {
            return TextPropertyTable::new();
        }

        let intervals: Vec<PropertyInterval> = self
            .intervals
            .range(..end)
            .rev()
            .take_while(|(_, node)| node.end > start)
            .map(|(interval_start, node)| {
                let new_start = (*interval_start).max(start) - start;
                let new_end = node.end.min(end) - start;
                PropertyInterval::from_plist(new_start, new_end, &node.plist)
            })
            .filter(|pi| pi.start < pi.end)
            .collect();

        TextPropertyTable::from_dump(intervals)
    }

    pub fn append_shifted(&mut self, other: &TextPropertyTable, offset: usize) {
        for (&start, node) in &other.intervals {
            self.intervals.insert(
                start + offset,
                IntervalNode::new(node.end + offset, node.plist.clone()),
            );
        }
        self.merge_adjacent_after_mutation(
            offset,
            offset
                + other
                    .intervals
                    .last_key_value()
                    .map(|(_, n)| n.end)
                    .unwrap_or(0),
        );
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
                    for (name, value) in &node.plist {
                        if plist_get(&target.plist, *name).is_none() {
                            target.plist.insert(0, (*name, *value));
                        }
                    }
                    target.refresh_cache();
                }
            }

            // If no intervals in range, insert the source directly
            if affected.is_empty() {
                self.intervals.insert(
                    shifted_start,
                    IntervalNode::new(shifted_end, node.plist.clone()),
                );
            }
        }
        self.merge_adjacent_after_mutation(offset, usize::MAX);
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
                    IntervalNode::new(interval.end, interval.into_plist()),
                );
            }
        }
        table.merge_adjacent_after_mutation(0, usize::MAX);
        table
    }

    pub(crate) fn for_each_root(&self, mut f: impl FnMut(Value)) {
        for (_, node) in &self.intervals {
            for (key, value) in &node.plist {
                f(*key);
                f(*value);
            }
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
