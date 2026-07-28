use expect_test::expect;

use super::assert_agtags_parity;

#[test]
fn agtags_completion_xref_and_open_file_work_together_on_a_real_project_tree() {
    let elisp_form = r####"(let* ((root
                                  (file-name-as-directory
                                   (expand-file-name
                                    "agtags-xref-workflow"
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
                                 (header
                                  (expand-file-name
                                   "include/parser.h"
                                   root))
                                 main-buffer
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
                                      "  /* parser_re */\n"
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
                                      main-buffer
                                      (find-file-noselect
                                       main))
                                     (with-current-buffer
                                         main-buffer
                                       (agtags-mode
                                        1)
                                       (goto-char
                                        (point-min))
                                       (search-forward
                                        "parser_re")
                                       (let* ((capf
                                               (run-hook-with-args-until-success
                                                'completion-at-point-functions))
                                              (begin
                                               (nth
                                                0
                                                capf))
                                              (end
                                               (nth
                                                1
                                                capf))
                                              (table
                                               (nth
                                                2
                                                capf))
                                              (annotation
                                               (plist-get
                                                (nthcdr
                                                 3
                                                 capf)
                                                :annotation-function))
                                              (prefix
                                               (buffer-substring-no-properties
                                                begin
                                                end))
                                              (first-completion
                                               (all-completions
                                                prefix
                                                table))
                                              (second-completion
                                               (all-completions
                                                prefix
                                                table))
                                              (backend
                                               (run-hook-with-args-until-success
                                                'xref-backend-functions))
                                              (definitions
                                               (xref-backend-definitions
                                                backend
                                                "parser_reset"))
                                              (references
                                               (xref-backend-references
                                                backend
                                                "parser_reset"))
                                              (describe
                                               (lambda (item)
                                                 (let* ((location
                                                         (xref-item-location
                                                          item))
                                                        (marker
                                                         (xref-location-marker
                                                          location)))
                                                   (list
                                                    (xref-item-summary
                                                     item)
                                                    (file-relative-name
                                                     (buffer-file-name
                                                      (marker-buffer
                                                       marker))
                                                     root)
                                                    (with-current-buffer
                                                        (marker-buffer
                                                         marker)
                                                      (save-excursion
                                                        (goto-char
                                                         marker)
                                                        (list
                                                         (line-number-at-pos)
                                                         (current-column)
                                                         (buffer-substring-no-properties
                                                          (line-beginning-position)
                                                          (line-end-position)))))))))
                                              (definition-data
                                               (mapcar
                                                describe
                                                definitions))
                                              (reference-data
                                               (mapcar
                                                describe
                                                references))
                                              (first-reference
                                               (xref-location-marker
                                                (xref-item-location
                                                 (car
                                                  references)))))
                                         (switch-to-buffer
                                          (marker-buffer
                                           first-reference))
                                         (goto-char
                                          first-reference)
                                         (let ((navigated-reference
                                                (list
                                                 (file-relative-name
                                                  buffer-file-name
                                                  root)
                                                 (line-number-at-pos)
                                                 (current-column)
                                                 (buffer-substring-no-properties
                                                  (line-beginning-position)
                                                  (line-end-position)))))
                                           (cl-letf
                                               (((symbol-function
                                                  'completing-read)
                                                 (lambda
                                                   (&rest _arguments)
                                                   "include/parser.h")))
                                             (agtags-open-file))
                                           (setq
                                            result
                                            (list
                                             (list
                                              begin
                                              end
                                              prefix
                                              first-completion
                                              second-completion
                                              (mapcar
                                               annotation
                                               first-completion)
                                              (plist-get
                                               (nthcdr
                                                3
                                                capf)
                                               :exclusive)
                                              agtags--global-to-list-cache)
                                             backend
                                             definition-data
                                             reference-data
                                             navigated-reference
                                             (file-relative-name
                                              buffer-file-name
                                              root)
                                             (point)
                                             (substring-no-properties
                                              (buffer-string))
                                             (neomacs-agtags-test-file-string
                                              (cadr
                                               tools)))))))))
                               (neomacs-agtags-test-cleanup
                                root))
                             result)"####;
    let expect = expect![[
        r##"OK ((61 70 "parser_re" ("parser_reset") ("parser_reset") (" Gtags") no ("[ORACLE-SANDBOX]/agtags-xref-workflow/$-c$parser_re" "parse_request" "parser_init" "parser_reset")) agtags (("int parser_reset(int state) {" "src/parser.c" (2 0 "int parser_reset(int state) {"))) (("return parser_reset(input);" "src/main.c" (5 0 "  return parser_reset(input);")) ("return parser_reset(state - 1);" "src/parser.c" (7 0 "  return parser_reset(state - 1);"))) ("src/main.c" 5 0 "  return parser_reset(input);") "include/parser.h" 1 "#pragma once\nint parser_reset(int state);\n" "global cwd=[ORACLE-SANDBOX]/agtags-xref-workflow <-c> <parser_re>\nglobal cwd=[ORACLE-SANDBOX]/agtags-xref-workflow <-d> <-x> <-a> <parser_reset>\nglobal cwd=[ORACLE-SANDBOX]/agtags-xref-workflow <-r> <-x> <-a> <parser_reset>\n")"##
    ]];

    assert_agtags_parity(elisp_form, expect);
}
