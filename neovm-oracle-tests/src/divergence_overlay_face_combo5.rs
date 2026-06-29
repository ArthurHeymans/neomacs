//! Complex/combo divergence probes (batch 5): font-lock actual fontification,
//! marker↔overlay position coupling under insert, undo with simultaneous
//! text-property + overlay changes, button.el buttons, narrowing + marker +
//! overlay. These exercise deep interactions neomacs is likely to implement
//! only partially.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- font-lock actual fontification -----------------------------------------

#[test]
fn div_combo5_font_lock_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(defun foo ()\n  (+ 1 2))\n")
  (font-lock-fontify-buffer)
  (list (get-text-property 2 'face)
        (get-text-property 9 'face)
        (get-text-property 15 'face)))
"##,
        expect_test::expect![[r#""OK (font-lock-keyword-face font-lock-function-name-face nil)""#]],
    );
}

#[test]
fn div_combo5_font_lock_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(defvar my-var 42)")
  (font-lock-fontify-buffer)
  (list (get-text-property 2 'face)
        (get-text-property 9 'face)
        (get-text-property 15 'face)))
"##,
        expect_test::expect![[r#""OK (font-lock-keyword-face font-lock-variable-name-face nil)""#]],
    );
}

#[test]
fn div_combo5_font_lock_elisp_string_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(message \"hi\") ; note")
  (font-lock-fontify-buffer)
  (list (get-text-property 10 'face)
        (get-text-property 16 'face)))
"##,
        expect_test::expect![[r#""OK (font-lock-string-face font-lock-comment-delimiter-face)""#]],
    );
}

#[test]
fn div_combo5_font_lock_mode_toggled() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(setq x 1)")
  (font-lock-mode 1)
  (font-lock-fontify-buffer)
  (get-text-property 2 'face))
"##,
        expect_test::expect![[r#""OK font-lock-keyword-face""#]],
    );
}

#[test]
fn div_combo5_font_lock_region_partial() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(defun a ()) (defun b ())")
  (font-lock-fontify-region 1 12)
  (list (get-text-property 2 'face)
        (get-text-property 14 'face)))
"##,
        expect_test::expect![[r#""OK (font-lock-keyword-face nil)""#]],
    );
}

// --- marker ↔ overlay position coupling -------------------------------------

#[test]
fn div_combo5_marker_overlay_positions_after_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let* ((ov (make-overlay 2 5))
         (m1 (set-marker (make-marker) 2))
         (m2 (set-marker (make-marker) 5)))
    (overlay-put ov 'face 'bold)
    (goto-char 3)
    (insert "XYZ")
    (list (overlay-start ov) (overlay-end ov)
          (marker-position m1) (marker-position m2))))
"##,
        expect_test::expect![[r#""OK (2 8 2 8)""#]],
    );
}

#[test]
fn div_combo5_marker_insertion_type_advance() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((m (set-marker (make-marker) 2)))
    (set-marker-insertion-type m t)
    (goto-char 2)
    (insert "X")
    (marker-position m)))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_combo5_marker_in_overlay_extent_deletion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let* ((ov (make-overlay 2 5))
         (m (set-marker (make-marker) 4)))
    (overlay-put ov 'face 'bold)
    (delete-region 3 4)
    (list (overlay-start ov) (overlay-end ov) (marker-position m))))
"##,
        expect_test::expect![[r#""OK (2 4 3)""#]],
    );
}

#[test]
fn div_combo5_marker_relocated_overlay_extents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (let ((ov (make-overlay (set-marker (make-marker) 3)
                          (set-marker (make-marker) 6))))
    (overlay-put ov 'face 'bold)
    (goto-char 8)
    (insert "X")
    (list (overlay-start ov) (overlay-end ov))))
"##,
        expect_test::expect![[r#""OK (3 6)""#]],
    );
}

// --- undo with simultaneous property + overlay changes ---------------------

#[test]
fn div_combo5_undo_prop_and_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "hello")
  (undo-boundary)
  (put-text-property 1 3 'face 'bold)
  (undo-boundary)
  (goto-char 3)
  (insert "X")
  (undo)
  (list (buffer-string) (text-properties-at 1)))
"##,
        expect_test::expect![[r#""OK (#(\"hello\" 0 2 (face nil)) (face nil))""#]],
    );
}

#[test]
fn div_combo5_undo_overlay_creation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "abcdef")
  (undo-boundary)
  (let ((ov (make-overlay 2 4)))
    (overlay-put ov 'face 'bold)
    (undo-boundary)
    (goto-char 3)
    (insert "X")
    (undo)
    (list (buffer-string) (length (overlays-at 2)))))
"##,
        expect_test::expect![[r#""OK (\"\" 0)""#]],
    );
}

#[test]
fn div_combo5_undo_set_text_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "abcdef")
  (undo-boundary)
  (set-text-properties 1 4 '(face bold mouse-face highlight))
  (undo-boundary)
  (put-text-property 1 2 'font-lock-face 'keyword)
  (undo)
  (list (text-properties-at 1) (text-properties-at 2)))
"##,
        expect_test::expect![[
            r#""OK ((font-lock-face nil face nil mouse-face nil) (face nil mouse-face nil))""#
        ]],
    );
}

#[test]
fn div_combo5_undo_multiple_boundaries_to_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "abc")
  (undo-boundary)
  (put-text-property 1 2 'face 'bold)
  (undo-boundary)
  (goto-char 1) (insert "X")
  (undo-boundary)
  (goto-char 1) (insert "Y")
  (while (condition-case nil (progn (undo) nil) (error t)))
  (list (buffer-string) (text-properties-at 1)))
