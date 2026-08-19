//! The eighteen window/frame/face geometry names are Lisp, and only Lisp --
//! DIVERGENCES.md 154.
//!
//! Ledger 146 enumerated the class -- Rust subrs whose function cell the
//! `loadup.el` preloads overwrite -- and ranked this group LAST and RISKIEST,
//! because the display stack sits downstream of it.  148 took the type
//! predicates and the `defalias` names, 149 the process launchers, 150 the undo
//! commands, 152 the leftovers.  This is what was left: eighteen names from
//! `lisp/window.el`, `lisp/frame.el` and `lisp/faces.el`.
//!
//! `grep 'DEFUN ("NAME"' src/*.c` against emacs-mirror 31.0.90 (0ee48ac4df2)
//! finds nothing for any of the eighteen.  Every one of them is a plain
//! `defun` -- no `defsubst`, no `compiler-macro`, no `byte-compile` property --
//! so every one compiles to an ordinary call through the constants vector and
//! a compiled caller really does read the cell.  The shadow was the only thing
//! between those callers and the Rust subr.
//!
//! 146's parenthetical had the layering backwards: it said `window-edges' was
//! built "on `window-pixel-edges'".  GNU is the other way round --
//! `window-pixel-edges' (lisp/window.el:3922) is a one-line wrapper
//! `(window-edges window nil nil t)', and it is `window-edges' that is written
//! over the C primitives.  Both are Lisp; neither is a DEFUN.  See the dated
//! correction note on entry 146.
//!
//! The C primitives each of the eighteen is written over must survive the
//! deletion, and that is the half a careless sweep would have got wrong:
//! `delete-window' is Lisp but `delete-window-internal' is C, `color-values' is
//! Lisp but `xw-color-values' is C, `display-color-cells' is Lisp but
//! `x-display-color-cells' and `tty-display-color-cells' are C, and `make-frame'
//! is Lisp but `x-create-frame' and `make-terminal-frame' are C.
//!
//! `rust_subrs_shadowed_by_lisp_test.rs` is the scan that finds new shadows;
//! this is the per-name statement for the eighteen entry 154 deleted, and the
//! statement that the C names beneath them are still here.

use crate::emacs_core::eval::Context;
use crate::emacs_core::eval::lookup_global_subr_entry;
use crate::emacs_core::intern::intern;
use crate::test_utils::{runtime_startup_eval_all, runtime_startup_eval_one};

/// GNU has no C version of these, so a bare evaluator -- which is GNU before
/// `loadup.el` -- must have nothing to answer with.  `loadup.el` loads
/// `window` at :138, `faces` at :160 and `frame` at :255.
const LISP_ONLY_WINDOW_FRAME_NAMES: &[&str] = &[
    "balance-windows",              // lisp/window.el:6222
    "color-defined-p",              // lisp/faces.el:1923
    "color-values",                 // lisp/faces.el:1940
    "delete-other-windows",         // lisp/window.el:4453
    "delete-window",                // lisp/window.el:4318
    "display-buffer",               // lisp/window.el:8166
    "display-color-cells",          // lisp/frame.el:2966
    "enlarge-window",               // lisp/window.el:3714
    "fit-window-to-buffer",         // lisp/window.el:10307
    "make-frame",                   // lisp/frame.el:1019
    "pop-to-buffer",                // lisp/window.el:9403
    "select-frame-set-input-focus", // lisp/frame.el:1262
    "shrink-window",                // lisp/window.el:3759
    "switch-to-buffer",             // lisp/window.el:9558
    "window-absolute-pixel-edges",  // lisp/window.el:3937
    "window-edges",                 // lisp/window.el:3839
    "window-pixel-edges",           // lisp/window.el:3922
    "window-tree",                  // lisp/window.el:3999
];

