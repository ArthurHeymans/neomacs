//! Complex combo divergence probes batch 5 — adjacent to known bugs.
//!
//! coding-system-for-read let-binding + subprocess (parallel to process-env
//! propagation bug), buffer-read-only interactions, modification-hook
//! inhibition, print truncation + circular, window-config + overlay/marker
//! restore, circular vector, closure-over-loop, timer + process wait,
//! remap key binding, custom error hierarchy, print-length + print-level.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx5_coding_system_for_read_subprocess() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coding-system-for-read 'utf-8-unix))
  (with-temp-buffer
    (call-process "printf" nil t nil "caf\\303\\251")
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx5_buffer_read_only_set_modified() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello")
  (setq buffer-read-only t)
  (let ((inhibit-read-only t))
    (insert "X")
    (list (buffer-modified-p) (buffer-string)))
  (setq buffer-read-only nil)
  (set-buffer-modified-p nil)
  (list (buffer-modified-p) buffer-read-only))
"##,
    );
}

#[test]
fn div_cx5_inhibit_modification_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (fired)
  (with-temp-buffer
    (add-hook 'after-change-functions
              (lambda (beg end len) (push :fired fired)) nil t)
    (let ((inhibit-modification-hooks t))
      (insert "X"))
    (insert "Y"))
  fired)
"##,
    );
}

#[test]
fn div_cx5_print_length_plus_level_circular() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((print-length 3) (print-level 2) (print-circle t))
  (prin1-to-string '((1 2 3 4 5) (6 7 8 9 10) (11 12 13) (14 15))))
"##,
    );
}

#[test]
fn div_cx5_window_config_overlay_marker_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefgh")
  (let ((m (set-marker (make-marker) 3))
        (ov (make-overlay 2 5)))
    (overlay-put ov 'face 'bold)
    (let ((cfg (current-window-configuration)))
      (goto-char 6)
      (let ((p1 (point)) (m1 (marker-position m)))
        (set-window-configuration cfg)
        (list p1 (point) m1 (marker-position m)
              (overlay-start ov) (overlay-end ov))))))
"##,
    );
}

#[test]
fn div_cx5_circular_vector_print_circle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((v (vector 1 2 3)) (print-circle t))
  (aset v 2 v)
  (prin1-to-string v))
"##,
    );
}

#[test]
fn div_cx5_closure_over_loop_lexical() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((lexical-binding t))
  (mapcar #'funcall
          (let (acc)
            (dotimes (i 3)
              (push (byte-compile (lambda () i)) acc))
            (nreverse acc))))
"##,
    );
}

#[test]
fn div_cx5_timer_fires_during_process_wait() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (fired)
  (run-with-timer 0 nil (lambda () (setq fired :timer-fired)))
  (let ((p (make-process :name "neo-cx5-t" :command '("true"))))
    (accept-process-output p 1))
  fired)
"##,
    );
}

#[test]
fn div_cx5_remap_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((m (make-sparse-keymap)))
  (define-key m [remap forward-char] 'my-forward)
  (list (lookup-key m [remap forward-char])
        (command-remapping 'forward-char m)
        (eq (lookup-key m (kbd "C-f")) 'my-forward)))
"##,
    );
}

#[test]
fn div_cx5_custom_error_hierarchy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (define-error 'neo-cx5-error "Custom error" '(error))
  (define-error 'neo-cx5-sub-error "Sub error" '(neo-cx5-error))
  (list (condition-case e (signal 'neo-cx5-sub-error "msg") (neo-cx5-error :caught-parent) (error :missed))
        (condition-case e (signal 'neo-cx5-sub-error "msg") (neo-cx5-sub-error :caught-exact) (error :missed))))
"##,
    );
}

#[test]
fn div_cx5_set_multibyte_multiple_raw_bytes_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 202 65 66))
  (set-buffer-multibyte t)
  (list (length (buffer-string)) (point-max)
        (char-after 1) (char-after 2) (char-after 3) (char-after 4)))
"##,
    );
}

#[test]
fn div_cx5_process_sentinel_lifecycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (sentinel-fired)
  (let ((p (make-process :name "neo-cx5-sl" :command '("true")
                         :sentinel (lambda (proc event) (push event sentinel-fired)))))
    (accept-process-output p 2))
  (if sentinel-fired (car sentinel-fired) :no-sentinel))
"##,
    );
}

#[test]
fn div_cx5_read_circle_vector_labels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((v (car (read-from-string "#1=[a b #1#]"))))
  (eq (aref v 2) v))
"##,
    );
}

#[test]
fn div_cx5_sort_stability_plist_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(sort (copy-sequence '((1 . :a) (2 . :b) (1 . :c) (3 . :d) (1 . :e) (2 . :f)))
      (lambda (x y) (< (car x) (car y))))
"##,
    );
}

#[test]
fn div_cx5_char_table_range_t_syntax_override() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((st (make-syntax-table)))
  (set-char-table-range st t (string-to-syntax "."))
  (with-temp-buffer
    (with-syntax-table st
      (insert "(a)b(c)")
      (goto-char 1)
      (condition-case e (progn (forward-sexp) (point)) (error (car e)))))
"##,
    );
}

#[test]
fn div_cx5_after_change_functions_inhibit_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (log)
  (with-temp-buffer
    (add-hook 'after-change-functions (lambda (b e l) (push :change log)) nil t)
    (insert "a")
    (let ((inhibit-modification-hooks t))
      (insert "b")
      (insert "c"))
    (insert "d"))
  (reverse log))
"##,
    );
}

#[test]
fn div_cx5_buffer_read_only_var_write_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-file "neo-cx5-ro-")))
  (with-temp-buffer
    (setq buffer-read-only t)
    (insert "hello")
    (write-region (buffer-string) nil f nil 0))
  (prog1 (with-temp-buffer (insert-file-contents f) (buffer-string))
    (ignore-errors (delete-file f))))
"##,
    );
}

#[test]
fn div_cx5_overlay_invisible_line_move() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "line1\nline2\nline3\nline4\n")
  (put-text-property 7 13 'invisible t)
  (goto-char 1)
  (forward-line 2)
  (point))
"##,
    );
}

#[test]
fn div_cx5_cl_typep_defstruct_hierarchy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (cl-defstruct neo-cx5-base)
  (cl-defstruct (neo-cx5-sub (:include neo-cx5-base)) field)
  (let ((o (make-neo-cx5-sub :field 42)))
    (list (cl-typep o 'neo-cx5-base)
          (cl-typep o 'neo-cx5-sub)
          (cl-typep o 'neo-cx5-base))))
"##,
    );
}

#[test]
fn div_cx5_format_spec_nested_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (format-spec "%a %b %% literal" '((97 . "x") (98 . "y")))
  (error (cons 'errored (car e))))
"##,
    );
}
