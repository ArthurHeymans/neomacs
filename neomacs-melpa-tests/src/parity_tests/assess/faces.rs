use super::assert_assess_parity;
use expect_test::{Expect, expect};

#[test]
fn face_location_checker_handles_symbols_face_lists_property_plists_and_match_ranges() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "keyword function plain")
  (put-text-property
   1 8 'face
   'font-lock-keyword-face)
  (put-text-property
   9 17 'face
   '(bold font-lock-function-name-face))
  (put-text-property
   18 23 'face
   '(:weight bold))
  (let ((start
         (copy-marker 1))
        (end
         (copy-marker 8)))
    (list
     (assess--face-at-location=
      2
      'font-lock-keyword-face
      'face nil)
     (assess--face-at-location=
      2
      'font-lock-variable-name-face
      'face nil)
     (condition-case condition
         (assess--face-at-location=
          10
          'font-lock-function-name-face
          'face nil)
       (error
        condition))
     (assess--face-at-location=
      19 'bold 'face nil)
     (assess--face-at-location=
      (list start end)
      'font-lock-keyword-face
      'face nil)
     (catch
         'face-non-match
       (assess--face-at-location=
        18
        'font-lock-variable-name-face
        'face t)))))
"##;
    let expect: Expect = expect![[
        r#"OK (t nil (wrong-type-argument listp bold) nil t #("Face does not match expected value\n\11Expected: font-lock-variable-name-face\n\11Actual: (:weight bold)\n\11Location: 18\n\11Line Context: keyword function plain\n\11bol Position: 1\n" 128 135 (face font-lock-keyword-face) 136 144 (face (bold font-lock-function-name-face)) 145 150 (face (:weight bold))))"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn face_checker_normalizes_integer_string_function_marker_and_match_locations() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "defun alpha defmacro beta")
  (put-text-property
   1 6 'face
   'font-lock-keyword-face)
  (put-text-property
   7 12 'face
   'font-lock-function-name-face)
  (put-text-property
   13 21 'face
   'font-lock-keyword-face)
  (put-text-property
   22 26 'face
   'font-lock-function-name-face)
  (let* ((buffer (current-buffer))
         (marker
          (copy-marker 2))
         (match-data
          (list
           (copy-marker 1)
           (copy-marker 6))))
    (list
     (assess--face-at=
      buffer 2
      'font-lock-keyword-face
      'face nil)
     (assess--face-at=
      buffer
      '("defun" "defmacro")
      'font-lock-keyword-face
      'face nil)
     (assess--face-at=
      buffer
      (list marker)
      'font-lock-keyword-face
      'face nil)
     (assess--face-at=
      buffer
      (list match-data)
      'font-lock-keyword-face
      'face nil)
     (assess--face-at=
      buffer
      (lambda (_)
        '(2 8 14 23))
      '(font-lock-keyword-face
        font-lock-function-name-face)
      'face nil)
     (catch
         'face-non-match
       (assess--face-at=
        buffer 8
        'font-lock-keyword-face
        'face t)))))
"##;
    let expect: Expect = expect![[
        r#"OK (t t t t t #("Face does not match expected value\n\11Expected: font-lock-keyword-face\n\11Actual: font-lock-function-name-face\n\11Location: #<marker at 8 in  *temp*>\n\11Line Context: defun alpha defmacro beta\n\11bol Position: 1\n" 159 164 (face font-lock-keyword-face) 165 170 (face font-lock-function-name-face) 171 179 (face font-lock-keyword-face) 180 184 (face font-lock-function-name-face)))"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn public_face_predicate_fontifies_real_elisp_across_positions_regexps_and_functions() {
    let elisp_form = r##"
(let ((source
       "(defun alpha (x) x)\n(defmacro beta (&rest body) `(progn ,@body))"))
  (list
   (assess--face-at=-1
    source
    'emacs-lisp-mode
    '(2 9 23 32)
    '(font-lock-keyword-face
      font-lock-function-name-face)
    nil nil)
   (assess-face-at=
    source
    'emacs-lisp-mode
    '("defun" "defmacro")
    'font-lock-keyword-face)
   (assess-face-at=
    source
    'emacs-lisp-mode
    (lambda (buffer)
      (m-buffer-match
       buffer
       "\\_<\\(?:alpha\\|beta\\)\\_>"))
    'font-lock-function-name-face)
   (assess-face-at=
    "plain text"
    'fundamental-mode
    "text"
    'font-lock-type-face)))
"##;
    let expect: Expect = expect!["OK (t t t nil)"];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn face_explainer_returns_nil_for_matches_and_context_rich_text_for_mismatches() {
    let elisp_form = r##"
(let ((match
       (assess-explain-face-at=
        "(defun alpha ())"
        'emacs-lisp-mode
        2
        'font-lock-keyword-face))
      (mismatch
       (assess-explain-face-at=
        "(defun alpha ())\nplain"
        'emacs-lisp-mode
        10
        'font-lock-variable-name-face)))
  (list
   match
   mismatch
   (string-match-p
    "Expected: font-lock-variable-name-face"
    mismatch)
   (string-match-p
    "Line Context:"
    mismatch)))
"##;
    let expect: Expect = expect![[
        r#"OK (t "Face does not match expected value\n\11Expected: font-lock-variable-name-face\n\11Actual: font-lock-function-name-face\n\11Location: #<marker at 10 in  *temp*>\n\11Line Context: plain\n\11bol Position: 1\n" 36 152)"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn file_face_pipeline_uses_auto_mode_fontification_and_cleans_related_visit_buffers() {
    let elisp_form = r##"
(let* ((path
        (assess-test-path
         "fontification/fixture.el"))
       (content
        "(defun alpha (x)\n  (+ x 1))\n"))
  (make-directory
   (file-name-directory path)
   t)
  (with-temp-file path
    (insert content))
  (let ((auto-mode-alist
         '(("\\.el\\'" .
            emacs-lisp-mode))))
    (list
     (assess--file-face-at=-1
      path 2
      'font-lock-keyword-face
      nil nil)
     (assess-file-face-at=
      path
      '(2 9)
      '(font-lock-keyword-face
        font-lock-function-name-face))
     (assess-file-face-at=
      path 9
      'font-lock-variable-name-face)
     (let ((diagnostic
            (assess-explain-file-face-at=
             path 9
             'font-lock-variable-name-face)))
       (list
        (string-match-p
         "Expected: font-lock-variable-name-face"
         diagnostic)
        (string-match-p
         "Actual: font-lock-function-name-face"
         diagnostic)
        (string-match-p
         "Line Context: (defun alpha"
         diagnostic)))
     (assess-test-read-file path)
     (find-buffer-visiting path))))
"##;
    let expect: Expect =
        expect![[r#"OK (t t nil (36 76 163) "(defun alpha (x)\n  (+ x 1))\n" nil)"#]];
    assert_assess_parity(elisp_form, expect);
}
