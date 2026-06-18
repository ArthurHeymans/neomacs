//! Complex combo batch 402 — 20 divergence probes on fresh interaction
//! surfaces: face-remapping, process filter + message, completion tables,
//! json-parse edge cases, color-values, documentation-property,
//! defvaralias / indirect-variable, local key remapping, syntax-ppss
//! after text-prop changes, font-lock-add-keywords, add-hook with depth,
//! batch minibuffer behavior, truncate-string-to-width with display,
//! pos-visible-in-window-p, count-screen-lines, font-spec/match,
//! format-prompt, and xml/dom parsing.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// face-remapping: face-remap-add-relative temporarily alters
/// face attributes; Neomacs may not apply the remapping.
#[test]
fn div_cx402_face_remapping_add_relative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((remap (face-remap-add-relative 'bold '((:foreground "red")))))
  (unwind-protect
      (list (face-attribute 'bold :foreground nil 'default)
            (face-remap-p 'bold))
    (face-remap-remove-relative remap)))
"##,
    );
}

/// Process output collected into a buffer using :buffer argument
/// directly — no filter closure needed.
#[test]
fn div_cx402_process_buffer_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx402-out*")))
  (let ((proc (make-process :name "neo-cx402-out"
                            :command '("sh" "-c" "echo hello from 402")
                            :connection-type 'pipe :buffer buf)))
    (accept-process-output proc 2))
  (prog1 (with-current-buffer buf
           (string-trim-right (buffer-string)))
    (kill-buffer buf)))
"##,
    );
}

/// completion-try-completion with different table types:
/// obarray, hash-table, alist, and function table.
#[test]
fn div_cx402_completion_table_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((alist '(("apple" . 1) ("apply" . 2) ("apt" . 3)))
      (ht (let ((h (make-hash-table :test 'equal)))
            (puthash "banana" 1 h)
            (puthash "band" 2 h)
            (puthash "bang" 3 h)
            h)))
  (list (try-completion "app" alist)
        (try-completion "ban" ht)
        (try-completion "zzz" alist)
        (all-completions "ap" alist)
        (all-completions "ban" ht)))
"##,
    );
}

/// json-parse-string edge cases: empty, nested, with unicode
/// escapes and unusual values.
#[test]
fn div_cx402_json_parse_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((json-encoding-type 'json-object-type))
  (list (json-parse-string "{}")
        (json-parse-string "{\"a\":1,\"b\":2}")
        (condition-case e
            (json-parse-string "{invalid}")
          (error (car e)))
        (json-parse-string "[1,2,3]")
        (json-parse-string "{\"x\":\"café\"}")))
"##,
    );
}

/// color-values + color-name-to-rgb + defined-colors:
/// color name resolution may differ between the two Emacsen.
#[test]
fn div_cx402_color_values_name_to_rgb() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (color-values "red")
      (color-name-to-rgb "red")
      (color-values "#ff0000")
      (color-name-to-rgb "#ff0000")
      (color-values "AliceBlue")
      (member "red" (defined-colors)))
"##,
    );
}

/// documentation-property for symbol plist vs function doc:
/// Neomacs may return different docstrings or nil.
#[test]
fn div_cx402_documentation_property_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (put 'neo-cx402-sym 'variable-documentation "my var doc")
  (put 'neo-cx402-sym 'custom-documentation "my custom doc")
  (list (documentation-property 'neo-cx402-sym 'variable-documentation)
        (documentation-property 'neo-cx402-sym 'custom-documentation)
        (documentation-property 'forward-word 'function-documentation t)))
"##,
    );
}

/// defvaralias + indirect-variable: variable aliases may not
/// resolve correctly in Neomacs.
#[test]
fn div_cx402_defvaralias_indirect_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((a (make-symbol "neo-cx402-a"))
      (b (make-symbol "neo-cx402-b")))
  (set a 42)
  (defvaralias a b)
  (list (indirect-variable a)
        (indirect-variable b)
        (symbol-value a)
        (symbol-value b)
        (boundp a)
        (boundp b)))
"##,
    );
}

/// local-set-key / global-set-key with command remapping:
/// key lookup with remapped commands may diverge.
#[test]
fn div_cx402_key_remapping_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (local-set-key (kbd "C-c C-n") 'forward-line)
  (local-set-key (kbd "C-c C-p") 'previous-line)
  (list (keymap-lookup nil "C-c C-n")
        (keymap-lookup nil "C-c C-p")
        (where-is-internal 'forward-line nil t)))
"##,
    );
}

/// syntax-ppss after text property syntax changes:
/// point-to-point syntax scan may not reflect property changes.
#[test]
fn div_cx402_syntax_ppss_after_textprop_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a b c) (d e f)")
  (let ((before (syntax-ppss 10)))
    (put-text-property 3 4 'syntax-table (string-to-syntax ")"))
    (list before
          (syntax-ppss 10)
          (syntax-ppss 4))))
"##,
    );
}

