use super::*;
use crate::descriptor::{
    EphemeronVisitor, Relocator, Trace, Tracer, TypeFlags, WeakProcessor, fixed_type_desc,
};
use crate::index_state::{HeapIndexState, ObjectIndex, ObjectLocator};
use crate::object_store::FlatReadView;
use crate::plan::{CollectionKind, CollectionPhase, CollectionPlan};
use crate::root::{Gc, RootStack};
use crate::runtime_state::RuntimeStateHandle;
use crate::spaces::{NurseryConfig, NurseryState, OldGenConfig, OldGenState};
use crate::stats::{HeapStats, SpaceStats};
use crate::weak::{Ephemeron, Weak};

#[derive(Debug)]
struct Leaf;

unsafe impl Trace for Leaf {
    fn trace(&self, _tracer: &mut dyn Tracer) {}

    fn relocate(&self, _relocator: &mut dyn Relocator) {}
}

#[derive(Debug)]
struct EphemeronHolder {
    pair: Ephemeron<Leaf, Leaf>,
}

unsafe impl Trace for EphemeronHolder {
    fn trace(&self, _tracer: &mut dyn Tracer) {}

    fn relocate(&self, _relocator: &mut dyn Relocator) {}

    fn process_weak(&self, processor: &mut dyn WeakProcessor) {
        self.pair.process(processor);
    }

    fn visit_ephemerons(&self, visitor: &mut dyn EphemeronVisitor) {
        self.pair.visit(visitor);
    }

    fn type_flags() -> TypeFlags
    where
        Self: Sized,
    {
        TypeFlags::WEAK | TypeFlags::EPHEMERON_KEY
    }
}

fn object_index_for(objects: &[ObjectRecord]) -> ObjectIndex {
    objects
        .iter()
        .enumerate()
        .map(|(index, object)| (object.object_key(), ObjectLocator::flat(index)))
        .collect()
}

#[test]
fn trace_major_marks_seeded_source() {
    let desc = Box::leak(Box::new(fixed_type_desc::<Leaf>()));
    let object =
        ObjectRecord::allocate(desc, SpaceKind::Pinned, Leaf).expect("allocate pinned leaf");
    let source = object.erased();
    let objects = vec![object];
    let indexes = HeapIndexState {
        object_index: object_index_for(&objects),
        ..HeapIndexState::default()
    };
    let view = FlatReadView::new(&objects, &indexes);

    let (steps, rounds) = super::trace_major(view.raw(), 1, 8, [source]);

    assert_eq!(steps, 1);
    assert_eq!(rounds, 1);
    assert!(objects[0].is_marked());
}

#[test]
fn trace_minor_marks_seeded_nursery_source() {
    let desc = Box::leak(Box::new(fixed_type_desc::<Leaf>()));
    let object =
        ObjectRecord::allocate(desc, SpaceKind::Nursery, Leaf).expect("allocate nursery leaf");
    let source = object.erased();
    let objects = vec![object];
    let indexes = HeapIndexState {
        object_index: object_index_for(&objects),
        ..HeapIndexState::default()
    };
    let view = FlatReadView::new(&objects, &indexes);

    let (steps, rounds) = super::trace_minor(view.raw(), &[], &[], 1, 8, [source]);

    assert_eq!(steps, 1);
    assert_eq!(rounds, 1);
    assert!(objects[0].is_marked());
}

