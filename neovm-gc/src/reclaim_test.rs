use super::*;
use crate::descriptor::{Trace, Tracer, TypeFlags, fixed_type_desc};
use crate::heap::{HeapConfig, HeapCore};
use crate::index_state::{HeapIndexState, ObjectLocator, PreparedIndexReclaim};
use crate::object::{ObjectRecord, SpaceKind};
use crate::plan::{CollectionKind, CollectionPhase};
use crate::runtime_state::RuntimeStateHandle;
use crate::spaces::PreparedOldGenReclaim;
use crate::stats::PreparedHeapStats;
use std::cell::RefCell;

fn major_plan() -> CollectionPlan {
    CollectionPlan {
        kind: CollectionKind::Major,
        phase: CollectionPhase::Remark,
        concurrent: true,
        parallel: true,
        worker_count: 4,
        mark_slice_budget: 8,
        target_old_regions: 2,
        selected_old_blocks: vec![0, 3],
        estimated_compaction_bytes: 64,
        estimated_reclaim_bytes: 32,
    }
}

fn prepared_reclaim() -> PreparedReclaim {
    PreparedReclaim {
        prepared_object_count: 0,
        promoted_bytes: 0,
        old_gen: PreparedOldGenReclaim::default(),
        indexes: PreparedIndexReclaim::default(),
        survivors: Vec::new(),
        stats: PreparedHeapStats::default(),
    }
}

fn locator(slot: usize) -> ObjectLocator {
    ObjectLocator::flat(slot)
}

#[derive(Debug)]
struct Leaf;

unsafe impl Trace for Leaf {
    fn trace(&self, _tracer: &mut dyn Tracer) {}

    fn relocate(&self, _relocator: &mut dyn crate::descriptor::Relocator) {}
}

fn leaf_desc() -> &'static crate::descriptor::TypeDesc {
    Box::leak(Box::new(fixed_type_desc::<Leaf>()))
}

#[derive(Debug)]
struct FinalizableLeaf;

unsafe impl Trace for FinalizableLeaf {
    fn trace(&self, _tracer: &mut dyn Tracer) {}

    fn relocate(&self, _relocator: &mut dyn crate::descriptor::Relocator) {}

    fn type_flags() -> TypeFlags
    where
        Self: Sized,
    {
        TypeFlags::FINALIZABLE
    }
}

fn finalizable_leaf_desc() -> &'static crate::descriptor::TypeDesc {
    Box::leak(Box::new(fixed_type_desc::<FinalizableLeaf>()))
}

#[test]
fn prepare_major_reclaim_runs_weak_processing_before_preparing() {
    let log = RefCell::new(Vec::new());
    let reclaim = super::prepare_major_reclaim(
        &major_plan(),
        |_plan| log.borrow_mut().push("weak"),
        |_plan| {
            log.borrow_mut().push("prepare");
            prepared_reclaim()
        },
    );

    assert_eq!(&*log.borrow(), &["weak", "prepare"]);
    assert_eq!(reclaim.promoted_bytes, 0);
}

#[test]
fn prepare_full_reclaim_propagates_promotion_and_relocation_order() {
    let log = RefCell::new(Vec::new());
    let reclaim = super::prepare_full_reclaim(
        &mut (),
        &CollectionPlan {
            kind: CollectionKind::Full,
            ..major_plan()
        },
        |_heap| {
            log.borrow_mut().push("evacuate");
            Ok((41usize, 17usize))
        },
        |_heap, forwarding| {
            assert_eq!(*forwarding, 41);
            log.borrow_mut().push("relocate");
        },
        |_heap, _plan, forwarding| {
            assert_eq!(*forwarding, 41);
            log.borrow_mut().push("weak");
        },
        |_heap, _plan| {
            log.borrow_mut().push("prepare");
            prepared_reclaim()
        },
    )
    .expect("full reclaim prep should succeed");

    assert_eq!(&*log.borrow(), &["evacuate", "relocate", "weak", "prepare"]);
    assert_eq!(reclaim.promoted_bytes, 17);
}

