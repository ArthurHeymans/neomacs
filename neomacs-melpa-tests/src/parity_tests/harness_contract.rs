use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::{
    EmacsRuntime, ErtScenario, MelpaSandbox, PackageScenario, PackageSource, ScenarioPhase,
    prepare_cached_melpa_package, prepare_shared_package_source, run_elisp_oracle,
    run_ert_scenario, run_oracle_scenario, run_scenario, workspace_root,
};
use neomacs_test_oracle::EvalOutcome;

#[test]
fn sandbox_keeps_process_state_under_workspace_tmp() {
    let sandbox = MelpaSandbox::new("environment-contract").expect("create MELPA sandbox");
    let scratch_base = workspace_root().join("tmp/melpa");

    assert!(sandbox.root().starts_with(&scratch_base));
    assert!(sandbox.home().starts_with(sandbox.root()));
    assert!(sandbox.tmp_dir().starts_with(sandbox.root()));
    assert!(sandbox.home().is_dir());
    assert!(sandbox.tmp_dir().is_dir());

    #[cfg(unix)]
    {
        let mut command = Command::new("sh");
        command.args([
            "-c",
            r##"printf '%s\n' "$HOME" "$TMPDIR" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$PWD" "$USER" "$LOGNAME" "$HOSTNAME" "$EMAIL" "$TZ" "$LC_ALL" "$TERM""##,
        ]);
        sandbox.configure(&mut command);
        let output = command.output().expect("inspect sandbox environment");
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).expect("environment output is UTF-8");
        let paths = stdout
            .lines()
            .take(7)
            .map(Path::new)
            .map(Path::to_path_buf)
            .collect::<Vec<_>>();

        assert_eq!(paths[0], sandbox.home());
        assert_eq!(paths[1], sandbox.tmp_dir());
        assert!(
            paths[2..6]
                .iter()
                .all(|path| path.starts_with(sandbox.root()))
        );
        assert_eq!(paths[6], sandbox.root());

        let values = stdout.lines().collect::<Vec<_>>();
        assert_eq!(
            &values[7..],
            [
                "melpa-test",
                "melpa-test",
                "melpa-host",
                "melpa-test@melpa-host",
                "UTC",
                "C.UTF-8",
                "dumb",
            ]
        );
    }
}

#[cfg(unix)]
#[test]
fn scenario_installs_then_probes_in_a_fresh_process() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = MelpaSandbox::new("runtime-contract").expect("create fixture sandbox");
    let invocation_log = fixture.root().join("invocations");
    let runtime_script = fixture.root().join("fake-emacs");
    fs::write(
        &runtime_script,
        format!(
            r##"#!/bin/sh
printf '%s\n' invoke >> '{}'
printf 'NEOMACS-MELPA-INSTALLED:simple-single\t1.3\n'
printf '%s\n' 'NEOMACS-MELPA-OUTCOME:OK (:package simple-single :value 42)'
"##,
            invocation_log.display()
        ),
    )
    .expect("write fake runtime");
    fs::set_permissions(&runtime_script, fs::Permissions::from_mode(0o755))
        .expect("make fake runtime executable");

    let runtime = EmacsRuntime::new("fake", runtime_script);
    let source =
        PackageSource::frozen(workspace_root().join("test/lisp/emacs-lisp/package-resources"));
    let scenario = PackageScenario::new(
        "two-process-contract",
        ["simple-single"],
        r##"'(:package simple-single :value 42)"##,
    );

    let report = run_scenario(&runtime, &source, &scenario).expect("run fake scenario");

    let invocations = fs::read_to_string(invocation_log).expect("read runtime invocations");
    assert_eq!(invocations.lines().count(), 2);
    assert_eq!(report.phases.len(), 2);
    assert_eq!(report.phases[0].phase, ScenarioPhase::Install);
    assert_eq!(report.phases[1].phase, ScenarioPhase::RestartProbe);
    assert_eq!(
        report.outcome,
        EvalOutcome::Value("(:package simple-single :value 42)".to_string())
    );
    assert_eq!(report.installed_packages.len(), 1);
    assert_eq!(report.installed_packages[0].name, "simple-single");
    assert_eq!(report.installed_packages[0].version, "1.3");
}

#[cfg(unix)]
#[test]
fn oracle_scenario_compares_matching_lisp_signals() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = MelpaSandbox::new("oracle-signal-contract").expect("create fixture sandbox");
    let runtime_script = fixture.root().join("signal-emacs");
    fs::write(
        &runtime_script,
        r##"#!/bin/sh
printf 'NEOMACS-MELPA-INSTALLED:simple-single\t1.3\n'
case "$*" in
  *NEOMACS-MELPA-OUTCOME*)
    printf '%s\n' 'NEOMACS-MELPA-OUTCOME:ERR (wrong-type-argument numberp "x")'
    ;;
