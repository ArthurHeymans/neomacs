//! Complex combo batch 424 — 20 probes into remaining corner cases:
//! decode-coding-region roundtrip, process-status deeper, regex backref
//! with nested groups, window-tree deeper, char-table-p extra-slot deep,
//! syntax-after deeper, category-set-all, keymap-prompt, font-unique,
//! color-lab-to-xyz, buffer-narrowed-p, line-pixel-height, window-line-height,
//! face-attribute-relative-p, default-font-width, default-line-height,
//! frame-text-lines, tool-bar-height, scroll-bar-width, fringe-colors.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// decode-coding-region after encode: roundtrip.
#[test]
fn div_cx424_decode_coding_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "héllo")
  (encode-coding-region (point-min) (point-max) 'utf-8)
  (decode-coding-region (point-min) (point-max) 'utf-8)
  (list (buffer-size) (buffer-string)))
"##,
    );
}

/// process-status deeper: exited with specific code.
#[test]
fn div_cx424_process_status_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx424-pe"
                          :command '("sh" "-c" "exit 42")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (prog1 (process-exit-status proc)
    (delete-process proc)))
"##,
    );
}

/// regex backreference with nested groups.
#[test]
fn div_cx424_regex_backref_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-match "\\(a\\(b\\)c\\)\\1" "abcabc")
      (match-string 0 "abcabc")
      (string-match "\\(a\\(b\\)c\\)\\2" "abcabcabc")
      (match-string 0 "abcabcabc"))
"##,
    );
}

/// window-tree: hierarchical window layout.
#[test]
fn div_cx424_window_tree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((tree (window-tree (selected-frame))))
  (list (listp tree)
        (consp tree)))
"##,
    );
}

/// char-table-p extra-slot deep access.
#[test]
fn div_cx424_char_table_extra_slot_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'case-table 0)))
  (list (char-table-p ct)
        (char-table-subtype ct)
        (condition-case e (set-char-table-extra-slot ct 0 'slot0) (error (car e)))
        (condition-case e (char-table-extra-slot ct 0) (error (car e)))))
"##,
    );
}

/// syntax-after with various syntax characters in multibyte.
#[test]
fn div_cx424_syntax_after_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a [b {c")
  (list (syntax-after 1) (syntax-after 4) (syntax-after 7)))
"##,
    );
}

/// category-set-all: setting all categories for a character.
#[test]
fn div_cx424_category_set_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (copy-category-table)))
  (define-category ?a "test-a" ct)
  (define-category ?b "test-b" ct)
  (modify-category-entry ?x ?a ct)
  (modify-category-entry ?x ?b ct)
  (list (char-category-set ?x ct)
        (category-docstring ?a ct)))
"##,
    );
}

/// keymap-prompt: getting/setting keymap prompt strings.
#[test]
fn div_cx424_keymap_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((map (make-sparse-keymap "Test prompt")))
  (keymap-prompt map))
"##,
    );
}

/// font-unique / font-xlfd-name: font identification.
#[test]
fn div_cx424_font_unique_xlfd() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (font-spec :family "monospace" :size 12)))
  (list (fontp f)
        (font-get f :family)
        (font-get f :size)))
"##,
    );
}

/// color-lab-to-xyz / color-xyz-to-lab: color space conversion.
#[test]
fn div_cx424_color_lab_xyz() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'color)
  (list (condition-case e (color-lab-to-xyz 50 20 -30) (error (car e)))
        (condition-case e (color-xyz-to-lab 0.5 0.5 0.5) (error (car e)))))
"##,
    );
}

/// buffer-narrowed-p / point-min-marker / point-max-marker.
#[test]
fn div_cx424_buffer_narrowed_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghij")
  (narrow-to-region 3 7)
  (list (buffer-narrowed-p)
        (marker-position (point-min-marker))
        (marker-position (point-max-marker))
        (marker-buffer (point-min-marker))))
"##,
    );
}

/// line-pixel-height / window-line-height.
#[test]
fn div_cx424_line_pixel_height() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "test line")
  (condition-case e
      (line-pixel-height)
    (error (car e))))
"##,
    );
}

/// face-attribute-relative-p: checking relative face specs.
#[test]
fn div_cx424_face_attribute_relative_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (face-attribute-relative-p '(:foreground "red"))
      (face-attribute-relative-p '(:background unspecified))
      (face-attribute-relative-p nil))
"##,
    );
}

/// default-font-width / default-line-height.
#[test]
fn div_cx424_default_font_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (default-font-width)
      (condition-case e (default-line-height) (error (car e))))
"##,
    );
}

/// frame-text-lines / tool-bar-height / scroll-bar-width.
#[test]
fn div_cx424_frame_tool_scroll() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (frame-text-lines) (error (car e)))
      (condition-case e (frame-text-cols) (error (car e)))
      (condition-case e (scroll-bar-width) (error (car e))))
"##,
    );
}

/// fringe-colors / fringe-style.
#[test]
fn div_cx424_fringe_colors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (fringe-colors) (error (car e)))
      (condition-case e (fringe-style) (error (car e))))
"##,
    );
}

/// window-max-characters / window-max-chars.
#[test]
fn div_cx424_window_max_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (condition-case e
      (list (window-max-chars w nil)
            (window-body-width w))
    (error (car e))))
"##,
    );
}

/// subr-primitive-p / subr-lambda-p / function-interactive-p.
#[test]
fn div_cx424_subr_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (lambda (x) (interactive "p") (* x 2))))
  (list (subrp (symbol-function 'car))
        (subrp (symbol-function 'if))
        (commandp f)
        (function-interactive-p f)))
"##,
    );
}

/// eventp / mouse-event-p in batch.
#[test]
fn div_cx424_event_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (eventp 'C-a)
      (eventp 'mouse-1)
      (mouse-event-p 'down-mouse-1))
"##,
    );
}
