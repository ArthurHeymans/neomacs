use std::fs;
use std::num::{NonZeroU32, NonZeroU64};
use std::path::PathBuf;
use std::process::Command;

use super::{
    Frontend, NativeProfiler, PerfCallGraph, PerfCapture, PerfCaptureConfiguration, PerfHarness,
    PerfSamplingEvent, ProfileArtifact, ProfileRejection, ProfileRequest, ProfileVerdict,
    RunVerdict, ScenarioId, perf_data_sample_count,
};

#[test]
fn captured_profile_artifact_links_raw_data_report_and_scenario_run_without_timings() {
    let artifact = ProfileArtifact {
        schema_version: ProfileArtifact::SCHEMA_VERSION,
        profile_id: "rust-lsp-typing-profile-42".to_string(),
        scenario: ScenarioId::RustLspTyping,
        frontend: Frontend::Tui {
            rows: 40,
            columns: 120,
        },
        editor: PathBuf::from("target/profiling/neomacs"),
        iterations: NonZeroU32::new(40).expect("non-zero literal"),
        profiler: NativeProfiler::Perf,
        configuration: PerfCaptureConfiguration {
            event: PerfSamplingEvent::UserCpuClock,
            frequency_hz: NonZeroU32::new(999).expect("non-zero literal"),
            call_graph: PerfCallGraph::Dwarf {
                stack_size_bytes: NonZeroU32::new(16_384).expect("non-zero literal"),
            },
        },
        run_artifact_path: PathBuf::from("artifact.json"),
        verdict: ProfileVerdict::Captured {
            perf_data_path: PathBuf::from("perf.data"),
            hotspot_report_path: PathBuf::from("perf-report.txt"),
            sample_count: NonZeroU64::new(8_192).expect("non-zero literal"),
        },
    };

    let json = serde_json::to_string_pretty(&artifact).expect("serialize profile artifact");
    let decoded: ProfileArtifact =
        serde_json::from_str(&json).expect("deserialize profile artifact");

    assert_eq!(decoded, artifact);
    assert!(json.contains(r##""event": "user-cpu-clock""##));
    assert!(json.contains(r##""perf_data_path": "perf.data""##));
    assert!(!json.contains("measurements"));
}

#[test]
fn malformed_perf_data_is_rejected_by_the_binary_parser() {
    let manifest_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = tempfile::Builder::new()
        .prefix("neomacs-perf-malformed-data-")
        .tempdir_in(manifest_directory.join("../tmp"))
        .expect("create workspace-local profile scratch directory");
    let perf_data = scratch.path().join("perf.data");
    fs::write(&perf_data, b"not perf data").expect("write malformed profile");

    let error = perf_data_sample_count(&perf_data).expect_err("malformed profile must fail");
    assert!(error.contains("failed to parse native profile"));
}

#[test]
fn unavailable_profile_target_persists_a_rejected_diagnostic_artifact() {
    let manifest_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_directory
        .parent()
        .expect("crate is a workspace member");
    let scratch = tempfile::Builder::new()
        .prefix("neomacs-perf-profile-rejection-")
        .tempdir_in(workspace_root.join("tmp"))
        .expect("create workspace-local profile scratch directory");
    let request = ProfileRequest::new(
        ScenarioId::RustLspTyping,
        scratch.path().join("missing-neomacs"),
        NonZeroU32::new(3).expect("non-zero literal"),
        NativeProfiler::Perf,
    );

    let report = PerfHarness::new(scratch.path())
        .profile(&request)
        .expect("persist rejected profile");

    assert_eq!(
        report.artifact.verdict,
        ProfileVerdict::Rejected {
            reason: ProfileRejection::InfrastructureFailure {
                message: format!(
                    "missing editor executable {}",
                    scratch.path().join("missing-neomacs").display()
                ),
            },
        }
    );
    assert!(matches!(
        report.run.artifact.verdict,
        RunVerdict::InfrastructureFailure { .. }
    ));
    assert!(report.artifact_path.ends_with("profile.json"));
    assert!(report.run.artifact_path.ends_with("artifact.json"));
    assert!(
        report
            .artifact_path
            .starts_with(scratch.path().join("tmp/perf-profiles"))
    );
}

#[test]
fn native_perf_support_is_compile_time_gated_to_linux() {
    assert_eq!(
        NativeProfiler::Perf.platform_rejection().is_none(),
        cfg!(target_os = "linux")
    );
}

#[test]
fn gui_capture_profiles_only_the_app_via_the_frontend_hook() {
    let manifest_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = tempfile::Builder::new()
        .prefix("neomacs-perf-gui-profile-command-")
        .tempdir_in(manifest_directory.join("../tmp"))
        .expect("create workspace-local profile scratch directory");
    let capture = PerfCapture::new(scratch.path(), PerfCaptureConfiguration::standard());
    let command = capture.wrap(
        Command::new("tools/bench/gui-run.sh"),
        Frontend::Gui {
            width: 1200,
            height: 800,
        },
    );

    assert_eq!(command.get_program(), "tools/bench/gui-run.sh");
    let environment = command
        .get_envs()
        .filter_map(|(name, value)| Some((name.to_str()?, value?.to_str()?)))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(environment.get("GUI_PERF_EVENT"), Some(&"cpu-clock:u"));
    assert_eq!(environment.get("GUI_PERF_FREQUENCY"), Some(&"999"));
    assert_eq!(environment.get("GUI_PERF_CALL_GRAPH"), Some(&"dwarf,16384"));
    assert!(
        PathBuf::from(
            environment
                .get("GUI_PERF_RECORD")
                .expect("GUI capture path")
        )
        .ends_with("perf.data")
    );
}

#[test]
fn tui_capture_profiles_only_the_app_inside_the_private_pty() {
    let manifest_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = tempfile::Builder::new()
        .prefix("neomacs-perf-tui-profile-command-")
        .tempdir_in(manifest_directory.join("../tmp"))
        .expect("create workspace-local profile scratch directory");
    let capture = PerfCapture::new(scratch.path(), PerfCaptureConfiguration::standard());
    let command = capture.wrap(
        Command::new("python3"),
        Frontend::Tui {
            rows: 40,
            columns: 120,
        },
    );

    assert_eq!(command.get_program(), "python3");
    let environment = command
        .get_envs()
        .filter_map(|(name, value)| Some((name.to_str()?, value?.to_str()?)))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(environment.get("PTY_PERF_EVENT"), Some(&"cpu-clock:u"));
    assert_eq!(environment.get("PTY_PERF_FREQUENCY"), Some(&"999"));
    assert_eq!(environment.get("PTY_PERF_CALL_GRAPH"), Some(&"dwarf,16384"));
    assert!(
        PathBuf::from(
            environment
                .get("PTY_PERF_RECORD")
                .expect("PTY capture path")
        )
        .ends_with("perf.data")
    );
}
