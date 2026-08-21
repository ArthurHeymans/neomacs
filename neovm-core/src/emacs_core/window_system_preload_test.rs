//! Standing check: which of GNU's window-system preloads does this build take?
//!
//! GNU's `lisp/loadup.el` splits the graphical preloads in two, and the split
//! is not cosmetic -- the two halves answer two different questions about the
//! build:
//!
//! 1. **"Is there a window system at all?"**  `lisp/loadup.el:291-299`
//!    (`(if (fboundp 'x-create-frame) (progn (load "fringe") (load
//!    "emacs-lisp/regexp-opt") (load "image") (load
//!    "international/fontset") (load "dnd") (load "tool-bar")))`).
//!    `x-create-frame` is `DEFUN`ed in `src/xfns.c`, `src/pgtkfns.c`,
//!    `src/nsfns.m`, `src/haikufns.c`, `src/androidfns.c` and `src/w32fns.c`,
//!    i.e. once per window system, so `fboundp` here is GNU's Lisp-level
//!    spelling of `HAVE_WINDOW_SYSTEM`.
//! 2. **"WHICH window system?"**  `lisp/loadup.el:304-362`, six sibling
//!    branches on `(featurep 'x)`, `'haiku`, `'android`, `'w32`, `'ns` and
//!    `'pgtk`.  Each provides its own `term/FOO-win.el`, and **every one of
//!    the six also loads `term/common-win.el`** -- the file whose name says
//!    what it is.  The only branch that skips it is `ms-dos`, and it says why
//!    in a comment: "Don't load term/common-win: it isn't appropriate for the
//!    `pc' ``window system'', which generally behaves like a terminal"
//!    (`lisp/loadup.el:341-343`).
//!
//! This port answers **yes** to (1): `x-create-frame` is registered, and
//! `loadup.el` takes that branch exactly as GNU does.  It answers **none** to
//! (2): there is no `neo` branch, because `term/neo-win.el` is a GUI-runtime
//! concern (it opens with `(unless (featurep 'neomacs) (error ...))`, and
//! `neomacs` is not provided in a dumped batch image).
//!
//! Measured over the 1972 names of ledger 178's deleted seed tables, against
//! GNU Emacs 31.0.90 `-Q --batch` built three ways -- `--with-x-toolkit=gtk3`,
//! `--without-x --without-all`, and this port:
//!
//! | `documentation-property` answers | GNU+X | GNU tty-only | neomacs | n |
//! | --- | --- | --- | --- | --- |
//! | | doc | doc | doc | 1856 |
//! | question (1)'s files | doc | **nil** | **doc** | **77** |
//! | question (2)'s files | doc | nil | nil | **28** |
//! | names GNU 31 no longer documents | nil | nil | nil | 11 |
//!
//! The 77 row is the load-bearing one: on every variable that separates a
//! tty-only GNU from a graphical one, **this port answers with the graphical
//! build.**  It is not a tty build, so "the tty build skips `common-win` too"
//! is not available as a defence for skipping it here.
//!
//! Ledger 179.

use crate::test_utils::runtime_startup_eval_one;

