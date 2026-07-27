use expect_test::expect;

use super::assert_anki_vocabulary_parity;

#[test]
fn normal_text_uses_the_active_region_and_flattens_multiline_propertized_content() {
    let elisp_form = r##"(with-temp-buffer
  (insert "Header\n")
  (let ((start (point)))
    (insert (propertize "A practical\n\nmultiline\r\nexample"
                        'face 'bold
                        'source 'reader))
    (let ((end (point)))
      (insert "\nFooter")
      (goto-char end)
      (set-mark start)
      (setq mark-active t
            transient-mark-mode t)
      (let ((text (anki-vocabulary--get-normal-text)))
        (list
         text
         (text-properties-at 1 text)
         (buffer-substring-no-properties
          (region-beginning)
          (region-end)))))))"##;
    let expect = expect![[
        r#"OK ("A practical multiline example" nil "A practical\n\nmultiline\15\nexample")"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn normal_text_falls_back_to_the_sentence_at_point_in_a_real_paragraph() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "A short opening sentence.  "
   "The target sentence spans\n"
   "two physical lines and ends here!  "
   "A final sentence follows.")
  (search-backward "target")
  (setq mark-active nil)
  (list
   (anki-vocabulary--get-normal-text)
   (sentence-at-point)
   (thing-at-point 'line t)))"##;
    let expect = expect![[
        r#"OK ("The target sentence spans two physical lines and ends here!" "The target sentence spans\ntwo physical lines and ends here!" "A short opening sentence.  The target sentence spans\n")"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn normal_text_uses_the_current_line_when_sentence_detection_returns_nil() {
    let elisp_form = r##"(with-temp-buffer
  (insert "first\nsecond line with useful text\nthird\n")
  (goto-char (point-min))
  (forward-line 1)
  (setq mark-active nil)
  (cl-letf (((symbol-function 'sentence-at-point)
             (lambda () nil)))
    (list
     (anki-vocabulary--get-normal-text)
     (thing-at-point 'line t)
     (line-number-at-pos))))"##;
    let expect =
        expect![[r#"OK ("second line with useful text " "second line with useful text\n" 2)"#]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn pdf_text_requires_a_selection_joins_fragments_and_flattens_page_breaks() {
    let elisp_form = r##"(let (events)
  (cl-letf
      (((symbol-function 'pdf-view-assert-active-region)
        (lambda ()
          (push 'asserted events)
          'selection-ok))
       ((symbol-function 'pdf-view-active-region-text)
        (lambda ()
          (push 'read events)
          '("First fragment"
            "second\nfragment"
            "third\r\nfragment"))))
    (list
     (anki-vocabulary--get-pdf-text)
     (nreverse events))))"##;
    let expect =
        expect![[r#"OK ("First fragment second fragment third fragment" (asserted read))"#]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn text_dispatch_uses_pdf_extraction_only_in_pdf_derived_buffers() {
    let elisp_form = r##"(progn
  (unless (fboundp 'pdf-view-mode)
    (define-derived-mode pdf-view-mode special-mode "PDF"))
  (define-derived-mode anki-vocabulary-test-pdf-mode
    pdf-view-mode "Vocabulary PDF")
  (let (events)
    (cl-letf
        (((symbol-function 'anki-vocabulary--get-pdf-text)
          (lambda ()
            (push 'pdf events)
            "PDF selection"))
         ((symbol-function 'anki-vocabulary--get-normal-text)
          (lambda ()
            (push 'normal events)
            "Buffer sentence")))
      (list
       (with-temp-buffer
         (fundamental-mode)
         (anki-vocabulary--get-text))
       (with-temp-buffer
         (anki-vocabulary-test-pdf-mode)
         (anki-vocabulary--get-text))
       (nreverse events)))))"##;
    let expect = expect![[r#"OK ("Buffer sentence" "PDF selection" (normal pdf))"#]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn word_selection_tokenizes_real_punctuation_and_passes_the_point_word_as_initial_input() {
    let elisp_form = r##"(let (call)
  (cl-letf
      (((symbol-function 'completing-read)
        (lambda (prompt collection predicate require-match
                        &optional initial-input history default
                        inherit-input-method)
          (setq call
                (list prompt collection predicate require-match
                      initial-input history default
                      inherit-input-method))
          (concat initial-input "-chosen"))))
    (list
     (anki-vocabulary--select-word-in-string
      "Well-tested, practical: workflows; handle (edge-cases) too."
      "practical")
     call)))"##;
    let expect = expect![[
        r#"OK ("practical-chosen" ("Pick The Word: " ("Well-tested" "practical" "workflows" "handle" "edge-cases" "too" "") nil nil "practical" nil nil nil))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn point_word_is_available_in_text_buffers_and_suppressed_in_pdf_buffers() {
    let elisp_form = r##"(progn
  (unless (fboundp 'pdf-view-mode)
    (define-derived-mode pdf-view-mode special-mode "PDF"))
  (list
   (with-temp-buffer
     (insert "prefix card-reader suffix")
     (search-backward "reader")
     (list
      (anki-vocabulary--get-word)
      (word-at-point)))
   (with-temp-buffer
     (insert "visual document text")
     (pdf-view-mode)
     (goto-char 8)
     (list
      (anki-vocabulary--get-word)
      (word-at-point)))))"##;
    let expect = expect![[r#"OK (("reader" "reader") (nil "document"))"#]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}
