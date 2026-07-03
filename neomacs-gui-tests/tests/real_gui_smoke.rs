use std::path::PathBuf;
use std::time::Duration;

use neomacs_gui_tests::{
    CommandSpec, DisplayHarness, GuiBackend, GuiCommandRunner, GuiRunOptions, GuiRunStatus,
    GuiScenario, GuiTestPlan, ProcessGuiCommandRunner,
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

#[test]
fn real_gui_font_selection_oracle_matches_gnu_emacs_result_structure() {
    if !cfg!(target_os = "linux") {
        eprintln!("skipping real GUI font selection oracle; X11 comparator is Linux-only");
        return;
    }

    let backend = GuiBackend::LinuxX11;
    let workspace_root = workspace_root();
    let binary = neomacs_binary(&workspace_root);
    assert!(
        binary.exists(),
        "build {binary:?} before running the real GUI font selection oracle"
    );
    let gnu_emacs = gnu_emacs_binary();
    assert!(
        gnu_emacs.exists(),
        "GNU Emacs binary {gnu_emacs:?} should exist for font selection oracle"
    );

    let artifact_root = workspace_root.join("target/neomacs-gui-tests");
    let session = DisplayHarness::for_backend(backend)
        .start_session(&artifact_root)
        .expect("display session should start");
    let scenario = GuiScenario::new(
        "font-selection-noto-bold",
        workspace_root.join("neomacs-gui-tests/fixtures/font-selection-noto-bold.el"),
    );
    let artifacts = neomacs_gui_tests::GuiArtifactSet::new(&artifact_root, backend, &scenario.name);
    let _ = std::fs::remove_file(&artifacts.gnu_font_result);
    let _ = std::fs::remove_file(&artifacts.neomacs_font_result);
    let _ = std::fs::remove_file(&artifacts.font_oracle_diff);
    let _ = std::fs::remove_file(&artifacts.neomacs_log);

    let mut gnu_runner = ProcessGuiCommandRunner;
    let gnu_output = gnu_runner
        .run(
            &gnu_font_oracle_command(&gnu_emacs, &scenario.script, &artifacts, session.env()),
            &artifacts,
            &GuiRunOptions::with_timeout(Duration::from_secs(12)),
        )
        .expect("GNU Emacs font oracle should run");
    assert_eq!(
        gnu_output.exit_code,
        Some(0),
        "GNU Emacs oracle failed; stderr:\n{}",
        gnu_output.stderr
    );

    let mut plan = GuiTestPlan::new(backend, &workspace_root, &artifact_root, scenario)
        .with_program(binary)
        .with_env(
            "NEOMACS_LOG_FILE",
            artifacts.neomacs_log.display().to_string(),
        )
        .with_env(
            "RUST_LOG",
            "info,neomacs_renderer_wgpu::glyph_atlas=debug,neomacs::font_at=debug",
        );
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
    let gnu_result = std::fs::read_to_string(&artifacts.gnu_font_result)
        .expect("GNU Emacs oracle result should be readable");
    let neomacs_result = std::fs::read_to_string(&artifacts.neomacs_font_result)
        .expect("NEO Emacs oracle result should be readable");
    write_font_oracle_diff(&artifacts.font_oracle_diff, &gnu_result, &neomacs_result)
        .expect("font oracle diff artifact should be written");

    if neomacs_result != gnu_result {
        panic!(
            "font selection oracle result diverged; diff at {}\n\n--- GNU Emacs result ---\n{}\n--- NEO Emacs result ---\n{}",
            artifacts.font_oracle_diff.display(),
            gnu_result,
            neomacs_result
        );
    }
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

fn gnu_emacs_binary() -> PathBuf {
    std::env::var_os("NEOMACS_GUI_TEST_GNU_EMACS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/exec/.local/bin/emacs"))
}

fn gnu_font_oracle_command(
    program: &std::path::Path,
    script: &std::path::Path,
    artifacts: &neomacs_gui_tests::GuiArtifactSet,
    display_env: &[(String, String)],
) -> CommandSpec {
    let mut env = vec![
        (
            "NEOMACS_GUI_FONT_SELECTION_RESULT".to_string(),
            artifacts.gnu_font_result.display().to_string(),
        ),
        ("GDK_BACKEND".to_string(), "x11".to_string()),
    ];
    env.extend(display_env.iter().cloned());
    CommandSpec {
        program: program.to_path_buf(),
        args: vec![
            "-Q".to_string(),
            "-l".to_string(),
            script.display().to_string(),
        ],
        env,
    }
}

fn write_font_oracle_diff(
    path: &std::path::Path,
    gnu_result: &str,
    neomacs_result: &str,
) -> std::io::Result<()> {
    let status = if gnu_result == neomacs_result {
        "matched"
    } else {
        "diverged"
    };
    let diff = format!(
        "status: {status}\n\n--- GNU Emacs result ---\n{gnu_result}\n--- NEO Emacs result ---\n{neomacs_result}\n"
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, diff)
}