#[test]
fn advance_prepared_reclaim_commit_removes_dead_keys_before_final_commit() {
    let dead = ObjectRecord::allocate(finalizable_leaf_desc(), SpaceKind::Pinned, FinalizableLeaf)
        .expect("allocate dead");
    let live = ObjectRecord::allocate(leaf_desc(), SpaceKind::Pinned, Leaf).expect("allocate live");
    let tail = ObjectRecord::allocate(leaf_desc(), SpaceKind::Pinned, Leaf).expect("allocate tail");
    assert!(live.mark_if_unmarked());

    let dead_key = dead.object_key();
    let live_key = live.object_key();
    let tail_key = tail.object_key();
    let mut objects = vec![dead, live, tail];
    let mut indexes = HeapIndexState::default();
    for (index, object) in objects.iter().enumerate() {
        indexes.record_allocated_object(
            object.object_key(),
            locator(index),
            object.header().desc(),
            object.space(),
        );
    }

    let prepared = PreparedReclaim {
        prepared_object_count: 2,
        promoted_bytes: 0,
        old_gen: PreparedOldGenReclaim::default(),
        indexes: PreparedIndexReclaim {
            finalize_indices: vec![0],
            ..PreparedIndexReclaim::default()
        },
        survivors: vec![PreparedReclaimSurvivor { object_index: 1 }],
        stats: PreparedHeapStats::default(),
    };
    let mut commit_state = begin_prepared_reclaim_commit(&prepared, &objects);

    assert!(!advance_prepared_reclaim_commit(
        objects.as_mut_slice(),
        &mut indexes,
        &prepared,
        &mut commit_state,
        1,
    ));
    assert!(!indexes.object_index.contains_key(&dead_key));
    assert_eq!(indexes.object_index.get(&live_key), Some(&locator(1)));
    assert_eq!(indexes.object_index.get(&tail_key), Some(&locator(2)));

    assert!(advance_prepared_reclaim_commit(
        objects.as_mut_slice(),
        &mut indexes,
        &prepared,
        &mut commit_state,
        1,
    ));
    assert_eq!(indexes.object_index.get(&live_key), Some(&locator(0)));
    assert_eq!(indexes.object_index.get(&tail_key), Some(&locator(2)));
}

#[test]
fn apply_prepared_reclaim_preserves_post_prepare_allocations() {
    let runtime_state = RuntimeStateHandle::default();
    let dead = ObjectRecord::allocate(finalizable_leaf_desc(), SpaceKind::Pinned, FinalizableLeaf)
        .expect("allocate dead");
    let live = ObjectRecord::allocate(leaf_desc(), SpaceKind::Pinned, Leaf).expect("allocate live");
    let tail = ObjectRecord::allocate(leaf_desc(), SpaceKind::Pinned, Leaf).expect("allocate tail");
    assert!(live.mark_if_unmarked());

    let live_key = live.object_key();
    let tail_key = tail.object_key();
    let mut heap = HeapCore::new(HeapConfig::default());
    heap.with_flat_store_for_reclaim_commit(|flat, old_gen, stats| {
        flat.objects = vec![dead, live, tail];
        flat.indexes = HeapIndexState::default();
        for (index, object) in flat.objects.iter().enumerate() {
            flat.indexes.record_allocated_object(
                object.object_key(),
                locator(index),
                object.header().desc(),
                object.space(),
            );
        }

        let commit = apply_prepared_reclaim(
            &mut flat.objects,
            &mut flat.indexes,
            old_gen,
            stats,
            &runtime_state,
            PreparedReclaim {
                prepared_object_count: 2,
                promoted_bytes: 0,
                old_gen: PreparedOldGenReclaim::default(),
                indexes: PreparedIndexReclaim {
                    finalize_indices: vec![0],
                    ..PreparedIndexReclaim::default()
                },
                survivors: vec![PreparedReclaimSurvivor { object_index: 1 }],
                stats: PreparedHeapStats::default(),
            },
            |_pending| 1,
        );

        assert_eq!(commit.queued_finalizers, 1);
        assert_eq!(flat.objects.len(), 2);
        assert_eq!(flat.objects[0].object_key(), live_key);
        assert_eq!(flat.objects[1].object_key(), tail_key);
        assert_eq!(flat.indexes.object_index.get(&live_key), Some(&locator(0)));
        assert_eq!(flat.indexes.object_index.get(&tail_key), Some(&locator(1)));
    });
}
