//! Collection latency benchmarks.
//!
//! Measures the wall-clock cost of minor and major
//! collection cycles triggered by `Mutator::collect`. Each
//! bench sets up a heap with a known live set, runs one
//! collection, and reports the cycle duration.
//!
//! Criterion's statistical analysis gives a median plus
//! outlier detection, which is the right shape for pause
//! latency — the important number for a GC consumer is
//! "what's the typical pause under this workload" plus
//! "what's the tail" (P95/P99).
//!
//! Each bench uses `iter_custom` to control setup-per-
//! iteration carefully: the heap is reconstructed from
//! scratch for every measured cycle so the starting state
//! is deterministic.
//!
//! Runs with `cargo bench --bench collection_latency -p
//! neovm-gc`.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use neovm_gc::spaces::{LargeObjectSpaceConfig, NurseryConfig, OldGenConfig};
use neovm_gc::{
    CollectionKind, Heap, HeapConfig, Relocator, Trace, Tracer, estimated_allocation_size,
};
use std::time::{Duration, Instant};

#[path = "common/mod.rs"]
mod common;
use common::*;

#[derive(Debug)]
struct OldLeaf {
    _bytes: [u8; 32],
}

unsafe impl Trace for OldLeaf {
    fn trace(&self, _: &mut dyn Tracer) {}
    fn relocate(&self, _: &mut dyn Relocator) {}
}

fn drive_major_mark_to_completion(mutator: &mut neovm_gc::Mutator<'_>) {
    let plan = mutator.plan_for(CollectionKind::Major);
    mutator.begin_major_mark(plan).expect("begin major mark");
    while !mutator
        .poll_active_major_mark()
        .expect("poll active major mark")
        .expect("major-mark session should stay active")
        .completed
    {}
}

fn bench_incremental_major_reclaim_commit_slice(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection_latency/incremental_major/reclaim_commit_slice");
    group.throughput(Throughput::Elements(64));
    group.bench_function("dead_old_4096/budget64", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let heap = Heap::new(HeapConfig {
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
                {
                    let mut scope = mutator.handle_scope();
                    for byte in 0..4_096u64 {
                        mutator
                            .alloc(
                                &mut scope,
                                OldLeaf {
                                    _bytes: [(byte & 0xff) as u8; 32],
                                },
                            )
                            .expect("alloc dead old leaf");
                    }
                }

                drive_major_mark_to_completion(&mut mutator);
                while !mutator
                    .prepare_active_reclaim_if_needed()
                    .expect("prepare active reclaim")
                {}

                let start = Instant::now();
                black_box(
                    mutator
                        .advance_active_reclaim_commit_with_budget(64)
                        .expect("advance reclaim commit slice"),
                );
                total += start.elapsed();
            }
            total
        });
    });
    group.finish();
}

fn bench_deferred_auto_compaction_slice(c: &mut Criterion) {
    let old_bytes = estimated_allocation_size::<OldLeaf>().expect("old leaf allocation size");
    let mut group = c.benchmark_group("collection_latency/incremental_major/auto_compaction_slice");
    group.throughput(Throughput::Bytes((old_bytes.saturating_mul(2)) as u64));
    group.bench_function("two_sparse_blocks/budget2_records", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let heap = Heap::new(HeapConfig {
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
                let mut mutator = heap.mutator();
                let mut keep_scope = mutator.handle_scope();
                mutator
                    .alloc(&mut keep_scope, OldLeaf { _bytes: [10; 32] })
                    .expect("alloc first live old leaf");
                {
                    let mut dead_scope = mutator.handle_scope();
                    mutator
                        .alloc(&mut dead_scope, OldLeaf { _bytes: [11; 32] })
                        .expect("alloc dead old leaf in first sparse block");
                }
                mutator
                    .alloc(&mut keep_scope, OldLeaf { _bytes: [12; 32] })
                    .expect("alloc third live old leaf");
                mutator
                    .alloc(&mut keep_scope, OldLeaf { _bytes: [20; 32] })
                    .expect("alloc fourth live old leaf");
                {
                    let mut dead_scope = mutator.handle_scope();
                    mutator
                        .alloc(&mut dead_scope, OldLeaf { _bytes: [21; 32] })
                        .expect("alloc dead old leaf in second sparse block");
                }
                mutator
                    .alloc(&mut keep_scope, OldLeaf { _bytes: [22; 32] })
                    .expect("alloc sixth live old leaf");

                drive_major_mark_to_completion(&mut mutator);
                let cycle = mutator
                    .finish_active_major_collection_if_ready()
                    .expect("finish active major collection")
                    .expect("completed major collection");
                assert_eq!(cycle.major_collections, 1);

                let start = Instant::now();
                black_box(
                    mutator.advance_auto_compaction_with_byte_budget(old_bytes.saturating_mul(2)),
                );
                total += start.elapsed();
                drop(keep_scope);
            }
            total
        });
    });
    group.finish();
}

