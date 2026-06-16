//! Complex combo batch 183 — `advice` deep: `advice-add` with `:before`
//! /`:after`/`:around`/`:override`/`:filter-args`/`:filter-return` on
//! closures, subrs, macros, and eieio generics.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx183_advice_remove_restores_original() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (calls)
  (defun neo-cx183-target (x) (push (list :primary x) calls) (* x 2))
  (let ((orig (symbol-function 'neo-cx183-target)))
    (advice-add 'neo-cx183-target :before
                (lambda (x) (push (list :before x) calls))
                '((name . adv)))
    (let ((with-advice (neo-cx183-target 5)))
      (advice-remove 'neo-cx183-target 'adv)
      (let ((after-remove (neo-cx183-target 5)))
        (list with-advice after-remove (nreverse calls))))))
"##,
    );
}

#[test]
fn div_cx183_advice_filter_args_then_primary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (calls)
  (defun neo-cx183-fa (&rest args) (push (list :primary args) calls) (apply #'+ args))
  (advice-add 'neo-cx183-fa :filter-args
              (lambda (args) (mapcar (lambda (x) (* x 10)) args))
              '((name . fa-adv)))
  (let ((r (neo-cx183-fa 1 2 3)))
    (advice-remove 'neo-cx183-fa 'fa-adv)
    (list r (nreverse calls))))
"##,
    );
}

#[test]
fn div_cx183_advice_filter_return_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (calls)
  (defun neo-cx183-fr () (push :primary calls) 100)
  (advice-add 'neo-cx183-fr :filter-return
              (lambda (r) (push (list :filtered r) calls) (* r 2))
              '((name . fr-adv)))
  (let ((r (neo-cx183-fr)))
    (advice-remove 'neo-cx183-fr 'fr-adv)
    (list r (nreverse calls))))
"##,
    );
}

#[test]
fn div_cx183_advice_around_calls_next() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (calls)
  (defun neo-cx183-ar (x) (push (list :primary x) calls) (* x 3))
  (advice-add 'neo-cx183-ar :around
              (lambda (fn x)
                (push (list :around-enter x) calls)
                (let ((r (funcall fn x)))
                  (push (list :around-exit r) calls)
                  r))
              '((name . ar-adv)))
  (let ((r (neo-cx183-ar 7)))
    (advice-remove 'neo-cx183-ar 'ar-adv)
    (list r (nreverse calls))))
"##,
    );
}

#[test]
fn div_cx183_advice_override_completely() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (calls)
  (defun neo-cx183-ov (x) (push :orig calls) (* x 2))
  (advice-add 'neo-cx183-ov :override
              (lambda (x) (push :override calls) (* x 100))
              '((name . ov-adv)))
  (let ((r (neo-cx183-ov 5)))
    (advice-remove 'neo-cx183-ov 'ov-adv)
    (list r (nreverse calls))))
"##,
    );
}

#[test]
fn div_cx183_advice_before_after_combined_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (calls)
  (defun neo-cx183-combo (x) (push (list :primary x) calls) x)
  (advice-add 'neo-cx183-combo :before
              (lambda (x) (push (list :before-1 x) calls)) '((name . b1)))
  (advice-add 'neo-cx183-combo :before
              (lambda (x) (push (list :before-2 x) calls)) '((name . b2)))
  (advice-add 'neo-cx183-combo :after
              (lambda (x) (push (list :after-1 x) calls)) '((name . a1)))
  (advice-add 'neo-cx183-combo :after
              (lambda (x) (push (list :after-2 x) calls)) '((name . a2)))
  (let ((r (neo-cx183-combo 99)))
    (dolist (n '(b1 b2 a1 a2))
      (advice-remove 'neo-cx183-combo n))
    (list r (nreverse calls))))
"##,
    );
}

#[test]
fn div_cx183_advice_member_p_named() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(defun neo-cx183-mp () :ok)
(advice-add 'neo-cx183-mp :before (lambda () :a) '((name . my-advice)))
(list (advice--p (advice-member-p 'my-advice 'neo-cx183-mp))
      (advice--p (advice-member-p 'missing 'neo-cx183-mp)))
"##,
    );
}

#[test]
fn div_cx183_advice_mapc_iterate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(defun neo-cx183-mc () :ok)
(advice-add 'neo-cx183-mc :before (lambda () :a) '((name . adv-a)))
(advice-add 'neo-cx183-mc :after  (lambda () :b) '((name . adv-b)))
(let (names)
  (advice-mapc (lambda (adv props)
                 (push (plist-get props 'name) names))
               'neo-cx183-mc)
  (sort names #'symbol<))
"##,
    );
}

#[test]
fn div_cx183_advice_on_subr_builtin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let (calls)
      (advice-add 'car :around
                  (lambda (fn x) (push :around calls) (funcall fn x))
                  '((name . subr-adv)))
      (let ((r (car '(1 2 3))))
        (advice-remove 'car 'subr-adv)
        (list r (length calls) (car '(4 5 6)))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx183_advice_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (calls)
  (defun neo-cx183-mega (x) (push (list :primary x) calls) (* x 2))
  (advice-add 'neo-cx183-mega :before
              (lambda (x) (push (list :before x) calls))
              '((name . mega-adv-1)))
  (advice-add 'neo-cx183-mega :after
              (lambda (x) (push (list :after x) calls))
              '((name . mega-adv-2)))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "Advice mega test buffer content")
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)
      (let ((r (neo-cx183-mega 21)))
        (let ((state (list r (nreverse calls)
                           (buffer-string)
                           (marker-position m)
                           (overlay-start ov) (overlay-end ov)
                           (text-properties-at 1))))
          (undo)
          (widen)
          (advice-remove 'neo-cx183-mega 'mega-adv-1)
          (advice-remove 'neo-cx183-mega 'mega-adv-2)
          (list state (buffer-string) (marker-position m)
                (overlay-start ov) (overlay-end ov)
                (text-properties-at 1)))))))
"##,
    );
}
