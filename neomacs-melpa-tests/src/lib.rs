//! MELPA package activation test harness.
//!
//! Each test creates an isolated HOME with pre-populated ELPA package
//! fixtures, then launches NeoMacs in batch mode to exercise
//! `package-initialize` and verify zero errors.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A MELPA package fixture: the package name, version, dependencies,
/// and the elisp source files that go in its ELPA directory.
pub struct MelpaFixture {
    /// Package name as a symbol (e.g., "dash").
    pub name: &'static str,
    /// Package version (e.g., "20231025.1234").
    pub version: &'static str,
    /// List of dependencies, each a (name, version) pair.
    pub deps: &'static [(&'static str, &'static str)],
    /// Each tuple is (filename, content).
    pub files: &'static [(&'static str, &'static str)],
    /// Additional `require` calls needed to exercise this package.
    pub requires: &'static [&'static str],
}

// ---------------------------------------------------------------------------
// Famous MELPA packages — minimal fixtures with real autoload boilerplate.
// ---------------------------------------------------------------------------

// -- dash -------------------------------------------------------------------

const DASH_AUTOLOADS: &str = r#";;; dash-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

;;;### (autoloads nil "dash" "dash.el" (1 2 3 4 5 6 7 8 9 10 11 12))
;;; Generated autoloads from dash.el

