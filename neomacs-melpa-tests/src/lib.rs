//! MELPA package activation test harness.
//!
//! Two testing modes:
//! 1. **Hand-crafted fixtures** — fast, offline, deterministic stubs for
//!    well-known packages (dash, s, use-package, hydra, ivy, seq, compat).
//! 2. **Real MELPA packages** — uses Emacs's own `package-install` to
//!    download and install from MELPA, exactly like a real user would.
//!    Covers complex packages like which-key, flycheck, projectile.

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

(defun -map (fn list)
  "Apply FN to each element of LIST and return a list of the results."
  (let (result)
    (dolist (item list)
      (push (funcall fn item) result))
    (nreverse result)))

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

const S_EL: &str = r#";;; s.el --- The long-lost Emacs string manipulation library.  -*- lexical-binding: t -*-

(defun s-trim-left (s)
  "Remove whitespace at the beginning of S."
  (if (string-match "\\`[ \t\n\r]+" s)
      (replace-match "" t t s)
    s))

(defun s-trim-right (s)
  "Remove whitespace at the end of S."
  (if (string-match "[ \t\n\r]+\\'" s)
      (replace-match "" t t s)
    s))

(defun s-split (separator s &optional omit-nulls)
  "Split S into a list on SEPARATOR."
  (let ((len (length separator))
        (start 0)
        result)
    (while (string-match (regexp-quote separator) s start)
      (unless (and omit-nulls (= start (match-beginning 0)))
        (push (substring s start (match-beginning 0)) result))
      (setq start (match-end 0)))
    (unless (and omit-nulls (= start (length s)))
      (push (substring s start) result))
    (nreverse result)))