#[test]
fn trace_collection_records_major_phases() {
    let desc = Box::leak(Box::new(fixed_type_desc::<Leaf>()));
    let object =
        ObjectRecord::allocate(desc, SpaceKind::Pinned, Leaf).expect("allocate pinned leaf");
    let source = object.erased();
    let objects = vec![object];
    let indexes = HeapIndexState {
        object_index: object_index_for(&objects),
        ..HeapIndexState::default()
    };
    let mut phases = Vec::new();

    let (steps, rounds) = super::trace_collection(
        &crate::plan::CollectionPlan {
            kind: crate::plan::CollectionKind::Major,
            phase: crate::plan::CollectionPhase::ConcurrentMark,
            concurrent: true,
            parallel: true,
            worker_count: 1,
            mark_slice_budget: 8,
            target_old_regions: 0,
            selected_old_blocks: Vec::new(),
            estimated_compaction_bytes: 0,
            estimated_reclaim_bytes: 0,
        },
        &objects,
        &indexes,
        &[source],
        |phase| phases.push(phase),
    );

    assert_eq!(steps, 1);
    assert_eq!(rounds, 1);
    assert_eq!(
        phases,
        vec![
            crate::plan::CollectionPhase::InitialMark,
            crate::plan::CollectionPhase::ConcurrentMark,
            crate::plan::CollectionPhase::Remark,
        ]
    );
}

#[test]
fn execute_collection_plan_records_minor_phases() {
    // `nursery_state` must be declared BEFORE `objects` so that Rust's
    // reverse-declaration-order local drops release the Vec<ObjectRecord>
    // first, giving arena-backed records a chance to run their
    // drop_in_place before the backing arena buffer is freed.
    let nursery = NurseryConfig::default();
    let mut nursery_state = NurseryState::new(nursery.semispace_bytes);
    let desc = Box::leak(Box::new(fixed_type_desc::<Leaf>()));
    let object =
        ObjectRecord::allocate(desc, SpaceKind::Nursery, Leaf).expect("allocate nursery leaf");
    let object_size = object.total_size();
    let source = object.erased();
    let mut objects = vec![object];
    let mut indexes = HeapIndexState {
        object_index: object_index_for(&objects),
        ..HeapIndexState::default()
    };
    let mut roots = RootStack::default();
    roots.push(source);
    let mut old_gen = OldGenState::default();
    let old = OldGenConfig::default();
    let mut stats = HeapStats {
        nursery: SpaceStats {
            reserved_bytes: nursery.semispace_bytes.saturating_mul(2),
            live_bytes: object_size,
        },
        ..HeapStats::default()
    };
    let runtime_state = RuntimeStateHandle::default();
    let mut phases = Vec::new();

    let cycle = execute_collection_plan(
        &CollectionPlan {
            kind: CollectionKind::Minor,
            phase: CollectionPhase::InitialMark,
            concurrent: false,
            parallel: true,
            worker_count: 1,
            mark_slice_budget: 8,
            target_old_regions: 0,
            selected_old_blocks: Vec::new(),
            estimated_compaction_bytes: 0,
            estimated_reclaim_bytes: 0,
        },
        &mut roots,
        &mut objects,
        &mut indexes,
        &mut old_gen,
        &old,
        &nursery,
        &mut stats,
        &mut nursery_state,
        &runtime_state,
        None,
        None,
        |phase| phases.push(phase),
    )
    .expect("minor collection should succeed");

    assert_eq!(cycle.minor_collections, 1);
    assert_eq!(
        phases,
        vec![CollectionPhase::Evacuate, CollectionPhase::Reclaim]
    );
    assert_eq!(objects.len(), 1);
}

#[test]
fn collect_global_sources_includes_roots_and_immortal_objects() {
    let desc = Box::leak(Box::new(fixed_type_desc::<Leaf>()));
    let rooted =
        ObjectRecord::allocate(desc, SpaceKind::Pinned, Leaf).expect("allocate rooted object");
    let immortal =
        ObjectRecord::allocate(desc, SpaceKind::Immortal, Leaf).expect("allocate immortal object");
    let nursery =
        ObjectRecord::allocate(desc, SpaceKind::Nursery, Leaf).expect("allocate nursery object");
    let rooted_source = rooted.erased();
    let immortal_source = immortal.erased();
    let nursery_source = nursery.erased();
    let objects = vec![rooted, immortal, nursery];
    let mut roots = RootStack::default();
    roots.push(rooted_source);

    let indexes = HeapIndexState {
        object_index: object_index_for(&objects),
        ..HeapIndexState::default()
    };
    let view = FlatReadView::new(&objects, &indexes);
    let sources = super::collect_global_sources(&roots, &view, None);

    assert!(sources.contains(&rooted_source));
    assert!(sources.contains(&immortal_source));
    assert!(!sources.contains(&nursery_source));
}

