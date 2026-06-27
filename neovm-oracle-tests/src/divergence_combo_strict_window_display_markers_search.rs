//! Strict combo oracle probes: window tree/selection, display motion and
//! screen-line counting (frame-geometry-sensitive), minibuffer state, faces
//! and colors, mark/mark-marker, search match-data / looking-back,
//! forward-comment, plist accessors, and sequence/predicate edges.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_wdm_window_tree_and_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b1 (get-buffer-create " *probe-wts-a*"))
      (b2 (get-buffer-create " *probe-wts-b*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b1)
        (let* ((root (selected-window))
               (w2 (split-window nil nil 'right)))
          (set-window-buffer w2 b2)
          (list (window-live-p w2)
                (eq (window-parent w2) (window-parent root))
                (window-combined-p w2 t)
                (count-windows)
                (length (window-list nil 'nomini))
                (eq (next-window) w2)
                (get-buffer-window b1)
                (eq (get-buffer-window b2) w2)
                (length (get-buffer-window-list b1 nil t))
                (mapcar (lambda (w) (buffer-name (window-buffer w)))
                        (window-list nil 'nomini)))))
    (when (buffer-live-p b1) (kill-buffer b1))
    (when (buffer-live-p b2) (kill-buffer b2))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_wdm_display_motion_and_screen_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK (2 23 80 151 1)
    // Neomacs:   OK (1 23 1 1 1)
    // count-screen-lines and vertical-motion wrapping diverge: GNU wraps the
    // 30-word buffer across its 80-column batch frame (2 screen lines; one
    // vertical-motion step lands at char 80), while Neomacs treats the whole
    // buffer as a single screen line — its batch frame is wider than 80.
    assert_oracle_parity(
        r##"
(let ((b (get-buffer-create " *probe-screen*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (with-current-buffer b
          (erase-buffer)
          (dotimes (i 30) (insert "word ")))
        (switch-to-buffer b)
        (list (count-screen-lines (point-min) (point-max))
              (window-text-height)
              (progn (with-current-buffer b (goto-char (point-min)))
                     (vertical-motion 1)
                     (point))
              (progn (with-current-buffer b (goto-char (point-min)))
                     (vertical-motion 2)
                     (point))
              (count-lines (point-min) (point-max))))
    (when (buffer-live-p b) (kill-buffer b))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_wdm_minibuffer_state_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((mw (minibuffer-window)))
  (list (window-live-p mw)
        (window-minibuffer-p mw)
        (eq (window-frame mw) (selected-frame))
        (eq (active-minibuffer-window) mw)
        (minibuffer-depth)
        (windowp mw)))
"##,
    );
}

#[test]
fn div_wdm_window_object_print_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK ("#<window 1 on *scratch*>" "#<window 2 on  *Minibuf-0*>" "#<window 1 on *scratch*>")
    // Neomacs:   OK ("#<window 1 on *scratch*>" "#<window 281479271677952 on  *Minibuf-0*>" "#<window 1 on *scratch*>")
    // The minibuffer window is prin1'd with its raw internal id (a huge
    // integer) instead of a small, stable window number.  The ordinary
    // selected window is printed correctly.
    assert_oracle_parity(
        r##"
(list (format "%s" (selected-window))
      (format "%s" (minibuffer-window))
      (prin1-to-string (selected-window)))
"##,
    );
}

#[test]
fn div_wdm_faces_and_colors_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (facep 'default)
      (facep 'bold)
      (facep 'nonexistent-probe-face)
      (memq 'default (face-list))
      (face-id 'default)
      (face-id 'bold)
      (color-defined-p "red")
      (color-defined-p "nonexistent-probe-color")
      (color-values "red")
      (color-values "black")
      (color-values "white")
      (defined-colors))
"##,
    );
}

#[test]
fn div_wdm_color_gray_p_named_grays() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK (t t t t nil)
    // Neomacs:   OK (nil t nil t nil)
    // color-gray-p fails to recognize the grayNN/greyNN numeric names
    // ("gray50", "grey80") as gray, while the bare name "gray" and "black"
    // are handled correctly.
    assert_oracle_parity(
        r##"
(list (color-gray-p "gray50")
      (color-gray-p "gray")
      (color-gray-p "grey80")
      (color-gray-p "black")
      (color-gray-p "red"))
"##,
    );
}

#[test]
fn div_wdm_markers_mark_and_exchange() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((m (point-marker)))
    (goto-char 3)
    (push-mark 5 t)
    (list (marker-position m)
          (mark)
          (marker-position (mark-marker))
          (progn (exchange-point-and-mark) (point))
          (mark t)
          (markerp m))))
"##,
    );
}

#[test]
fn div_wdm_search_looking_match_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "foo bar baz")
  (goto-char 1)
  (list (looking-at "foo")
        (match-beginning 0)
        (match-end 0)
        (progn (re-search-forward "b.r") (match-data t))
        (point)
        (looking-back "bar" 4)
        (looking-back "bar" 5)))
"##,
    );
}

#[test]
fn div_wdm_forward_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (modify-syntax-entry ?\; "<")
  (modify-syntax-entry ?\n ">")
  (insert "foo ;; a comment\nbar")
  (goto-char 1)
  (let ((p0 (point)))
    (forward-comment 1)
    (list p0 (point) (forward-comment (point-max)) (point))))
"##,
    );
}

#[test]
fn div_wdm_plist_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (plist-get '(:a 1 :b 2) :a)
      (plist-get '(:a 1 :b 2) :b)
      (plist-get '(:a 1 :b 2) :c)
      (plist-member '(:a 1 :b 2) :a)
      (plist-member '(:a 1 :b 2) :c)
      (let ((p (copy-tree '(:a 1 :b 2))))
        (setq p (plist-put p :c 3))
        p)
      (lax-plist-get '(a 1 b 2) 'b)
      (progn (put 'probe-sym-x 'foo 42) (get 'probe-sym-x 'foo))
      (progn (put 'probe-sym-x 'bar nil) (get 'probe-sym-x 'bar))
      (symbol-plist 'probe-sym-x))
"##,
    );
}

#[test]
fn div_wdm_sequence_and_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (length [1 2 3])
      (length "abc")
      (length '(1 2 3))
      (safe-length (cons 1 (cons 2 'oo)))
      (elt [1 2 3] 1)
      (elt "abc" 0)
      (reverse [1 2 3])
      (reverse "abc")
      (reverse '(1 2 3))
      (natnump -1)
      (natnump 0)
      (booleanp nil)
      (booleanp t)
      (booleanp 0)
      (characterp ?a)
      (characterp 65)
      (char-or-string-p "a")
      (wholenump 5))
"##,
    );
}

#[test]
fn div_wdm_get_lru_largest_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b1 (get-buffer-create " *probe-lru-a*"))
      (b2 (get-buffer-create " *probe-lru-b*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b1)
        (let ((w2 (split-window nil nil 'below)))
          (set-window-buffer w2 b2)
          (list (window-live-p (get-lru-window))
                (window-live-p (get-largest-window))
                (count-windows)
                (window-total-size (get-largest-window) nil))))
    (when (buffer-live-p b1) (kill-buffer b1))
    (when (buffer-live-p b2) (kill-buffer b2))
    (delete-other-windows)))
"##,
    );
}
