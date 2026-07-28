use expect_test::expect;

use super::assert_agtags_parity;

#[test]
fn agtags_searches_a_real_tag_result_navigates_to_it_edits_it_and_refreshes_the_database() {
    let elisp_form = r####"(let* ((root
                                  (file-name-as-directory
                                   (expand-file-name
                                    "agtags-navigation-workflow"
                                    (getenv
                                     "NEOMACS_TEST_SANDBOX_ROOT"))))
                                 (default-directory
                                  root)
                                 (source
                                  (expand-file-name
                                   "src/parser.c"
                                   root))
                                 (main
                                  (expand-file-name
                                   "src/main.c"
                                   root))
                                 source-buffer
                                 grep-buffer
                                 result)
                             (unwind-protect
                                 (progn
                                   (neomacs-agtags-test-cleanup
                                    root)
                                   (make-directory
                                    (file-name-directory
                                     source)
                                    t)
                                   (with-temp-file
                                       source
                                     (insert
                                      "#include \"parser.h\"\n"
                                      "int parser_reset(int state) {\n"
                                      "  if (state < 0) return 0;\n"
                                      "  return state + 1;\n"
                                      "}\n"
                                      "int parse_request(int state) {\n"
                                      "  return parser_reset(state - 1);\n"
                                      "}\n"))
                                   (with-temp-file
                                       main
                                     (insert
                                      "#include \"parser.h\"\n"
                                      "int main(void) {\n"
                                      "  int input = 41;\n"
                                      "  /* Recover before dispatch. */\n"
                                      "  return parser_reset(input);\n"
                                      "}\n"))
                                   (process-file
                                    "git"
                                    nil
                                    nil
                                    nil
                                    "init"
                                    "-q"
                                    root)
                                   (let ((tools
                                          (neomacs-agtags-test-install-tools
                                           root)))
                                     (neomacs-agtags-test-use-tools
                                      tools)
                                     (with-temp-file
                                         (expand-file-name
                                          "GTAGS"
                                          root)
                                       (insert
                                        "definitions=parser_reset\n"))
                                     (setq
                                      source-buffer
                                      (find-file-noselect
                                       source))
                                     (with-current-buffer
                                         source-buffer
                                       (agtags-mode
                                        1)
                                       (goto-char
                                        (point-min))
                                       (search-forward
                                        "parser_reset"))
                                     (cl-letf
                                         (((symbol-function
                                            'completing-read)
                                           (lambda
                                             (&rest _arguments)
                                             "parser_reset")))
                                       (with-current-buffer
                                           source-buffer
                                         (agtags-find-tag)))
                                     (setq
                                      grep-buffer
                                      (get-buffer
                                       "*agtags-grep*"))
                                     (neomacs-agtags-test-wait-for-buffer
                                      grep-buffer)
                                     (let ((search-buffer
                                            (with-current-buffer
                                                grep-buffer
                                              (list
                                               (buffer-name)
                                               major-mode
                                               mode-name
                                               (neomacs-agtags-test-normalize-result-buffer
                                                (substring-no-properties
                                                 (buffer-string)))))))
                                       (switch-to-buffer
                                        grep-buffer)
                                       (goto-char
                                        (point-min))
                                       (search-forward
                                        "src/parser.c:2:")
                                       (beginning-of-line)
                                       (compile-goto-error)
                                       (let ((navigated
                                              (list
                                               (file-relative-name
                                                buffer-file-name
                                                root)
                                               (line-number-at-pos)
                                               (current-column)
                                               (buffer-substring-no-properties
                                                (line-beginning-position)
                                                (line-end-position)))))
                                         (search-forward
                                          "return state + 1;")
                                         (replace-match
                                          "return state + 2;"
                                          t
                                          t)
                                         (save-buffer)
                                         (let ((updated-source
                                                (substring-no-properties
                                                 (buffer-string)))
                                               (updated-database
                                                (neomacs-agtags-test-file-string
                                                 (expand-file-name
                                                  "GTAGS"
                                                  root))))
                                           (agtags-switch-dwim)
                                           (setq
                                            result
                                            (list
                                             search-buffer
                                             navigated
                                             updated-source
                                             (buffer-modified-p
                                              source-buffer)
                                             updated-database
                                             (buffer-name)
                                             major-mode
                                             (neomacs-agtags-test-normalize-result-buffer
                                              (substring-no-properties
                                               (buffer-string)))
                                             (neomacs-agtags-test-file-string
                                              (cadr
                                               tools)))))))))
                               (neomacs-agtags-test-cleanup
                                root))
                             result)"####;
    let expect = expect![[
        r##"OK (("*agtags-grep*" agtags-grep-mode "Global Grep" "-*- mode: agtags-grep; default-directory: \"[ORACLE-SANDBOX]/agtags-navigation-workflow/\" -*-\nGlobal Grep started at TIME\n\nglobal --result=grep parser_reset\nsrc/parser.c:2:int parser_reset(int state) {\n\nGlobal Grep finished with matches found at TIME\n") ("src/parser.c" 2 0 "int parser_reset(int state) {") "#include \"parser.h\"\nint parser_reset(int state) {\n  if (state < 0) return 0;\n  return state + 2;\n}\nint parse_request(int state) {\n  return parser_reset(state - 1);\n}\n" nil "definitions=parser_reset\nupdated=\n" "*agtags-grep*" agtags-grep-mode "-*- mode: agtags-grep; default-directory: \"[ORACLE-SANDBOX]/agtags-navigation-workflow/\" -*-\nGlobal Grep started at TIME\n\nglobal --result=grep parser_reset\nsrc/parser.c:2:int parser_reset(int state) {\n\nGlobal Grep finished with matches found at TIME\n" "global cwd=[ORACLE-SANDBOX]/agtags-navigation-workflow <--result=grep> <parser_reset>\nglobal cwd=[ORACLE-SANDBOX]/agtags-navigation-workflow <-u> <--single-update=>\n")"##
    ]];

    assert_agtags_parity(elisp_form, expect);
}
