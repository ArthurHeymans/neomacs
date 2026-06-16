//! Complex combo batch 269 — `backquote` deep nesting with `,@` splicing
//! chains, `defmacro` with `&environment`, `cl-macrolet` with `&body`/
//! `&whole`, `with-silent-modifications` hook suppression verification.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx269_backquote_deeply_nested_splicing() {
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
fn div_cx269_defmacro_with_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(defmacro neo-cx269-env-macro (form &environment env)
  (list 'quote (list :lexical (bound-and-true-p lexical-binding)
                     :env-present (not (null env))))))
(let ((lexical-binding t))
  (list (neo-cx269-env-macro test)
        (let ((lexical-binding nil))
          (neo-cx269-env-macro test2))))
"##,
    )
}

#[test]
fn div_cx269_cl_macrolet_with_body() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-macrolet ((double-when (cond &body body)
                `(if ,cond (progn ,@body) nil)))
  (let ((x 10))
    (list (double-when (> x 5) (cl-incf x) (cl-incf x))
          x)))
"##,
    )
}

#[test]
fn div_cx269_cl_macrolet_with_whole() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-macrolet ((trace-call (&whole form name &rest args)
                `(list :traced ',name (list ,@args) :form ',form)))
  (trace-call alpha 1 2 3))
"##,
    )
}

#[test]
fn div_cx269_with_silent_modifications_suppression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (calls)
  (with-temp-buffer
    (add-hook 'after-change-functions
              (lambda (&rest _) (push :change calls)) nil t)
    (insert "before")
    (let ((before-silent (length calls)))
      (setq calls nil)
      (with-silent-modifications
        (insert "MUTED1")
        (insert "MUTED2"))
      (let ((during-silent (length calls)))
        (setq calls nil)
        (insert "after")
        (list before-silent during-silent (length calls))))))
"##,
    )
}

#[test]
fn div_cx269_combine_change_calls_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (calls)
  (with-temp-buffer
    (buffer-enable-undo)
    (add-hook 'after-change-functions
              (lambda (beg end len) (push (list beg end len) calls)) nil t)
    (insert "ABCDEFGHIJ")
    (setq calls nil)
    (combine-change-calls 3 7
      (goto-char 3)
      (insert "X")
      (delete-region 4 6)
      (insert "YY"))
    (list (nreverse calls) (buffer-string))))
"##,
    )
}

#[test]
fn div_cx269_backquote_with_conditional_splicing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((include-a t)
      (include-b nil)
      (items '(x y z)))
  (list `(start ,@(when include-a '(:a)) ,@(when include-b '(:b)) ,@items end)
        `(data ,@(mapcan (lambda (x) (list x x)) '(1 2 3)))))
"##,
    )
}

#[test]
fn div_cx269_defmacro_recursive_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(defmacro neo-cx269-my-while (cond &rest body)
  (declare (indent 1))
  `(if ,cond (progn ,@body (neo-cx269-my-while ,cond ,@body)) nil))
(let ((result (macroexpand '(neo-cx269-my-while (> x 0) (cl-decf x)))))
  (list (consp result) (eq (car result) 'if)))
"##,
    )
}

#[test]
fn div_cx269_pcase_with_let_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (v)
          (pcase v
            ((let expanded (upcase (format "%s" v)))
             (list :expanded expanded))
            (_ :other)))
        '("alpha" "beta" 42))
"##,
    )
}

#[test]
fn div_cx269_backquote_macro_silent_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((items '(alpha beta gamma))
      (calls nil))
  (with-temp-buffer
    (buffer-enable-undo)
    (add-hook 'after-change-functions (lambda (&rest _) (push :ch calls)) nil t)
    (insert (format "Items: %S" items))
    (put-text-property 1 5 'face 'bold)
    (let ((m (set-marker (make-marker) 10))
          (ov (make-overlay 4 18)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 25)
      (with-silent-modifications
        (insert "SILENT"))
      (let ((state (list items
                         (length calls)
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
    )
}
