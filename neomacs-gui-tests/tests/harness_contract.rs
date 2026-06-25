use std::path::PathBuf;
use std::time::Duration;

use neomacs_gui_tests::{
    DisplayHarness, GuiArtifactSet, GuiBackend, GuiCommandOutput, GuiCommandRunner, GuiRunOptions,
    GuiRunStatus, GuiScenario, GuiTestPlan, RunnerKind,
};

#[test]
fn all_supported_backends_have_explicit_runner_kind() {
    let cases = [
        (GuiBackend::LinuxX11, RunnerKind::Xvfb),
        (GuiBackend::LinuxWayland, RunnerKind::WestonHeadless),
        (GuiBackend::Macos, RunnerKind::CurrentDesktopSession),
        (GuiBackend::Windows, RunnerKind::CurrentDesktopSession),
    ];

    for (backend, runner) in cases {
        assert_eq!(backend.runner_kind(), runner);
    }
}

#[test]
fn artifact_paths_are_backend_and_scenario_qualified() {
    let artifacts = GuiArtifactSet::new(
        PathBuf::from("target/neomacs-gui-tests"),
        GuiBackend::LinuxWayland,
        "startup-smoke",
    );

    assert_eq!(
        artifacts.json,
        PathBuf::from("target/neomacs-gui-tests/linux-wayland/startup-smoke.json")
    );
    assert_eq!(
        artifacts.png,
        PathBuf::from("target/neomacs-gui-tests/linux-wayland/startup-smoke.png")
    );
    assert_eq!(
        artifacts.stderr,
        PathBuf::from("target/neomacs-gui-tests/linux-wayland/startup-smoke.stderr.log")
    );
    assert_eq!(
        artifacts.gui_state,
        PathBuf::from("target/neomacs-gui-tests/linux-wayland/startup-smoke.gui-state.json")
    );
}

#[test]
fn linux_x11_plan_sets_backend_and_readback_environment() {
    let scenario = GuiScenario::new("startup-smoke", "test/neomacs/neomacs-face-test.el");
    let plan = GuiTestPlan::new(
        GuiBackend::LinuxX11,
        PathBuf::from("/repo"),
        PathBuf::from("/repo/target/neomacs-gui-tests"),
        scenario,
    );
    let command = plan.command_spec();

    assert_eq!(
        command.program,
        PathBuf::from("/repo/target/release/neomacs")
    );
    assert!(command.args.contains(&"-Q".into()));
    assert!(command.args.contains(&"-l".into()));
    assert!(
        command
            .args
            .contains(&"test/neomacs/neomacs-face-test.el".into())
    );
    assert_eq!(command.env_value("WINIT_UNIX_BACKEND"), Some("x11"));
    assert_eq!(
        command.env_value("NEOMACS_DEBUG_FIRST_FRAME_READBACK"),
        Some("1")
    );
    assert_eq!(
        command.env_value("NEOMACS_DEBUG_SURFACE_READBACK"),
        Some("1")
    );
    assert_eq!(
        command.env_value("NEOMACS_DEBUG_SURFACE_READBACK_PNG"),
        Some("/repo/target/neomacs-gui-tests/linux-x11/startup-smoke.png")
    );
}

#[test]
fn display_harness_reports_missing_linux_display_inputs() {
    let x11 = DisplayHarness::for_backend(GuiBackend::LinuxX11);
    let wayland = DisplayHarness::for_backend(GuiBackend::LinuxWayland);

    assert_eq!(x11.required_env(), &["DISPLAY"]);
    assert_eq!(
        wayland.required_env(),
        &["XDG_RUNTIME_DIR", "WAYLAND_DISPLAY"]
    );
}

#[test]
fn test_plan_injects_display_session_environment() {
    let scenario = GuiScenario::new("startup-smoke", "test/neomacs/neomacs-face-test.el");
    let plan = GuiTestPlan::new(
        GuiBackend::LinuxWayland,
        PathBuf::from("/repo"),
        PathBuf::from("/repo/target/neomacs-gui-tests"),
        scenario,
    )
    .with_env("XDG_RUNTIME_DIR", "/tmp/neomacs-wayland")
    .with_env("WAYLAND_DISPLAY", "neomacs-gui-tests");
    let command = plan.command_spec();

    assert_eq!(command.env_value("WINIT_UNIX_BACKEND"), Some("wayland"));
    assert_eq!(
        command.env_value("XDG_RUNTIME_DIR"),
        Some("/tmp/neomacs-wayland")
    );
    assert_eq!(
        command.env_value("WAYLAND_DISPLAY"),
        Some("neomacs-gui-tests")
    );
}

