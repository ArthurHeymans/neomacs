//! MELPA package activation tests.
//!
//! Each test installs a set of famous MELPA packages into an isolated
//! HOME directory and runs `package-activate` or `package-initialize`
//! in batch mode, verifying that no errors occur.

use neomacs_melpa_tests::*;

fn run_activation(home: &std::path::Path, pkg: &str) -> Result<String, String> {
    run_neomacs_ok(
        home,
        &format!(
            r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-activate '{pkg})
  (message "ACTIVATED-{pkg}"))"#,
            pkg = pkg
        ),
    )
}

fn fixture_by_name(name: &str) -> MelpaFixture {
    famous_packages()
        .into_iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("fixture {name} not found"))
}

// ---------------------------------------------------------------------------
// Individual package activation
// ---------------------------------------------------------------------------

#[test]
fn activate_dash_standalone() {
    let f = &[fixture_by_name("dash")];
    let home = setup_isolated_home(f);
    run_activation(home.path(), "dash").expect("dash activation");
}

#[test]
fn activate_s_standalone() {
    let f = &[fixture_by_name("s")];
    let home = setup_isolated_home(f);
    run_activation(home.path(), "s").expect("s activation (#$ boilerplate)");
}

#[test]
fn activate_hydra_with_lv_dep() {
    let f = &[fixture_by_name("lv"), fixture_by_name("hydra")];
    let home = setup_isolated_home(f);
    run_activation(home.path(), "hydra").expect("hydra + lv activation");
}

#[test]
fn activate_use_package_with_bind_key_dep() {
    let f = &[fixture_by_name("bind-key"), fixture_by_name("use-package")];
    let home = setup_isolated_home(f);
    run_activation(home.path(), "use-package").expect("use-package + bind-key activation");
}

#[test]
fn activate_ivy_standalone() {
    let f = &[fixture_by_name("ivy")];
    let home = setup_isolated_home(f);
    run_activation(home.path(), "ivy").expect("ivy activation");
}

// ---------------------------------------------------------------------------
// package-initialize — full batch activation
// ---------------------------------------------------------------------------

#[test]
fn package_initialize_all_famous_packages_no_errors() {
    let fixtures = famous_packages();
    let home = setup_isolated_home(&fixtures);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-initialize)
  (message "PACKAGE-INIT-OK"))"#,
    )
    .expect("package-initialize should succeed without errors");
}
