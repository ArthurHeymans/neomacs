//! Complex combo batch 112 — abbreviation / dynamic-abbrev / hippie-expand
//! / dabbrev / completion-at-point with various styles.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx112_abbrev_table_basic_define_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((table (make-abbrev-table)))
      (define-abbrev table "abc" "alpha beta gamma" nil)
      (define-abbrev table "xyz" "X-ray Yankee Zulu" nil)
      (list (abbrev-table-p table)
            (abbrev-expansion "abc" table)
            (abbrev-expansion "xyz" table)
            (abbrev-expansion "missing" table)
            (abbrev-symbol "abc" table)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx112_global_abbrev_table_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((before (abbrev-table-p global-abbrev-table)))
      (define-abbrev global-abbrev-table "neoCx112abc" "expanded abc" nil)
      (let ((expansion (abbrev-expansion "neoCx112abc" global-abbrev-table)))
        (list before expansion
              (abbrev-symbol "neoCx112abc" global-abbrev-table))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx112_dabbrev_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'dabbrev)
      (list (fboundp 'dabbrev-expand)
            (fboundp 'dabbrev-completion)
            (boundp 'dabbrev-limit)
            (boundp 'dabbrev-case-fold-search)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx112_hippie_expand_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'hippie-exp)
      (list (fboundp 'hippie-expand)
            (boundp 'hippie-expand-try-functions-list)
            (fboundp 'try-expand-dabbrev)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx112_completion_at_point_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'completion-at-point)
          (fboundp 'complete-symbol)
          (fboundp 'completion--in-region)
          (boundp 'completion-at-point-functions))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx112_completion_styles_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((styles '(basic partial-completion substring initials emacs22
                    emacs21)))
      (mapcar (lambda (s)
                (list s (memq s completion-styles)))
              styles))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx112_try_completion_basic_with_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coll '("alpha" "alphabet" "alpine" "amplitude" "antelope")))
  (list (try-completion "al" coll)
        (try-completion "alph" coll)
        (try-completion "amp" coll)
        (try-completion "anti" coll)
        (try-completion "z" coll)))
"##,
    );
}

#[test]
fn div_cx112_all_completions_with_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coll '("apple" "apricot" "avocado" "banana" "blueberry")))
  (list (all-completions "a" coll)
        (all-completions "ap" coll)
        (all-completions "b" coll)
        (all-completions "av" coll)))
"##,
    );
}

#[test]
fn div_cx112_completion_with_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ob (make-obarray 31)))
  (intern "alpha" ob)
  (intern "alphabet" ob)
  (intern "alpine" ob)
  (intern "amplitude" ob)
  (list (all-completions "al" ob)
        (all-completions "amp" ob)
        (try-completion "al" ob)))
"##,
    );
}

#[test]
fn div_cx112_completion_case_insensitive_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coll '("Alpha" "ALPHA" "alpha" "Beta"))
      (completion-ignore-case t))
  (list (length (all-completions "a" coll))
        (length (all-completions "A" coll))
        (try-completion "a" coll)
        (try-completion "A" coll)))
"##,
    );
}

#[test]
fn div_cx112_completion_with_hash_table_via_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :test 'equal)))
  (puthash "alpha" 1 ht)
  (puthash "alphabet" 2 ht)
  (puthash "beta" 3 ht)
  (let* ((keys nil))
    (maphash (lambda (k _) (push k keys)) ht)
    (list (sort (all-completions "al" keys) #'string<)
          (try-completion "al" keys))))
"##,
    );
}

#[test]
fn div_cx112_completion_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coll '("alpha" "alphabet" "alpine" "amplitude")))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert (mapconcat #'identity coll " "))
    (put-text-property 1 5 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 22)
      (let ((state (list (try-completion "al" coll)
                         (all-completions "al" coll)
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    );
}
