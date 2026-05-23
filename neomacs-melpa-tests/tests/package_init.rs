//! MELPA package activation tests.
//!
//! Three categories:
//! 1. Hand-crafted fixture tests (fast, offline)
//! 2. Real MELPA package download tests (needs network, cached)
//! 3. Byte-compilation tests

use neomacs_melpa_tests::*;

fn run_activation(home: &std::path::Path, pkg: &str) -> Result<String, String> {
    run_neomacs_ok(
        home,
        &format!(
            r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-initialize)
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

fn real_pkg_by_name(name: &str) -> MelpaPackage {
    real_melpa_packages()
        .into_iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("real package {name} not found"))
}

// ===========================================================================
// Hand-crafted fixture tests — individual package activation
// ===========================================================================

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

// ===========================================================================
// Hand-crafted fixture tests — post-activation usage
// ===========================================================================

#[test]
fn dash_usage_after_activation() {
    let f = &[fixture_by_name("dash")];
    let home = setup_isolated_home(f);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-initialize)
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
  (package-initialize)
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
  (package-initialize)
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
  (package-initialize)
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
  (package-initialize)
  (package-activate 'use-package)
  (unless (fboundp 'use-package)
    (error "use-package not defined after activation"))
  (message "USE-PACKAGE-OK"))))"#,
    )
    .expect("use-package should be callable after activation");
}

// ===========================================================================
// Hand-crafted fixture tests — package-initialize full batch
// ===========================================================================

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

// ===========================================================================
// Real MELPA package tests — activation
// ===========================================================================

#[test]
fn real_melpa_activate_dash() {
    let pkg = real_pkg_by_name("dash");
    let home = setup_real_melpa_home(&[pkg]);
    run_activation(home.path(), "dash").expect("real dash activation");
}

#[test]
fn real_melpa_activate_s() {
    let pkg = real_pkg_by_name("s");
    let home = setup_real_melpa_home(&[pkg]);
    run_activation(home.path(), "s").expect("real s activation");
}

#[test]
fn real_melpa_activate_which_key() {
    let pkg = real_pkg_by_name("which-key");
    let home = setup_real_melpa_home(&[pkg]);
    run_activation(home.path(), "which-key").expect("real which-key activation");
}

#[test]
fn real_melpa_activate_flycheck() {
    let pkg = real_pkg_by_name("flycheck");
    let home = setup_real_melpa_home(&[pkg]);
    run_activation(home.path(), "flycheck").expect("real flycheck activation");
}

#[test]
fn real_melpa_activate_projectile() {
    let pkg = real_pkg_by_name("projectile");
    let home = setup_real_melpa_home(&[pkg]);
    run_activation(home.path(), "projectile").expect("real projectile activation");
}

// ===========================================================================
// Real MELPA package tests — package-initialize with all packages
// ===========================================================================

#[test]
fn real_melpa_package_initialize_all_no_errors() {
    let packages = real_melpa_packages();
    let home = setup_real_melpa_home(&packages);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-initialize)
  (message "PACKAGE-INIT-OK"))"#,
    )
    .expect("real package-initialize should succeed without errors");
}

// ===========================================================================
// Real MELPA package tests — post-activation usage
// ===========================================================================

#[test]
fn real_melpa_dash_usage() {
    let pkg = real_pkg_by_name("dash");
    let home = setup_real_melpa_home(&[pkg]);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-initialize)
  (package-activate 'dash)
  (require 'dash)
  (let ((result (-map (lambda (n) (* n 2)) '(1 2 3 4))))
    (unless (equal result '(2 4 6 8))
      (error "dash -map failed: got %S" result)))
  (message "REAL-DASH-OK"))))"#,
    )
    .expect("real dash -map should work after activation");
}

#[test]
fn real_melpa_s_usage() {
    let pkg = real_pkg_by_name("s");
    let home = setup_real_melpa_home(&[pkg]);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-initialize)
  (package-activate 's)
  (require 's)
  (let ((result (s-trim-left "  hello")))
    (unless (string= result "hello")
      (error "s-trim-left failed: got %S" result)))
  (message "REAL-S-OK"))))"#,
    )
    .expect("real s-trim-left should work after activation");
}

#[test]
fn real_melpa_which_key_usage() {
    let pkg = real_pkg_by_name("which-key");
    let home = setup_real_melpa_home(&[pkg]);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-initialize)
  (package-activate 'which-key)
  (require 'which-key)
  (unless (fboundp 'which-key-mode)
    (error "which-key-mode not defined after activation"))
  (message "REAL-WHICH-KEY-OK"))))"#,
    )
    .expect("real which-key should be usable after activation");
}