"##,
        expect_test::expect![[r#""OK (#(\"abc\" 0 1 (face bold)) (face bold))""#]],
    );
}

// --- button.el --------------------------------------------------------------

#[test]
fn div_combo5_make_text_button_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "Click here now")
  (condition-case err
      (progn (make-text-button 1 6 'action (lambda (_) 'clicked))
             (list (get-text-property 1 'button)
                   (overlayp (button-at 1))
                   (button-label (button-at 1))))
    (error (cons 'errored (car err)))))
"##,
        expect_test::expect![[r#""OK ((t) nil \"Click\")""#]],
    );
}

#[test]
fn div_combo5_insert_button_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case err
    (with-temp-buffer
      (insert-button "[help]" 'type 'help-echo 'help-echo "hi")
      (list (button-at 1) (button-label (button-at 1)) (buffer-string)))
  (error (cons 'errored (car err))))
"##,
        expect_test::expect![[r#""OK (errored . error)""#]],
    );
}

#[test]
fn div_combo5_button_overlay_backed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case err
    (with-temp-buffer
      (insert "abcdef")
      (make-button 2 5 'action (lambda (_) nil))
      (list (button-at 2) (button-start (button-at 2)) (button-end (button-at 2))))
  (error (cons 'errored (car err))))
"##,
        expect_test::expect![[r#""OK (#<overlay in no buffer> 2 5)""#]],
    );
}

// --- narrowing + marker + overlay ------------------------------------------

#[test]
fn div_combo5_narrow_marker_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (let* ((ov (make-overlay 3 7))
         (m (set-marker (make-marker) 4)))
    (overlay-put ov 'face 'bold)
    (narrow-to-region 3 8)
    (goto-char 5)
    (insert "X")
    (list (overlay-start ov) (overlay-end ov)
          (marker-position m) (point-min) (point-max))))
"##,
        expect_test::expect![[r#""OK (3 8 4 3 9)""#]],
    );
}

#[test]
fn div_combo5_overlay_marker_move_overlay_after_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcdefgh")
  (let* ((m (set-marker (make-marker) 3))
         (ov (make-overlay m 6)))
    (overlay-put ov 'face 'bold)
    (goto-char 1)
    (insert "XX")
    (list (marker-position m) (overlay-start ov) (overlay-end ov))))
"##,
        expect_test::expect![[r#""OK (5 5 8)""#]],
    );
}

// --- field + narrowing + read-only combo ------------------------------------

#[test]
fn div_combo5_field_narrow_readonly_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "header\nbody line one\nbody line two")
  (put-text-property 8 20 'field 'body)
  (put-text-property 8 20 'read-only t)
  (narrow-to-region 1 20)
  (goto-char 12)
  (let ((inhibit-field-text-motion nil))
    (list (line-beginning-position)
          (condition-case err (progn (insert "X") 'inserted) (error (car err))))))
"##,
        expect_test::expect![[r#""OK (8 text-read-only)""#]],
    );
}
