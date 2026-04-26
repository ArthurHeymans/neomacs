use super::*;
use crate::descriptor::{EphemeronVisitor, Relocator, Trace, Tracer, TypeFlags, WeakProcessor};
use crate::heap::HeapConfig;
use crate::spaces::{LargeObjectSpaceConfig, NurseryConfig, OldGenConfig};
use crate::weak::{Ephemeron, Weak};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct NurseryLeaf {
    _byte: u8,
}

unsafe impl Trace for NurseryLeaf {
    fn trace(&self, _tracer: &mut dyn Tracer) {}

    fn relocate(&self, _relocator: &mut dyn Relocator) {}
}

#[derive(Debug)]
struct OldLeaf {
    _bytes: [u8; 32],
}

unsafe impl Trace for OldLeaf {
    fn trace(&self, _tracer: &mut dyn Tracer) {}

    fn relocate(&self, _relocator: &mut dyn Relocator) {}
}

#[derive(Debug)]
struct EphemeronOldHolder {
    pair: Ephemeron<OldLeaf, OldLeaf>,
}

unsafe impl Trace for EphemeronOldHolder {
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

#[test]
fn synchronous_minor_collect_records_one_pause_sample() {
    let heap = crate::Heap::new(HeapConfig::default());
    assert_eq!(heap.pause_histogram().total_samples, 0);

    let mut mutator = heap.mutator();
    let mut scope = mutator.handle_scope();
    for byte in 0..8u8 {
        mutator
            .alloc(&mut scope, NurseryLeaf { _byte: byte })
            .expect("allocate nursery leaf");
    }
    drop(scope);

    mutator
        .collect(CollectionKind::Minor)
        .expect("run synchronous minor collect");

    let snapshot = heap.pause_histogram();
    assert_eq!(snapshot.total_samples, 1);
    assert_eq!(snapshot.sample_count, 1);
}

#[test]
fn pause_histogram_records_reclaim_prepare_and_each_commit_slice() {
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            concurrent_mark_workers: 2,
            mutator_assist_slices: 0,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let mut mutator = heap.mutator();
    let mut scope = mutator.handle_scope();
    for byte in 0..2048u16 {
        mutator
            .alloc(
                &mut scope,
                OldLeaf {
                    _bytes: [byte as u8; 32],
                },
            )
            .expect("allocate old leaf");
    }

    mutator
        .begin_major_mark(CollectionPlan {
            mark_slice_budget: usize::MAX,
            ..mutator.plan_for(CollectionKind::Major)
        })
        .expect("begin major mark");

    let progress = mutator
        .poll_active_major_mark()
        .expect("poll active major mark")
        .expect("major-mark session should stay active");
    assert!(progress.completed);
    assert_eq!(
        mutator
            .active_major_mark_plan()
            .map(|plan| plan.phase == CollectionPhase::Reclaim),
        Some(false),
        "the first bounded reclaim-prep slice should not force the session into reclaim yet"
    );

    let mut prepare_slices = 1u64;
    let after_initial_prepare = heap.pause_histogram();
    assert_eq!(
        after_initial_prepare.total_samples, 1,
        "the first reclaim-prep slice should record its own stop-the-world sample"
    );
    while !mutator
        .prepare_active_reclaim_if_needed()
        .expect("follow-up reclaim prep assist")
    {
        prepare_slices = prepare_slices.saturating_add(1);
    }
    prepare_slices = prepare_slices.saturating_add(1);
    assert_eq!(
        mutator.active_major_mark_plan().map(|plan| plan.phase),
        Some(CollectionPhase::Reclaim)
    );
    assert_eq!(heap.pause_histogram().total_samples, prepare_slices);

    let first = mutator
        .advance_active_reclaim_commit_with_budget(1)
        .expect("first reclaim assist");
    assert!(first.is_none());
    assert_eq!(
        heap.pause_histogram().total_samples,
        prepare_slices + 1,
        "an incomplete reclaim assist should immediately record one pause sample"
    );

    let mut assists = 1u64;
    while mutator
        .advance_active_reclaim_commit_with_budget(1)
        .expect("follow-up reclaim assist")
        .is_none()
    {
        assists = assists.saturating_add(1);
    }
    assists = assists.saturating_add(1);

    let snapshot = heap.pause_histogram();
    assert_eq!(
        snapshot.total_samples,
        prepare_slices + assists,
        "expected one sample per reclaim-prep slice plus one sample per reclaim assist"
    );
    assert!(mutator.active_major_mark_plan().is_none());
}

#[test]
fn commit_active_reclaim_if_ready_uses_bounded_commit_slices() {
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            concurrent_mark_workers: 2,
            mutator_assist_slices: 0,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let mut mutator = heap.mutator();
    let mut scope = mutator.handle_scope();
    for byte in 0..300u16 {
        mutator
            .alloc(
                &mut scope,
                OldLeaf {
                    _bytes: [byte as u8; 32],
                },
            )
            .expect("allocate old leaf");
    }

    mutator
        .begin_major_mark(CollectionPlan {
            mark_slice_budget: usize::MAX,
            ..mutator.plan_for(CollectionKind::Major)
        })
        .expect("begin major mark");
    let progress = mutator
        .poll_active_major_mark()
        .expect("poll active major mark")
        .expect("major-mark session should stay active");
    assert!(progress.completed);
    while mutator
        .active_major_mark_plan()
        .is_some_and(|plan| plan.phase != CollectionPhase::Reclaim)
    {
        mutator
            .prepare_active_reclaim_if_needed()
            .expect("advance reclaim prep");
    }

    let samples_before_commit = heap.pause_histogram().total_samples;
    let first = mutator
        .commit_active_reclaim_if_ready()
        .expect("first bounded reclaim commit");
    assert!(
        first.is_none(),
        "default commit should not finish a large reclaim in one pause"
    );
    assert_eq!(
        mutator.active_major_mark_plan().map(|plan| plan.phase),
        Some(CollectionPhase::Reclaim)
    );
    assert_eq!(
        heap.pause_histogram().total_samples,
        samples_before_commit + 1
    );

    let mut commit_slices = 1usize;
    while mutator
        .commit_active_reclaim_if_ready()
        .expect("follow-up bounded reclaim commit")
        .is_none()
    {
        commit_slices = commit_slices.saturating_add(1);
        assert!(commit_slices < 16, "reclaim commit did not converge");
    }
    commit_slices = commit_slices.saturating_add(1);
    assert!(commit_slices > 1);
    assert!(mutator.active_major_mark_plan().is_none());
}

#[test]
fn finish_major_collection_drains_reclaim_with_bounded_commit_slices() {
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            concurrent_mark_workers: 2,
            mutator_assist_slices: 0,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let mut mutator = heap.mutator();
    let mut scope = mutator.handle_scope();
    for byte in 0..5000u16 {
        mutator
            .alloc(
                &mut scope,
                OldLeaf {
                    _bytes: [byte as u8; 32],
                },
            )
            .expect("allocate old leaf");
    }

    mutator
        .begin_major_mark(CollectionPlan {
            mark_slice_budget: usize::MAX,
            ..mutator.plan_for(CollectionKind::Major)
        })
        .expect("begin major mark");
    let progress = mutator
        .poll_active_major_mark()
        .expect("poll active major mark")
        .expect("major-mark session should stay active");
    assert!(progress.completed);
    while mutator
        .active_major_mark_plan()
        .is_some_and(|plan| plan.phase != CollectionPhase::Reclaim)
    {
        mutator
            .prepare_active_reclaim_if_needed()
            .expect("advance reclaim prep");
    }

    let samples_before_finish = heap.pause_histogram().total_samples;
    let cycle = mutator
        .finish_major_collection()
        .expect("finish active major collection");
    assert_eq!(cycle.major_collections, 1);
    assert!(
        heap.pause_histogram().total_samples >= samples_before_finish + 3,
        "large synchronous finish should drain reclaim through multiple bounded commit slices"
    );
    assert!(mutator.active_major_mark_plan().is_none());
}

#[test]
fn major_collect_uses_bounded_active_pipeline() {
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            concurrent_mark_workers: 2,
            mutator_assist_slices: 0,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let mut mutator = heap.mutator();
    {
        let mut scope = mutator.handle_scope();
        for byte in 0..5000u16 {
            mutator
                .alloc(
                    &mut scope,
                    OldLeaf {
                        _bytes: [byte as u8; 32],
                    },
                )
                .expect("allocate old leaf");
        }
    }

    let samples_before = heap.pause_histogram().total_samples;
    let cycle = mutator
        .collect(CollectionKind::Major)
        .expect("run bounded major collect");
    assert_eq!(cycle.major_collections, 1);
    assert!(mutator.active_major_mark_plan().is_none());
    assert!(
        heap.pause_histogram().total_samples >= samples_before + 3,
        "major collect should drain through multiple bounded active-major slices"
    );
}

#[test]
fn shared_collector_runtime_reclaim_commit_budget_adapts_after_observed_slice() {
    let shared = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            concurrent_mark_workers: 2,
            mutator_assist_slices: 0,
            ..OldGenConfig::default()
        },
        pacer: crate::pacer::PacerConfig {
            target_pause: std::time::Duration::from_secs(1),
            ..crate::pacer::PacerConfig::default()
        },
        ..HeapConfig::default()
    })
    .into_shared();
    let plan = shared
        .with_mutator(|mutator| {
            let mut scope = mutator.handle_scope();
            for byte in 0..512u16 {
                mutator
                    .alloc(
                        &mut scope,
                        OldLeaf {
                            _bytes: [byte as u8; 32],
                        },
                    )
                    .expect("allocate old leaf");
            }
            CollectionPlan {
                mark_slice_budget: usize::MAX,
                ..mutator.plan_for(CollectionKind::Major)
            }
        })
        .expect("seed shared heap and compute major plan");
    let runtime = shared.collector_runtime();

    runtime
        .begin_major_mark(plan)
        .expect("begin shared major mark");
    while !runtime
        .poll_active_major_mark()
        .expect("poll shared major mark")
        .expect("active major mark should stay active")
        .completed
    {}
    while !runtime
        .prepare_active_reclaim_if_needed()
        .expect("prepare shared major reclaim")
    {}

    assert_eq!(
        runtime
            .shared_reclaim_commit_budget()
            .expect("read initial shared reclaim commit budget"),
        DEFAULT_RECLAIM_COMMIT_SLICE_BUDGET
    );
    assert!(
        runtime
            .advance_active_reclaim_commit_with_budget(1)
            .expect("advance first shared reclaim commit slice")
            .is_none()
    );
    let adaptive_budget = runtime
        .shared_reclaim_commit_budget()
        .expect("read adaptive shared reclaim commit budget");
    assert!(
        adaptive_budget > DEFAULT_RECLAIM_COMMIT_SLICE_BUDGET,
        "observed small shared commit slice should grow the next budget, got {adaptive_budget}"
    );
}

