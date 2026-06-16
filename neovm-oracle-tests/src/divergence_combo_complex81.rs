//! Complex combo batch 81 — debug / backtrace / edebug / profiler / trace
//! availability, frame introspection, `backtrace-frame`, `mapbacktrace`,
//! `mapatoms`, and feature-loading side effects.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx81_backtrace_frame_inside_catch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (frames)
  (let ((fns (list (lambda () (push :level-2 frames))
                   (lambda () (push :level-1 frames) (funcall (car fns))))))
    (funcall (cadr fns))
    (nreverse frames)))
"##,
    );
}

#[test]
fn div_cx81_mapatoms_collect_interned_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(intern "neo-cx81-symbol-alpha")
(intern "neo-cx81-symbol-beta")
(intern "neo-cx81-symbol-gamma")
(let (collected)
  (mapatoms (lambda (s)
              (when (string-prefix-p "neo-cx81-symbol-" (symbol-name s))
                (push s collected))))
  (sort (mapcar #'symbol-name collected) #'string<))
"##,
    );
}

#[test]
fn div_cx81_features_after_require() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((before features))
  (condition-case e
      (progn (require 'cl-lib)
             (require 'subr-x)
             (list :before before
                   :after features
                   :added (cl-set-difference features before)))
    (error (list :errored (car e)))))
"##,
    );
}

#[test]
fn div_cx81_featurep_after_provide() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(provide 'neo-cx81-feature-x)
(list (featurep 'neo-cx81-feature-x)
      (featurep 'no-such-feature-y)
      (memq 'neo-cx81-feature-x features)
      (assq 'neo-cx81-feature-x load-history))
"##,
    );
}

#[test]
fn div_cx81_autoload_resolve_on_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (autoloadp (symbol-function 'forward-char))
          (autoloadp (symbol-function 'car))
          (fboundp 'forward-char)
          (fboundp 'no-such-fn-zzz))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx81_mapbacktrace_inside_signal_handler() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (frames)
  (condition-case e
      (signal 'wrong-type-argument '(integerp "x"))
    (error
     (mapbacktrace (lambda (evald func args flags)
                      (push (list evald (when (symbolp func) func)) frames))
                   t)))
  (list (length frames) (car frames)))
"##,
    );
}

#[test]
fn div_cx81_load_history_inspection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((cl-lib-entry (cl-find-if (lambda (e) (eq (car e) (locate-library "cl-lib")))
                                  load-history)))
  (list (consp cl-lib-entry)
        (car cl-lib-entry)
        (listp (cdr cl-lib-entry))))
"##,
    );
}

#[test]
fn div_cx81_documentation_for_subr_and_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((fn-with-doc (lambda () "doc for fn" :result)))
  (list (documentation 'car)
        (documentation 'forward-char)
        (documentation (symbol-function 'car))
        (documentation fn-with-doc)
        (documentation-property 'neo-cx81-no-such 'variable-documentation)))
"##,
    );
}

#[test]
fn div_cx81_help_function_arglist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((fn (lambda (a b &optional c &rest d) (list a b c d))))
      (list (help-function-arglist fn)
            (help-function-arglist (symbol-function 'car))
            (help-function-arglist (symbol-function 'list))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx81_inner_backtrace_through_unwind_protect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (trace)
  (unwind-protect
      (progn
        (push :body-start trace)
        (error "induced"))
    (push :unwind trace)
    (condition-case e
        (error "during cleanup")
      (error (push :cleanup-caught trace))))
  trace)
"##,
    );
}

#[test]
fn div_cx81_profiler_memory_with_marker_overlay_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((before (garbage-collect)))
      (with-temp-buffer
        (buffer-enable-undo)
        (insert "Profile test buffer with content here")
        (put-text-property 1 5 'face 'bold)
        (let ((m (set-marker (make-marker) 8))
              (ov (make-overlay 4 14)))
          (overlay-put ov 'face 'italic)
          (overlay-put ov 'evaporate t)
          (narrow-to-region 2 20)
          (let ((after (garbage-collect)))
            (undo)
            (widen)
            (list (consp before)
                  (consp after)
                  (buffer-string)
                  (marker-position m)
                  (overlay-start ov) (overlay-end ov)
                  (text-properties-at 1))))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx81_symbol_plist_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((sym (intern "neo-cx71-meta-sym")))
  (put sym 'variable-documentation "test doc")
  (put sym 'custom-type 'string)
  (put sym 'neo-cx81-custom :val)
  (list (get sym 'variable-documentation)
        (get sym 'custom-type)
        (get sym 'neo-cx81-custom)
        (get sym 'no-such)
        (symbol-plist sym)
        (plist-member (symbol-plist sym) 'custom-type)
        (remprop sym 'neo-cx81-custom)
        (get sym 'neo-cx81-custom)))
"##,
    );
}

#[test]
fn div_cx81_obarray_hash_table_internals() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ob (make-obarray 31)))
  (dotimes (i 20)
    (intern (format "sym-%d" i) ob))
  (list (obarrayp ob)
        (hash-table-p ob)
        (hash-table-count ob)
        (> (hash-table-count ob) 0)
        (intern-soft "sym-0" ob)
        (intern-soft "sym-19" ob)
        (intern-soft "sym-20" ob)))
"##,
    );
}
