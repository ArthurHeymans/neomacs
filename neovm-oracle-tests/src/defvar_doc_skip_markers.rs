//! Oracle guards for GNU's `SKIP` doc marker -- the one string GNU's C says,
//! in as many words, is not documentation.
//!
//! A variable several window systems declare carries its doc text in exactly
//! one file and a placeholder in the rest: `x-pointer-shape` is `DEFVAR_LISP`
//! in `src/xfns.c:10327`, `src/w32fns.c:11809`, `src/haikufns.c:3284` and
//! `src/androidfns.c:3587`, and three of those four read
//! `doc: /* SKIP: real doc in xfns.c.  */`.  170 `DEFVAR` blocks across GNU's
//! `src/*.c` carry the marker.
//!
//! `Fsnarf_documentation` is where it is honoured, and the guard is explicit:
//!
//! ```c
//! /* Ignore docs that start with SKIP.  These mark
//!    placeholders where the real doc is elsewhere.  */
//! if ((!NILP (Fboundp (sym)) || !NILP (Fmemq (sym, delayed_init)))
//!     && strncmp (end, "\nSKIP", 5))
//!   Fput (sym, Qvariable_documentation, make_fixnum (pos + end + 1 - buf));
//! ```
//!
//! (`src/doc.c:600-608`.)  So no GNU build ever shows a `SKIP` string to a
//! user, in any window system, for any variable -- which makes it exactly the
//! kind of value that must be unrepresentable rather than merely absent.
//! Neomacs's `var_docs::gnu_table` is generated from all of GNU's `src/*.c`
//! and kept the alphabetically first copy of a duplicated name, so 35 rows
//! held the placeholder and `C-h v x-pointer-shape` answered
//! "SKIP: real text in xfns.c.".
//!
//! Both pins below are about the same table from two directions: the first
//! says the marker never reaches Lisp, the second says the text that reaches
//! Lisp instead is GNU's own.

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

/// No built-in variable's documentation is GNU's placeholder.
///
/// Asked over every name whose generated row held one, and asked by prefix
/// rather than by equality so a future generator that invents a *different*
/// placeholder ("TODO", "see xterm.c") is caught by the same pin.  GNU's own
/// test is `strncmp (end, "\nSKIP", 5)`, a prefix test, for the same reason.
#[test]
fn oracle_no_builtin_variable_documentation_is_gnus_skip_placeholder() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let (bad)
  (dolist (s '(font-use-system-font next-selection-coding-system
               selection-coding-system selection-converter-alist x-alt-keysym
               x-ctrl-keysym x-cursor-fore-pixel x-gtk-file-dialog-help-text
               x-gtk-show-hidden-files x-gtk-use-old-file-dialog
               x-hourglass-pointer-shape x-hyper-keysym x-max-tooltip-size
               x-meta-keysym x-mode-pointer-shape x-no-window-manager
               x-nontext-pointer-shape x-pixel-size-width-font-regexp
               x-pointer-shape x-sensitive-text-pointer-shape x-super-keysym
               x-toolkit-scroll-bars x-underline-at-descent-line
               x-use-underline-position-properties x-wait-for-event-timeout
               x-window-bottom-edge-cursor x-window-bottom-left-corner-cursor
               x-window-bottom-right-corner-cursor
               x-window-horizontal-drag-cursor x-window-left-edge-cursor
               x-window-right-edge-cursor x-window-top-edge-cursor
               x-window-top-left-corner-cursor x-window-top-right-corner-cursor
               x-window-vertical-drag-cursor))
    (let ((doc (documentation-property s 'variable-documentation)))
      (when (and (stringp doc) (string-prefix-p "SKIP" doc))
        (push s bad))))
  (nreverse bad))"#;
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

/// And the text that appears instead is the canonical file's.
///
/// The two names left out are `x-mode-pointer-shape` and
/// `x-nontext-pointer-shape`: GNU's only declarations of either are inside
/// `#if false /* This doesn't really do anything.  */` (`src/xfns.c:10333-10338`,
/// `10347-10352`, and the same pair in `src/androidfns.c`), so no build
/// declares them, `Fsnarf_documentation`'s `Fboundp` gate never fires, and GNU
/// answers nil.  They are pinned for *existence* in
/// `cus_start_platform_declarations`-style form below rather than for text.
#[test]
fn oracle_platform_duplicated_variables_carry_the_canonical_doc_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(mapcar (lambda (s)
          (let ((doc (documentation-property s 'variable-documentation)))
            (cons s (and (stringp doc) (car (split-string doc "\n"))))))
        '(font-use-system-font next-selection-coding-system
          selection-coding-system selection-converter-alist x-alt-keysym
          x-ctrl-keysym x-cursor-fore-pixel x-gtk-file-dialog-help-text
          x-gtk-show-hidden-files x-gtk-use-old-file-dialog
          x-hourglass-pointer-shape x-hyper-keysym x-max-tooltip-size
          x-meta-keysym x-no-window-manager x-pixel-size-width-font-regexp
          x-pointer-shape x-sensitive-text-pointer-shape x-super-keysym
          x-toolkit-scroll-bars x-underline-at-descent-line
          x-use-underline-position-properties x-wait-for-event-timeout
          x-window-bottom-edge-cursor x-window-bottom-left-corner-cursor
          x-window-bottom-right-corner-cursor x-window-horizontal-drag-cursor
          x-window-left-edge-cursor x-window-right-edge-cursor
          x-window-top-edge-cursor x-window-top-left-corner-cursor
          x-window-top-right-corner-cursor x-window-vertical-drag-cursor))"#;
    let expect = expect_test::expect![[r#""OK PLACEHOLDER""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

/// The two names GNU compiles out of every build.
///
/// `#if false /* This doesn't really do anything.  */` wraps both
/// `DEFVAR_LISP`s in `src/xfns.c` (`10333-10338`, `10347-10352`) and both in
/// `src/androidfns.c`; `w32fns.c` and `haikufns.c` do not declare them at all.
/// A declaration inside a dead preprocessor branch is not a declaration, so
/// GNU leaves the symbols unbound -- entry 138's rule ("a build without MS-DOS
/// support needs `dos-hyper-key` to NOT exist") reaching a case that is not
/// about a platform at all.  `Vx_mode_pointer_shape` the C global still
/// exists and is still assigned `Qnil` on the line after the `#endif`, which
/// is what makes the seed look justified from the C side.
#[test]
fn oracle_defvars_inside_a_dead_preprocessor_branch_are_unbound() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list (mapcar #'boundp '(x-mode-pointer-shape x-nontext-pointer-shape))
      ;; The live neighbours in the same `syms_of_xfns' block, so the pin
      ;; fails if the answer swings the other way and the group is deleted.
      (mapcar #'boundp '(x-pointer-shape x-hourglass-pointer-shape
                         x-sensitive-text-pointer-shape)))"#;
    let expect = expect_test::expect![[r#""OK ((nil nil) (t t t))""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