(autoload 'dash-fontify-mode "dash" "Minor mode for fontifying dash." t nil)

(provide 'dash-autoloads)
;; Local Variables:
;; no-byte-compile: t
;; no-update-autoloads: t
;; End:
;;; dash-autoloads.el ends here
"#;

const DASH_PKG: &str = r#"(define-package "dash" "20240404.1234" "A modern list library for Emacs." '((emacs "24.1")))
"#;

const DASH_EL: &str = r#";;; dash.el --- A modern list library for Emacs.  -*- lexical-binding: t -*-

;; Copyright (C) 2012-2024 Magnar Sveen

;; Package-Requires: ((emacs "24.1"))

(provide 'dash)
;;; dash.el ends here
"#;

// -- s ---------------------------------------------------------------------

const S_AUTOLOADS: &str = r#";;; s-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

(autoload 's-trim-left "s" nil nil nil)
(autoload 's-trim-right "s" nil nil nil)
(autoload 's-split "s" nil nil nil)

(provide 's-autoloads)
;; Local Variables:
;; no-byte-compile: t
;; no-update-autoloads: t
;; End:
;;; s-autoloads.el ends here
"#;

const S_PKG: &str = r#"(define-package "s" "20220902.1511" "The long-lost Emacs string manipulation library." '((emacs "24.1")))
"#;

// -- use-package -----------------------------------------------------------

const USE_PACKAGE_AUTOLOADS: &str = r#";;; use-package-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

(autoload 'use-package "use-package" nil nil t)
(autoload 'use-package-autoloads "use-package" nil nil t)

(provide 'use-package-autoloads)
;; Local Variables:
;; no-byte-compile: t
;; no-update-autoloads: t
;; End:
;;; use-package-autoloads.el ends here
"#;

const USE_PACKAGE_PKG: &str = r#"(define-package "use-package" "20230426.2320" "A configuration macro." '((emacs "24.3") (bind-key "20230203.2007")))
"#;

const BIND_KEY_AUTOLOADS: &str = r#";;; bind-key-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

(autoload 'bind-key "bind-key" nil nil t)

(provide 'bind-key-autoloads)
;; Local Variables:
;; no-byte-compile: t
;; no-update-autoloads: t
;; End:
;;; bind-key-autoloads.el ends here
"#;

const BIND_KEY_PKG: &str = r#"(define-package "bind-key" "20230203.2007" "A simple way to manage personal keybindings." '((emacs "24.3")))
"#;

// -- hydra -----------------------------------------------------------------

const HYDRA_AUTOLOADS: &str = r#";;; hydra-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

(autoload 'defhydra "hydra" nil nil t)
(autoload 'hydra-default-pre "hydra" nil nil nil)

(provide 'hydra-autoloads)
;; Local Variables:
;; no-byte-compile: t
;; no-update-autoloads: t
;; End:
;;; hydra-autoloads.el ends here
"#;

const HYDRA_PKG: &str = r#"(define-package "hydra" "20220910.1206" "Make bindings that stick around." '((emacs "24.4") (lv "20200507.1518")))
"#;

const LV_AUTOLOADS: &str = r#";;; lv-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

(autoload 'lv-message "lv" nil nil nil)
(autoload 'lv-delete-window "lv" nil nil nil)

(provide 'lv-autoloads)
;; Local Variables:
;; no-byte-compile: t
;; no-update-autoloads: t
;; End:
;;; lv-autoloads.el ends here
"#;

const LV_PKG: &str = r#"(define-package "lv" "20200507.1518" "Other echo area." '((emacs "24.4")))
"#;

// -- ivy -------------------------------------------------------------------

const IVY_AUTOLOADS: &str = r#";;; ivy-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

(autoload 'ivy-read "ivy" nil nil nil)
(autoload 'ivy-mode "ivy" nil nil nil)
(autoload 'ivy-completing-read "ivy" nil nil nil)

(provide 'ivy-autoloads)
;; Local Variables:
;; no-byte-compile: t
;; no-update-autoloads: t
;; End:
;;; ivy-autoloads.el ends here
"#;

const IVY_PKG: &str = r#"(define-package "ivy" "20231025.2311" "Incremental Vertical completYon." '((emacs "24.5")))
"#;

// ===========================================================================
// Fixture list
// ===========================================================================

/// All packages to install into the isolated HOME for testing.
pub fn famous_packages() -> Vec<MelpaFixture> {
    vec![
        MelpaFixture {
            name: "dash",
            version: "20240404.1234",
            deps: &[],
            files: &[
                ("dash-pkg.el", DASH_PKG),
                ("dash-autoloads.el", DASH_AUTOLOADS),
                ("dash.el", DASH_EL),
            ],
            requires: &[],
        },
        MelpaFixture {
            name: "s",
            version: "20220902.1511",
            deps: &[],
            files: &[("s-pkg.el", S_PKG), ("s-autoloads.el", S_AUTOLOADS)],
            requires: &[],
        },
        MelpaFixture {
            name: "bind-key",
            version: "20230203.2007",
            deps: &[],
            files: &[
                ("bind-key-pkg.el", BIND_KEY_PKG),
                ("bind-key-autoloads.el", BIND_KEY_AUTOLOADS),
            ],
            requires: &[],
        },
        MelpaFixture {
            name: "use-package",
            version: "20230426.2320",
            deps: &[("bind-key", "20230203.2007")],
            files: &[
                ("use-package-pkg.el", USE_PACKAGE_PKG),
                ("use-package-autoloads.el", USE_PACKAGE_AUTOLOADS),
            ],
            requires: &[],
        },
        MelpaFixture {
            name: "lv",
            version: "20200507.1518",
            deps: &[],
            files: &[("lv-pkg.el", LV_PKG), ("lv-autoloads.el", LV_AUTOLOADS)],
            requires: &[],
        },
        MelpaFixture {
            name: "hydra",
            version: "20220910.1206",
            deps: &[("lv", "20200507.1518")],
            files: &[
                ("hydra-pkg.el", HYDRA_PKG),
                ("hydra-autoloads.el", HYDRA_AUTOLOADS),
            ],
            requires: &[],
        },
        MelpaFixture {
            name: "ivy",
            version: "20231025.2311",
            deps: &[],
            files: &[("ivy-pkg.el", IVY_PKG), ("ivy-autoloads.el", IVY_AUTOLOADS)],
            requires: &[],
        },
    ]
}

// ===========================================================================
// Test harness
// ===========================================================================

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// The path to the `neomacs` binary.
pub fn neomacs_binary() -> PathBuf {
    std::env::var("NEOMACS_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            workspace_root()
                .join("target")
                .join("release")
                .join("neomacs")
        })
}

/// Create an isolated `$HOME` directory and populate its
/// `~/.emacs.d/elpa/` with the given MELPA fixture packages.
pub fn setup_isolated_home(fixtures: &[MelpaFixture]) -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("create isolated HOME");
    let elpa = home.path().join(".emacs.d").join("elpa");
    std::fs::create_dir_all(&elpa).expect("create elpa dir");

    for pkg in fixtures {
        let pkg_dir = elpa.join(format!("{}-{}", pkg.name, pkg.version));
        std::fs::create_dir_all(&pkg_dir).expect("create pkg dir");
        for (filename, content) in pkg.files {
            std::fs::write(pkg_dir.join(filename), content).expect("write pkg file");
        }
    }

    home
}

/// Run NeoMacs in batch mode with the given HOME and Elisp forms.
pub fn run_neomacs(home: &Path, elisp: &str) -> std::process::Output {
    Command::new(neomacs_binary())
        .env("HOME", home)
        .env("NEOMACS_RUNTIME_ROOT", workspace_root())
        .args(["--batch", "--eval", elisp])
        .output()
        .expect("run neomacs")
}

/// Run NeoMacs and check that it exits successfully with no errors.
pub fn run_neomacs_ok(home: &Path, elisp: &str) -> Result<String, String> {
    let output = run_neomacs(home, elisp);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    for needle in &[
        "wrong-type-argument",
        "void-function",
        "file-missing",
        "invalid-read-syntax",
        "end-of-file",
        "error:",
    ] {
        if stdout.contains(needle) || stderr.contains(needle) {
            return Err(format!(
                "neomacs emitted `{needle}`:\nstdout:\n{stdout}\nstderr:\n{stderr}"
            ));
        }
    }
    if !output.status.success() {
        return Err(format!(
            "neomacs exit status {}:\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status
        ));
    }
    Ok(stdout)
}