#[test]
fn active_major_ephemeron_trace_slices_candidate_scan() {
    let holder_desc = Box::leak(Box::new(fixed_type_desc::<EphemeronHolder>()));
    let first = ObjectRecord::allocate(
        holder_desc,
        SpaceKind::Pinned,
        EphemeronHolder {
            pair: Ephemeron::default(),
        },
    )
    .expect("allocate first ephemeron holder");
    let second = ObjectRecord::allocate(
        holder_desc,
        SpaceKind::Pinned,
        EphemeronHolder {
            pair: Ephemeron::default(),
        },
    )
    .expect("allocate second ephemeron holder");
    let first_key = first.object_key();
    let second_key = second.object_key();
    first.mark_if_unmarked();
    second.mark_if_unmarked();
    let objects = vec![first, second];
    let indexes = HeapIndexState {
        object_index: object_index_for(&objects),
        ..HeapIndexState::default()
    };
    let view = FlatReadView::new(&objects, &indexes);
    let mut trace = begin_active_major_ephemeron_trace(view.raw(), &[first_key, second_key]);

    let first_progress = advance_active_major_ephemeron_trace(view.raw(), &mut trace, 1, 1, 1);
    assert!(!first_progress.completed);
    assert_eq!(first_progress.scanned_candidates_delta, 1);
    assert_eq!(trace.scanned_candidates(), 1);

    let second_progress = advance_active_major_ephemeron_trace(view.raw(), &mut trace, 1, 1, 1);
    assert!(second_progress.completed);
    assert_eq!(second_progress.scanned_candidates_delta, 1);
    assert_eq!(trace.scanned_candidates(), 2);
}

#[test]
fn active_major_ephemeron_trace_slices_fixpoint_mark_work() {
    let leaf_desc = Box::leak(Box::new(fixed_type_desc::<Leaf>()));
    let holder_desc = Box::leak(Box::new(fixed_type_desc::<EphemeronHolder>()));
    let key = ObjectRecord::allocate(leaf_desc, SpaceKind::Pinned, Leaf).expect("allocate key");
    let value = ObjectRecord::allocate(leaf_desc, SpaceKind::Pinned, Leaf).expect("allocate value");
    let key_gc = unsafe { Gc::<Leaf>::from_erased(key.erased()) };
    let value_gc = unsafe { Gc::<Leaf>::from_erased(value.erased()) };
    let holder = ObjectRecord::allocate(
        holder_desc,
        SpaceKind::Pinned,
        EphemeronHolder {
            pair: Ephemeron::new(Weak::new(key_gc), Weak::new(value_gc)),
        },
    )
    .expect("allocate ephemeron holder");
    let holder_key = holder.object_key();
    key.mark_if_unmarked();
    holder.mark_if_unmarked();
    let objects = vec![key, value, holder];
    let indexes = HeapIndexState {
        object_index: object_index_for(&objects),
        ..HeapIndexState::default()
    };
    let view = FlatReadView::new(&objects, &indexes);
    let mut trace = begin_active_major_ephemeron_trace(view.raw(), &[holder_key]);

    let first_progress = advance_active_major_ephemeron_trace(view.raw(), &mut trace, 1, 1, 1);
    assert!(!first_progress.completed);
    assert_eq!(first_progress.scanned_candidates_delta, 1);
    assert_eq!(first_progress.mark_steps_delta, 1);
    assert_eq!(first_progress.mark_rounds_delta, 1);
    assert!(objects[1].is_marked());

    let second_progress = advance_active_major_ephemeron_trace(view.raw(), &mut trace, 1, 1, 1);
    assert!(second_progress.completed);
    assert_eq!(second_progress.scanned_candidates_delta, 1);
    assert_eq!(second_progress.mark_steps_delta, 0);
    assert_eq!(trace.scanned_candidates(), 2);
}