/// The C primitives the eighteen are written over.  Deleting the Lisp names is
/// not a licence to delete these, and for six of the eighteen the C neighbour
/// is one character away from the Lisp name.
const C_PRIMITIVES_BENEATH_THEM: &[&str] = &[
    // `window-edges' reads all of these, and `window-pixel-edges' and
    // `window-absolute-pixel-edges' read `window-edges'.
    "frame-char-height",           // src/frame.c
    "frame-char-width",            // src/frame.c
    "frame-internal-border-width", // src/frame.c
    "window-body-height",          // src/window.c
    "window-body-width",           // src/window.c
    "window-fringes",              // src/window.c
    "window-header-line-height",   // src/window.c
    "window-left-column",          // src/window.c
    "window-margins",              // src/window.c
    "window-pixel-height",         // src/window.c
    "window-pixel-left",           // src/window.c
    "window-pixel-top",            // src/window.c
    "window-pixel-width",          // src/window.c
    "window-scroll-bar-width",     // src/window.c
    "window-tab-line-height",      // src/window.c
    "window-top-line",             // src/window.c
    "window-total-height",         // src/window.c
    "window-total-width",          // src/window.c
    // `delete-window' and `delete-other-windows' are Lisp over these two.
    "delete-other-windows-internal", // src/window.c
    "delete-window-internal",        // src/window.c
    // `window-tree' walks from here.
    "frame-root-window",   // src/window.c
    "window-next-sibling", // src/window.c
    // `balance-windows', `enlarge-window' and `shrink-window' apply through
    // these.
    "window-resize-apply",       // src/window.c
    "window-resize-apply-total", // src/window.c
    // `switch-to-buffer', `display-buffer' and `pop-to-buffer' are Lisp over
    // these; `record_buffer' is reachable from Lisp only via `select-window'
    // (src/window.c:582, and the comment at :540 says exactly that).
    "select-window",     // src/window.c
    "set-buffer",        // src/buffer.c
    "set-window-buffer", // src/window.c
    // `color-defined-p' and `color-values' dispatch on `display-graphic-p' to
    // the `xw-' pair; `display-color-cells' dispatches to the `x-'/`tty-' pair.
    "tty-display-color-cells", // src/term.c
    "x-display-color-cells",   // src/xfns.c
    "xw-color-defined-p",      // src/xfns.c
    "xw-color-values",         // src/xfns.c
    // `make-frame' funcalls `frame-creation-function', which reaches one of
    // these; `select-frame-set-input-focus' is `select-frame' + `raise-frame' +
    // `x-focus-frame'.
    "make-terminal-frame", // src/frame.c
    "raise-frame",         // src/frame.c
    "select-frame",        // src/frame.c
    "x-create-frame",      // src/xfns.c
    "x-focus-frame",       // src/xfns.c
];

#[test]
fn the_eighteen_window_frame_names_are_void_on_a_bare_evaluator_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut eval = Context::new();

    for primitive in C_PRIMITIVES_BENEATH_THEM {
        let result = eval.eval_str(&format!("(fboundp '{primitive})"));
        assert_eq!(
            crate::emacs_core::error::format_eval_result_with_eval(&eval, &result),
            "OK t",
            "{primitive} is DEFUN'ed in GNU src/ and must remain a subr",
        );
    }

    for name in LISP_ONLY_WINDOW_FRAME_NAMES {
        let result = eval.eval_str(&format!("(fboundp '{name})"));
        assert_eq!(
            crate::emacs_core::error::format_eval_result_with_eval(&eval, &result),
            "OK nil",
            "{name} must be void before window.el/faces.el/frame.el load: \
             GNU's src/ has no DEFUN of that name, so a bare evaluator has \
             nothing to answer with",
        );
    }
}

#[test]
fn no_rust_subr_is_registered_for_the_eighteen_window_frame_names() {
    crate::test_utils::init_test_tracing();
    // The global subr registry is populated by `init_builtins`, which runs
    // when an evaluator is built; ask for one before reading the table.
    let _eval = Context::new();
    for name in LISP_ONLY_WINDOW_FRAME_NAMES {
        assert!(
            lookup_global_subr_entry(intern(name)).is_none(),
            "{name} must have no Rust subr: GNU implements it in Lisp and \
             nowhere in src/",
        );
    }
    for name in C_PRIMITIVES_BENEATH_THEM {
        assert!(
            lookup_global_subr_entry(intern(name)).is_some(),
            "{name} IS a C DEFUN in GNU and must stay registered here",
        );
    }
}

