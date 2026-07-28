use expect_test::expect;

use super::assert_agtags_parity;

#[test]
fn agtags_builds_a_project_database_then_updates_it_when_an_edited_source_is_saved() {
    let elisp_form = r####"(let* ((root
                                  (file-name-as-directory
                                   (expand-file-name
                                    "agtags-database-workflow"
                                    (getenv
                                     "NEOMACS_TEST_SANDBOX_ROOT"))))
                                 (default-directory
                                  root)
                                 (source
                                  (expand-file-name
                                   "src/parser.c"
                                   root))
                                 (header
                                  (expand-file-name
                                   "include/parser.h"
                                   root))
                                 (main
                                  (expand-file-name
                                   "src/main.c"
                                   root))
                                 source-buffer
                                 result)
                             (unwind-protect
                                 (progn
                                   (neomacs-agtags-test-cleanup
                                    root)
                                   (make-directory
                                    (file-name-directory
                                     source)
                                    t)
                                   (make-directory
                                    (file-name-directory
                                     header)
                                    t)
                                   (with-temp-file
                                       header
                                     (insert
                                      "#pragma once\n"
                                      "int parser_reset(int state);\n"))
                                   (with-temp-file
                                       source
                                     (insert
                                      "#include \"parser.h\"\n"
                                      "int parser_reset(int state) {\n"
                                      "  if (state < 0) return 0;\n"
                                      "  return state + 1;\n"
                                      "}\n"))
                                   (with-temp-file
                                       main
                                     (insert
                                      "#include \"parser.h\"\n"
                                      "int main(void) {\n"
                                      "  int input = 41;\n"
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
                                     (dolist (name
                                              agtags-created-tag-files)
                                       (with-temp-file
                                           (expand-file-name
                                            name
                                            root)
                                         (insert
                                          "stale database\n")))
                                     (setq
                                      agtags--history-list
                                      '("stale-query")
                                      agtags--global-to-list-cache
                                      '("stale-key"
                                        "stale-result"))
                                     (cl-letf
                                         (((symbol-function
                                            'read-directory-name)
                                           (lambda
                                             (&rest _arguments)
                                             root)))
                                       (agtags-update-tags))
                                     (let ((creation-message
                                            (current-message))
                                           (created
                                            (mapcar
                                             (lambda (name)
                                               (list
                                                name
                                                (file-exists-p
                                                 (expand-file-name
                                                  name
                                                  root))
                                                (neomacs-agtags-test-file-string
                                                 (expand-file-name
                                                  name
                                                  root))))
                                             agtags-created-tag-files)))
                                       (setq
                                        source-buffer
                                        (find-file-noselect
                                         source))
                                       (with-current-buffer
                                           source-buffer
                                         (agtags-mode
                                          1)
                                         (setq
                                          agtags--history-list
                                          '("parser_reset"
                                            "parse_request")
                                          agtags--global-to-list-cache
                                          '("cached-completion"
                                            "parser_reset"))
                                         (goto-char
                                          (point-min))
                                         (search-forward
                                          "return state + 1;")
                                         (replace-match
                                          "return state + 2;"
                                          t
                                          t)
                                         (save-buffer)
                                         (setq
                                          result
                                          (list
                                           creation-message
                                           created
                                           (substring-no-properties
                                            (buffer-string))
                                           (buffer-modified-p)
                                           agtags-mode
                                           (and
                                            (memq
                                             'agtags--auto-update
                                             before-save-hook)
                                            t)
                                           (and
                                            (memq
                                             'agtags-xref--backend
                                             xref-backend-functions)
                                            t)
                                           (and
                                            (memq
                                             #'agtags--completion-at-point
                                             completion-at-point-functions)
                                            t)
                                           agtags--history-list
                                           agtags--global-to-list-cache
                                           (neomacs-agtags-test-file-string
                                            (expand-file-name
                                             "GTAGS"
                                             root))
                                           (neomacs-agtags-test-file-string
                                            (cadr
                                             tools))))))))
                               (neomacs-agtags-test-cleanup
                                root))
                             result)"####;
    let expect = expect![[
        r##"OK (nil (("GPATH" t "paths=include/parser.h,src/main.c,src/parser.c\n") ("GTAGS" t "definitions=parser_init,parser_reset,parse_request\n") ("GRTAGS" t "references=src/main.c,src/parser.c\n")) "#include \"parser.h\"\nint parser_reset(int state) {\n  if (state < 0) return 0;\n  return state + 2;\n}\n" nil t t t t nil nil "definitions=parser_init,parser_reset,parse_request\nupdated=\n" "gtags cwd=[ORACLE-SANDBOX]/agtags-database-workflow <-i>\nglobal cwd=[ORACLE-SANDBOX]/agtags-database-workflow <-u> <--single-update=>\n")"##
    ]];

    assert_agtags_parity(elisp_form, expect);
}