fn bench_minor_gc_small_nursery(c: &mut Criterion) {
    // A minor cycle with a modest nursery load. Most
    // allocations die; the survivor set is small.
    let mut group = c.benchmark_group("collection_latency/minor/small");
    group.throughput(Throughput::Elements(1));
    group.bench_function("drop_all", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let heap = Heap::new(fast_alloc_config());
                let mut mutator = heap.mutator();
                {
                    let mut scope = mutator.handle_scope();
                    for i in 0..1_000u64 {
                        mutator.alloc(&mut scope, SmallLeaf(i)).expect("alloc");
                    }
                }
                // Scope is dropped; all 1000 leaves are
                // unreachable. The minor cycle should
                // reclaim them in bulk.
                let start = Instant::now();
                black_box(
                    mutator
                        .collect(CollectionKind::Minor)
                        .expect("minor collect"),
                );
                total += start.elapsed();
            }
            total
        });
    });
    group.finish();
}

fn bench_minor_gc_all_survive(c: &mut Criterion) {
    // Every nursery allocation survives and must be copied
    // to the to-space (or promoted to old gen if old
    // enough).
    let mut group = c.benchmark_group("collection_latency/minor/all_survive");
    group.throughput(Throughput::Elements(1));
    group.bench_function("1000_survivors", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let heap = Heap::new(fast_alloc_config());
                let mut mutator = heap.mutator();
                let mut scope = mutator.handle_scope();
                for i in 0..1_000u64 {
                    mutator.alloc(&mut scope, SmallLeaf(i)).expect("alloc");
                }
                // Scope is still alive: every allocation
                // is rooted. The minor cycle must evacuate
                // all 1000.
                let start = Instant::now();
                black_box(
                    mutator
                        .collect(CollectionKind::Minor)
                        .expect("minor collect"),
                );
                total += start.elapsed();
                // Scope drop at end of iteration releases
                // roots for the next iteration.
                drop(scope);
            }
            total
        });
    });
    group.finish();
}

fn bench_major_gc_small(c: &mut Criterion) {
    // Major cycle on a small old-gen population. First
    // force survivors into old gen via a minor cycle,
    // then trigger a major on the populated old gen.
    let mut group = c.benchmark_group("collection_latency/major/small");
    group.throughput(Throughput::Elements(1));
    group.bench_function("1000_old_survivors", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let heap = Heap::new(fast_alloc_config());
                let mut mutator = heap.mutator();
                let mut scope = mutator.handle_scope();
                for i in 0..1_000u64 {
                    mutator.alloc(&mut scope, SmallLeaf(i)).expect("alloc");
                }
                // Run two minor cycles to age survivors
                // into the old gen (default promotion_age
                // is 2).
                let _ = mutator.collect(CollectionKind::Minor);
                let _ = mutator.collect(CollectionKind::Minor);
                // Now measure one major cycle.
                let start = Instant::now();
                black_box(
                    mutator
                        .collect(CollectionKind::Major)
                        .expect("major collect"),
                );
                total += start.elapsed();
                drop(scope);
            }
            total
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_minor_gc_small_nursery,
    bench_minor_gc_all_survive,
    bench_major_gc_small,
    bench_incremental_major_reclaim_commit_slice,
    bench_deferred_auto_compaction_slice,
);
criterion_main!(benches);
