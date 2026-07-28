use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_readme_style_c_guard_waits_for_statement_completion_then_indents() {
    let elisp_form = r####"(with-temp-buffer
                            (c-mode)
                            (setq
                             aggressive-indent-sit-for-time
                             60)
                            (let ((aggressive-indent-dont-indent-if
                                   '((and
                                      (derived-mode-p
                                       'c-mode)
                                      (null
                                       (string-match
                                        "\\([;{}]\\|\\b\\(if\\|for\\|while\\)\\b\\)"
                                        (thing-at-point
                                         'line)))))))
                              (insert
                               "int reconcile(int ready) {\n"
                               "  if (ready) {\n"
                               "printf(\"read\")\n"
                               "  }\n"
                               "  return ready;\n"
                               "}\n")
                              (goto-char
                               (point-min))
                              (search-forward
                               "printf(\"read")
                              (aggressive-indent-mode
                               1)
                              (insert
                               "y")
                              (timer-event-handler
                               aggressive-indent--idle-timer)
                              (let ((waiting
                                     (list
                                      (buffer-substring-no-properties
                                       (point-min)
                                       (point-max))
                                      (point)
                                      (and
                                       aggressive-indent--changed-list
                                       t)
                                      aggressive-indent--idle-timer)))
                                (end-of-line)
                                (insert
                                 ";")
                                (timer-event-handler
                                 aggressive-indent--idle-timer)
                                (list
                                 waiting
                                 (buffer-substring-no-properties
                                  (point-min)
                                  (point-max))
                                 (point)
                                 aggressive-indent--changed-list
                                 aggressive-indent--idle-timer))))"####;
    let expect = expect![[
        r#"OK (("int reconcile(int ready) {\n  if (ready) {\nprintf(\"ready\")\n  }\n  return ready;\n}\n" 56 t nil) "int reconcile(int ready) {\n  if (ready) {\n    printf(\"ready\");\n  }\n  return ready;\n}\n" 63 nil nil)"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}
