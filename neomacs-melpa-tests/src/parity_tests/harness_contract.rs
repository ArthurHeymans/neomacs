use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::source_lock::SHALLOW_GIT_FETCH_ARGS;
use crate::{
    EmacsRuntime, ErtScenario, MelpaSandbox, PackageScenario, PackageSource, ScenarioPhase,
    SourceBuild, locked_melpa_install_plan, locked_melpa_source, locked_melpa_sources,
    run_elisp_oracle, run_ert_scenario, run_oracle_scenario, run_scenario, workspace_root,
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

#[test]
fn nextest_runs_melpa_infrastructure_preflight_once_before_parity_tests() {
    let nextest = include_str!("../../../.config/nextest.toml");
    let preflight = include_str!("../../../scripts/melpa-infra-preflight.sh");

    assert!(nextest.contains(r#"experimental = ["wrapper-scripts", "setup-scripts"]"#));
    assert!(nextest.contains("[scripts.setup.melpa-infra-preflight]"));
    assert!(nextest.contains("scripts/melpa-infra-preflight.sh"));
    assert!(nextest.contains(
        "filter = 'package(neomacs-melpa-tests) and not test(~parity_tests::harness_contract::)'"
    ));
    assert!(nextest.contains("setup = 'melpa-infra-preflight'"));
    assert!(preflight.contains("NEXTEST_WORKSPACE_ROOT"));
    assert!(preflight.contains(r#"mktemp -d "$scratch_parent/preflight.XXXXXX""#));
    assert!(preflight.contains("resolve_executable Git git"));
    assert!(preflight.contains("NEOMACS-MELPA-PREFLIGHT:ready"));
    assert!(!preflight.contains("mktemp -d /tmp"));
    assert!(!preflight.contains("TMPDIR=/tmp"));
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

#[test]
fn exact_git_package_uses_upstream_with_an_emacsmirror_fallback() {
    let source = locked_melpa_source(("agent-shell", "20260728.953"))
        .expect("resolve the revision-pinned agent-shell source");

    assert_eq!(source.package(), ("agent-shell", "20260728.953"));
    assert_eq!(
        source.upstream_repository(),
        "https://github.com/xenodium/agent-shell"
    );
    assert_eq!(
        source.upstream_revision(),
        "a59891a9d8f1d26afb8358239346e081708cf2cb"
    );
    assert_eq!(
        source.repository(),
        "https://github.com/xenodium/agent-shell"
    );
    assert_eq!(
        source.revision(),
        "a59891a9d8f1d26afb8358239346e081708cf2cb"
    );
    assert_eq!(
        source.fallback_repository(),
        Some("https://github.com/emacsmirror/agent-shell")
    );
    assert_eq!(source.build(), SourceBuild::MelpaRecipe);

    let error = locked_melpa_source(("agent-shell", "20260724.1019"))
        .expect_err("an obsolete rolling pin must not resolve");
    assert!(error.contains("revision-pinned source lock"));
}

#[test]
fn non_git_upstream_is_acquired_from_an_exact_emacsmirror_git_revision() {
    let source = locked_melpa_source(("2048-game", "20230809.356"))
        .expect("resolve the mirrored 2048-game source");

    assert_eq!(
        source.upstream_repository(),
        "https://hg.sr.ht/~zck/game-2048"
    );
    assert_eq!(
        source.upstream_revision(),
        "8175ca5191175183b9522141dcb55d30673d2323"
    );
    assert_eq!(
        source.repository(),
        "https://github.com/emacsmirror/2048-game"
    );
    assert_eq!(
        source.revision(),
        "8976bb8875fc638806d0db5e0ba9c573f6ca7a25"
    );
    assert_eq!(source.fallback_repository(), None);
    assert_eq!(source.build(), SourceBuild::DefaultFiles);
}

#[test]
fn source_build_can_exclude_upstream_test_code_from_the_runtime_package() {
    let source = locked_melpa_source(("alectryon", "20260525.2000"))
        .expect("resolve the Alectryon runtime source");

    assert_eq!(source.build(), SourceBuild::Files("etc/elisp/alectryon.el"));
}

#[test]
fn exact_source_install_plan_orders_dependencies_before_the_main_package() {
    let plan = locked_melpa_install_plan(("arxiv-citation", "20230713.627"))
        .expect("resolve the source-locked arxiv-citation dependency closure");
    let packages = plan
        .into_iter()
        .map(|source| source.package())
        .collect::<Vec<_>>();

    assert_eq!(
        packages,
        [
            ("dash", "20260221.1346"),
            ("s", "20220902.1511"),
            ("arxiv-citation", "20230713.627"),
        ]
    );
}

#[test]
fn every_exact_package_has_a_complete_acyclic_source_plan() {
    let sources = locked_melpa_sources().expect("parse the source lock");
    assert_eq!(
        sources.len(),
        362,
        "every root package, exact dependency, and legacy all-ext dependency stays pinned"
    );

    for source in sources {
        let package = source.package();
        let plan = locked_melpa_install_plan(package)
            .unwrap_or_else(|error| panic!("resolve {} {}: {error}", package.0, package.1));
        assert_eq!(
            plan.last().map(|planned| planned.package()),
            Some(package),
            "the selected package must be installed after its dependencies"
        );
        assert!(!source.repository().contains("melpa.org"));
        assert!(!source.repository().contains("/releases/download/"));
        if let Some(fallback) = source.fallback_repository() {
            assert!(
                fallback.starts_with("https://github.com/emacsmirror/")
                    || fallback.starts_with("https://github.com/emacsattic/")
            );
            assert!(!fallback.contains("/releases/download/"));
        }
    }

    let all_ext_plan = locked_melpa_install_plan(("all-ext", "20200315.1443"))
        .expect("resolve the legacy source dependency");
    assert_eq!(
        all_ext_plan
            .into_iter()
            .map(|source| source.package())
            .collect::<Vec<_>>(),
        [("all", "1.0"), ("all-ext", "20200315.1443")]
    );
}

#[test]
fn git_source_acquisition_is_shallow_and_never_reads_a_package_catalog() {
    let source_harness = include_str!("../source_lock.rs");

    assert_eq!(SHALLOW_GIT_FETCH_ARGS, ["fetch", "--depth=1", "--no-tags"]);
    assert!(source_harness.contains("--is-shallow-repository"));
    assert!(!source_harness.contains("package-refresh-contents"));
    assert!(!source_harness.contains("package-archive-contents"));
    assert!(!source_harness.contains("url-copy-file"));
    assert!(!source_harness.contains("melpa.org/packages"));
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
