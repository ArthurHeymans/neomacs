//! Text-property interval storage for buffers and strings.
//!
//! GNU Emacs represents text properties as an interval tree rooted from the
//! owning string or buffer.  This module keeps the existing Rust API name
//! (`TextPropertyTable`) for callers while preserving GNU's mutation shape:
//! split at the edit range, change the affected interval plists, and preserve
//! raw interval boundaries.  Higher-level property-change queries decide
//! whether adjacent interval plists are semantically equal.

use std::collections::HashMap;

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
// IntervalNode — GNU-shaped internal interval tree node
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IntervalId(usize);

/// Internal interval node.
///
/// This mirrors GNU `struct interval`: each node stores subtree
/// `total_length`, a cached `position`, parent/left/right links, cached bits,
/// and the Lisp plist.  A node's own length is derived from `total_length`
/// minus the left and right subtree lengths, just like GNU's `LENGTH` macro.
#[derive(Clone, Debug)]
struct IntervalNode {
    total_length: usize,
    position: usize,
    left: Option<IntervalId>,
    right: Option<IntervalId>,
    parent: Option<IntervalId>,
    front_sticky: bool,
    rear_sticky: bool,
    write_protect: bool,
    visible: bool,
    plist: IntervalPlist,
}

impl IntervalNode {
    fn new(length: usize, position: usize, plist: IntervalPlist) -> Self {
        let (front_sticky, rear_sticky, write_protect, visible) = Self::extract_cached(plist);
        Self {
            total_length: length,
            position,
            left: None,
            right: None,
            parent: None,
            front_sticky,
            rear_sticky,
            write_protect,
            visible,
            plist,
        }
    }

