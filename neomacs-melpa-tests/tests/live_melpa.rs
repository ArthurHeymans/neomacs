use std::time::Duration;

use neomacs_melpa_tests::{
    DASH_MELPA_PIN, EmacsRuntime, PackageScenario, prepare_shared_package_source,
    run_oracle_scenario, workspace_root,
};
use neomacs_test_oracle::EvalOutcome;

#[test]
#[ignore = "live network canary: installs current packages from GNU ELPA and MELPA"]
fn live_melpa_ecosystem_installs_and_survives_restart() {
    let packages = [
        DASH_MELPA_PIN,
        ("s", "20220902.1511"),
        ("hydra", "20250316.1254"),
        ("ivy", "20260413.2102"),
        ("flycheck", "20260725.1551"),
        ("projectile", "20260725.1657"),
    ];
    let package_names = packages.map(|(name, _version)| name);
    let behavior = PackageScenario::from_probe_file(
        "live-melpa-ecosystem",
        package_names,
        workspace_root().join("neomacs-melpa-tests/scenarios/live-melpa-ecosystem.el"),
    )
    .expect("load live MELPA behavior probe");
    let surface = PackageScenario::autoload_surface("live-melpa-surface", package_names);
    let scenario = PackageScenario::versioned(
        "live-melpa-ecosystem",
        packages,
        format!(
            r##"(let* ((surface (progn {}))
                        (behavior (progn {})))
                   (list :behavior behavior
                         :surface surface))"##,
            surface.probe, behavior.probe
        ),
    );
    let gnu = EmacsRuntime::gnu_emacs().with_timeout(Duration::from_secs(900));
    let source = prepare_shared_package_source(&gnu, &scenario)
        .expect("download one shared package transaction below ./tmp");
    let report = run_oracle_scenario(
        &EmacsRuntime::neomacs().with_timeout(Duration::from_secs(900)),
        &gnu,
        source.source(),
        &scenario,
    )
    .expect("current MELPA package behavior must match GNU Emacs");

    let EvalOutcome::Value(value) = &report.neomacs.outcome else {
        panic!(
            "expected a live surface value, got {}",
            report.neomacs.outcome
        );
    };
    assert!(
        value.contains("global-flycheck-mode"),
        "known MELPA custom missing from live surface: {value}"
    );
    eprintln!("{}", report.neomacs);
    eprintln!("{}", report.gnu_emacs);
}
