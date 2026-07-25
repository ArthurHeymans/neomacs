use std::time::Duration;

use neomacs_melpa_tests::{
    EmacsRuntime, PackageScenario, PackageSource, run_scenario, workspace_root,
};

#[test]
#[ignore = "live network canary: installs current packages from GNU ELPA and MELPA"]
fn live_melpa_ecosystem_installs_and_survives_restart() {
    let scenario = PackageScenario::from_probe_file(
        "live-melpa-ecosystem",
        ["dash", "s", "hydra", "ivy", "flycheck", "projectile"],
        workspace_root().join("neomacs-melpa-tests/scenarios/live-melpa-ecosystem.el"),
    )
    .expect("load live MELPA probe");
    let report = run_scenario(
        &EmacsRuntime::neomacs().with_timeout(Duration::from_secs(900)),
        &PackageSource::live_melpa(),
        &scenario,
    )
    .expect("live MELPA ecosystem scenario");

    assert_eq!(
        report.result,
        "(:live-packages t :dependencies t :autoloads t :macros t :minor-mode t :restart t)"
    );
    eprintln!("{report}");
}
