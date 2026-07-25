use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use neomacs_melpa_tests::{
    EmacsRuntime, MelpaSandbox, PackageScenario, PackageSource, ScenarioPhase, run_scenario,
    workspace_root,
};

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
printf '%s\n' 'NEOMACS-MELPA-RESULT:(:package simple-single :value 42)'
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
        r##"(princ "NEOMACS-MELPA-RESULT:(:package simple-single :value 42)")"##,
    );

    let report = run_scenario(&runtime, &source, &scenario).expect("run fake scenario");

    let invocations = fs::read_to_string(invocation_log).expect("read runtime invocations");
    assert_eq!(invocations.lines().count(), 2);
    assert_eq!(report.phases.len(), 2);
    assert_eq!(report.phases[0].phase, ScenarioPhase::Install);
    assert_eq!(report.phases[1].phase, ScenarioPhase::RestartProbe);
    assert_eq!(
        report.result,
        "(:package simple-single :value 42)".to_string()
    );
    assert_eq!(report.installed_packages.len(), 1);
    assert_eq!(report.installed_packages[0].name, "simple-single");
    assert_eq!(report.installed_packages[0].version, "1.3");
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
    let scenario = PackageScenario::new(
        "timeout-contract",
        ["simple-single"],
        r##"(princ "NEOMACS-MELPA-RESULT:t")"##,
    );

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
    let scenario = PackageScenario::new(
        "error-marker-contract",
        ["simple-single"],
        r##"(princ "NEOMACS-MELPA-RESULT:t")"##,
    );

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
