//! Complex combo batch 86 — packages / require / load paths / autoload /
//! `with-eval-after-load` / `eval-after-load` semantics.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx86_require_already_loaded_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((before-load features))
      (let ((first (require 'cl-lib))
            (second (require 'cl-lib)))
        (list (null first)
              (null second)
              (eq first second)
              (eq before-load features))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx86_with_eval_after_load_runs_once() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let (ran)
      (with-eval-after-load 'cl-lib
        (push :after-load ran))
      (require 'cl-lib)
      (list ran (memq 'cl-lib features)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx86_locate_library_for_known_libs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (locate-library "cl-lib")
 (locate-library "subr-x")
 (file-name-nondirectory (or (locate-library "cl-lib") ""))
 (locate-library "definitely-no-such-lib-xyz"))
"##,
    );
}

#[test]
fn div_cx86_load_file_path_resolution() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((cl-path (locate-library "cl-lib")))
  (list (stringp cl-path)
        (file-exists-p cl-path)
        (file-name-extension cl-path)
        (member (file-name-nondirectory cl-path) '("cl-lib.el" "cl-lib.elc" "cl-lib.el.gz"))))
"##,
    );
}

#[test]
fn div_cx86_featurep_with_subfeature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(require 'cl-lib)
(list
 (featurep 'cl-lib)
 (featurep 'cl-lib 'struct)
 (featurep 'no-such-feature)
 (condition-case e (featurep 'cl-lib 'no-such-subfeature) (error :err)))
"##,
    );
}

#[test]
fn div_cx86_provide_features_with_subfeature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(provide 'neo-cx86-pkg 'sub1)
(provide 'neo-cx86-pkg 'sub2)
(list
 (featurep 'neo-cx86-pkg)
 (featurep 'neo-cx86-pkg 'sub1)
 (featurep 'neo-cx86-pkg 'sub2)
 (featurep 'neo-cx86-pkg 'missing)
 (memq 'neo-cx86-pkg features))
"##,
    );
}

#[test]
fn div_cx86_load_suffixes_and_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((suffixes load-file-rep-suffixes))
  (list (consp load-suffixes)
        (member ".elc" load-suffixes)
        (member ".el" load-suffixes)
        (consp load-path)
        (stringp (car load-path))))
"##,
    );
}

#[test]
fn div_cx86_autoload_function_definition_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((fn-cell (symbol-function 'forward-char)))
      (list (subrp fn-cell)
            (autoloadp fn-cell)
            (functionp fn-cell)
            (subr-name fn-cell)
            (subr-arity fn-cell)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx86_define_autoload_then_use() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((before (symbol-function 'cl-incf)))
      (list (or (macrop before) (autoloadp before))
            (fboundp 'cl-incf)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx86_load_history_after_require() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(require 'cl-lib)
(require 'subr-x)
(let* ((cl-lib-path (locate-library "cl-lib"))
       (entry (cl-find-if (lambda (e) (equal (car e) cl-lib-path)) load-history)))
  (list (consp entry)
        (stringp (car entry))
        (listp (cdr entry))
        (> (length entry) 0)))
"##,
    );
}

#[test]
fn div_cx86_loaded_features_consistent_after_re_require() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((before-1 features))
  (require 'cl-lib)
  (let ((after-1 features))
    (require 'cl-lib)
    (let ((after-2 features))
      (list (eq after-1 after-2)
            (eq before-1 after-1)
            (memq 'cl-lib after-1)
            (memq 'cl-lib after-2)))))
"##,
    );
}

#[test]
fn div_cx86_load_features_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(require 'cl-lib)
(require 'subr-x)
(with-temp-buffer
  (buffer-enable-undo)
  (insert "Feature test buffer with content")
  (put-text-property 1 7 'face 'bold)
  (let ((m (set-marker (make-marker) 10))
        (ov (make-overlay 3 18)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 2 25)
    (let ((state (list (memq 'cl-lib features)
                       (memq 'subr-x features)
                       (buffer-string)
                       (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (text-properties-at 1))))
      (undo)
      (widen)
      (list state (buffer-string) (marker-position m)
            (overlay-start ov) (overlay-end ov)
            (text-properties-at 1)))))
"##,
    );
}