#[test]
fn full_collect_uses_bounded_active_pipeline() {
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            concurrent_mark_workers: 2,
            mutator_assist_slices: 0,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let mut mutator = heap.mutator();
    {
        let mut scope = mutator.handle_scope();
        for byte in 0..5000u16 {
            mutator
                .alloc(
                    &mut scope,
                    OldLeaf {
                        _bytes: [byte as u8; 32],
                    },
                )
                .expect("allocate old leaf");
        }
    }

    let samples_before = heap.pause_histogram().total_samples;
    let cycle = mutator
        .collect(CollectionKind::Full)
        .expect("run bounded full collect");
    assert_eq!(cycle.major_collections, 1);
    assert!(mutator.active_major_mark_plan().is_none());
    assert_eq!(
        heap.recent_phase_trace(),
        &[
            CollectionPhase::InitialMark,
            CollectionPhase::ConcurrentMark,
            CollectionPhase::Remark,
            CollectionPhase::Evacuate,
            CollectionPhase::Reclaim,
        ]
    );
    assert!(
        heap.pause_histogram().total_samples >= samples_before + 3,
        "full collect should drain through multiple active-collection slices"
    );
}

#[test]
fn full_reclaim_prepare_builds_snapshot_in_bounded_slices() {
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            concurrent_mark_workers: 2,
            mutator_assist_slices: 0,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let mut mutator = heap.mutator();
    {
        let mut scope = mutator.handle_scope();
        for byte in 0..512u16 {
            mutator
                .alloc(
                    &mut scope,
                    OldLeaf {
                        _bytes: [byte as u8; 32],
                    },
                )
                .expect("allocate old leaf");
        }
    }
    drop(mutator);

    let mut plan = heap.plan_for(CollectionKind::Full);
    plan.mark_slice_budget = usize::MAX;
    {
        let mut runtime = heap.collector_runtime();
        runtime.begin_major_mark(plan).expect("begin full mark");
        while !runtime
            .poll_active_major_mark()
            .expect("poll full mark")
            .expect("active full mark should stay active")
            .completed
        {}

        let prepared = runtime
            .runtime()
            .prepare_active_reclaim_if_needed_with_budget(1)
            .expect("prepare first full reclaim slice");
        assert!(!prepared);
        assert_ne!(
            runtime
                .active_major_mark_plan()
                .expect("active plan after partial full prep")
                .phase,
            CollectionPhase::Reclaim
        );
    }

    assert_eq!(
        heap.recent_phase_trace(),
        &[
            CollectionPhase::InitialMark,
            CollectionPhase::ConcurrentMark,
            CollectionPhase::Remark,
            CollectionPhase::Evacuate,
        ]
    );

    let cycle = heap
        .finish_major_collection()
        .expect("finish bounded full collection");
    assert_eq!(cycle.major_collections, 1);
    assert_eq!(
        heap.recent_phase_trace(),
        &[
            CollectionPhase::InitialMark,
            CollectionPhase::ConcurrentMark,
            CollectionPhase::Remark,
            CollectionPhase::Evacuate,
            CollectionPhase::Reclaim,
        ]
    );
}

