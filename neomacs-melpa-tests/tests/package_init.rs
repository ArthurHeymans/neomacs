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
  (package-load-all-descriptors)
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
// Post-activation usage — actually call package functions
// ---------------------------------------------------------------------------

#[test]
fn dash_usage_after_activation() {
    let f = &[fixture_by_name("dash")];
    let home = setup_isolated_home(f);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-load-all-descriptors)
  (package-activate 'dash)
  (require 'dash)
  (let ((result (-map (lambda (n) (* n 2)) '(1 2 3 4))))
    (unless (equal result '(2 4 6 8))
      (error "dash -map failed: got %S" result)))
  (with-current-buffer "*Messages*"
    (goto-char (point-min))
    (when (search-forward "Error" nil t)
      (error "Error found in *Messages* after dash test")))
  (message "DASH-OK"))))"#,
    )
    .expect("dash -map should work after activation");
}

#[test]
fn s_usage_after_activation() {
    let f = &[fixture_by_name("s")];
    let home = setup_isolated_home(f);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-load-all-descriptors)
  (package-activate 's)
  (require 's)
  (let ((result (s-trim-left "  hello")))
    (unless (string= result "hello")
      (error "s-trim-left failed: got %S" result)))
  (with-current-buffer "*Messages*"
    (goto-char (point-min))
    (when (search-forward "Error" nil t)
      (error "Error found in *Messages* after s test")))
  (message "S-OK"))))"#,
    )
    .expect("s-trim-left should work after activation");
}

#[test]
fn hydra_usage_after_activation() {
    let f = &[fixture_by_name("lv"), fixture_by_name("hydra")];
    let home = setup_isolated_home(f);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-load-all-descriptors)
  (package-activate 'hydra)
  (require 'hydra)
  (defhydra hydra-test (:exit t) "test" ("a" (message "hi")))
  (unless (fboundp 'hydra-test/body)
    (error "defhydra did not create hydra-test/body"))
  (with-current-buffer "*Messages*"
    (goto-char (point-min))
    (when (search-forward "Error" nil t)
      (error "Error found in *Messages* after hydra test")))
  (message "HYDRA-OK"))))"#,
    )
    .expect("defhydra should work after activation");
}

#[test]
fn bind_key_usage_after_activation() {
    let f = &[fixture_by_name("bind-key")];
    let home = setup_isolated_home(f);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-load-all-descriptors)
  (package-activate 'bind-key)
  (bind-key "C-c z" (lambda () (interactive) (message "z")))
  (unless (fboundp 'bind-key)
    (error "bind-key not defined after activation"))
  (message "BIND-KEY-OK"))))"#,
    )
    .expect("bind-key should work after activation");
}

#[test]
fn use_package_usage_after_activation() {
    let f = &[fixture_by_name("bind-key"), fixture_by_name("use-package")];
    let home = setup_isolated_home(f);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-load-all-descriptors)
  (package-activate 'use-package)
  (unless (fboundp 'use-package)
    (error "use-package not defined after activation"))
  (message "USE-PACKAGE-OK"))))"#,
    )
    .expect("use-package should be callable after activation");
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
