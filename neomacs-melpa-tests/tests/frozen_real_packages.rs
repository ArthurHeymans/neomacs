use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use neomacs_melpa_tests::{
    EmacsRuntime, PackageScenario, PackageSource, run_scenario, workspace_root,
};
use sha2::{Digest, Sha256};

const PINNED_TARBALLS: [(&str, &str); 4] = [
    (
        "dash-20260221.1346.tar",
        "24255e35ea71a6f753b7e7b8507d8aefde146bae7634688d5e1f6bc97c7cbce0",
    ),
    (
        "hydra-20250316.1254.tar",
        "63682d55f9c77933aab5fc1ad7b023c933aec9b0772e994d52eee78131451a64",
    ),
    (
        "lv-20200507.1518.tar",
        "066037b788cacb2c557f8c5451de66cc0beb0b190969981012a3e436fe814ed9",
    ),
    (
        "s-20220902.1511.tar",
        "55da7f0728ad8f3388a34f49e729aaf493e829084429acd853c14eded24b4f06",
    ),
];

fn archive_root() -> PathBuf {
    workspace_root().join("neomacs-melpa-tests/fixtures/frozen-melpa")
}

fn scenario() -> PackageScenario {
    PackageScenario::from_probe_file(
        "frozen-real-packages",
        ["dash", "s", "hydra"],
        workspace_root().join("neomacs-melpa-tests/scenarios/frozen-real-packages.el"),
    )
    .expect("load frozen real-package probe")
}

fn sha256(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| {
        panic!("failed to read pinned package {}: {error}", path.display())
    });
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[test]
fn frozen_real_archive_matches_pinned_checksums() {
    for (file, expected) in PINNED_TARBALLS {
        assert_eq!(sha256(&archive_root().join(file)), expected, "{file}");
    }
}

#[test]
fn pinned_real_packages_match_gnu_emacs_after_restart() {
    let source = PackageSource::frozen(archive_root());
    let scenario = scenario();
    let neomacs = run_scenario(
        &EmacsRuntime::neomacs().with_timeout(Duration::from_secs(180)),
        &source,
        &scenario,
    )
    .expect("run pinned real packages with Neomacs");
    let gnu = run_scenario(
        &EmacsRuntime::gnu_emacs().with_timeout(Duration::from_secs(180)),
        &source,
        &scenario,
    )
    .expect("run pinned real packages with GNU Emacs");

    assert_eq!(
        neomacs.result, gnu.result,
        "Neomacs and GNU Emacs produced different real-package results"
    );
    assert_eq!(
        neomacs.installed_packages, gnu.installed_packages,
        "Neomacs and GNU Emacs installed different real-package/version graphs"
    );
    eprintln!("{neomacs}");
    eprintln!("{gnu}");
}