#[test]
fn active_reclaim_prep_slices_ephemeron_fixpoint_before_snapshot() {
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            mutator_assist_slices: 0,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let mut mutator = heap.mutator();
    let mut keep_scope = mutator.handle_scope();
    let (holder_gc, key_gc, value_gc) = {
        let mut setup_scope = mutator.handle_scope();
        let key = mutator
            .alloc(&mut setup_scope, OldLeaf { _bytes: [1; 32] })
            .expect("allocate ephemeron key");
        let value = mutator
            .alloc(&mut setup_scope, OldLeaf { _bytes: [2; 32] })
            .expect("allocate ephemeron value");
        let holder = mutator
            .alloc(
                &mut setup_scope,
                EphemeronOldHolder {
                    pair: Ephemeron::new(Weak::new(key.as_gc()), Weak::new(value.as_gc())),
                },
            )
            .expect("allocate ephemeron holder");
        (holder.as_gc(), key.as_gc(), value.as_gc())
    };
    let _holder = mutator.root(&mut keep_scope, holder_gc);
    let _key = mutator.root(&mut keep_scope, key_gc);

    mutator
        .begin_major_mark(CollectionPlan {
            mark_slice_budget: usize::MAX,
            ..mutator.plan_for(CollectionKind::Major)
        })
        .expect("begin active major mark");
    while !mutator
        .poll_active_major_mark()
        .expect("poll active major mark")
        .expect("major mark should stay active")
        .completed
    {}

    assert_eq!(
        mutator.active_major_mark_plan().map(|plan| plan.phase),
        Some(CollectionPhase::Remark)
    );

    let prepared = mutator
        .prepare_active_reclaim_if_needed()
        .expect("stable ephemeron reclaim-prep slice");
    assert!(prepared);
    assert_eq!(
        mutator.active_major_mark_plan().map(|plan| plan.phase),
        Some(CollectionPhase::Reclaim)
    );

    let cycle = mutator
        .finish_major_collection()
        .expect("finish active major after ephemeron prep");
    assert_eq!(cycle.major_collections, 1);
    assert!(mutator.heap().contains(value_gc));
}