/// font-lock-add-keywords + font-lock-fontify-buffer in
/// emacs-lisp-mode: custom keyword fontification.
#[test]
fn div_cx402_font_lock_add_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "defun my-fun (x) (* x 2)")
    (font-lock-add-keywords nil '(("\\<my-fun\\>" 0 'bold)))
    (font-lock-fontify-buffer)
    (list (get-text-property 7 'face)
          (get-text-property 1 'face))))
"##,
    );
}

/// add-hook with depth and local: hook ordering with depth
/// may differ.
#[test]
fn div_cx402_add_hook_depth_local() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((hook (make-symbol "neo-cx402-hook"))
      (calls ()))
  (add-hook hook (lambda () (push :a calls)) 50)
  (add-hook hook (lambda () (push :b calls)) 0)
  (add-hook hook (lambda () (push :c calls)) 100)
  (let ((before (copy-sequence (symbol-value hook))))
    (run-hooks hook)
    (list before (nreverse calls))))
"##,
    );
}

/// batch minibuffer behavior: read-string / completing-read
/// in --batch mode — GNU signals error, Neomacs may handle
/// differently.
#[test]
fn div_cx402_batch_minibuffer_read() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (read-string "prompt: " "default") (error (car e)))
      (condition-case e (read-from-minibuffer "prompt: " "default") (error (car e)))
      (condition-case e (completing-read "prompt: " '("a" "b" "c")) (error (car e))))
"##,
    );
}

/// truncate-string-to-width with display text property:
/// the display substitution should affect visual width.
#[test]
fn div_cx402_truncate_string_to_width_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "abcde"))
  (put-text-property 2 3 'display "XXXX" s)
  (list (truncate-string-to-width s 3)
        (truncate-string-to-width s 5)
        (truncate-string-to-width s 7 nil nil t)
        (truncate-string-to-width s 2 nil nil t)))
"##,
    );
}

/// pos-visible-in-window-p with display property + invisible
/// text: visibility detection may differ.
#[test]
fn div_cx402_pos_visible_in_window_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghij\nklmnopqrst\nuvwxyz")
  (put-text-property 3 6 'display "ZZZ")
  (put-text-property 10 15 'invisible t)
  (list (pos-visible-in-window-p 1)
        (pos-visible-in-window-p 4)
        (pos-visible-in-window-p 12)
        (pos-visible-in-window-p 30)))
"##,
    );
}

/// count-screen-lines with display property, invisible text,
/// and truncation: screen line count may diverge.
#[test]
fn div_cx402_count_screen_lines_display_invisible() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abc def ghi jkl mno pqr stu vwx yz")
  (put-text-property 5 9 'display "ZZ")
  (put-text-property 15 20 'invisible t)
  (list (count-screen-lines (point-min) (point-max))
        (count-screen-lines (point-min) 12)
        (count-screen-lines 10 (point-max))))
"##,
    );
}

/// font-spec + font-match: font specification and matching
/// may return different results in batch mode.
#[test]
fn div_cx402_font_spec_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((spec (font-spec :family "monospace" :size 12)))
  (list (fontp spec)
        (font-get spec :family)
        (font-get spec :size)
        (condition-case e
            (font-match spec)
          (error (car e)))))
"##,
    );
}

/// format-prompt with default value: format-prompt formatting
/// may differ between the two.
#[test]
fn div_cx402_format_prompt_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format-prompt "Enter name" nil)
      (format-prompt "Enter name" "default")
      (format-prompt "Select file" "/tmp/foo"))
"##,
    );
}

/// xml / dom parsing: xml-parse-region / libxml-parse-xml-region
/// may produce different DOM structure.
#[test]
fn div_cx402_xml_parse_dom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "<root><item id='1'>alpha</item><item id='2'>beta</item></root>")
      (let ((dom (libxml-parse-xml-region (point-min) (point-max))))
        (list (car dom)
              (length dom)
              (caadr dom)
              (car (last dom)))))
  (error (car e)))
"##,
    );
}

/// split-string with multibyte separators and limit:
/// multibyte aware splitting may diverge.
#[test]
fn div_cx402_split_string_multibyte_sep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (split-string "aαbβcγd" "α\\|β\\|γ")
      (split-string "hello世界world" "世界")
      (split-string "one::two::three" "::" t)
      (split-string "x..y..z" "\\.\\."))
"##,
    );
}

/// window-text-pixel-size with invisible text and overlay:
/// pixel size measurement may not exclude invisible spans.
#[test]
fn div_cx402_window_text_pixel_size_invisible_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "visible1\nvisible2\nvisible3\n")
  (let ((ov (make-overlay 1 10)))
    (overlay-put ov 'invisible t))
  (list (window-text-pixel-size)
        (car (window-text-pixel-size))
        (cdr (window-text-pixel-size))))
"##,
    );
}