    fn with_cached(
        length: usize,
        position: usize,
        front_sticky: bool,
        rear_sticky: bool,
        write_protect: bool,
        visible: bool,
        plist: IntervalPlist,
    ) -> Self {
        Self {
            total_length: length,
            position,
            left: None,
            right: None,
            parent: None,
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

#[derive(Clone, Debug)]
struct IntervalRun {
    start: usize,
    end: usize,
    front_sticky: bool,
    rear_sticky: bool,
    write_protect: bool,
    visible: bool,
    plist: IntervalPlist,
}

impl IntervalRun {
    fn new(start: usize, end: usize, plist: IntervalPlist) -> Self {
        let (front_sticky, rear_sticky, write_protect, visible) =
            IntervalNode::extract_cached(plist);
        Self {
            start,
            end,
            front_sticky,
            rear_sticky,
            write_protect,
            visible,
            plist,
        }
    }

    fn default(start: usize, end: usize) -> Self {
        Self::new(start, end, Value::NIL)
    }

    fn from_node(start: usize, node: &IntervalNode, length: usize) -> Self {
        Self {
            start,
            end: start + length,
            front_sticky: node.front_sticky,
            rear_sticky: node.rear_sticky,
            write_protect: node.write_protect,
            visible: node.visible,
            plist: node.plist,
        }
    }

    fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    fn is_empty_plist(&self) -> bool {
        self.plist.is_nil()
    }

    fn refresh_cache(&mut self) {
        let (fs, rs, wp, vis) = IntervalNode::extract_cached(self.plist);
        self.front_sticky = fs;
        self.rear_sticky = rs;
        self.write_protect = wp;
        self.visible = vis;
    }

    fn to_node(&self) -> IntervalNode {
        IntervalNode::with_cached(
            self.len(),
            self.start,
            self.front_sticky,
            self.rear_sticky,
            self.write_protect,
            self.visible,
            self.plist,
        )
    }
}

#[derive(Clone, Debug, Default)]
struct IntervalTree {
    root: Option<IntervalId>,
    nodes: Vec<IntervalNode>,
}

impl IntervalTree {
    fn new() -> Self {
        Self::default()
    }

    fn from_runs(runs: Vec<IntervalRun>) -> Self {
        let runs = Self::normalize_runs(runs, true);
        Self::from_normalized_runs(runs)
    }

    fn from_runs_preserving_empty(runs: Vec<IntervalRun>) -> Self {
        let runs = Self::normalize_runs(runs, false);
        Self::from_normalized_runs(runs)
    }

    fn from_normalized_runs(runs: Vec<IntervalRun>) -> Self {
        let mut tree = Self {
            root: None,
            nodes: Vec::with_capacity(runs.len()),
        };
        tree.root = tree.build_balanced(&runs, 0, runs.len(), None);
        tree
    }

    fn normalize_runs(mut runs: Vec<IntervalRun>, drop_empty_tail: bool) -> Vec<IntervalRun> {
        runs.retain(|run| run.start < run.end);
        runs.sort_by_key(|run| run.start);

        let mut normalized = Vec::new();
        let mut cursor = 0;
        for mut run in runs {
            if run.end <= cursor {
                continue;
            }
            if run.start < cursor {
                run.start = cursor;
            }
            if cursor < run.start {
                normalized.push(IntervalRun::default(cursor, run.start));
            }
            cursor = run.end;
            normalized.push(run);
        }

        if drop_empty_tail {
            while normalized.last().is_some_and(IntervalRun::is_empty_plist) {
                normalized.pop();
            }
            if normalized.iter().all(IntervalRun::is_empty_plist) {
                return Vec::new();
            }
        }
        normalized
    }

    fn build_balanced(
        &mut self,
        runs: &[IntervalRun],
        start: usize,
        end: usize,
        parent: Option<IntervalId>,
    ) -> Option<IntervalId> {
        if start >= end {
            return None;
        }

        let mid = start + (end - start) / 2;
        let id = IntervalId(self.nodes.len());
        self.nodes.push(runs[mid].to_node());

        let left = self.build_balanced(runs, start, mid, Some(id));
        let right = self.build_balanced(runs, mid + 1, end, Some(id));

        let left_len = left
            .map(|child| self.nodes[child.0].total_length)
            .unwrap_or(0);
        let right_len = right
            .map(|child| self.nodes[child.0].total_length)
            .unwrap_or(0);
        let node_len = runs[mid].len();
        let node = &mut self.nodes[id.0];
        node.left = left;
        node.right = right;
        node.parent = parent;
        node.total_length = left_len + node_len + right_len;
        node.position = runs[mid].start;

        Some(id)
    }

    fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    fn len(&self) -> usize {
        self.root
            .map(|root| self.nodes[root.0].total_length)
            .unwrap_or(0)
    }

    fn subtree_len(&self, id: Option<IntervalId>) -> usize {
        id.map(|id| self.nodes[id.0].total_length).unwrap_or(0)
    }

    fn node_len(&self, id: IntervalId) -> usize {
        let node = &self.nodes[id.0];
        node.total_length - self.subtree_len(node.left) - self.subtree_len(node.right)
    }

    fn runs(&self) -> Vec<IntervalRun> {
        let mut runs = Vec::new();
        self.push_runs(self.root, 0, &mut runs);
        runs
    }

    fn push_runs(&self, id: Option<IntervalId>, base: usize, runs: &mut Vec<IntervalRun>) -> usize {
        let Some(id) = id else {
            return base;
        };
        let node = &self.nodes[id.0];
        let after_left = self.push_runs(node.left, base, runs);
        let length = self.node_len(id);
        runs.push(IntervalRun::from_node(after_left, node, length));
        self.push_runs(node.right, after_left + length, runs)
    }

    fn find(&self, pos: usize) -> Option<(usize, &IntervalNode)> {
        let (start, id) = self.find_id(pos)?;
        Some((start, &self.nodes[id.0]))
    }

    fn find_id(&self, pos: usize) -> Option<(usize, IntervalId)> {
        let mut id = self.root?;
        let mut base = 0;
        loop {
            let node = &self.nodes[id.0];
            let left_len = self.subtree_len(node.left);
            let node_start = base + left_len;
            let node_len = self.node_len(id);
            if pos < node_start {
                id = node.left?;
            } else if pos < node_start + node_len {
                return Some((node_start, id));
            } else {
                base = node_start + node_len;
                id = node.right?;
            }
        }
    }

    fn first_start_after(&self, current: usize) -> Option<usize> {
        let mut best = None;
        self.find_first_start_after(self.root, 0, current, &mut best);
        best
    }

    fn find_first_start_after(
        &self,
        id: Option<IntervalId>,
        base: usize,
        current: usize,
        best: &mut Option<usize>,
    ) {
        let Some(id) = id else {
            return;
        };
        let node = &self.nodes[id.0];
        let left_len = self.subtree_len(node.left);
        let node_start = base + left_len;
        let node_len = self.node_len(id);
        if node_start > current {
            *best = Some(best.map_or(node_start, |old| old.min(node_start)));
            self.find_first_start_after(node.left, base, current, best);
        } else {
            self.find_first_start_after(node.right, node_start + node_len, current, best);
        }
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

/// Text-property interval storage.
///
/// The backing store is a GNU-shaped augmented interval tree.  Callers still
/// use absolute character positions; internally node starts are derived from
/// subtree lengths.
#[derive(Clone, Debug)]
pub struct TextPropertyTable {
    intervals: IntervalTree,
}

impl TextPropertyTable {
    pub fn new() -> Self {
        Self {
            intervals: IntervalTree::new(),
        }
    }

    /// Copy interval nodes and plist cons spines, while preserving plist
    /// values by reference.  GNU `copy_intervals` copies intervals and
    /// `copy_properties` copies `interval->plist` with `copy-sequence`.
    pub fn copy_interval_plist_spines(&self) -> Self {
        let mut runs = self.intervals.runs();
        for run in &mut runs {
            run.plist = plist_value_from_pairs(&plist_pairs(run.plist));
            run.refresh_cache();
        }
        Self::from_interval_runs(runs)
    }

    // -- Query helpers -------------------------------------------------------

    /// Find the interval containing `pos`, returning its (start, node) pair.
    fn find_interval(&self, pos: usize) -> Option<(usize, &IntervalNode)> {
        self.intervals.find(pos)
    }

    /// Find the interval containing `pos`, returning mutable (start, node).
    fn find_interval_mut(&mut self, pos: usize) -> Option<(usize, &mut IntervalNode)> {
        let (start, id) = self.intervals.find_id(pos)?;
        Some((start, &mut self.intervals.nodes[id.0]))
    }

    /// Find the first stored interval that overlaps `[start, end)`.
    ///
    /// GNU interval users first locate the interval at START and then advance
    /// with `next_interval`; they don't scan from the beginning of the object.
    /// Keep that traversal shape centralized so callers follow GNU's
    /// interval-access path instead of depending on the tree representation.
    fn first_interval_start_overlapping(&self, start: usize, end: usize) -> Option<usize> {
        if start >= end {
            return None;
        }
        if let Some((interval_start, node)) = self.find_interval(start)
            && interval_start + self.interval_node_len_at(interval_start, node) > start
        {
            return Some(interval_start);
        }
        self.intervals
            .first_start_after(start)
            .filter(|next| *next < end)
    }

    fn next_interval_start_after(&self, current: usize, end: usize) -> Option<usize> {
        self.intervals
            .first_start_after(current)
            .filter(|next| *next < end)
    }

    fn for_each_interval_overlapping(
        &self,
        start: usize,
        end: usize,
        mut f: impl FnMut(usize, &IntervalNode),
    ) {
        let Some(mut key) = self.first_interval_start_overlapping(start, end) else {
            return;
        };
        loop {
            if key >= end {
                break;
            }
            let Some((node_start, node)) = self.find_interval(key) else {
                break;
            };
            let node_end = node_start + self.interval_node_len_at(node_start, node);
            if node_end > start {
                f(node_start, node);
            }
            let Some(next_key) = self.next_interval_start_after(key, end) else {
                break;
            };
            key = next_key;
        }
    }

    fn any_interval_overlapping(
        &self,
        start: usize,
        end: usize,
        mut predicate: impl FnMut(usize, &IntervalNode) -> bool,
    ) -> bool {
        let Some(mut key) = self.first_interval_start_overlapping(start, end) else {
            return false;
        };
        loop {
            if key >= end {
                return false;
            }
            let Some((node_start, node)) = self.find_interval(key) else {
                return false;
            };
            let node_end = node_start + self.interval_node_len_at(node_start, node);
            if node_end > start && predicate(node_start, node) {
                return true;
            }
            let Some(next_key) = self.next_interval_start_after(key, end) else {
                return false;
            };
            key = next_key;
        }
    }

    fn try_for_each_interval_overlapping<E>(
        &self,
        start: usize,
        end: usize,
        mut f: impl FnMut(usize, &IntervalNode) -> Result<(), E>,
    ) -> Result<(), E> {
        let Some(mut key) = self.first_interval_start_overlapping(start, end) else {
            return Ok(());
        };
        loop {
            if key >= end {
                break;
            }
            let Some((node_start, node)) = self.find_interval(key) else {
                break;
            };
            let node_end = node_start + self.interval_node_len_at(node_start, node);
            if node_end > start {
                f(node_start, node)?;
            }
            let Some(next_key) = self.next_interval_start_after(key, end) else {
                break;
            };
            key = next_key;
        }
        Ok(())
    }

    fn interval_node_len_at(&self, start: usize, node: &IntervalNode) -> usize {
        self.intervals
            .find_id(start)
            .and_then(|(found_start, id)| {
                if found_start == start && std::ptr::eq(node, &self.intervals.nodes[id.0]) {
                    Some(self.intervals.node_len(id))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                self.intervals
                    .runs()
                    .into_iter()
                    .find(|run| run.start == start)
                    .map(|run| run.len())
                    .unwrap_or(0)
            })
    }

    fn from_interval_runs(runs: Vec<IntervalRun>) -> Self {
        Self {
            intervals: IntervalTree::from_runs(runs),
        }
    }

    fn replace_runs(&mut self, runs: Vec<IntervalRun>) {
        self.intervals = IntervalTree::from_runs(runs);
    }

    fn replace_runs_preserving_empty(&mut self, runs: Vec<IntervalRun>) {
        self.intervals = IntervalTree::from_runs_preserving_empty(runs);
    }

    fn split_runs_at(runs: &mut Vec<IntervalRun>, pos: usize) {
        if pos == 0 {
            return;
        }
        let Some(index) = runs.iter().position(|run| run.start < pos && pos < run.end) else {
            return;
        };

        let right_plist = plist_value_from_pairs(&plist_pairs(runs[index].plist));
        let mut right = runs[index].clone();
        right.start = pos;
        right.plist = right_plist;
        right.refresh_cache();
        runs[index].end = pos;
        runs.insert(index + 1, right);
    }

    fn ensure_runs_cover(runs: &mut Vec<IntervalRun>, end: usize) {
        let current_end = runs.last().map(|run| run.end).unwrap_or(0);
        if current_end < end {
            runs.push(IntervalRun::default(current_end, end));
        }
    }

    fn splice_interval_run(runs: &mut Vec<IntervalRun>, run: IntervalRun) {
        if run.start >= run.end {
            return;
        }
        Self::ensure_runs_cover(runs, run.end);
        Self::split_runs_at(runs, run.start);
        Self::split_runs_at(runs, run.end);
        runs.retain(|existing| !(existing.start < run.end && existing.end > run.start));
        runs.push(run);
    }

    // -- Split and merge helpers --------------------------------------------

    /// Ensure no interval straddles `pos`. If one does, split it into two:
    /// [start, pos) and [pos, original end).  Idempotent if already split.
    fn split_at(&mut self, pos: usize) {
        let mut runs = self.intervals.runs();
        Self::split_runs_at(&mut runs, pos);
        self.replace_runs(runs);
    }

    /// Rebuild the tree and drop trailing/all-empty nil intervals.
    ///
    /// GNU keeps interval boundaries even when adjacent plists are equal; for
    /// example, two adjacent `put-text-property' calls can produce two equal
    /// intervals, and `(next-property-change POS OBJ t)' must still expose the
    /// raw boundary.  The tree keeps leading and interior nil-property
    /// intervals because GNU interval positions are derived from subtree
    /// lengths, so gaps before a later property are real intervals.
    fn prune_empty_intervals_after_mutation(&mut self) {
        let runs = self.intervals.runs();
        self.replace_runs(runs);
    }

    // -- Public API ----------------------------------------------------------

    pub fn put_property(&mut self, start: usize, end: usize, name: Value, value: Value) -> bool {
        if start >= end {
            return false;
        }
        let mut runs = self.intervals.runs();
        Self::ensure_runs_cover(&mut runs, end);
        Self::split_runs_at(&mut runs, start);
        Self::split_runs_at(&mut runs, end);
        let mut changed = false;

        for run in &mut runs {
            if run.start < end && run.end > start {
                if plist_value_put_replace(&mut run.plist, name, value) {
                    run.refresh_cache();
                    changed = true;
                }
            }
        }

        self.replace_runs(runs);
        changed
    }

    pub(crate) fn from_plist_runs(runs: Vec<(usize, usize, Vec<(Value, Value)>)>) -> Self {
        Self::from_interval_runs(
            runs.into_iter()
                .filter(|(start, end, plist)| start < end && !plist.is_empty())
                .map(|(start, end, plist)| {
                    IntervalRun::new(start, end, plist_value_from_pairs(&plist))
                })
                .collect(),
        )
    }

    pub fn get_property(&self, pos: usize, name: Value) -> Option<Value> {
        let (_, node) = self.find_interval(pos)?;
        plist_value_get(node.plist, name)
    }

    pub fn get_properties(&self, pos: usize) -> HashMap<Value, Value> {
        let Some((start, node)) = self.find_interval(pos) else {
            return HashMap::new();
        };
        let end = start + self.interval_node_len_at(start, node);
        PropertyInterval::from_plist(start, end, &plist_pairs(node.plist)).properties
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

    pub fn range_has_all_properties(
        &self,
        start: usize,
        end: usize,
        properties: &[(Value, Value)],
    ) -> bool {
        if start >= end || properties.is_empty() {
            return true;
        }

        let mut cursor = start;
        while cursor < end {
            let Some((interval_start, node)) = self.find_interval(cursor) else {
                return false;
            };
            for (name, value) in properties {
                let Some(existing) = plist_value_get(node.plist, *name) else {
                    return false;
                };
                if !eq_value(&existing, value) {
                    return false;
                }
            }
            let node_end = interval_start + self.interval_node_len_at(interval_start, node);
            if node_end <= cursor {
                return false;
            }
            cursor = node_end.min(end);
        }

        true
    }

    pub fn range_has_any_property_named(&self, start: usize, end: usize, names: &[Value]) -> bool {
        if start >= end || names.is_empty() {
            return false;
        }

        self.any_interval_overlapping(start, end, |_, node| {
            names
                .iter()
                .any(|name| plist_value_get(node.plist, *name).is_some())
        })
    }

    pub fn range_has_any_interval(&self, start: usize, end: usize) -> bool {
        if start >= end {
            return false;
        }

        self.any_interval_overlapping(start, end, |_, node| !node.is_empty_plist())
    }

    pub fn remove_property(&mut self, start: usize, end: usize, name: Value) -> bool {
        if start >= end {
            return false;
        }
        let mut runs = self.intervals.runs();
        Self::split_runs_at(&mut runs, start);
        Self::split_runs_at(&mut runs, end);
        let mut changed = false;
        for run in &mut runs {
            if run.start < end && run.end > start && plist_value_remove(&mut run.plist, name) {
                run.refresh_cache();
                changed = true;
            }
        }

        if changed {
            self.replace_runs_preserving_empty(runs);
        }
        changed
    }

    pub fn remove_all_properties(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let mut runs = self.intervals.runs();
        Self::split_runs_at(&mut runs, start);
        Self::split_runs_at(&mut runs, end);
        for run in &mut runs {
            if run.start < end && run.end > start {
                run.plist = Value::NIL;
                run.refresh_cache();
            }
        }
        self.replace_runs(runs);
    }

    pub fn set_properties(&mut self, start: usize, end: usize, plist: Vec<(Value, Value)>) {
        if start >= end {
            return;
        }
        if plist.is_empty() && self.intervals.is_empty() {
            return;
        }
        let mut runs = self.intervals.runs();
        Self::ensure_runs_cover(&mut runs, end);
        Self::split_runs_at(&mut runs, start);
        Self::split_runs_at(&mut runs, end);
        runs.retain(|run| !(run.start < end && run.end > start));
        runs.push(IntervalRun::new(start, end, plist_value_from_pairs(&plist)));
        if plist.is_empty() {
            self.replace_runs_preserving_empty(runs);
        } else {
            self.replace_runs(runs);
        }
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
        if let Some((start, id)) = self.intervals.find_id(pos) {
            return Some(start + self.intervals.node_len(id));
        }

        self.intervals.first_start_after(pos)
    }

    /// Return the previous raw interval boundary before `pos`.
    pub fn previous_interval_boundary(&self, pos: usize) -> Option<usize> {
        if pos == 0 {
            return None;
        }

        let scan_pos = pos - 1;
        if let Some((start, _)) = self.intervals.find_id(scan_pos) {
            return Some(start);
        }

        let tree_len = self.intervals.len();
        (tree_len > 0 && tree_len < pos).then_some(tree_len)
    }

    pub fn adjust_for_insert(&mut self, pos: usize, len: usize) {
        if len == 0 {
            return;
        }

        let mut runs = self.intervals.runs();
        if pos > runs.last().map(|run| run.end).unwrap_or(0) {
            return;
        }
        Self::split_runs_at(&mut runs, pos);
        let insert_at = runs
            .iter()
            .position(|run| run.start >= pos)
            .unwrap_or(runs.len());
        for run in runs.iter_mut().skip(insert_at) {
            run.start += len;
            run.end += len;
        }
        runs.insert(insert_at, IntervalRun::default(pos, pos + len));

        self.replace_runs(runs);
    }

    pub fn adjust_for_delete(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }

        let len = end - start;
        let old_runs = self.intervals.runs();
        let mut adjusted = Vec::new();

        for mut run in old_runs {
            let old_start = run.start;
            let old_end = run.end;

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
                run.start = new_start;
                run.end = new_end;
                adjusted.push(run);
            }
        }

        self.replace_runs(adjusted);
    }

    pub fn intervals_snapshot(&self) -> Vec<PropertyInterval> {
        self.intervals
            .runs()
            .into_iter()
            .filter(|run| !run.is_empty_plist())
            .map(|run| PropertyInterval::from_plist(run.start, run.end, &plist_pairs(run.plist)))
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
        for run in self.intervals.runs() {
            let start = run.start.min(len);
            let end = run.end.min(len);
            if cursor < start {
                runs.push((cursor, start, Value::NIL));
            }
            if start < end {
                runs.push((start, end, run.plist));
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
        let mut key = self.first_interval_start_overlapping(start, end)?;
        loop {
            if key >= end {
                return None;
            }
            let (_, node) = self.find_interval(key)?;
            if plist_value_get(node.plist, name).is_some_and(|found| eq_value(&found, &value)) {
                return Some(key.max(start));
            }
            let Some(next_key) = self.next_interval_start_after(key, end) else {
                return None;
            };
            key = next_key;
        }
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
        self.try_for_each_interval_overlapping(start, end, |interval_start, node| {
            let pairs = plist_pairs(node.plist);
            let interval_end = interval_start + self.interval_node_len_at(interval_start, node);
            f(interval_start, interval_end, &pairs)
        })
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
            .intervals
            .find_id(pos)
            .map(|(start, id)| start + self.intervals.node_len(id))
            .or_else(|| self.intervals.first_start_after(pos))
            .unwrap_or(end);
        let next_start = self.intervals.first_start_after(pos).unwrap_or(end);
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

        let mut runs: Vec<(usize, usize, Vec<(Value, Value)>)> = Vec::new();
        self.for_each_interval_overlapping(start, end, |interval_start, node| {
            let new_start = interval_start.max(start) - start;
            let node_end = interval_start + self.interval_node_len_at(interval_start, node);
            let new_end = node_end.min(end) - start;
            if new_start < new_end {
                runs.push((new_start, new_end, plist_pairs(node.plist)));
            }
        });

        TextPropertyTable::from_plist_runs(runs)
    }

    pub fn slice_copy_text_properties(&self, start: usize, end: usize) -> TextPropertyTable {
        if start >= end {
            return TextPropertyTable::new();
        }

        let mut table = TextPropertyTable::new();
        self.for_each_interval_overlapping(start, end, |interval_start, node| {
            let new_start = interval_start.max(start) - start;
            let node_end = interval_start + self.interval_node_len_at(interval_start, node);
            let new_end = node_end.min(end) - start;
            if new_start >= new_end {
                return;
            }
            for (name, value) in plist_pairs(node.plist) {
                table.put_property(new_start, new_end, name, value);
            }
        });
        table
    }

    pub fn append_shifted(&mut self, other: &TextPropertyTable, offset: usize) {
        let mut runs = self.intervals.runs();
        for mut run in other.intervals.runs() {
            run.start += offset;
            run.end += offset;
            Self::splice_interval_run(&mut runs, run);
        }
        self.replace_runs(runs);
    }

    pub fn append_shifted_via_add_text_properties(
        &mut self,
        other: &TextPropertyTable,
        offset: usize,
    ) {
        for run in other.intervals.runs() {
            if run.is_empty_plist() {
                continue;
            }
            for (name, value) in plist_pairs(run.plist) {
                self.put_property(run.start + offset, run.end + offset, name, value);
            }
        }
    }

    pub fn merge_missing_shifted(&mut self, other: &TextPropertyTable, offset: usize) {
        let mut target_runs = self.intervals.runs();
        for source in other.intervals.runs() {
            if source.is_empty_plist() {
                continue;
            }
            let shifted_start = source.start + offset;
            let shifted_end = source.end + offset;

            Self::ensure_runs_cover(&mut target_runs, shifted_end);
            Self::split_runs_at(&mut target_runs, shifted_start);
            Self::split_runs_at(&mut target_runs, shifted_end);

            for target in &mut target_runs {
                if target.start < shifted_end && target.end > shifted_start {
                    for (name, value) in plist_pairs(source.plist) {
                        if plist_value_get(target.plist, name).is_none() {
                            target.plist = plist_value_prepend_pair(target.plist, name, value);
                        }
                    }
                    target.refresh_cache();
                }
            }
        }
        self.replace_runs(target_runs);
    }

    pub fn merge_adjacent_equal_properties_around(&mut self, start: usize, end: usize) {
        let mut runs = self.intervals.runs();
        if start > end || runs.len() < 2 {
            return;
        }

        loop {
            let mut merged = false;
            let mut idx = 0;
            while idx + 1 < runs.len() {
                let left_end = runs[idx].end;
                let right_start = runs[idx + 1].start;
                if left_end != right_start
                    || runs[idx].plist.is_nil()
                    || !plists_equal_eq(
                        &plist_pairs(runs[idx].plist),
                        &plist_pairs(runs[idx + 1].plist),
                    )
                    || left_end < start
                    || right_start > end
                {
                    idx += 1;
                    continue;
                }

                let right_end = runs[idx + 1].end;
                runs[idx].end = right_end;
                runs[idx].refresh_cache();
                runs.remove(idx + 1);
                merged = true;
                break;
            }

            if !merged {
                break;
            }
        }
        self.replace_runs(runs);
    }

    pub(crate) fn dump_intervals(&self) -> Vec<PropertyInterval> {
        self.intervals_snapshot()
    }

    #[cfg(test)]
    pub(crate) fn debug_interval_bounds(&self) -> Vec<(usize, usize, bool)> {
        self.intervals
            .runs()
            .into_iter()
            .map(|run| (run.start, run.end, run.is_empty_plist()))
            .collect()
    }

    pub(crate) fn from_dump(intervals: Vec<PropertyInterval>) -> Self {
        Self::from_interval_runs(
            intervals
                .into_iter()
                .filter(|interval| interval.start < interval.end)
                .map(|interval| {
                    IntervalRun::new(
                        interval.start,
                        interval.end,
                        plist_value_from_pairs(&interval.into_plist()),
                    )
                })
                .collect(),
        )
    }

    pub(crate) fn for_each_root(&self, mut f: impl FnMut(Value)) {
        for node in &self.intervals.nodes {
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