#[test]
fn test_plan_can_override_neomacs_binary_path() {
    let scenario = GuiScenario::new("startup-smoke", "test/neomacs/neomacs-face-test.el");
    let plan = GuiTestPlan::new(
        GuiBackend::LinuxWayland,
        PathBuf::from("/repo"),
        PathBuf::from("/repo/target/neomacs-gui-tests"),
        scenario,
    )
    .with_program("/repo/target/release/neomacs");

    assert_eq!(
        plan.command_spec().program,
        PathBuf::from("/repo/target/release/neomacs")
    );
}

#[test]
fn test_plan_materializes_json_manifest_artifact() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("test crate should live below workspace root")
        .to_path_buf();
    let artifact_root = workspace_root.join("target/neomacs-gui-tests");
    let artifacts = GuiArtifactSet::new(&artifact_root, GuiBackend::LinuxWayland, "startup-smoke");
    let _ = std::fs::remove_file(&artifacts.json);

    let scenario = GuiScenario::new("startup-smoke", "test/neomacs/neomacs-face-test.el");
    let plan = GuiTestPlan::new(
        GuiBackend::LinuxWayland,
        &workspace_root,
        &artifact_root,
        scenario,
    );

    let written = plan.write_manifest().expect("manifest should be written");
    let manifest = std::fs::read_to_string(&written.json).expect("manifest should be readable");

    assert_eq!(written.json, artifacts.json);
    assert!(!written.png.exists());
    assert!(manifest.contains(r##""status":"planned""##));
    assert!(manifest.contains(r##""backend":"linux-wayland""##));
    assert!(manifest.contains(r##""runner":"weston-headless""##));
    assert!(manifest.contains(&format!(
        r##""program":"{}""##,
        workspace_root.join("target/release/neomacs").display()
    )));
    assert!(manifest.contains(r##""expected_artifacts":"##));
    assert!(manifest.contains(&format!(r##""png":"{}""##, artifacts.png.display())));
}

#[test]
fn run_with_runner_writes_ai_readable_result_artifacts() {
    let workspace_root = workspace_root();
    let artifact_root = workspace_root.join("target/neomacs-gui-tests");
    let artifacts = GuiArtifactSet::new(&artifact_root, GuiBackend::LinuxWayland, "runner-success");
    let _ = std::fs::remove_file(&artifacts.json);
    let _ = std::fs::remove_file(&artifacts.png);
    let _ = std::fs::remove_file(&artifacts.stderr);

    let plan = GuiTestPlan::new(
        GuiBackend::LinuxWayland,
        &workspace_root,
        &artifact_root,
        GuiScenario::new("runner-success", "test/neomacs/neomacs-face-test.el"),
    );
    let mut runner = FakeRunner {
        output: GuiCommandOutput {
            exit_code: Some(0),
            timed_out: false,
            stdout: "ready\n".to_string(),
            stderr: "Debug surface readback: bottom_band_avg=(1.0, 2.0, 3.0, 4.0)\n".to_string(),
        },
        create_png: true,
        gui_state: None,
    };

    let result = plan
        .run_with(
            &mut runner,
            GuiRunOptions::with_timeout(Duration::from_secs(1)),
        )
        .expect("runner result should be written");
    let manifest = std::fs::read_to_string(&artifacts.json).expect("result json should exist");
    let stderr = std::fs::read_to_string(&artifacts.stderr).expect("stderr log should exist");

    assert_eq!(result.status, GuiRunStatus::Passed);
    assert_eq!(result.png_bytes, Some(7));
    assert_eq!(result.stderr_bytes, stderr.len() as u64);
    assert!(manifest.contains(r##""status":"passed""##));
    assert!(manifest.contains(r##""png_exists":true"##));
    assert!(manifest.contains(r##""readback_diagnostics":["Debug surface readback:"##));
    assert!(stderr.contains("bottom_band_avg"));
}

#[test]
fn run_with_runner_reports_missing_png_as_failed_text_result() {
    let workspace_root = workspace_root();
    let artifact_root = workspace_root.join("target/neomacs-gui-tests");
    let artifacts = GuiArtifactSet::new(&artifact_root, GuiBackend::LinuxWayland, "runner-no-png");
    let _ = std::fs::remove_file(&artifacts.json);
    let _ = std::fs::remove_file(&artifacts.png);
    let _ = std::fs::remove_file(&artifacts.stderr);

    let plan = GuiTestPlan::new(
        GuiBackend::LinuxWayland,
        &workspace_root,
        &artifact_root,
        GuiScenario::new("runner-no-png", "test/neomacs/neomacs-face-test.el"),
    );
    let mut runner = FakeRunner {
        output: GuiCommandOutput {
            exit_code: Some(0),
            timed_out: false,
            stdout: String::new(),
            stderr: "renderer exited without readback\n".to_string(),
        },
        create_png: false,
        gui_state: None,
    };

    let result = plan
        .run_with(
            &mut runner,
            GuiRunOptions::with_timeout(Duration::from_secs(1)),
        )
        .expect("runner result should be written");
    let manifest = std::fs::read_to_string(&artifacts.json).expect("result json should exist");

    assert_eq!(result.status, GuiRunStatus::Failed);
    assert_eq!(result.png_bytes, None);
    assert!(manifest.contains(r##""status":"failed""##));
    assert!(manifest.contains(r##""png_exists":false"##));
    assert!(manifest.contains("PNG artifact was not generated"));
}

#[test]
fn run_with_runner_treats_timeout_after_png_as_successful_capture() {
    let workspace_root = workspace_root();
    let artifact_root = workspace_root.join("target/neomacs-gui-tests");
    let artifacts = GuiArtifactSet::new(
        &artifact_root,
        GuiBackend::LinuxWayland,
        "runner-timeout-png",
    );
    let _ = std::fs::remove_file(&artifacts.json);
    let _ = std::fs::remove_file(&artifacts.png);
    let _ = std::fs::remove_file(&artifacts.stderr);

    let plan = GuiTestPlan::new(
        GuiBackend::LinuxWayland,
        &workspace_root,
        &artifact_root,
        GuiScenario::new("runner-timeout-png", "test/neomacs/neomacs-face-test.el"),
    );
    let mut runner = FakeRunner {
        output: GuiCommandOutput {
            exit_code: None,
            timed_out: true,
            stdout: String::new(),
            stderr: "First-frame surface readback: ok\n".to_string(),
        },
        create_png: true,
        gui_state: None,
    };

    let result = plan
        .run_with(
            &mut runner,
            GuiRunOptions::with_timeout(Duration::from_secs(1)),
        )
        .expect("runner result should be written");
    let manifest = std::fs::read_to_string(&artifacts.json).expect("result json should exist");

    assert_eq!(result.status, GuiRunStatus::Passed);
    assert!(result.timed_out);
    assert!(manifest.contains(r##""status":"passed""##));
    assert!(manifest.contains(r##""timed_out":true"##));
}

#[test]
fn run_with_runner_includes_fixture_visible_text_snapshot() {
    let workspace_root = workspace_root();
    let artifact_root = workspace_root.join("target/neomacs-gui-tests");
    let artifacts =
        GuiArtifactSet::new(&artifact_root, GuiBackend::LinuxWayland, "runner-gui-state");
    let _ = std::fs::remove_file(&artifacts.json);
    let _ = std::fs::remove_file(&artifacts.png);
    let _ = std::fs::remove_file(&artifacts.stderr);
    let _ = std::fs::remove_file(&artifacts.gui_state);

    let plan = GuiTestPlan::new(
        GuiBackend::LinuxWayland,
        &workspace_root,
        &artifact_root,
        GuiScenario::new("runner-gui-state", "test/neomacs/neomacs-face-test.el"),
    );
    let mut runner = FakeRunner {
        output: GuiCommandOutput {
            exit_code: Some(0),
            timed_out: false,
            stdout: String::new(),
            stderr: "First-frame surface readback: ok\n".to_string(),
        },
        create_png: true,
        gui_state: Some(r##"{"buffer_name":"*neomacs-gui-smoke*","visible_text":"NeoMacs GUI smoke line 00\nNeoMacs GUI smoke line 01\n"}"##.to_string()),
    };

    let result = plan
        .run_with(
            &mut runner,
            GuiRunOptions::with_timeout(Duration::from_secs(1)),
        )
        .expect("runner result should be written");
    let manifest = std::fs::read_to_string(&artifacts.json).expect("result json should exist");

    assert!(result.gui_state_bytes.unwrap_or_default() > 0);
    assert!(manifest.contains(r##""gui_state":"##));
    assert!(manifest.contains(r##""buffer_name":"*neomacs-gui-smoke*""##));
    assert!(manifest.contains("NeoMacs GUI smoke line 00"));
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("test crate should live below workspace root")
        .to_path_buf()
}

struct FakeRunner {
    output: GuiCommandOutput,
    create_png: bool,
    gui_state: Option<String>,
}

impl GuiCommandRunner for FakeRunner {
    fn run(
        &mut self,
        _command: &neomacs_gui_tests::CommandSpec,
        artifacts: &GuiArtifactSet,
        _options: &GuiRunOptions,
    ) -> std::io::Result<GuiCommandOutput> {
        if self.create_png {
            std::fs::create_dir_all(artifacts.png.parent().expect("png should have parent"))?;
            std::fs::write(&artifacts.png, b"not png")?;
        }
        if let Some(gui_state) = &self.gui_state {
            std::fs::create_dir_all(
                artifacts
                    .gui_state
                    .parent()
                    .expect("gui state should have parent"),
            )?;
            std::fs::write(&artifacts.gui_state, gui_state)?;
        }
        Ok(self.output.clone())
    }
}
