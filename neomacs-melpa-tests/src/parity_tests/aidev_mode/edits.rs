use expect_test::expect;

use super::assert_aidev_mode_parity;

#[test]
fn aidev_mode_insert_chat_uses_region_context_and_inserts_sanitized_code_at_point() {
    let elisp_form = r##"(let ((aidev-provider 'claude)
               (aidev-default-model
                "claude-frozen-model")
               calls)
         (cl-letf
             (((symbol-function
                'aidev---claude)
               (lambda
                 (messages system model)
                 (push
                  (list messages system model)
                  calls)
                 "  ```elisp\n(defun square (number)\n  (* number number))\n```\nUse lexical binding.  ")))
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              "(defun placeholder (number)\n"
              "  number)\n")
             (let ((transient-mark-mode t))
               (goto-char (point-min))
               (push-mark (point-max) t t)
               (let ((result
                      (aidev-insert-chat
                       "Generate a square helper")))
                 (list
                  result
                  (buffer-string)
                  (point)
                  (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (nil "(defun square (number)\n  (* number number))\n;Use lexical binding.(defun placeholder (number)\n  number)\n" 66 ((((("role" . "user") ("content" . "(defun placeholder (number)\n  number)\n")) (("role" . "user") ("content" . "Generate a square helper"))) "You are an extremely competent programmer. You have an encyclopedic understanding, high-level understanding of all programming languages and understand how to write the most understandeable, elegant code in all of them.\nThe user is currently working in the major mode 'emacs-lisp-mode', so please return code appropriate for that context.\nThe likeliest requests involve generating code. If you are asked to generate code, only return code, and no commentary. If you must, provide minor points and/or testing examples in the form of code comments (commented in the appropriate syntax) but no longer prose unless explicitly requested." "claude-frozen-model")))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_refactor_region_replaces_only_selection_with_provider_result() {
    let elisp_form = r##"(let ((aidev-provider 'claude)
               calls)
         (cl-letf
             (((symbol-function
                'aidev---claude)
               (lambda
                 (messages system model)
                 (push
                  (list messages system model)
                  calls)
                 "```elisp\n(mapcar #'string-trim rows)\n```\nKeeps ordering stable.")))
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              "(let ((rows input))\n  "
              "(mapcar (lambda (row) (string-trim row)) rows)"
              ")\n")
             (let ((transient-mark-mode t))
               (goto-char (point-min))
               (search-forward "(mapcar")
               (goto-char (match-beginning 0))
               (push-mark
                (progn
                  (search-forward " rows)")
                  (point))
                t t)
               (let ((result
                      (aidev-refactor-region-with-chat
                       "Use a named function")))
                 (list
                  result
                  (buffer-string)
                  (point)
                  (region-active-p)
                  (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (nil "(let ((rows input))\n  (mapcar (lambda (row) (string-trim row)) rows))\n" 69 t nil)"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_refactor_region_without_selection_is_noop_and_skips_provider() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'aidev---claude)
               (lambda (&rest arguments)
                 (push arguments calls)
                 "unexpected")))
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert "(message \"unchanged\")")
             (goto-char 8)
             (deactivate-mark)
             (list
              (aidev-refactor-region-with-chat
               "Do not run")
              (buffer-string)
              (point)
              calls))))"##;
    let expect = expect![[r#"OK (nil "(message \"unchanged\")" 8 nil)"#]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_refactor_buffer_replaces_complete_program_and_uses_prompt_only() {
    let elisp_form = r##"(let ((aidev-provider 'openai)
               (aidev-default-model "o3-frozen")
               calls)
         (cl-letf
             (((symbol-function
                'aidev---openai)
               (lambda
                 (messages system model)
                 (push
                  (list messages system model)
                  calls)
                 "```python\nfrom pathlib import Path\n\n\ndef load(path):\n    return Path(path).read_text()\n```")))
           (with-temp-buffer
             (python-mode)
             (insert
              "def load(path):\n"
              "    return open(path).read()\n")
             (goto-char 9)
             (let ((result
                    (aidev-refactor-buffer-with-chat
                     "Use pathlib and preserve behavior")))
               (list
                result
                (buffer-string)
                (point)
                major-mode
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (nil "from pathlib import Path\n\n\ndef load(path):\n    return Path(path).read_text()" 77 python-mode ((((("role" . "user") ("content" . "Use pathlib and preserve behavior"))) "You are an extremely competent programmer. You have an encyclopedic understanding, high-level understanding of all programming languages and understand how to write the most understandeable, elegant code in all of them.\nThe user is currently working in the major mode 'python-mode', so please return code appropriate for that context.\nThe user wants you to help them refactor a piece of code they've already written. Unless specified by their prompt, you should output code in the same language as the input code. Output absolutely nothing but code; the message you return should be a drop-in replacement for the code the user needs help with." "o3-frozen")))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_new_buffer_uses_generated_name_switches_buffer_and_inserts_result() {
    let elisp_form = r##"(let ((origin (current-buffer))
               (aidev-provider 'ollama)
               generated
               calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'aidev---ollama)
                   (lambda
                     (messages system model)
                     (push
                      (list messages system model)
                      calls)
                     "```elisp\n(defun generated-command ()\n  (interactive)\n  (message \"ready\"))\n```")))
               (emacs-lisp-mode)
               (insert "(message \"source context\")")
               (let ((result
                      (aidev-new-buffer-from-chat
                       "Create an interactive command")))
                 (setq generated
                       (current-buffer))
                 (list
                  result
                  (eq
                   (current-buffer)
                   origin)
                  (buffer-name generated)
                  (buffer-local-value
                   'major-mode generated)
                  (with-current-buffer generated
                    (buffer-string))
                  (nreverse calls))))
           (when (buffer-live-p generated)
             (kill-buffer generated))
           (when (buffer-live-p origin)
             (set-buffer origin))))"##;
    let expect = expect![[
        r#"OK ((:buffer nil) nil "*AI Generated Code*" fundamental-mode "(defun generated-command ()\n  (interactive)\n  (message \"ready\"))" ((((("role" . "user") ("content" . "Create an interactive command"))) "You are an extremely competent programmer. You have an encyclopedic understanding, high-level understanding of all programming languages and understand how to write the most understandeable, elegant code in all of them.\nThe user is currently working in the major mode 'emacs-lisp-mode', so please return code appropriate for that context.\nThe likeliest requests involve generating code. If you are asked to generate code, only return code, and no commentary. If you must, provide minor points and/or testing examples in the form of code comments (commented in the appropriate syntax) but no longer prose unless explicitly requested." "deepseek-coder-v2:latest")))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_supports_multi_step_region_refactor_then_generated_helper_insertion() {
    let elisp_form = r##"(let ((aidev-provider 'claude)
               (responses
                '("```elisp\n(defun total (values)\n  (apply #'+ values))\n```"
                  "```elisp\n\n(defun average (values)\n  (/ (float (total values)) (length values)))\n```"))
               calls)
         (cl-letf
             (((symbol-function
                'aidev---claude)
               (lambda
                 (messages system model)
                 (push
                  (list messages system model)
                  calls)
                 (prog1
                     (car responses)
                   (setq responses
                         (cdr responses))))))
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              "(defun total (values)\n"
              "  (let ((sum 0))\n"
              "    (dolist (value values sum)\n"
              "      (setq sum (+ sum value)))))\n")
             (let ((transient-mark-mode t))
               (goto-char (point-min))
               (push-mark (point-max) t t)
               (aidev-refactor-region-with-chat
                "Use the standard summation primitive")
               (deactivate-mark)
               (goto-char (point-max))
               (aidev-insert-chat
                "Add an average helper")
               (list
                (buffer-string)
                (point)
                (nreverse calls)
                responses)))))"##;
    let expect = expect![[
        r#"OK ("(defun total (values)\n  (apply #'+ values))\n(defun average (values)\n  (/ (float (total values)) (length values)))" 114 ((((("role" . "user") ("content" . "(defun total (values)\n  (let ((sum 0))\n    (dolist (value values sum)\n      (setq sum (+ sum value)))))\n")) (#1=("role" . "user") ("content" . "Use the standard summation primitive"))) "You are an extremely competent programmer. You have an encyclopedic understanding, high-level understanding of all programming languages and understand how to write the most understandeable, elegant code in all of them.\nThe user is currently working in the major mode 'emacs-lisp-mode', so please return code appropriate for that context.\nThe user wants you to help them refactor a piece of code they've already written. Unless specified by their prompt, you should output code in the same language as the input code. Output absolutely nothing but code; the message you return should be a drop-in replacement for the code the user needs help with." "deepseek-coder-v2:latest") (((#1# ("content" . "Add an average helper"))) "You are an extremely competent programmer. You have an encyclopedic understanding, high-level understanding of all programming languages and understand how to write the most understandeable, elegant code in all of them.\nThe user is currently working in the major mode 'emacs-lisp-mode', so please return code appropriate for that context.\nThe likeliest requests involve generating code. If you are asked to generate code, only return code, and no commentary. If you must, provide minor points and/or testing examples in the form of code comments (commented in the appropriate syntax) but no longer prose unless explicitly requested." "deepseek-coder-v2:latest")) nil)"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}
