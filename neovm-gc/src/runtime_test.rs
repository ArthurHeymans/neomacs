use super::*;
use crate::descriptor::{Relocator, Trace, Tracer};
use crate::heap::HeapConfig;
use crate::spaces::{LargeObjectSpaceConfig, NurseryConfig, OldGenConfig};

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
        mutator.active_major_mark_plan().map(|plan| plan.phase),
        Some(CollectionPhase::Reclaim)
    );

    let after_prepare = heap.pause_histogram();
    assert_eq!(
        after_prepare.total_samples, 1,
        "reclaim preparation should record its own stop-the-world sample"
    );

    let first = mutator
        .advance_active_reclaim_commit_with_budget(1)
        .expect("first reclaim assist");
    assert!(first.is_none());
    assert_eq!(
        heap.pause_histogram().total_samples,
        after_prepare.total_samples + 1,
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
        1 + assists,
        "expected one sample for reclaim prep plus one sample per reclaim assist"
    );
    assert!(mutator.active_major_mark_plan().is_none());
}