#[test]
fn execute_major_plan_uses_bounded_active_pipeline() {
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            concurrent_mark_workers: 2,
            mutator_assist_slices: 0,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let mut mutator = heap.mutator();
    {
        let mut scope = mutator.handle_scope();
        for byte in 0..5000u16 {
            mutator
                .alloc(
                    &mut scope,
                    OldLeaf {
                        _bytes: [byte as u8; 32],
                    },
                )
                .expect("allocate old leaf");
        }
    }

    let plan = mutator.plan_for(CollectionKind::Major);
    let samples_before = heap.pause_histogram().total_samples;
    let cycle = mutator
        .execute_plan(plan)
        .expect("run bounded major execute_plan");
    assert_eq!(cycle.major_collections, 1);
    assert!(mutator.active_major_mark_plan().is_none());
    assert!(
        heap.pause_histogram().total_samples >= samples_before + 3,
        "major execute_plan should drain through multiple bounded active-major slices"
    );
}

#[test]
fn adaptive_pause_target_budget_scales_with_observed_throughput() {
    assert_eq!(
        adaptive_pause_target_budget(Duration::from_millis(10), 128, 2_000_000, 64, 1, 4096),
        640
    );
    assert_eq!(
        adaptive_pause_target_budget(Duration::from_millis(10), 0, 0, 64, 1, 4096),
        64
    );
    assert_eq!(
        adaptive_pause_target_budget(Duration::from_millis(10), 128, 2_000_000, 64, 1, 128),
        128
    );
}

