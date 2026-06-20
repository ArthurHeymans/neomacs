//! Cross-subsystem oracle tests targeting documented divergence
//! areas: faces/overlays, UTF-8/charset, display/column,
//! case-fold-search with non-ASCII, and Bidi/RTL text.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// FACES: get-text-property 'face on org buffer text
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn face_text_property_on_headline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (font-lock-mode 1)
    (insert "* TODO Task :tag:\n")
    (goto-char (point-min))
    (let ((r '()))
      ;; check face property at various positions
      (dotimes (i (1- (point-max)))
        (let ((face (get-text-property (1+ i) 'face)))
          (when face
            (push (list :pos (1+ i) :face
                        (cond ((symbolp face) face)
                              ((consp face) (car face))
                              (t :complex)))
                  r))))
      (nreverse r))))"##,
    );
}

#[test]
fn face_property_on_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (font-lock-mode 1)
    (insert "Plain *bold* /italic/ =code= text.\n")
    (goto-char (point-min))
    (let ((r '()))
      (dotimes (i (1- (point-max)))
        (let ((face (get-text-property (1+ i) 'face)))
          (when face
            (push (list :pos (1+ i)
                        :face (if (symbolp face) face (if (consp face) (car face) :complex)))
                  r))))
      (nreverse r))))"##,
    );
}

#[test]
fn face_count_total_in_org_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (font-lock-mode 1)
    (insert "* TODO [#A] Task :work:\nSCHEDULED: <2024-01-15>\nBody *bold*.\n")
    (goto-char (point-min))
    (let ((count 0))
      (dotimes (i (1- (point-max)))
        (when (get-text-property (1+ i) 'face)
          (setq count (1+ count))))
      (list :face-prop-count count))))"##,
    );
}

#[test]
fn overlay_lists_in_org_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* A\n** B\nBody.\n* C\n")
    (goto-char (point-min))
    (org-overview)
    (let ((overs (overlays-in (point-min) (point-max))))
      (list :overlay-count (length overs)
            :overlay-types (mapcar (lambda (o) (overlay-get o 'invisible)) overs)))))"##,
    );
}

#[test]
fn face_all_attributes_org_level1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-faces)
  (let ((attrs (face-all-attributes 'org-level-1)))
    (list :attr-keys (sort (mapcar #'car attrs) #'string-lessp)
          :attr-count (length attrs))))"##,
    );
}

#[test]
fn face_all_attributes_org_todo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-faces)
  (let ((attrs (face-all-attributes 'org-todo)))
    (list :attr-count (length attrs)
          :has-foreground (assq :foreground attrs)
          :has-weight (assq :weight attrs))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// CHARSET: char-charset on various characters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn charset_ascii_characters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :A (char-charset ?A)
   :a (char-charset ?a)
   :0 (char-charset ?0)
   :space (char-charset ?\s)
   :newline (char-charset ?\n)
   :exclaim (char-charset ?!)
   :tilde (char-charset ?~)))"##,
    );
}

#[test]
fn charset_high_bytes_128_255() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :byte-128 (char-charset (make-char 'eight-bit 128))
   :byte-160 (char-charset (make-char 'eight-bit 160))
   :byte-200 (char-charset (make-char 'eight-bit 200))
   :byte-255 (char-charset (make-char 'eight-bit 255))))"##,
    );
}

#[test]
fn charset_cjk_japanese() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :hiragana-a (char-charset ?あ)
   :katakana-a (char-charset ?ア)
   :kanji-nichi (char-charset ?日)
   :kanji-hon (char-charset ?本)))"##,
    );
}

#[test]
fn charset_cjk_chinese_korean() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :chinese-zhong (char-charset ?中)
   :chinese-wen (char-charset ?文)
   :korean-han (char-charset ?한)
   :korean-gug (char-charset ?국)))"##,
    );
}

#[test]
fn charset_greek_cyrillic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :alpha (char-charset ?α)
   :omega (char-charset ?ω)
   :Alpha (char-charset ?Α)
   :cyrillic-a (char-charset ?а)
   :cyrillic-ya (char-charset ?я)
   :cyrillic-A (char-charset ?А)))"##,
    );
}

#[test]
fn charset_emoji_math_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :emoji (char-charset ?🎉)
   :sum-sign (char-charset ?∑)
   :integral (char-charset ?∫)
   :infinite (char-charset ?∞)
   :arrow-right (char-charset ?→)
   :approx (char-charset ?≈)))"##,
    );
}

#[test]
fn char_bytes_width_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :ascii-A (char-bytes ?A)
   :greek-alpha (char-bytes ?α)
   :cjk-nichi (char-bytes ?日)
   :emoji (char-bytes ?🎉)
   :arrow (char-bytes ?→)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// CASE-FOLD: non-ASCII case conversion and folding
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn case_fold_greek_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (let ((case-fold-search t))
    (list
     :alpha-upper (string-match-p "Α" "α")
     :alpha-lower (string-match-p "α" "Α")
     :omega-upper (string-match-p "Ω" "ω")
     :sigma (string-match-p "Σ" "σ"))))"##,
    );
}

#[test]
fn case_fold_cyrillic_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (let ((case-fold-search t))
    (list
     :cyr-A (string-match-p "А" "а")
     :cyr-ya (string-match-p "Я" "я")
     :cyr-r (string-match-p "Р" "р")
     :cyr-p (string-match-p "П" "п"))))"##,
    );
}

#[test]
fn downcase_greek_special() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :alpha (downcase ?Α)
   :beta (downcase ?Β)
   :gamma (downcase ?Γ)
   :sigma-final (downcase ?Σ)
   :omega (downcase ?Ω)))"##,
    );
}

