use super::assert_assess_parity;
use expect_test::{Expect, expect};

#[test]
fn indent_buffer_exercises_explicit_column_mode_function_and_line_fallback_paths() {
    let elisp_form = r##"
(let (column-result
      function-result
      fallback-result
      calls)
  (with-temp-buffer
    (insert "alpha\nbeta\n")
    (assess--indent-buffer 3)
    (setq column-result
          (buffer-string)))
  (with-temp-buffer
    (insert "alpha\nbeta\n")
    (setq-local
     indent-region-function
     (lambda (start end)
       (setq calls
             (list start end))
       (goto-char start)
       (while (< (point) end)
         (insert ">")
         (forward-line 1))))
    (assess--indent-buffer)
    (setq function-result
          (buffer-string)))
  (with-temp-buffer
    (insert
     "(progn\n(message \"a\")\n(list 1 2))")
    (emacs-lisp-mode)
    (setq-local indent-region-function nil)
    (assess--indent-buffer)
    (setq fallback-result
          (buffer-string)))
  (list
   column-result
   function-result
   calls
   fallback-result))
"##;
    let expect: Expect = expect![[
        r#"OK ("   alpha\n   beta\n" ">alpha\n>beta\n" (1 12) "(progn\n  (message \"a\")\n  (list 1 2))")"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn indentation_pipeline_formats_real_elisp_and_reports_exact_match_and_mismatch() {
    let elisp_form = r##"
(let* ((unindented
        "(let ((x 1))\n(message \"x=%s\" x)\n(list x\n(+ x 1)))")
       (indented
        "(let ((x 1))\n  (message \"x=%s\" x)\n  (list x\n        (+ x 1)))")
       (actual
        (assess--indent-in-mode
         'emacs-lisp-mode
         unindented)))
  (list
   actual
   (assess-indentation=
    'emacs-lisp-mode
    unindented
    indented)
   (assess-indentation=
    'emacs-lisp-mode
    unindented
    (concat indented " "))
   (cl-letf
       (((symbol-function
          'assess-explain=)
         (lambda (left right)
           (list
            :explain left right))))
     (assess-explain-indentation=
      'emacs-lisp-mode
      unindented
      indented))))
"##;
    let expect: Expect = expect![[
        r#"OK ("(let ((x 1))\n  (message \"x=%s\" x)\n  (list x\n\11(+ x 1)))" nil nil (:explain "(let ((x 1))\n  (message \"x=%s\" x)\n  (list x\n\11(+ x 1)))" "(let ((x 1))\n  (message \"x=%s\" x)\n  (list x\n        (+ x 1)))"))"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn buffer_unindent_and_roundtrip_helpers_preserve_correct_layout_and_expose_bad_layout() {
    let elisp_form = r##"
(let ((correct
       "(defun fixture (x)\n  (if x\n      (message \"yes\")\n    (message \"no\")))")
      (incorrect
       "(defun fixture (x)\n(if x\n(message \"yes\")\n(message \"no\")))")
      unindented)
  (with-temp-buffer
    (insert correct)
    (emacs-lisp-mode)
    (assess--buffer-unindent
     (current-buffer))
    (setq unindented
          (buffer-string)))
  (list
   unindented
   (assess--roundtrip-1
    #'assess-indentation=
    'emacs-lisp-mode
    correct)
   (assess-roundtrip-indentation=
    'emacs-lisp-mode
    correct)
   (assess-roundtrip-indentation=
    'emacs-lisp-mode
    incorrect)
   (cl-letf
       (((symbol-function
          'assess-explain-indentation=)
         (lambda (mode left right)
           (list mode left right))))
     (assess-explain-roundtrip-indentation=
      'emacs-lisp-mode
      correct))))
"##;
    let expect: Expect = expect![[
        r#"OK ("(defun fixture (x)\n(if x\n(message \"yes\")\n(message \"no\")))" t t nil (emacs-lisp-mode "(defun fixture (x)\n(if x\n(message \"yes\")\n(message \"no\")))" "(defun fixture (x)\n  (if x\n      (message \"yes\")\n    (message \"no\")))"))"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn file_roundtrip_pipeline_uses_file_mode_copies_input_and_leaves_original_unchanged() {
    let elisp_form = r##"
(let* ((good
        (assess-test-path
         "indent/good.el"))
       (bad
        (assess-test-path
         "indent/bad.el"))
       (good-content
        "(defun good (x)\n  (list x\n        (+ x 1)))\n")
       (bad-content
        "(defun bad (x)\n(list x\n(+ x 1)))\n"))
  (make-directory
   (file-name-directory good)
   t)
  (with-temp-file good
    (insert good-content))
  (with-temp-file bad
    (insert bad-content))
  (let ((auto-mode-alist
         '(("\\.el\\'" .
            emacs-lisp-mode))))
    (list
     (assess--file-roundtrip-1
      #'assess=
      good)
     (assess-file-roundtrip-indentation=
      good)
     (assess-file-roundtrip-indentation=
      bad)
     (cl-letf
         (((symbol-function
            'assess-explain=)
           (lambda (left right)
             (list
              :comparison
              (substring-no-properties
               left)
              (substring-no-properties
               right)))))
       (assess-explain-file-roundtrip-indentation=
        bad))
     (assess-test-read-file good)
     (assess-test-read-file bad)
     (find-buffer-visiting good)
     (find-buffer-visiting bad))))
"##;
    let expect: Expect = expect![[
        r#"OK (t t nil (:comparison "(defun bad (x)\n  (list x\n        (+ x 1)))\n" "(defun bad (x)\n(list x\n(+ x 1)))\n") "(defun good (x)\n  (list x\n        (+ x 1)))\n" "(defun bad (x)\n(list x\n(+ x 1)))\n" nil nil)"#
    ]];
    assert_assess_parity(elisp_form, expect);
}