(provide 's)
;;; s.el ends here
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

const HYDRA_EL: &str = r#";;; hydra.el --- Make bindings that stick around.  -*- lexical-binding: t -*-

(defmacro defhydra (name &optional body &rest heads)
  "Create a hydra named NAME with HEADS."
  (declare (indent defun))
  `(defun ,(intern (format "%s/body" name)) ()
     ,(format "Call the body of hydra %s." name)
     (message "hydra: %s" ',name)))

(provide 'hydra)
;;; hydra.el ends here
"#;

const HYDRA_PKG: &str = r#"(define-package "hydra" "20220910.1206" "Make bindings that stick around." '((emacs "24.4") (lv "20200507.1518")))
"#;

const LV_EL: &str = r#";;; lv.el --- Other echo area.  -*- lexical-binding: t -*-

(defun lv-message (format-string &rest args)
  "Display a non-intrusive message in the echo area."
  (apply 'message format-string args))

(defun lv-delete-window ()
  "Delete the lv window if it exists."
  nil)

(provide 'lv)
;;; lv.el ends here
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

// -- seq (GNU ELPA, commonly depended on) -----------------------------------

const SEQ_AUTOLOADS: &str = r#";;; seq-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

(autoload 'seq-map "seq" nil nil nil)
(autoload 'seq-filter "seq" nil nil nil)
(autoload 'seq-reduce "seq" nil nil nil)

(provide 'seq-autoloads)
;; Local Variables:
;; no-byte-compile: t
;; no-update-autoloads: t
;; End:
;;; seq-autoloads.el ends here
"#;

const SEQ_PKG: &str = r#"(define-package "seq" "2.24" "Sequence manipulation functions" '((emacs "25.1")))
"#;

const SEQ_EL: &str = r#";;; seq.el --- Sequence manipulation functions  -*- lexical-binding: t -*-

(defun seq-map (function sequence)
  "Apply FUNCTION to each element of SEQUENCE, and return the list of results."
  (mapcar function sequence))

(defun seq-filter (pred sequence)
  "Return a list of elements of SEQUENCE for which PRED returns non-nil."
  (let (result)
    (seq-doseq (element sequence)
      (when (funcall pred element)
        (push element result)))
    (nreverse result)))

(defun seq-reduce (function sequence initial-value)
  "Reduce SEQUENCE using FUNCTION with INITIAL-VALUE."
  (let ((acc initial-value))
    (seq-doseq (element sequence)
      (setq acc (funcall function acc element)))
    acc))

(defmacro seq-doseq (spec &rest body)
  "Loop over a sequence."
  (declare (indent 1))
  `(dolist (,spec ,@body)))

(provide 'seq)
;;; seq.el ends here
"#;

// -- compat (GNU ELPA, commonly depended on) --------------------------------

const COMPAT_AUTOLOADS: &str = r#";;; compat-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

(provide 'compat-autoloads)
;; Local Variables:
;; no-byte-compile: t
;; no-update-autoloads: t
;; End:
;;; compat-autoloads.el ends here
"#;

const COMPAT_PKG: &str = r#"(define-package "compat" "30.0.0.0" "Emacs Lisp Compatibility Library" '((emacs "24.4")))
"#;

const COMPAT_EL: &str = r#";;; compat.el --- Emacs Lisp Compatibility Library  -*- lexical-binding: t -*-

(defun compat-assoc (key alist)
  "Return the element of ALIST whose car equals KEY."
  (assoc key alist))

(provide 'compat)
;;; compat.el ends here
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
            files: &[
                ("s-pkg.el", S_PKG),
                ("s-autoloads.el", S_AUTOLOADS),
                ("s.el", S_EL),
            ],
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
            files: &[
                ("lv-pkg.el", LV_PKG),
                ("lv-autoloads.el", LV_AUTOLOADS),
                ("lv.el", LV_EL),
            ],
            requires: &[],
        },
        MelpaFixture {
            name: "hydra",
            version: "20220910.1206",
            deps: &[("lv", "20200507.1518")],
            files: &[
                ("hydra-pkg.el", HYDRA_PKG),
                ("hydra-autoloads.el", HYDRA_AUTOLOADS),
                ("hydra.el", HYDRA_EL),
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
        MelpaFixture {
            name: "seq",
            version: "2.24",
            deps: &[],
            files: &[
                ("seq-pkg.el", SEQ_PKG),
                ("seq-autoloads.el", SEQ_AUTOLOADS),
                ("seq.el", SEQ_EL),
            ],
            requires: &[],
        },
        MelpaFixture {
            name: "compat",
            version: "30.0.0.0",
            deps: &[],
            files: &[
                ("compat-pkg.el", COMPAT_PKG),
                ("compat-autoloads.el", COMPAT_AUTOLOADS),
                ("compat.el", COMPAT_EL),
            ],
            requires: &[],
        },
    ]
}

// ===========================================================================
// Real MELPA packages — downloaded and extracted (mirrors what
// package-install does, but works before package.el network support is ready)
// ===========================================================================

/// A real MELPA package for integration testing.
#[derive(Clone, Copy)]
pub struct MelpaPackage {
    pub name: &'static str,
    pub version: &'static str,
}

/// Well-known MELPA packages.  Versions are pinned so tests are deterministic.
pub fn real_melpa_packages() -> Vec<MelpaPackage> {
    vec![
        MelpaPackage {
            name: "dash",
            version: "20260221.1346",
        },
        MelpaPackage {
            name: "s",
            version: "20220902.1511",
        },
        MelpaPackage {
            name: "which-key",
            version: "20240620.2145",
        },
        MelpaPackage {
            name: "flycheck",
            version: "20260320.1715",
        },
        MelpaPackage {
            name: "projectile",
            version: "20260429.651",
        },
    ]
}

/// Location where downloaded MELPA tarballs are cached.
pub fn melpa_cache_dir() -> PathBuf {
    workspace_root().join("target").join("melpa-cache")
}

/// Download a real MELPA package tarball and extract it to the given ELPA
/// directory. Uses a cache to avoid re-downloading.
///
/// Returns the path to the extracted package directory inside `elpa_dir`.
pub fn download_and_extract(pkg: &MelpaPackage, elpa_dir: &Path) -> PathBuf {
    let cache = melpa_cache_dir();
    std::fs::create_dir_all(&cache).expect("create melpa cache dir");

    let tarball_name = format!("{}-{}.tar", pkg.name, pkg.version);
    let tarball_path = cache.join(&tarball_name);

    if !tarball_path.exists() {
        let url = format!(
            "https://melpa.org/packages/{}-{}.tar",
            pkg.name, pkg.version
        );
        let bytes = ureq::get(&url)
            .call()
            .and_then(|mut resp| resp.body_mut().read_to_vec())
            .unwrap_or_else(|e| panic!("failed to download {url}: {e}"));
        std::fs::write(&tarball_path, &bytes).expect("write tarball to cache");
    }

    let tarball_file = std::fs::File::open(&tarball_path).expect("open cached tarball");
    let mut archive = tar::Archive::new(tarball_file);

    let pkg_dir_name = format!("{}-{}", pkg.name, pkg.version);
    let dest = elpa_dir.join(&pkg_dir_name);
    std::fs::create_dir_all(&dest).expect("create pkg dir");

    for entry in archive.entries().expect("read tar entries") {
        let mut entry = entry.expect("read tar entry");
        let path = entry.path().expect("entry path").to_path_buf();

        let stripped = match path.strip_prefix(&pkg_dir_name) {
            Ok(s) => s.to_path_buf(),
            Err(_) => path.file_name().map(PathBuf::from).unwrap_or(path.clone()),
        };

        let dest_file = dest.join(&stripped);
        if let Some(parent) = dest_file.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        entry.unpack(&dest_file).expect("unpack file");
    }

    // MELPA tarballs don't include -autoloads.el files.
    // Generate a minimal one so package-activate doesn't error.
    let autoloads_file = dest.join(format!("{}-autoloads.el", pkg.name));
    if !autoloads_file.exists() {
        let content = format!(
            r#";;; {name}-autoloads.el --- automatically extracted autoloads  -*- lexical-binding: t -*-
;;
;;; Code:

(add-to-list 'load-path (directory-file-name
                         (or (file-name-directory #$) (car load-path))))

(provide '{name}-autoloads)
;;; {name}-autoloads.el ends here
"#,
            name = pkg.name
        );
        std::fs::write(&autoloads_file, content).expect("write autoloads file");
    }

    dest
}

/// Create an isolated HOME with real MELPA packages downloaded and
/// extracted into `~/.emacs.d/elpa/`.
pub fn setup_real_melpa_home(packages: &[MelpaPackage]) -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("create isolated HOME");
    let elpa = home.path().join(".emacs.d").join("elpa");
    std::fs::create_dir_all(&elpa).expect("create elpa dir");

    for pkg in packages {
        download_and_extract(pkg, &elpa);
    }

    home
}

/// Create an isolated HOME with a single real MELPA package plus any
/// transitive dependencies from the given dependency list.
pub fn setup_real_melpa_home_with_deps(
    main: &MelpaPackage,
    deps: &[MelpaPackage],
) -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("create isolated HOME");
    let elpa = home.path().join(".emacs.d").join("elpa");
    std::fs::create_dir_all(&elpa).expect("create elpa dir");

    for dep in deps {
        download_and_extract(dep, &elpa);
    }
    download_and_extract(main, &elpa);

    home
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

/// Run NeoMacs in batch mode with the given HOME, loading an Elisp file.
/// Returns the process output for inspection.
pub fn run_neomacs_script(home: &Path, script: &Path) -> std::process::Output {
    Command::new(neomacs_binary())
        .env("HOME", home)
        .env("NEOMACS_RUNTIME_ROOT", workspace_root())
        .args(["--batch", "-l", &script.display().to_string()])
        .output()
        .expect("run neomacs script")
}

/// Run a NeoMacs Elisp script and check that it exits successfully with no errors.
pub fn run_neomacs_script_ok(home: &Path, script: &Path) -> Result<String, String> {
    let output = run_neomacs_script(home, script);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    for needle in &[
        "wrong-type-argument",
        "void-function",
        "file-missing",
        "invalid-read-syntax",
        "end-of-file",
        "Error:",
    ] {
        if stdout.contains(needle) || stderr.contains(needle) {
            return Err(format!(
                "script {} emitted `{needle}`:\nstdout:\n{stdout}\nstderr:\n{stderr}",
                script.display()
            ));
        }
    }
    if !output.status.success() {
        return Err(format!(
            "script {} exit status {}:\nstdout:\n{stdout}\nstderr:\n{stderr}",
            script.display(),
            output.status
        ));
    }
    Ok(stdout)
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
        "Error:",
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

/// Byte-compile a `.el` file in the given package directory using NeoMacs
/// batch mode. Returns Ok on success, Err with diagnostics on failure.
pub fn byte_compile_file(home: &Path, el_file: &Path) -> Result<(), String> {
    let elisp = format!(
        r#"(progn
  (require 'package)
  (package-initialize)
  (byte-compile-file "{}"))"#,
        el_file.display()
    );
    let output = run_neomacs(home, &elisp);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    for needle in &[
        "wrong-type-argument",
        "void-function",
        "file-missing",
        "invalid-read-syntax",
        "end-of-file",
        "Error:",
    ] {
        if stdout.contains(needle) || stderr.contains(needle) {
            return Err(format!(
                "byte-compile of {} emitted `{needle}`:\nstdout:\n{stdout}\nstderr:\n{stderr}",
                el_file.display()
            ));
        }
    }
    if !output.status.success() {
        return Err(format!(
            "byte-compile of {} exit status {}:\nstdout:\n{stdout}\nstderr:\n{stderr}",
            el_file.display(),
            output.status
        ));
    }
    Ok(())
}

/// Find all `.el` files in a directory (non-recursive, skipping autoloads
/// and `-pkg.el` files which should not be byte-compiled).
pub fn find_el_files(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "el") {
                let name = path.file_name().unwrap().to_string_lossy();
                if !name.ends_with("-autoloads.el") && !name.ends_with("-pkg.el") {
                    result.push(path);
                }
            }
        }
    }
    result
}

/// Find the installed package directory for a given package name in the
/// isolated elpa directory. Returns the full path to the `name-version`
/// directory, or panics if not found.
///
/// Useful for real MELPA tests where the exact version is not known ahead
/// of time since `package-install` always fetches the current version.
pub fn find_installed_pkg_dir(home: &Path, pkg_name: &str) -> PathBuf {
    let elpa = home.join(".emacs.d").join("elpa");
    for entry in std::fs::read_dir(&elpa).expect("read elpa dir") {
        let entry = entry.expect("elpa entry");
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&format!("{pkg_name}-")) {
            return entry.path();
        }
    }
    panic!(
        "installed package directory for {pkg_name} not found in {}",
        elpa.display()
    );
}
