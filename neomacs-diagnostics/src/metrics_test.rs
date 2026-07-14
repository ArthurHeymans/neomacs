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
            composite_only_frames: 10,
            retained_static_builds: 3,
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