#[test]
fn upcase_greek_cyrillic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :alpha (upcase ?α)
   :omega (upcase ?ω)
   :cyrillic-a (upcase ?а)
   :cyrillic-ya (upcase ?я)))"##,
    );
}

#[test]
fn capitalize_mixed_case_nonascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :greek (capitalize "αβγ")
   :cyrillic (capitalize "абв")
   :mixed (capitalize "foo_bar")
   :german (capitalize "straße"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DISPLAY/COLUMN: current-column with display properties
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn current_column_with_display_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Hello World")
    (goto-char (point-min))
    (let ((col1 (current-column)))
      ;; add display property
      (put-text-property 1 6 'display "XX")
      (goto-char 7)
      (let ((col2 (current-column)))
        (list :col-before col1 :col-after col2 :point (point))))))"##,
    );
}

#[test]
fn move_to_column_with_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "abcdefghij")
    (goto-char (point-min))
    (put-text-property 1 4 'display "Y")
    (goto-char (point-min))
    (move-to-column 3)
    (list :point (point) :col (current-column)))))"##,
    );
}

#[test]
fn org_table_column_after_alignment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "| a | b |\n| 1 | 2 |\n")
    (goto-char (point-min))
    (org-table-align)
    (forward-line 1)
    (forward-char 2)
    (list :col (current-column) :at-table (org-at-table-p)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// BIDI/RTL: paragraph direction with RTL text
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn bidi_paragraph_direction_rtl() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "هذا نص بالعربية\n")
    (goto-char (point-min))
    (list :bidi-direction (buffer-local-value 'bidi-paragraph-direction (current-buffer))
          :rtl-detected (eq (buffer-local-value 'bidi-paragraph-direction (current-buffer)) 'right-to-left))))"##,
    );
}

#[test]
fn bidi_hebrew_paragraph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "זהו טקסט בעברית\n")
    (goto-char (point-min))
    (list :direction (buffer-local-value 'bidi-paragraph-direction (current-buffer)))))"##,
    );
}

#[test]
fn org_parse_rtl_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* عنوان بالعربية\nهذا نص.\n")
    (goto-char (point-min))
    (let* ((tree (org-element-parse-buffer))
           (hls (org-element-map tree 'headline #'identity)))
      (list :hl-count (length hls)
            :raw (mapcar (lambda (h) (substring-no-properties (org-element-property :raw-value h))) hls)
            :para-count (length (org-element-map tree 'paragraph #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// COMPOSITION: compose-region / find-composition
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn composition_find_in_org_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "abc")
    (goto-char (point-min))
    (let ((comp (find-composition 1)))
      (list :composition comp))))"##,
    );
}

#[test]
fn compose_region_and_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "hello")
    (compose-region 1 3 "X")
    (goto-char (point-min))
    (let ((comp (find-composition 1)))
      (list :composition (when comp (list (car comp) (cadr comp)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// TEXT PROPERTIES: plist ordering and invisible property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn text_property_plist_ordering() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "test text")
    (add-text-properties 1 5 '(face bold foo bar baz qux))
    (goto-char (point-min))
    (let ((props (text-properties-at 1)))
      (list :props props))))"##,
    );
}

#[test]
fn invisible_property_after_fold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nBody.\n* B\n")
    (goto-char (point-min))
    (org-overview)
    ;; Check invisible property at various positions
    (let ((r '()))
      (dotimes (i (point-max))
        (let ((inv (get-text-property (1+ i) 'invisible)))
          (when inv (push (list :pos (1+ i) :invisible inv) r))))
      (nreverse r))))"##,
    );
}

#[test]
fn org_hide_leading_stars_invisible() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (let ((org-hide-leading-stars t))
      (font-lock-mode 1)
      (insert "** Sub heading\n")
      (goto-char (point-min))
      (let ((r '()))
        (dotimes (i 4)
          (let ((inv (get-text-property (1+ i) 'invisible)))
            (push (list :pos (1+ i) :invisible inv) r)))
        (nreverse r)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ORG-SPECIFIC: multibyte buffer operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn org_multibyte_buffer_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* タイトル\n内容 *太字* here.\n")
    (goto-char (point-min))
    (let ((str (buffer-string)))
      (list :multibyte-p (multibyte-string-p str)
            :length (length str)
            :bytes (string-bytes str)))))"##,
    );
}

#[test]
fn org_set_buffer_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* H\nαβγ\n")
    (goto-char (point-min))
    (let ((before-multibyte (multibyte-string-p (buffer-string))))
      (list :before-multibyte before-multibyte
            :enable-multibyte (enable-multibyte-characters))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ERROR MESSAGE QUOTE STYLE
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn error_message_style_wrong_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (condition-case e
      (car 42)
    (error (list :msg (error-message-string e)
                 :type (car e)))))"##,
    );
}

#[test]
fn error_message_style_void_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (condition-case e
      (nonexistent-function-xyz)
    (error (list :msg (error-message-string e)
                 :type (car e)))))"##,
    );
}

#[test]
fn error_message_style_void_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (condition-case e
      nonexistent-variable-xyz
    (error (list :msg (error-message-string e)
                 :type (car e)))))"##,
    );
}

#[test]
fn error_message_style_args_out_of_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (condition-case e
      (substring "abc" 0 10)
    (error (list :msg (error-message-string e)
                 :type (car e)))))"##,
    );
}

#[test]
fn error_message_style_div_by_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (condition-case e
      (/ 1 0)
    (error (list :msg (error-message-string e)
                 :type (car e)))))"##,
    );
}