/// `term/common-win.el` is preloaded, because this build has a window system.
///
/// GNU loads it from all six window-system branches of `lisp/loadup.el`
/// (`:308`, `:313`, `:320`, `:326`, `:349`, `:361`), so a graphical GNU build
/// carries it in the dump whatever the window system is, and even in `--batch`
/// where `window-system` is nil.  This port is graphical by GNU's own
/// predicate -- `(fboundp 'x-create-frame)` is `t`, and `loadup.el` already
/// takes GNU's `HAVE_WINDOW_SYSTEM` branch on it -- so it must carry the file
/// too.
///
/// The four public variables asserted below are the ones `term/common-win.el`
/// defines with a docstring; GNU's own `symbol-file` attributes all four to
/// `term/common-win.elc`.  They are named rather than scanned out of the file
/// on purpose: this is GNU's published surface, not a table this port
/// maintains.  The anchor that cannot go green by attrition is
/// `(featurep 'term/common-win)` -- `lisp/term/common-win.el:416` ends with
/// `(provide 'term/common-win)`, so an emptied or unloaded file reports `nil`
/// here rather than an empty list of names to check.
///
/// The two functions are the reason this is a behaviour fix and not a
/// documentation one.  `x-setup-function-keys` is what installs
/// `x-alternatives-map` into `local-function-key-map`, and
/// `lisp/faces.el:2238` calls it unguarded from `x-create-frame-with-faces`;
/// `x-handle-args` is the command-line handler every `term/FOO-win.el`
/// delegates to.  Before ledger 179 neither was in the dumped image.
///
/// Ledger 179.  RED before the fix: `OK (nil ((x-alternatives-map nil nil)
/// (x-colors nil nil) (x-display-name t nil) (emacs-save-session-functions
/// nil nil)) nil nil t)` -- note `x-display-name`, bound with no
/// documentation, which is what a Rust stand-in for a Lisp `defvar` looks
/// like from Lisp.
#[test]
fn term_common_win_is_preloaded_because_this_build_has_a_window_system() {
    crate::test_utils::init_test_tracing();
    let result = runtime_startup_eval_one(
        "(list
           ;; `lisp/term/common-win.el:416'.  Not a name list: an emptied or
           ;; unloaded file answers nil here instead of vacuously passing.
           (featurep 'term/common-win)
           ;; GNU's four documented variables from that file.
           (mapcar (lambda (s)
                     (list s
                           (and (boundp s) t)
                           (and (stringp (documentation-property
                                          s 'variable-documentation t))
                                t)))
                   '(x-alternatives-map x-colors x-display-name
                     emacs-save-session-functions))
           ;; The two functions the dump was missing.
           (and (fboundp 'x-setup-function-keys) t)
           (and (fboundp 'x-handle-args) t)
           ;; ... and the predicate that says this build is entitled to all of
           ;; it: GNU's Lisp spelling of HAVE_WINDOW_SYSTEM.
           (and (fboundp 'x-create-frame) t))",
    );
    assert_eq!(
        result,
        "OK (t ((x-alternatives-map t t) (x-colors t t) (x-display-name t t) \
         (emacs-save-session-functions t t)) t t t)"
    );
}

/// The other half of the same statement: a window system this build does NOT
/// have contributes nothing to the dumped image.
///
/// This is the guard against "fixing" ledger 179's remaining divergences the
/// wrong way.  Twenty-three of the twenty-eight names ledger 178 handed over
/// come from `lisp/x-dnd.el` (18) and `lisp/term/x-win.el` (5), which GNU
/// preloads only behind `(featurep 'x)` (`lisp/loadup.el:304-309`), and `x` is
/// provided by `syms_of_xfns` (`src/xfns.c:10498`), compiled only under
/// `HAVE_X_WINDOWS`.  Preloading them here to make those rows match GNU's
/// X build would be an invention of the same kind ledger 178 deleted -- a
/// variable that exists with no window system behind it.
///
/// Measured rather than argued, three ways:
///
/// * GNU 31.0.90 built `--without-x --without-all` answers `nil` for all 28.
/// * GNU 31.0.90 built `--with-x-toolkit=gtk3` answers `nil`, in `--batch`,
///   for every Lisp variable of the five window systems it was NOT built
///   with -- `haiku-dnd-selection-value`, `haiku-normal-selection-encoders`,
///   `w32-standard-fontset-spec`, `w32-initialized`, `w32-non-USB-fonts`,
///   `android-primary-selection`, `android-preedit-overlay`,
///   `ns-working-overlay`, `ns-pop-up-frames`: 9 of 9 unbound and
///   undocumented.  GNU's own dump has this state; it is not a defect.
/// * `src/doc.c:585-594` says so in a comment -- "The (f)boundp checks below
///   ensure we don't report docs for eg w32-specific items on X" -- and
///   enforces it at `:606-613`, where the `Fput` of a
///   `variable-documentation` is gated on `Fboundp`.
///
/// The `x-dnd-` half is a `mapatoms` prefix scan over the whole obarray rather
/// than a list of the 18 names: ledger 173's law is that a predicate over rows
/// that exist cannot see a row that was never written, and `x-dnd.el` can grow
/// a nineteenth.  A prefix scan over 17k symbols has no empty state and counts
/// names nobody thought to list.
///
/// **It found ten on its first run, and they are the C half of exactly this
/// window system.**  `x-dnd-disable-motif-drag`, `x-dnd-disable-motif-protocol`,
/// `x-dnd-fix-motif-leave`, `x-dnd-movement-function`,
/// `x-dnd-native-test-function`, `x-dnd-preserve-selection-data`,
/// `x-dnd-targets-list`, `x-dnd-unsupported-drop-function`,
/// `x-dnd-use-unsupported-drop` and `x-dnd-wheel-function` are `DEFVAR`s in
/// `src/xterm.c` (`:32870`-`:32960`), so GNU binds them only under
/// `HAVE_X_WINDOWS` -- the tty-only build answers `nil` for all ten -- and this
/// port declares them on purpose, from the `syms_of_xterm` sweep at
/// `neovm-core/src/emacs_core/eval.rs:5680-5699` and
/// `defvar_bool.rs:269`.  They are pinned here BY NAME, not folded into the
/// count, so that the count that must stay zero (the `x-dnd.el` one) stays
/// readable and an eleventh cannot arrive unnoticed.  The tension they record
/// -- this build has GNU's X drag-and-drop callback variables and not the
/// `x-dnd.el` that assigns them -- is ledger 179's, deliberately not resolved
/// there.
///
/// Ledger 179.  GREEN before and after the fix; it is here so that the fix for
/// the other five cannot be extended to these by accident.
#[test]
fn x_only_lisp_variables_are_absent_because_this_build_does_not_provide_x() {
    crate::test_utils::init_test_tracing();
    let result = runtime_startup_eval_one(
        "(let ((xterm-c-dnd-vars
                ;; src/xterm.c:32870-32960, DEFVAR'd under HAVE_X_WINDOWS and
                ;; declared here by eval.rs:5680-5699 / defvar_bool.rs:269.
                '(x-dnd-disable-motif-drag x-dnd-disable-motif-protocol
                  x-dnd-fix-motif-leave x-dnd-movement-function
                  x-dnd-native-test-function x-dnd-preserve-selection-data
                  x-dnd-targets-list x-dnd-unsupported-drop-function
                  x-dnd-use-unsupported-drop x-dnd-wheel-function)))
           (list
             ;; src/xfns.c:10498 `Fprovide (Qx, Qnil)', under HAVE_X_WINDOWS.
             (featurep 'x)
             ;; lisp/x-dnd.el:1743 and lisp/term/x-win.el:1656-1657.
             (featurep 'x-dnd)
             (featurep 'x-win)
             (featurep 'term/x-win)
             ;; Whole-obarray: no `x-dnd-' name from `x-dnd.el' -- that is, none
             ;; beyond the C ones above -- may be bound or documented.
             (let ((n 0))
               (mapatoms
                (lambda (s)
                  (if (and (string-prefix-p \"x-dnd-\" (symbol-name s))
                           (not (memq s xterm-c-dnd-vars)))
                      (if (or (boundp s)
                              (documentation-property
                               s 'variable-documentation t))
                          (setq n (1+ n))))))
               n)
             ;; The C ones, pinned by name: exactly these, no more, no fewer.
             (let (found)
               (mapatoms
                (lambda (s)
                  (if (and (string-prefix-p \"x-dnd-\" (symbol-name s))
                           (boundp s))
                      (push (symbol-name s) found))))
               (sort found #'string<))
             ;; The five `term/x-win.el' names, spelled out because they share no
             ;; prefix with each other.
             (let ((n 0))
               (dolist (s '(icon-map-list x-gtk-stock-map x-initialized
                            x-preedit-overlay
                            x-display-cursor-at-start-of-preedit-string))
                 (if (or (boundp s)
                         (documentation-property s 'variable-documentation t))
                     (setq n (1+ n))))
               n)))",
    );
    assert_eq!(
        result,
        "OK (nil nil nil nil 0 \
         (\"x-dnd-disable-motif-drag\" \"x-dnd-disable-motif-protocol\" \
         \"x-dnd-fix-motif-leave\" \"x-dnd-movement-function\" \
         \"x-dnd-native-test-function\" \"x-dnd-preserve-selection-data\" \
         \"x-dnd-targets-list\" \"x-dnd-unsupported-drop-function\" \
         \"x-dnd-use-unsupported-drop\" \"x-dnd-wheel-function\") 0)"
    );
}