#[test]
fn deferred_auto_compaction_runs_in_bounded_slices_and_relocates_external_roots() {
    let old_bytes =
        crate::object::estimated_allocation_size::<OldLeaf>().expect("old leaf allocation size");
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            region_bytes: old_bytes.saturating_mul(3),
            line_bytes: 16,
            physical_compaction_density_threshold: 0.99,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let external_roots = Arc::new(Mutex::new(Vec::<crate::root::Gc<OldLeaf>>::new()));
    let scanner_roots = Arc::clone(&external_roots);
    heap.set_external_root_scanner(move |roots| {
        let roots_guard = scanner_roots.lock().expect("external root scanner lock");
        roots.extend(roots_guard.iter().map(|gc| gc.erase()));
    });
    let relocator_roots = Arc::clone(&external_roots);
    heap.set_external_root_relocator(move |relocator| {
        let mut roots_guard = relocator_roots
            .lock()
            .expect("external root relocator lock");
        for slot in roots_guard.iter_mut() {
            let relocated = relocator.relocate_erased(slot.erase());
            *slot = unsafe { crate::root::Gc::from_erased(relocated) };
        }
    });

    let mut mutator = heap.mutator();
    {
        let mut scope = mutator.handle_scope();
        let first = mutator
            .alloc(&mut scope, OldLeaf { _bytes: [10; 32] })
            .expect("alloc first old leaf");
        mutator
            .alloc(&mut scope, OldLeaf { _bytes: [11; 32] })
            .expect("alloc dead old leaf in first block");
        let third = mutator
            .alloc(&mut scope, OldLeaf { _bytes: [12; 32] })
            .expect("alloc third old leaf");
        let fourth = mutator
            .alloc(&mut scope, OldLeaf { _bytes: [20; 32] })
            .expect("alloc fourth old leaf");
        mutator
            .alloc(&mut scope, OldLeaf { _bytes: [21; 32] })
            .expect("alloc dead old leaf in second block");
        let sixth = mutator
            .alloc(&mut scope, OldLeaf { _bytes: [22; 32] })
            .expect("alloc sixth old leaf");
        external_roots
            .lock()
            .expect("external roots install lock")
            .extend([first.as_gc(), third.as_gc(), fourth.as_gc(), sixth.as_gc()]);
    }

    mutator
        .begin_major_mark(mutator.plan_for(CollectionKind::Major))
        .expect("begin major mark");
    while !mutator
        .poll_active_major_mark()
        .expect("poll active major mark")
        .expect("major-mark session should stay active")
        .completed
    {}

    let cycle = mutator
        .finish_active_major_collection_if_ready()
        .expect("finish active major collection")
        .expect("completed cycle");
    assert_eq!(cycle.major_collections, 1);
    assert_eq!(
        heap.compaction_stats().cycles,
        0,
        "incremental major completion should defer physical compaction"
    );

    let pause_samples_before = heap.pause_histogram().total_samples;
    let budget_bytes = old_bytes.saturating_mul(2);
    let first_slice = mutator.advance_auto_compaction_with_byte_budget(budget_bytes);
    assert!(
        first_slice > 0,
        "first deferred compaction assist should move one sparse block"
    );
    assert_eq!(heap.compaction_stats().cycles, 1);
    assert_eq!(
        heap.pause_histogram().total_samples,
        pause_samples_before + 1
    );

    let second_slice = mutator.advance_auto_compaction_with_byte_budget(budget_bytes);
    assert!(
        second_slice > 0,
        "second deferred compaction assist should finish the remaining sparse block"
    );
    assert!(heap.compaction_stats().cycles >= 2);
    assert_eq!(
        heap.pause_histogram().total_samples,
        pause_samples_before + 2
    );
    let mut extra_slices = 0u64;
    while extra_slices < 8 {
        if mutator.advance_auto_compaction_with_byte_budget(budget_bytes) == 0 {
            break;
        }
        extra_slices = extra_slices.saturating_add(1);
    }
    assert_eq!(
        mutator.advance_auto_compaction_with_byte_budget(budget_bytes),
        0,
        "deferred compaction should eventually drain after bounded assists"
    );

    let relocated_payloads: Vec<u8> = external_roots
        .lock()
        .expect("read relocated external roots")
        .iter()
        .map(|gc| unsafe { gc.as_non_null().as_ref() }._bytes[0])
        .collect();
    assert_eq!(relocated_payloads, vec![10, 12, 20, 22]);
}