#[test]
fn real_melpa_projectile_usage() {
    let pkg = real_pkg_by_name("projectile");
    let home = setup_real_melpa_home(&[pkg]);
    run_neomacs_ok(
        home.path(),
        r#"(progn
  (require 'package)
  (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
  (package-initialize)
  (package-activate 'projectile)
  (require 'projectile)
  (unless (fboundp 'projectile-mode)
    (error "projectile-mode not defined after activation"))
  (message "REAL-PROJECTILE-OK"))))"#,
    )
    .expect("real projectile should be usable after activation");
}

// ===========================================================================
// Byte-compilation tests — hand-crafted fixtures
// ===========================================================================

#[test]
fn byte_compile_dash_fixture() {
    let f = &[fixture_by_name("dash")];
    let home = setup_isolated_home(f);
    let elpa = home.path().join(".emacs.d").join("elpa");
    let el_file = elpa.join("dash-20240404.1234").join("dash.el");
    byte_compile_file(home.path(), &el_file).expect("byte-compile dash fixture");
}

#[test]
fn byte_compile_s_fixture() {
    let f = &[fixture_by_name("s")];
    let home = setup_isolated_home(f);
    let elpa = home.path().join(".emacs.d").join("elpa");
    let el_file = elpa.join("s-20220902.1511").join("s.el");
    byte_compile_file(home.path(), &el_file).expect("byte-compile s fixture");
}

#[test]
fn byte_compile_hydra_fixture() {
    let f = &[fixture_by_name("lv"), fixture_by_name("hydra")];
    let home = setup_isolated_home(f);
    let elpa = home.path().join(".emacs.d").join("elpa");
    let el_file = elpa.join("hydra-20220910.1206").join("hydra.el");
    byte_compile_file(home.path(), &el_file).expect("byte-compile hydra fixture");
}

// ===========================================================================
// Byte-compilation tests — real MELPA packages
// ===========================================================================

#[test]
fn byte_compile_real_dash() {
    let pkg = real_pkg_by_name("dash");
    let home = setup_real_melpa_home(&[pkg]);
    let elpa = home.path().join(".emacs.d").join("elpa");
    let el_file = elpa.join(format!("dash-{}", pkg.version)).join("dash.el");
    byte_compile_file(home.path(), &el_file).expect("byte-compile real dash");
}

#[test]
fn byte_compile_real_s() {
    let pkg = real_pkg_by_name("s");
    let home = setup_real_melpa_home(&[pkg]);
    let elpa = home.path().join(".emacs.d").join("elpa");
    let el_file = elpa.join(format!("s-{}", pkg.version)).join("s.el");
    byte_compile_file(home.path(), &el_file).expect("byte-compile real s");
}

#[test]
fn byte_compile_real_which_key() {
    let pkg = real_pkg_by_name("which-key");
    let home = setup_real_melpa_home(&[pkg]);
    let elpa = home.path().join(".emacs.d").join("elpa");
    let pkg_dir = elpa.join(format!("which-key-{}", pkg.version));
    for el_file in find_el_files(&pkg_dir) {
        byte_compile_file(home.path(), &el_file)
            .unwrap_or_else(|e| panic!("byte-compile {} failed:\n{e}", el_file.display()));
    }
}

#[test]
fn byte_compile_real_flycheck() {
    let pkg = real_pkg_by_name("flycheck");
    let home = setup_real_melpa_home(&[pkg]);
    let elpa = home.path().join(".emacs.d").join("elpa");
    let pkg_dir = elpa.join(format!("flycheck-{}", pkg.version));
    for el_file in find_el_files(&pkg_dir) {
        byte_compile_file(home.path(), &el_file)
            .unwrap_or_else(|e| panic!("byte-compile {} failed:\n{e}", el_file.display()));
    }
}

#[test]
fn byte_compile_real_projectile() {
    let pkg = real_pkg_by_name("projectile");
    let home = setup_real_melpa_home(&[pkg]);
    let elpa = home.path().join(".emacs.d").join("elpa");
    let pkg_dir = elpa.join(format!("projectile-{}", pkg.version));
    for el_file in find_el_files(&pkg_dir) {
        byte_compile_file(home.path(), &el_file)
            .unwrap_or_else(|e| panic!("byte-compile {} failed:\n{e}", el_file.display()));
    }
}
