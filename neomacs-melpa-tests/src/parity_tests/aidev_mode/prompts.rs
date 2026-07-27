use expect_test::expect;

use super::assert_aidev_mode_parity;

#[test]
fn aidev_mode_system_message_combines_programming_policy_major_mode_and_task() {
    let elisp_form = r##"(mapcar
         (lambda (mode)
           (with-temp-buffer
             (funcall mode)
             (list
              major-mode
              (aidev--prepare-system-message
               "Preserve public APIs and add focused tests."))))
         '(emacs-lisp-mode
           text-mode
           fundamental-mode))"##;
    let expect = expect![[
        r#"OK ((emacs-lisp-mode "You are an extremely competent programmer. You have an encyclopedic understanding, high-level understanding of all programming languages and understand how to write the most understandeable, elegant code in all of them.\nThe user is currently working in the major mode 'emacs-lisp-mode', so please return code appropriate for that context.\nPreserve public APIs and add focused tests.") (text-mode "You are an extremely competent programmer. You have an encyclopedic understanding, high-level understanding of all programming languages and understand how to write the most understandeable, elegant code in all of them.\nThe user is currently working in the major mode 'text-mode', so please return code appropriate for that context.\nPreserve public APIs and add focused tests.") (fundamental-mode "You are an extremely competent programmer. You have an encyclopedic understanding, high-level understanding of all programming languages and understand how to write the most understandeable, elegant code in all of them.\nThe user is currently working in the major mode 'fundamental-mode', so please return code appropriate for that context.\nPreserve public APIs and add focused tests."))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_prepare_prompt_includes_property_free_active_region_before_request() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "prefix "
          (propertize
           "(mapcar #'1+ values)"
           'face 'bold
           'aidev-test-property t)
          " suffix")
         (let ((transient-mark-mode t)
               region-start
               region-end)
           (goto-char (point-min))
           (search-forward "(")
           (backward-char)
           (setq region-start
                 (point))
           (search-forward ")")
           (setq region-end
                 (point))
           (goto-char region-start)
           (push-mark region-end t t)
           (let ((prepared
                  (aidev--prepare-prompt
                   "Make this lazy"
                   t)))
             (list
              prepared
              (text-properties-at
               0
               (cdr
                (assoc
                 "content"
                 (car prepared))))
              (region-beginning)
              (region-end)))))"##;
    let expect = expect![[
        r#"OK (((("role" . "user") ("content" . "(mapcar #'1+ values)")) (("role" . "user") ("content" . "Make this lazy"))) nil 8 28)"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_prepare_prompt_handles_disabled_region_empty_prompt_and_region_order() {
    let elisp_form = r##"(with-temp-buffer
         (insert "selected implementation")
         (let ((transient-mark-mode t))
           (goto-char (point-min))
           (push-mark (point-max) t t)
           (list
            (aidev--prepare-prompt
             "Refactor" nil)
            (aidev--prepare-prompt
             "Refactor" t)
            (progn
              (deactivate-mark)
              (aidev--prepare-prompt
               "" t)))))"##;
    let expect = expect![[
        r#"OK (((#1=("role" . "user") ("content" . "Refactor"))) ((("role" . "user") ("content" . "selected implementation")) (#1# ("content" . "Refactor"))) ((#1# ("content" . ""))))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_inverts_fenced_markdown_into_comments_and_executable_code() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (when (eq (car case)
                       'c-mode)
               (require 'cc-mode))
             (funcall (car case))
             (list
              major-mode
              (aidev--invert-markdown-code
               (cadr case)))))
         '((emacs-lisp-mode
            "```elisp\n(defun total (xs)\n  (apply #'+ xs))\n```\nUse this from the report command.")
           (c-mode
            "````c\nint total(int a, int b) {\n  return a + b;\n}\n````\nCompile with warnings enabled.")
           (fundamental-mode
            "```\nplain code\n```\nplain explanation")
           (emacs-lisp-mode
            "  ```elisp\n(message \"ready\")\n  ```")))"##;
    let expect = expect![[
        r#"OK ((emacs-lisp-mode "(defun total (xs)\n  (apply #'+ xs))\n;Use this from the report command.") (c-mode "int total(int a, int b) {\n  return a + b;\n}\n/* Compile with warnings enabled. */") (fundamental-mode "plain code\n;; plain explanation") (emacs-lisp-mode "(message \"ready\")"))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_leaves_nonleading_markdown_and_plain_provider_text_unchanged() {
    let elisp_form = r##"(mapcar
         #'aidev--invert-markdown-code
         '("The implementation is:\n```elisp\n(+ 1 2)\n```"
           "  prose before fence\n```\ncode\n```"
           "(defun already-code () t)"
           ""
           "`` not a fence"))"##;
    let expect = expect![[
        r#"OK (";The implementation is:\n(+ 1 2)" ";  prose before fence\ncode" "(defun already-code () t)" "" "`` not a fence")"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_strip_markdown_code_handles_languages_spacing_and_partial_fences() {
    let elisp_form = r##"(mapcar
         #'aidev--strip-markdown-code
         '("```elisp\n(+ 1 2)\n```"
           "```python   \nprint('ok')\n```   "
           "```\nline one\nline two\n```"
           "prefix\n```elisp\ncode\n```\nsuffix"
           "```elisp\nunterminated"
           "plain response"
           ""))"##;
    let expect = expect![[
        r#"OK ("(+ 1 2)\n" "print('ok')\n" "line one\nline two\n" "prefix\ncode\nsuffix" "unterminated" "plain response" "")"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_utf8_cleanup_matches_frozen_replacement_behavior() {
    let elisp_form = r##"(mapcar
         #'aidev---decode-utf8-string
         '("plain ASCII"
           "alphaâbetaâgamma"
           "real—em dash and ‘quotes’"
           "âââ"
           ""))"##;
    let expect =
        expect![[r#"OK ("plain ASCII" "alpha-beta-gamma" "real—em dash and ‘quotes’" "---" "")"#]];
    assert_aidev_mode_parity(elisp_form, expect);
}