#[test]
fn synchronous_major_collect_defers_auto_compaction_to_bounded_assists() {
    let old_bytes =
        crate::object::estimated_allocation_size::<OldLeaf>().expect("old leaf allocation size");
    let heap = crate::Heap::new(HeapConfig {
        nursery: NurseryConfig {
            max_regular_object_bytes: 1,
            ..NurseryConfig::default()
        },
        large: LargeObjectSpaceConfig {
            threshold_bytes: usize::MAX,
            ..LargeObjectSpaceConfig::default()
        },
        old: OldGenConfig {
            region_bytes: old_bytes.saturating_mul(3),
            line_bytes: 16,
            physical_compaction_density_threshold: 0.99,
            ..OldGenConfig::default()
        },
        ..HeapConfig::default()
    });
    let external_roots = Arc::new(Mutex::new(Vec::<crate::root::Gc<OldLeaf>>::new()));
    let scanner_roots = Arc::clone(&external_roots);
    heap.set_external_root_scanner(move |roots| {
        let roots_guard = scanner_roots.lock().expect("external root scanner lock");
        roots.extend(roots_guard.iter().map(|gc| gc.erase()));
    });
    let relocator_roots = Arc::clone(&external_roots);
    heap.set_external_root_relocator(move |relocator| {
        let mut roots_guard = relocator_roots
            .lock()
            .expect("external root relocator lock");
        for slot in roots_guard.iter_mut() {
            let relocated = relocator.relocate_erased(slot.erase());
            *slot = unsafe { crate::root::Gc::from_erased(relocated) };
        }
    });

    let mut mutator = heap.mutator();
    {
        let mut scope = mutator.handle_scope();
        let first = mutator
            .alloc(&mut scope, OldLeaf { _bytes: [10; 32] })
            .expect("alloc first old leaf");
        mutator
            .alloc(&mut scope, OldLeaf { _bytes: [11; 32] })
            .expect("alloc dead old leaf in first block");
        let third = mutator
            .alloc(&mut scope, OldLeaf { _bytes: [12; 32] })
            .expect("alloc third old leaf");
        let fourth = mutator
            .alloc(&mut scope, OldLeaf { _bytes: [20; 32] })
            .expect("alloc fourth old leaf");
        mutator
            .alloc(&mut scope, OldLeaf { _bytes: [21; 32] })
            .expect("alloc dead old leaf in second block");
        let sixth = mutator
            .alloc(&mut scope, OldLeaf { _bytes: [22; 32] })
            .expect("alloc sixth old leaf");
        external_roots
            .lock()
            .expect("external roots install lock")
            .extend([first.as_gc(), third.as_gc(), fourth.as_gc(), sixth.as_gc()]);
    }

    let pause_samples_before = heap.pause_histogram().total_samples;
    let cycle = mutator
        .collect(CollectionKind::Major)
        .expect("run synchronous major collect");
    assert_eq!(cycle.major_collections, 1);
    assert_eq!(
        mutator.runtime_work_status(),
        RuntimeWorkStatus::PendingAutoCompaction {
            remaining_bytes: old_bytes.saturating_mul(4),
        },
        "synchronous major completion should publish deferred auto-compaction work"
    );
    assert_eq!(
        heap.compaction_stats().cycles,
        0,
        "synchronous major completion should schedule, not inline, physical compaction"
    );
    let after_major_samples = heap.pause_histogram().total_samples;
    assert!(
        after_major_samples > pause_samples_before,
        "major collect should record its bounded collection pauses before deferred compaction"
    );

    let budget_bytes = old_bytes.saturating_mul(2);
    let first_slice = mutator.advance_auto_compaction_with_byte_budget(budget_bytes);
    assert!(
        first_slice > 0,
        "first deferred compaction assist should move one sparse block"
    );
    assert_eq!(heap.compaction_stats().cycles, 1);
    assert_eq!(
        heap.pause_histogram().total_samples,
        after_major_samples + 1
    );

    let mut extra_slices = 0u64;
    while extra_slices < 8 {
        if mutator.advance_auto_compaction_with_byte_budget(budget_bytes) == 0 {
            break;
        }
        extra_slices = extra_slices.saturating_add(1);
    }
    assert_eq!(
        mutator.advance_auto_compaction_with_byte_budget(budget_bytes),
        0,
        "deferred compaction should eventually drain after bounded assists"
    );

    let relocated_payloads: Vec<u8> = external_roots
        .lock()
        .expect("read relocated external roots")
        .iter()
        .map(|gc| unsafe { gc.as_non_null().as_ref() }._bytes[0])
        .collect();
    assert_eq!(relocated_payloads, vec![10, 12, 20, 22]);
}
