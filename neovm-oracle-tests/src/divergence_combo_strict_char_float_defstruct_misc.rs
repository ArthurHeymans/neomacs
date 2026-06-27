//! Strict combo oracle probes, batch 7: char/string width of special chars,
//! float-to-string printing edge cases, fixnum/bignum boundary, cl-defstruct
//! printing and slots, overlay priority ordering, abbrev expansion, char-table
//! ranges/parent, keymap introspection, and window/frame geometry metrics.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_e2_char_width_special_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (char-width ?a)
      (char-width ?\t)
      (char-width ?\n)
      (char-width ?\ )
      (char-width 0)
      (char-width 768)
      (char-width ?あ)
      (char-width 128578)
      (string-width "a\tb")
      (string-width "a\nb")
      (string-width "あいう")
      (string-width "🙂")
      (string-width "a🙂b"))
"##,
    );
}

#[test]
fn div_e2_float_string_printing_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%s" 1.0)
      (format "%s" -0.0)
      (format "%s" 0.0)
      (format "%s" 0.1)
      (format "%s" 1.5)
      (format "%s" 3.14159)
      (format "%s" 1e20)
      (format "%s" 1e16)
      (format "%s" 1e15)
      (format "%s" 1e-5)
      (format "%s" 0.0001)
      (format "%s" 1.5e300)
      (format "%s" 123456789.0)
      (number-to-string 100.0)
      (format "%s" 100.0))
"##,
    );
}

#[test]
fn div_e2_fixnum_bignum_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list most-positive-fixnum
      (1+ most-positive-fixnum)
      (fixnump (1+ most-positive-fixnum))
      (bignump (1+ most-positive-fixnum))
      most-negative-fixnum
      (1- most-negative-fixnum)
      (fixnump most-positive-fixnum)
      (format "%s" most-positive-fixnum)
      (format "%s" (1+ most-positive-fixnum))
      (* 2 most-positive-fixnum))
"##,
    );
}

#[test]
fn div_e2_cl_defstruct_print_and_slots() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (cl-defstruct (probe-struct2 (:constructor probe-cons2 (a b)) (:copier nil))
    a b c)
  (let ((s (probe-cons2 1 2)))
    (list (type-of s)
          (probe-struct2-a s)
          (probe-struct2-b s)
          (probe-struct2-c s)
          (format "%s" s)
          (probe-struct2-p s)
          (length s))))
"##,
    );
}

#[test]
fn div_e2_overlay_priority_and_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghij")
  (let ((o1 (make-overlay 2 8))
        (o2 (make-overlay 2 8))
        (o3 (make-overlay 2 8)))
    (overlay-put o1 'priority 5)
    (overlay-put o2 'priority 10)
    (overlay-put o3 'priority 1)
    (overlay-put o1 'face 'a)
    (overlay-put o2 'face 'b)
    (overlay-put o3 'face 'c)
    (list (mapcar (lambda (o) (overlay-get o 'priority)) (overlays-at 4))
          (length (overlays-at 4))
          (mapcar (lambda (o) (overlay-get o 'face)) (overlays-at 4))
          (mapcar (lambda (o) (overlay-get o 'priority)) (overlays-in 3 5))
          (overlay-start o1)
          (overlay-end o1))))
"##,
    );
}

#[test]
fn div_e2_abbrev_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((table (make-abbrev-table)))
  (define-abbrev table "foo" "bar")
  (with-temp-buffer
    (setq-local local-abbrev-table table)
    (setq-local abbrev-mode t)
    (insert "foo")
    (list (expand-abbrev)
          (buffer-string)
          last-abbrev-text)))
"##,
    );
}

#[test]
fn div_e2_char_table_range_and_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'syntax-table nil)))
  (set-char-table-range ct 0 'zero)
  (set-char-table-range ct '(?a . ?z) 'lower)
  (list (char-table-range ct 0)
        (char-table-range ct ?a)
        (char-table-range ct ?m)
        (char-table-range ct ?A)
        (char-table-subtype ct)
        (progn (set-char-table-parent ct (make-char-table 'syntax-table))
               (char-table-parent ct))
        (char-table-range ct ?b)))
"##,
    );
}

#[test]
fn div_e2_keymap_introspection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((map (make-sparse-keymap))
      collected)
  (define-key map "a" 'cmd-a)
  (define-key map (kbd "C-c C-d") 'cmd-cd)
  (map-keymap (lambda (k v) (push (cons k v) collected)) map)
  (list (lookup-key map "a")
        (lookup-key map (kbd "C-c C-d"))
        (where-is-internal 'cmd-a map t)
        (sort (mapcar (lambda (e) (cons (car e) (cdr e))) (nreverse collected))
              (lambda (x y) (< (prefix-numeric-value (car x))
                               (prefix-numeric-value (car y)))))
        (map-keymap-prompt nil map)
        (keymapp map)
        (accessible-keymaps nil map)))
"##,
    );
}

#[test]
fn div_e2_window_edges_geometry() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b (get-buffer-create " *probe-edges*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b)
        (list (window-edges)
              (window-inside-edges)
              (window-body-width)
              (window-body-height)
              (window-total-width)
              (window-total-height)
              (window-left-column)
              (window-top-line)))
    (when (buffer-live-p b) (kill-buffer b))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_e2_frame_char_metrics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (frame-char-width)
      (frame-char-height)
      (default-font-width)
      (default-font-height)
      (frame-parameter nil 'internal-border-width)
      (frame-parameter nil 'menu-bar-lines)
      (frame-parameter nil 'tool-bar-lines)
      (frame-parameter nil 'scroll-bar-width))
"##,
    );
}

#[test]
fn div_e2_frame_parameter_numeric_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK (0 nil nil nil nil)
    // Neomacs:   OK (nil nil nil nil nil)
    // (frame-parameter nil 'line-spacing) is 0 in GNU Emacs but nil in Neomacs.
    // The fringe and divider-width parameters agree (nil in both batch frames).
    assert_oracle_parity(
        r##"
(list (frame-parameter nil 'line-spacing)
      (frame-parameter nil 'left-fringe)
      (frame-parameter nil 'right-fringe)
      (frame-parameter nil 'right-divider-width)
      (frame-parameter nil 'bottom-divider-width))
"##,
    );
}
