//! Complex combo batch 66 — error / condition / signal hierarchy deep, with
//! catch-throw chains, unwind-protect ordering, error message formatting,
//! `condition-case` error-data propagation, and `ignore-errors`.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx66_error_data_propagation_car_of_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (condition-case e (signal 'wrong-type-argument '(listp 10)) (error e))
 (condition-case e (signal 'wrong-type-argument '("extra" detail)) (error e))
 (condition-case e (signal 'args-out-of-range '(5 0 3)) (error e))
 (condition-case e (error "formatted: %s" "msg") (error e))
 (condition-case e (signal 'file-missing '("/no/such/path")) (file-error e))
 (condition-case e (signal 'file-missing '("/no/such/path")) (error e)))
"##,
    );
}

#[test]
fn div_cx66_catch_throw_through_unwind_protect_ordering() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (trace)
  (prog1
      (catch 'outer
        (unwind-protect
            (catch 'inner
              (push :before-inner trace)
              (throw 'outer :caught-outer)
              (push :never-inner trace))
          (push :unwind-inner trace))
        (push :after-outer-body trace)
        :never-reached)
    (push :end trace))
  trace)
"##,
    );
}

#[test]
fn div_cx66_nested_unwind_protect_with_error_in_cleanup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (trace)
  (condition-case outer
      (unwind-protect
          (error "primary error")
        (push :enter-cleanup trace)
        (error "secondary error during cleanup")
        (push :after-secondary-in-cleanup trace))
    (error
     (push (cons :outer-caught outer) trace)))
  trace)
"##,
    );
}

#[test]
fn div_cx66_error_with_backtrace_in_condition_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (+ 1 "not a number"))
  (wrong-type-argument
   (list :caught (car e) (cdr e)))
  (error
   (list :uncaught (car e) (cdr e))))
"##,
    );
}

#[test]
fn div_cx66_user_defined_error_symbols_with_define_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (define-error 'neo-cx66-my-error "A custom error" '(error my-errors))
      (signal 'neo-cx66-my-error '("the detail data"))
      :never-reached)
  (neo-cx66-my-error (list :my-error-caught (cadr e)))
  (error (list :other-error (car e) (cadr e))))
"##,
    );
}

#[test]
fn div_cx66_signal_with_nil_data_and_empty_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (condition-case e (signal 'neo-cx66-err nil) (error e))
 (condition-case e (signal 'neo-cx66-err '()) (error e))
 (condition-case e (signal 'neo-cx66-err '("single")) (error e))
 (condition-case e (signal 'neo-cx66-err '("a" "b" "c")) (error e)))
"##,
    );
}

#[test]
fn div_cx66_condition_case_no_handler_match_propagates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case outer
    (condition-case inner
        (signal 'wrong-type-argument '(integerp "x"))
      (file-error (list :file-handler inner)))
  (wrong-type-argument
   (list :caught-outer outer))
  (error
   (list :caught-other outer)))
"##,
    );
}

#[test]
fn div_cx66_throw_with_marker_overlay_textprop_narrow_unwind_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (trace)
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    (put-text-property 1 5 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 3 20)
      (let ((result
             (catch 'done
               (unwind-protect
                   (progn
                     (delete-region 5 10)
                     (push (list :body (buffer-string)
                                 (marker-position m)
                                 (overlay-start ov)) trace)
                     (throw 'done :thrown)
                     (push :never trace))
                 (push (list :unwind (buffer-string)
                             (marker-position m)
                             (overlay-end ov)) trace)))))
        (undo)
        (widen)
        (list result (nreverse trace)
              (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    );
}

#[test]
fn div_cx66_ignore_errors_returns_nil_on_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (ignore-errors (+ 1 2))
 (ignore-errors (error "boom"))
 (ignore-errors (signal 'wrong-type-argument '(stringp 5)))
 (ignore-errors (car 'x))
 (ignore-errors (aref "abc" 99))
 (ignore-errors (/ 1 0)))
"##,
    );
}

#[test]
fn div_cx66_handler_only_condition_case_with_no_body_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (condition-case nil
     (error "no handlers, no var")
   (error))
 (condition-case nil
     (progn (error "still no handlers") :never)
   (error)))
"##,
    );
}

#[test]
fn div_cx66_deferred_errors_via_condition_case_after_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((form-to-eval '(+ 1 "two")))
  (list
   form-to-eval
   (condition-case e (eval form-to-eval t) (error (cons (car e) (cadr e))))
   (condition-case e (eval form-to-eval 'lexical) (error (cons (car e) (cadr e))))))
"##,
    );
}

#[test]
fn div_cx66_quit_signal_handling_with_catch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (signal 'quit nil)
      :never)
  (quit (list :caught-quit e))
  (error (list :other e)))
"##,
    );
}

#[test]
fn div_cx66_error_in_unwind_protect_body_after_throw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (trace)
  (catch 'tag
    (unwind-protect
        (throw 'tag :value)
      (condition-case cleanup-err
          (error "during cleanup")
        (error (push (cons :cleanup cleanup-err) trace)))))
  trace)
"##,
    );
}

#[test]
fn div_cx66_error_during_printing_with_circular_ref_print_circle_off() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((x (list 1 2 3)))
  (setcdr (cddr x) x)
  (list (let ((print-circle t)) (prin1-to-string x))
        (condition-case e
            (let ((print-circle nil)) (prin1-to-string x))
          (error (cons (car e) (cadr e))))))
"##,
    );
}
