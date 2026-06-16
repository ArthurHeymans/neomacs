//! Complex combo batch 89 — keyboard macros / registers / kmacro
//! persistence, with `read-key-sequence`, `execute-kbd-macro`, register
//! save/restore with multiple kinds, and point/mark preservation.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx89_execute_kbd_macro_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello")
  (goto-char 1)
  (execute-kbd-macro (kbd "C-a C-e"))
  (list (point) (buffer-string)))
"##,
    );
}

#[test]
fn div_cx89_execute_kbd_macro_with_insertion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "AB")
      (goto-char 2)
      (execute-kbd-macro "X")
      (list (point) (buffer-string)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx89_register_to_string_and_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((reg ?a))
      (set-register reg "text content")
      (set-register ?b 99)
      (set-register ?c '(1 2 3))
      (set-register ?d [vector content])
      (list (get-register ?a)
            (get-register ?b)
            (get-register ?c)
            (get-register ?d)
            (get-register ?z)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx89_register_with_rectangle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "AAA\nBBB\nCCC\n")
      (push-mark 1)
      (goto-char 11)
      (let ((reg ?r))
        (copy-rectangle-to-register reg 1 11)
        (list (get-register reg)
              (consp (get-register reg)))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx89_kmacro_define_run_and_save() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'kmacro)
      (fset 'neo-cx89-mac (kbd "C-a C-e"))
      (with-temp-buffer
        (insert "test")
        (goto-char 1)
        (neo-cx89-mac)
        (list (point) (buffer-string)))
      (list (fboundp 'neo-cx89-mac)
            (commandp 'neo-cx89-mac)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx89_register_jump_with_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((reg ?j))
  (with-temp-buffer
    (insert "register jump text")
    (goto-char 10)
    (point-to-register reg))
  (let ((jumped-to (get-register reg)))
    (list (markerp jumped-to)
          (integerp jumped-to))))
"##,
    );
}

#[test]
fn div_cx89_window_configuration_register() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((reg ?w))
  (window-configuration-to-register reg)
  (split-window)
  (let ((n-with-split (length (window-list))))
    (jump-to-register reg)
    (let ((n-restored (length (window-list))))
      (list n-with-split n-restored (get-register reg)))))
"##,
    );
}

#[test]
fn div_cx89_bookmark_set_and_jump() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'bookmark)
      (with-temp-buffer
        (insert "bookmark test content")
        (goto-char 8)
        (bookmark-set "neo-cx89-bm"))
      (let ((bm (assoc "neo-cx89-bm" bookmark-alist)))
        (list bm
              (bookmark-get-bookmark "neo-cx89-bm")
              (bookmark-get-position "neo-cx89-bm"))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx89_kmacro_append_with_counter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'kmacro)
      (setq kmacro-counter 0)
      (setq kmacro-counter-format-start "%d")
      (let ((m1 (vconcat "ABC"))
            (m2 (vconcat "DEF")))
        (list (aref m1 0)
              (aref m2 0)
              (length m1) (length m2)
              (kmacro-p m1))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx89_execute_kbd_macro_count_iterations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "hello")
      (goto-char 1)
      (execute-kbd-macro "X" 3)
      (list (point) (buffer-string)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx89_register_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "Register test content here")
  (put-text-property 1 7 'face 'bold)
  (let ((m (set-marker (make-marker) 10))
        (ov (make-overlay 4 18)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (point-to-register ?p)
    (narrow-to-region 2 22)
    (let ((state (list (buffer-string)
                       (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (text-properties-at 1)
                       (get-register ?p))))
      (undo)
      (widen)
      (list state (buffer-string) (marker-position m)
            (overlay-start ov) (overlay-end ov)
            (text-properties-at 1)))))
"##,
    );
}

#[test]
fn div_cx89_register_keys_unicode_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((regs '(?α ?β ?γ ?δ ?ε)))
  (dolist (r regs)
    (set-register r (format "value-for-%c" r)))
  (mapcar (lambda (r) (cons r (get-register r))) regs))
"##,
    );
}
