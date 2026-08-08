use crate::metrics::{FrameMetrics, GcMetrics, MetricsSnapshot};

#[test]
fn snapshot_serializes_stable_json_shape() {
    let snap = MetricsSnapshot {
        frame: FrameMetrics {
            presents: 100,
            scene_commits: 90,
            wakeups: 500,
            last_commit_to_present_us: 1200,
            max_commit_to_present_us: 8000,
            frame_p50_us: 2000,
            frame_p95_us: 8000,
            frame_p99_us: 16000,
            composite_only_frames: 10,
            retained_static_builds: 3,
            demand_reasons: [("cursor_animation".to_owned(), 12)].into_iter().collect(),
        },
        gc: GcMetrics {
            collections: 7,
            live_bytes: 4096,
            total_allocated_bytes: 1_000_000,
            cons_cells: 200,
            strings: 40,
            vector_cells: 60,
        },
    };
    let v: serde_json::Value = serde_json::to_value(&snap).unwrap();
    assert_eq!(v["frame"]["presents"], 100);
    assert_eq!(v["frame"]["last_commit_to_present_us"], 1200);
    assert_eq!(v["frame"]["demand_reasons"]["cursor_animation"], 12);
    assert_eq!(v["gc"]["collections"], 7);
    assert_eq!(v["gc"]["cons_cells"], 200);
}

#[test]
fn default_snapshot_is_all_zero() {
    let snap = MetricsSnapshot::default();
    assert_eq!(snap.frame.presents, 0);
    assert_eq!(snap.gc.collections, 0);
    let v = serde_json::to_value(&snap).unwrap();
    assert_eq!(v["frame"]["max_commit_to_present_us"], 0);
    assert_eq!(v["gc"]["vector_cells"], 0);
}

#[test]
fn percentile_from_buckets_picks_the_right_bucket() {
    use crate::metrics::percentile_from_buckets;
    // bounds: <=1,<=2,<=4,<=8,<=16,<=33,<=66 (thousands us), then unbounded.
    let bounds = [
        1_000u64,
        2_000,
        4_000,
        8_000,
        16_000,
        33_000,
        66_000,
        u64::MAX,
    ];
    // 100 frames: 90 in <=2ms, 8 in <=8ms, 2 in the unbounded (>66ms) bucket.
    let buckets = [0u64, 90, 0, 8, 0, 0, 0, 2];
    assert_eq!(percentile_from_buckets(&buckets, &bounds, 0.50), 2_000); // median
    assert_eq!(percentile_from_buckets(&buckets, &bounds, 0.95), 8_000); // 95th
    // 99th falls in the unbounded top bucket -> reported as its lower edge 66ms.
    assert_eq!(percentile_from_buckets(&buckets, &bounds, 0.99), 66_000);
    // No samples -> 0.
    assert_eq!(percentile_from_buckets(&[0, 0, 0], &[1, 2, 3], 0.5), 0);
}
