//! Complex combo batch 332 — `pcase`/`rx`/`gv`/`backquote` ultimate:
//! pcase with pred/app/quote/let/or/and/not/guard/map patterns, rx
//! construction with eval/let-eval, gv setf expander definition,
//! backquote deeply nested splicing chains.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx332_pcase_pred_app_quote_let_patterns() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (v)
          (pcase v
            ((pred integerp) :int)
            ((pred stringp) :str)
            ((pred consp) :cons)
            ((pred vectorp) :vec)
            ((pred null) :nil)
            ((pred symbolp) :sym)))
        '(42 "hello" (1 2) [1 2] nil alpha))
"##,
    )
}

#[test]
fn div_cx332_pcase_or_and_not_guard_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (v)
          (pcase v
            ((or 'yes 'y 'true) :yes)
            ((and (pred integerp) (pred (> _ 100))) :big-int)
            ((and (pred stringp) (app length len) (pred (> len 3))) (list :long-str len))
            ((and n (guard (and (integerp n) (evenp n))) :even)))
        '(yes 200 "hello" 42 "ab" 'no 99))
"##,
    )
}

#[test]
fn div_cx332_pcase_map_pattern_with_hash_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((ht (make-hash-table :test 'equal)))
      (puthash :name "alpha" ht)
      (puthash :age 30 ht)
      (pcase ht
        ((map (:name name) (:age age))
         (list :parsed name age))
        (_ :no-match)))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx332_pcase_destructuring_complex_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (v)
          (pcase v
            (`(start (,a ,b) ,c) (list :start-pattern a b c))
            (`(mid (,a . ,rest) ,c) (list :mid-pattern a rest c))
            (`(end . ,rest) (list :end-pattern rest))
            (_ :other)))
        '((start (1 2) 3) (mid (1 2 3) 4) (end one two three)))
"##,
    )
}

#[test]
fn div_cx332_rx_construction_with_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((kw 'identifier)
      (ws '(* (any " \t"))))
  (list (rx-to-string `(seq bos ,kw ,ws ":" ,ws (+ (any "a-zA-Z0-9_")) eos))
        (rx-let-eval ((ident () `(seq (any "a-zA-Z_") (* (any "a-zA-Z0-9_")))))
          (rx-to-string '(seq bos (ident) eos)))))
"##,
    )
}

#[test]
fn div_cx332_gv_setf_expander_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'gv)
      (gv-define-setter neo-cx332-access
        (store vec idx)
        `(aset ,vec ,idx ,store))
      (let ((v [1 2 3 4 5]))
        (setf (neo-cx332-access v 0) 100)
        v))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx332_backquote_deeply_nested_splicing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((inner '(a b c))
      (middle '(d e f))
      (outer '(g h i)))
  (list `(start ,inner ,@(list 1 2 3) ,middle ,@(mapcar #'1+ '(10 20)) ,outer end)
        `((,@inner ,@middle ,@outer))
        `(nested (deep (,inner)) ,@middle ,@outer)))
"##,
    )
}

#[test]
fn div_cx332_cl_letf_with_symbol_function_override() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((orig-fn (symbol-function '+)))
  (cl-letf (((symbol-function 'neo-cx332-temp-fn)
             (lambda (x) (* x 100))))
    (push (neo-cx332-temp-fn 5) '())))
"##,
    )
}

#[test]
fn div_cx332_pcase_let_destructuring_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(pcase-let* ((`(,a ,b) (list 1 2))
             (`(,c ,d) (list 3 4)))
  (list a b c d))
"##,
    )
}

#[test]
fn div_cx332_pcase_rx_gv_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((data '("alpha" 42 (1 2 3) [a b c])))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert (format "Pcase/rx/gv mega: %S" data))
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 10))
          (ov (make-overlay 4 20)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 28)
      (let ((state (list (mapcar (lambda (v)
                                   (pcase v
                                     ((pred stringp) :str)
                                     ((pred integerp) :int)
                                     ((pred consp) :cons)
                                     ((pred vectorp) :vec)))
                                 data)
                         (rx-to-string '(seq bos (+ word) eos))
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen()
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1)))))))
"##,
    )
}