esac
"##,
    )
    .expect("write signal runtime");
    fs::set_permissions(&runtime_script, fs::Permissions::from_mode(0o755))
        .expect("make signal runtime executable");

    let runtime = EmacsRuntime::new("fake", runtime_script);
    let source =
        PackageSource::frozen(workspace_root().join("test/lisp/emacs-lisp/package-resources"));
    let scenario = PackageScenario::new("signal-parity", ["simple-single"], "(+ 1 \"x\")");

    let report = run_oracle_scenario(&runtime, &runtime, &source, &scenario)
        .expect("matching signals have oracle parity");

    assert_eq!(
        report.neomacs.outcome,
        EvalOutcome::Signal(r##"(wrong-type-argument numberp "x")"##.to_string())
    );
    assert_eq!(report.neomacs.outcome, report.gnu_emacs.outcome);
}

#[cfg(unix)]
#[test]
fn oracle_scenario_reports_a_value_divergence() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = MelpaSandbox::new("oracle-divergence-contract").expect("create fixture sandbox");
    let neo_script = fixture.root().join("neo-emacs");
    let gnu_script = fixture.root().join("gnu-emacs");
    for (script, value) in [(&neo_script, "42"), (&gnu_script, "43")] {
        fs::write(
            script,
            format!(
                r##"#!/bin/sh
printf 'NEOMACS-MELPA-INSTALLED:simple-single\t1.3\n'
printf '%s\n' 'NEOMACS-MELPA-OUTCOME:OK {value}'
"##
            ),
        )
        .expect("write divergent runtime");
        fs::set_permissions(script, fs::Permissions::from_mode(0o755))
            .expect("make divergent runtime executable");
    }

    let source =
        PackageSource::frozen(workspace_root().join("test/lisp/emacs-lisp/package-resources"));
    let scenario = PackageScenario::new("value-divergence", ["simple-single"], "42");
    let error = run_oracle_scenario(
        &EmacsRuntime::new("neomacs", neo_script),
        &EmacsRuntime::new("gnu-emacs", gnu_script),
        &source,
        &scenario,
    )
    .expect_err("different values must fail oracle parity");

    assert!(error.contains("value-divergence"));
    assert!(error.contains("OK 42"));
    assert!(error.contains("OK 43"));
}

#[cfg(unix)]
#[test]
fn direct_elisp_oracle_runs_one_form_without_a_package_install() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = MelpaSandbox::new("direct-oracle-contract").expect("create fixture sandbox");
    let runtime_script = fixture.root().join("direct-emacs");
    fs::write(
        &runtime_script,
        r##"#!/bin/sh
case "$*" in
  *dash-sentinel*)
    printf '%s\n' 'NEOMACS-MELPA-OUTCOME:OK (:dash direct)'
    ;;
  *)
    exit 9
    ;;
esac
"##,
    )
    .expect("write direct runtime");
    fs::set_permissions(&runtime_script, fs::Permissions::from_mode(0o755))
        .expect("make direct runtime executable");

    let runtime = EmacsRuntime::new("fake", runtime_script);
    let report = run_elisp_oracle(&runtime, &runtime, "direct-dash-form", "", "'dash-sentinel")
        .expect("run direct differential form");

    assert_eq!(
        report.neomacs,
        EvalOutcome::Value("(:dash direct)".to_string())
    );
    assert_eq!(report.neomacs, report.gnu_emacs);
}

#[cfg(unix)]
#[test]
fn concurrent_cached_melpa_package_preparation_downloads_once_below_workspace_tmp() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = MelpaSandbox::new("cached-package-contract").expect("create fixture sandbox");
    let invocation_log = fixture.root().join("cache-invocations");
    let runtime_script = fixture.root().join("cache-emacs");
    let cache_nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let version = format!("0.0.{}-{cache_nonce}", std::process::id());
    fs::write(
        &runtime_script,
        format!(
            r##"#!/bin/sh
printf '%s\n' invoke >> '{}'
sleep 1
package_dir="$HOME/.emacs.d/elpa/neomacs-cache-contract-$CACHE_CONTRACT_VERSION"
mkdir -p "$package_dir"
: > "$package_dir/neomacs-cache-contract-pkg.el"
printf '%s\n' 'NEOMACS-PACKAGE-CACHE:ready'
"##,
            invocation_log.display()
        ),
    )
    .expect("write cache runtime");
    fs::set_permissions(&runtime_script, fs::Permissions::from_mode(0o755))
        .expect("make cache runtime executable");
    let runtime = EmacsRuntime::new("fake-cache", runtime_script)
        .with_env("CACHE_CONTRACT_VERSION", &version);

    let barrier = std::sync::Barrier::new(3);
    let package = ("neomacs-cache-contract", version.as_str());
    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            barrier.wait();
            prepare_cached_melpa_package(&runtime, package)
                .expect("prepare cached package concurrently")
        });
        let second = scope.spawn(|| {
            barrier.wait();
            prepare_cached_melpa_package(&runtime, package)
                .expect("reuse concurrently prepared cached package")
        });
        barrier.wait();
        (
            first.join().expect("join first cache caller"),
            second.join().expect("join second cache caller"),
        )
    });

    assert_eq!(first, second);
    assert!(first.starts_with(workspace_root().join("tmp/melpa/package-cache")));
    assert!(!first.starts_with(Path::new("/tmp")));
    let invocations = fs::read_to_string(invocation_log).expect("read cache invocations");
    assert_eq!(invocations.lines().count(), 1);
}

