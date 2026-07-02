use std::path::PathBuf;
use std::time::Duration;

use neomacs_gui_tests::{
    DisplayHarness, GuiBackend, GuiRunOptions, GuiRunStatus, GuiScenario, GuiTestPlan,
    ProcessGuiCommandRunner,
};

#[test]
fn real_gui_smoke_generates_surface_readback_png() {
    let Some(backend) = requested_backend() else {
        eprintln!("skipping real GUI smoke; set NEOMACS_GUI_TEST_BACKEND=wayland or x11 to run it");
        return;
    };

    let workspace_root = workspace_root();
    let binary = neomacs_binary(&workspace_root);
    assert!(
        binary.exists(),
        "build {binary:?} before running the real GUI smoke"
    );

    let artifact_root = workspace_root.join("target/neomacs-gui-tests");
    let session = DisplayHarness::for_backend(backend)
        .start_session(&artifact_root)
        .expect("display session should start");
    let scenario = GuiScenario::new(
        "real-startup-smoke",
        workspace_root.join("neomacs-gui-tests/fixtures/startup-smoke.el"),
    );
    let mut plan =
        GuiTestPlan::new(backend, &workspace_root, &artifact_root, scenario).with_program(binary);
    for (key, value) in session.env() {
        plan = plan.with_env(key.clone(), value.clone());
    }

    let mut runner = ProcessGuiCommandRunner;
    let result = plan
        .run_with(
            &mut runner,
            GuiRunOptions::with_timeout(Duration::from_secs(12)),
        )
        .expect("GUI run should produce text artifacts");

    assert_eq!(result.status, GuiRunStatus::Passed, "{result:#?}");
    assert!(
        result.png_bytes.unwrap_or_default() > 0,
        "readback PNG should be non-empty"
    );

    // Display oracle: assert what redisplay actually produced, not what the
    // fixture's Lisp said it intended.
    let snapshot_txt = std::fs::read_to_string(&result.artifacts.frame_snapshot_txt)
        .expect("frame snapshot text artifact (rebuild target/release/neomacs if stale)");
    assert!(
        snapshot_txt.contains("=== frame "),
        "snapshot frame header:\n{snapshot_txt:.500}"
    );
    assert!(
        snapshot_txt.contains("NeoMacs GUI smoke line 00"),
        "smoke buffer text visible on screen:\n{snapshot_txt:.2000}"
    );
    assert!(
        snapshot_txt.contains("*neomacs-gui-smoke*"),
        "smoke buffer name in window header:\n{snapshot_txt:.2000}"
    );
    let snapshot_json = std::fs::read_to_string(&result.artifacts.frame_snapshot_json)
        .expect("frame snapshot JSON artifact");
    let doc: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot JSON parses");
    assert!(
        !doc["frames"].as_array().expect("frames array").is_empty(),
        "at least one frame in snapshot"
    );
}

fn requested_backend() -> Option<GuiBackend> {
    match std::env::var("NEOMACS_GUI_TEST_BACKEND").ok()?.as_str() {
        "wayland" | "linux-wayland" => Some(GuiBackend::LinuxWayland),
        "x11" | "linux-x11" => Some(GuiBackend::LinuxX11),
        "macos" => Some(GuiBackend::Macos),
        "windows" => Some(GuiBackend::Windows),
        other => panic!("unsupported NEOMACS_GUI_TEST_BACKEND={other:?}"),
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("test crate should live below workspace root")
        .to_path_buf()
}

fn neomacs_binary(workspace_root: &std::path::Path) -> PathBuf {
    if let Some(path) = std::env::var_os("NEOMACS_GUI_TEST_BINARY") {
        return PathBuf::from(path);
    }

    workspace_root.join("target/release/neomacs")
}
