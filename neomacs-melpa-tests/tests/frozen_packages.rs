use std::time::Duration;

use neomacs_melpa_tests::{
    EmacsRuntime, PackageScenario, PackageSource, run_scenario, workspace_root,
};

fn frozen_source() -> PackageSource {
    PackageSource::frozen(workspace_root().join("test/lisp/emacs-lisp/package-resources"))
}

fn frozen_scenario() -> PackageScenario {
    PackageScenario::from_probe_file(
        "frozen-package-contract",
        ["simple-two-depend", "multi-file"],
        workspace_root().join("neomacs-melpa-tests/scenarios/frozen-package-contract.el"),
    )
    .expect("load frozen package probe")
}

fn neomacs_runtime() -> EmacsRuntime {
    EmacsRuntime::neomacs().with_timeout(Duration::from_secs(120))
}

#[test]
fn frozen_package_contract_survives_a_neomacs_restart() {
    let report = run_scenario(&neomacs_runtime(), &frozen_source(), &frozen_scenario())
        .expect("Neomacs must install and restart with frozen packages");

    assert_eq!(
        report.result,
        "(:dependency-chain t :multi-file t :autoloads t :restart t)"
    );
    eprintln!("{report}");
}

#[test]
fn frozen_package_contract_matches_gnu_emacs() {
    let scenario = frozen_scenario();
    let source = frozen_source();
    let neomacs = run_scenario(&neomacs_runtime(), &source, &scenario)
        .expect("run frozen scenario with Neomacs");
    let gnu = run_scenario(
        &EmacsRuntime::gnu_emacs().with_timeout(Duration::from_secs(120)),
        &source,
        &scenario,
    )
    .expect("run frozen scenario with GNU Emacs");

    assert_eq!(
        neomacs.result, gnu.result,
        "Neomacs and GNU Emacs produced different normalized scenario results"
    );
    assert_eq!(
        neomacs.installed_packages, gnu.installed_packages,
        "Neomacs and GNU Emacs installed different package/version graphs"
    );
    eprintln!("{neomacs}");
    eprintln!("{gnu}");
}