#[test]
fn shared_package_source_requires_a_hard_coded_version_per_package() {
    let scenario = PackageScenario::new("unversioned-package", ["dash"], "t");
    let error = prepare_shared_package_source(
        &EmacsRuntime::new("must-not-run", "missing-emacs"),
        &scenario,
    )
    .err()
    .expect("an unversioned package must be rejected");

    assert!(error.contains("hard-code exactly one version for `dash`"));
}

#[cfg(unix)]
#[test]
fn scenario_timeout_identifies_the_stalled_phase() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = MelpaSandbox::new("timeout-contract").expect("create fixture sandbox");
    let runtime_script = fixture.root().join("slow-emacs");
    fs::write(
        &runtime_script,
        r##"#!/bin/sh
exec sleep 5
"##,
    )
    .expect("write deliberately slow runtime");
    fs::set_permissions(&runtime_script, fs::Permissions::from_mode(0o755))
        .expect("make slow runtime executable");

    let runtime = EmacsRuntime::new("slow", runtime_script).with_timeout(Duration::from_millis(50));
    let source =
        PackageSource::frozen(workspace_root().join("test/lisp/emacs-lisp/package-resources"));
    let scenario = PackageScenario::new("timeout-contract", ["simple-single"], "t");

    let error = run_scenario(&runtime, &source, &scenario).expect_err("scenario must time out");
    assert!(
        error.contains("Install"),
        "error did not name phase: {error}"
    );
    assert!(
        error.contains("timed out"),
        "error did not name cause: {error}"
    );
}

#[cfg(unix)]
#[test]
fn scenario_error_markers_identify_runtime_scenario_and_phase() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = MelpaSandbox::new("error-marker-contract").expect("create fixture sandbox");
    let runtime_script = fixture.root().join("error-emacs");
    fs::write(
        &runtime_script,
        r##"#!/bin/sh
printf '%s\n' 'Error: deliberate package failure'
printf 'NEOMACS-MELPA-INSTALLED:simple-single\t1.3\n'
"##,
    )
    .expect("write failing fake runtime");
    fs::set_permissions(&runtime_script, fs::Permissions::from_mode(0o755))
        .expect("make failing runtime executable");

    let runtime = EmacsRuntime::new("error-runtime", runtime_script);
    let source =
        PackageSource::frozen(workspace_root().join("test/lisp/emacs-lisp/package-resources"));
    let scenario = PackageScenario::new("error-marker-contract", ["simple-single"], "t");

    let error = run_scenario(&runtime, &source, &scenario).expect_err("scenario must fail");
    for expected in [
        "error-runtime",
        "error-marker-contract",
        "Install",
        "Error:",
    ] {
        assert!(
            error.contains(expected),
            "error did not contain `{expected}`: {error}"
        );
    }
}

#[cfg(unix)]
#[test]
fn ert_scenario_forwards_the_test_file_and_selector() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = MelpaSandbox::new("ert-runtime-contract").expect("create fixture sandbox");
    let invocation_log = fixture.root().join("ert-invocation");
    let runtime_script = fixture.root().join("fake-ert-emacs");
    fs::write(
        &runtime_script,
        format!(
            r##"#!/bin/sh
printf '%s\n' "$*" > '{}'
printf '%s\n' 'Ran 3 tests, 3 results as expected, 0 unexpected, 1 skipped' >&2
"##,
            invocation_log.display()
        ),
    )
    .expect("write fake ERT runtime");
    fs::set_permissions(&runtime_script, fs::Permissions::from_mode(0o755))
        .expect("make fake ERT runtime executable");

    let test_file = workspace_root().join("test/lisp/emacs-lisp/package-tests.el");
    let scenario = ErtScenario::new(
        "upstream-install-contract",
        &test_file,
        r##"'(member package-test-install-single package-test-install-file)"##,
    );
    let report = run_ert_scenario(&EmacsRuntime::new("fake-ert", runtime_script), &scenario)
        .expect("run fake ERT scenario");

    let invocation = fs::read_to_string(invocation_log).expect("read ERT invocation");
    assert!(invocation.contains(&format!("-l {}", test_file.display())));
    assert!(invocation.contains("ert-run-tests-batch"));
    assert!(invocation.contains("package-test-install-single"));
    assert_eq!(report.phase.phase, ScenarioPhase::Ert);
    assert_eq!(report.summary.total, 3);
    assert_eq!(report.summary.expected, 3);
    assert_eq!(report.summary.unexpected, 0);
    assert_eq!(report.summary.skipped, 1);
}
