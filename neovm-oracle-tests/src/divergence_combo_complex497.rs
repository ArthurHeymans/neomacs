/// Batch 497: define-derived-mode, define-minor-mode, define-generic-mode.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx497_define_derived_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-derived-mode neo-cx497-mode text-mode "neo497")
  (with-temp-buffer
    (neo-cx497-mode)
    (list major-mode (derived-mode-p 'text-mode))))
"##,
    );
}

#[test]
fn div_cx497_define_minor_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-minor-mode neo-cx497-minor-mode "test" :lighter " 497")
  (with-temp-buffer
    (neo-cx497-minor-mode 1)
    neo-cx497-minor-mode))
"##,
    );
}

#[test]
fn div_cx497_define_global_minor_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-minor-mode neo-cx497-global "test" :global t)
  (neo-cx497-global 1)
  neo-cx497-global)
"##,
    );
}

#[test]
fn div_cx497_define_generic_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-generic-mode 'neo-cx497-generic-mode
    '(("fn" . 'font-lock-keyword-face))
    nil nil nil nil)
  (with-temp-buffer
    (neo-cx497-generic-mode)
    (list major-mode (derived-mode-p 'neo-cx497-generic-mode))))
"##,
    );
}

#[test]
fn div_cx497_define_derived_mode_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-derived-mode neo-cx497-hook-mode text-mode "hook497")
  (boundp 'neo-cx497-hook-mode-hook))
"##,
    );
}

#[test]
fn div_cx497_minor_mode_keymap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-minor-mode neo-cx497-keymap-minor "test"
    :keymap (let ((m (make-sparse-keymap))) (define-key m "a" 'forward-char) m))
  (boundp 'neo-cx497-keymap-minor-map))
"##,
    );
}

#[test]
fn div_cx497_derived_mode_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-derived-mode neo-cx497-kw-mode prog-mode "kw497"
    (setq-local comment-start "// "))
  (with-temp-buffer
    (neo-cx497-kw-mode)
    comment-start))
"##,
    );
}

#[test]
fn div_cx497_define_minor_mode_lighter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-minor-mode neo-cx497-lighter-mode "test" :lighter " L497")
  (neo-cx497-lighter-mode 1)
  (stringp (format-mode-line
    (list (list (list 'neo-cx497-lighter-mode " L497"))))))
"##,
    );
}

#[test]
fn div_cx497_define_derived_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-derived-mode neo-cx497-parent-mode text-mode "P497")
  (define-derived-mode neo-cx497-child-mode neo-cx497-parent-mode "C497")
  (with-temp-buffer
    (neo-cx497-child-mode)
    (list (derived-mode-p 'text-mode) (derived-mode-p 'neo-cx497-parent-mode))))
"##,
    );
}

#[test]
fn div_cx497_minor_mode_after_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-minor-mode neo-cx497-after-mode "test")
  (boundp 'neo-cx497-after-mode-after-hook))
"##,
    );
}

#[test]
fn div_cx497_define_derived_abbrev() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-derived-mode neo-cx497-abbrev-mode text-mode "AB497"
    (setq local-abbrev-table (make-abbrev-table)))
  (boundp 'neo-cx497-abbrev-mode-map))
"##,
    );
}

#[test]
fn div_cx497_define_derived_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-derived-mode neo-cx497-syntax-mode text-mode "SY497"
    (set-syntax-table (make-syntax-table)))
  (boundp 'neo-cx497-syntax-mode-syntax-table))
"##,
    );
}

#[test]
fn div_cx497_define_multiple_minor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-minor-mode neo-cx497-m1 "m1")
  (define-minor-mode neo-cx497-m2 "m2")
  (list (fboundp 'neo-cx497-m1) (fboundp 'neo-cx497-m2)))
"##,
    );
}

#[test]
fn div_cx497_define_derived_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-derived-mode neo-cx497-kw2-mode fundamental-mode "KW2"
    "A mode docstring")
  (documentation 'neo-cx497-kw2-mode))
"##,
    );
}

#[test]
fn div_cx497_define_globalized_minor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-minor-mode neo-cx497-global-minor "test")
  (define-globalized-minor-mode neo-cx497-globalized
    neo-cx497-global-minor neo-cx497-global-minor-on)
  (defun neo-cx497-global-minor-on ()
    (neo-cx497-global-minor 1))
  (fboundp 'neo-cx497-globalized))
"##,
    );
}