/// Every observable a Lisp caller can ask about the eighteen, measured on GNU
/// 31.0.90 `-Q --batch` first (tmp/pw61/gnu-observables.txt) and re-asked of
/// the loaded runtime, where the `.el` definitions are what reply.
///
/// The Rust subrs got SEVEN arities wrong and TEN `commandp`s wrong -- ten of
/// the eighteen are commands in GNU and not one Rust subr was registered
/// interactive -- and all eighteen answered "Built-in function." for
/// `documentation`.
#[test]
fn the_eighteen_window_frame_names_are_lisp_in_the_loaded_runtime_like_gnu() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        runtime_startup_eval_all(
            r#"
;; (subrp func-arity commandp) for each name.
(list (subrp (symbol-function 'balance-windows)) (func-arity 'balance-windows)
      (and (commandp 'balance-windows) t))
(list (subrp (symbol-function 'color-defined-p)) (func-arity 'color-defined-p)
      (and (commandp 'color-defined-p) t))
(list (subrp (symbol-function 'color-values)) (func-arity 'color-values)
      (and (commandp 'color-values) t))
(list (subrp (symbol-function 'delete-other-windows)) (func-arity 'delete-other-windows)
      (and (commandp 'delete-other-windows) t))
(list (subrp (symbol-function 'delete-window)) (func-arity 'delete-window)
      (and (commandp 'delete-window) t))
(list (subrp (symbol-function 'display-buffer)) (func-arity 'display-buffer)
      (and (commandp 'display-buffer) t))
(list (subrp (symbol-function 'display-color-cells)) (func-arity 'display-color-cells)
      (and (commandp 'display-color-cells) t))
(list (subrp (symbol-function 'enlarge-window)) (func-arity 'enlarge-window)
      (and (commandp 'enlarge-window) t))
(list (subrp (symbol-function 'fit-window-to-buffer)) (func-arity 'fit-window-to-buffer)
      (and (commandp 'fit-window-to-buffer) t))
(list (subrp (symbol-function 'make-frame)) (func-arity 'make-frame)
      (and (commandp 'make-frame) t))
(list (subrp (symbol-function 'pop-to-buffer)) (func-arity 'pop-to-buffer)
      (and (commandp 'pop-to-buffer) t))
(list (subrp (symbol-function 'select-frame-set-input-focus))
      (func-arity 'select-frame-set-input-focus)
      (and (commandp 'select-frame-set-input-focus) t))
(list (subrp (symbol-function 'shrink-window)) (func-arity 'shrink-window)
      (and (commandp 'shrink-window) t))
(list (subrp (symbol-function 'switch-to-buffer)) (func-arity 'switch-to-buffer)
      (and (commandp 'switch-to-buffer) t))
(list (subrp (symbol-function 'window-absolute-pixel-edges))
      (func-arity 'window-absolute-pixel-edges)
      (and (commandp 'window-absolute-pixel-edges) t))
(list (subrp (symbol-function 'window-edges)) (func-arity 'window-edges)
      (and (commandp 'window-edges) t))
(list (subrp (symbol-function 'window-pixel-edges)) (func-arity 'window-pixel-edges)
      (and (commandp 'window-pixel-edges) t))
(list (subrp (symbol-function 'window-tree)) (func-arity 'window-tree)
      (and (commandp 'window-tree) t))
;; The interactive forms of the five whose spec is a plain form or string --
;; the other five commands carry a compiled `interactive' body, which is
;; asserted by `commandp' above rather than spelled out here.
(interactive-form 'balance-windows)
(interactive-form 'delete-window)
(interactive-form 'delete-other-windows)
(interactive-form 'enlarge-window)
(interactive-form 'shrink-window)
(interactive-form 'fit-window-to-buffer)
(interactive-form 'make-frame)
;; The C primitives beneath them are untouched and still subrs.
(list (subrp (symbol-function 'window-pixel-left)) (func-arity 'window-pixel-left))
(list (subrp (symbol-function 'window-body-width)) (func-arity 'window-body-width))
(list (subrp (symbol-function 'delete-window-internal)) (func-arity 'delete-window-internal))
(list (subrp (symbol-function 'set-window-buffer)) (func-arity 'set-window-buffer))
(list (subrp (symbol-function 'select-window)) (func-arity 'select-window))
(list (subrp (symbol-function 'xw-color-values)) (func-arity 'xw-color-values))
(list (subrp (symbol-function 'x-create-frame)) (func-arity 'x-create-frame))
;; The first line of each docstring is `.elc' text, not "Built-in function."
(car (split-string (documentation 'window-edges) "\n"))
(car (split-string (documentation 'switch-to-buffer) "\n"))
(car (split-string (documentation 'color-values) "\n"))
(car (split-string (documentation 'make-frame) "\n"))
;; Three of the eighteen carry `declare (side-effect-free t)', which a subr
;; registration has no way to express.
(function-get 'window-edges 'side-effect-free)
(function-get 'window-pixel-edges 'side-effect-free)
(function-get 'window-absolute-pixel-edges 'side-effect-free)
(function-get 'window-tree 'side-effect-free)
"#,
        ),
        vec![
            "OK (nil (0 . 1) t)",
            "OK (nil (1 . 2) nil)",
            "OK (nil (1 . 2) nil)",
            "OK (nil (0 . 2) t)",
            "OK (nil (0 . 1) t)",
            "OK (nil (1 . 3) t)",
            "OK (nil (0 . 1) nil)",
            "OK (nil (1 . 2) t)",
            "OK (nil (0 . 6) t)",
            "OK (nil (0 . 1) t)",
            "OK (nil (1 . 3) t)",
            "OK (nil (1 . 2) nil)",
            "OK (nil (1 . 2) t)",
            "OK (nil (1 . 3) t)",
            "OK (nil (0 . 1) nil)",
            "OK (nil (0 . 4) nil)",
            "OK (nil (0 . 1) nil)",
            "OK (nil (0 . 1) nil)",
            // interactive forms
            "OK (interactive nil)",
            "OK (interactive nil)",
            "OK (interactive \"i\\np\")",
            "OK (interactive \"p\")",
            "OK (interactive \"p\")",
            "OK (interactive nil)",
            "OK (interactive nil)",
            // C primitives
            "OK (t (0 . 1))",
            "OK (t (0 . 2))",
            "OK (t (1 . 1))",
            "OK (t (2 . 3))",
            "OK (t (1 . 2))",
            "OK (t (1 . 2))",
            "OK (t (1 . 1))",
            // docstrings
            "OK \"Return a list of the edge distances of WINDOW.\"",
            "OK \"Display buffer BUFFER-OR-NAME in the selected window.\"",
            "OK \"Return a description of the color named COLOR on frame FRAME.\"",
            "OK \"Return a newly created frame displaying the current buffer.\"",
            // declare side-effect-free
            "OK t",
            "OK t",
            "OK t",
            "OK nil",
        ],
    );
}

/// Every one of the eighteen compiles to an ordinary call through the
/// constants vector, in GNU and here.  None has a `byte-compile` property, a
/// `compiler-macro` or a `byte-optimizer`, and none is a `defsubst`, so unlike
/// three of 152's thirteen there is no door by which a compiled caller could
/// avoid the function cell.  The shadow was the only thing between those
/// callers and the Rust subr.
///
/// Measured on GNU 31.0.90 with `lexical-binding` t (tmp/pw61/gnu-bytecode.txt).
/// 192 = Bconstant, 1/2 = Bstack_ref, 33 = Bcall1, 34 = Bcall2, 135 = Breturn.
#[test]
fn all_eighteen_are_ordinary_calls_that_read_the_cell_like_gnu() {
    crate::test_utils::init_test_tracing();
    for (form, codes, constants) in [
        (
            "(lambda (w) (balance-windows w))",
            "(192 1 33 135)",
            "[balance-windows]",
        ),
        (
            "(lambda (c f) (color-defined-p c f))",
            "(192 2 2 34 135)",
            "[color-defined-p]",
        ),
        (
            "(lambda (c f) (color-values c f))",
            "(192 2 2 34 135)",
            "[color-values]",
        ),
        (
            "(lambda (w) (delete-other-windows w))",
            "(192 1 33 135)",
            "[delete-other-windows]",
        ),
        (
            "(lambda (w) (delete-window w))",
            "(192 1 33 135)",
            "[delete-window]",
        ),
        (
            "(lambda (b) (display-buffer b))",
            "(192 1 33 135)",
            "[display-buffer]",
        ),
        (
            "(lambda (d) (display-color-cells d))",
            "(192 1 33 135)",
            "[display-color-cells]",
        ),
        (
            "(lambda (n) (enlarge-window n))",
            "(192 1 33 135)",
            "[enlarge-window]",
        ),
        (
            "(lambda (w) (fit-window-to-buffer w))",
            "(192 1 33 135)",
            "[fit-window-to-buffer]",
        ),
        (
            "(lambda (p) (make-frame p))",
            "(192 1 33 135)",
            "[make-frame]",
        ),
        (
            "(lambda (b) (pop-to-buffer b))",
            "(192 1 33 135)",
            "[pop-to-buffer]",
        ),
        (
            "(lambda (f) (select-frame-set-input-focus f))",
            "(192 1 33 135)",
            "[select-frame-set-input-focus]",
        ),
        (
            "(lambda (n) (shrink-window n))",
            "(192 1 33 135)",
            "[shrink-window]",
        ),
        (
            "(lambda (b) (switch-to-buffer b))",
            "(192 1 33 135)",
            "[switch-to-buffer]",
        ),
        (
            "(lambda (w) (window-absolute-pixel-edges w))",
            "(192 1 33 135)",
            "[window-absolute-pixel-edges]",
        ),
        (
            "(lambda (w) (window-edges w))",
            "(192 1 33 135)",
            "[window-edges]",
        ),
        (
            "(lambda (w) (window-pixel-edges w))",
            "(192 1 33 135)",
            "[window-pixel-edges]",
        ),
        (
            "(lambda (f) (window-tree f))",
            "(192 1 33 135)",
            "[window-tree]",
        ),
    ] {
        assert_eq!(
            runtime_startup_eval_one(&format!("(append (aref (byte-compile '{form}) 1) nil)")),
            format!("OK {codes}"),
            "{form} should compile to GNU's opcode sequence",
        );
        assert_eq!(
            runtime_startup_eval_one(&format!("(aref (byte-compile '{form}) 2)")),
            format!("OK {constants}"),
            "{form} should compile to GNU's constants vector",
        );
    }
}

/// `window-pixel-edges' and `window-absolute-pixel-edges' are wrappers over
/// `window-edges', not over any primitive -- the layering 146 stated backwards.
/// Their bodies are one call each, so a redefinition of `window-edges' is
/// visible through both, which is only true because all three are Lisp.
#[test]
fn the_pixel_edge_wrappers_go_through_window_edges_like_gnu() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        runtime_startup_eval_all(
            r#"
(cl-letf (((symbol-function 'window-edges)
           (lambda (&optional _w body absolute pixelwise)
             (list 'edges body absolute pixelwise))))
  (list (window-pixel-edges) (window-absolute-pixel-edges)))
;; lisp/window.el:3922 and :3937 verbatim: (window-edges window nil nil t) and
;; (window-edges window nil t t).
(equal (window-pixel-edges) (window-edges nil nil nil t))
(equal (window-absolute-pixel-edges) (window-edges nil nil t t))
"#,
        ),
        vec!["OK ((edges nil nil t) (edges nil t t))", "OK t", "OK t",],
    );
}
