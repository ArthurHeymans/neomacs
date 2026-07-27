use expect_test::expect;

use super::assert_agtags_parity;

#[test]
fn agtags_completion_at_point_returns_real_symbol_bounds_candidates_annotation_and_nonexclusive_policy()
 {
    let elisp_form = r##"(cl-letf (((symbol-function
                     'agtags--run-cached-global-to-list)
                    (lambda (arguments)
                      (list
                       (concat
                        (cadr arguments)
                        "ha")
                       "alphabet"
                       "beta"))))
         (list
          (with-temp-buffer
            (emacs-lisp-mode)
            (insert "(alpha beta)")
            (goto-char 5)
            (let* ((completion
                    (agtags--completion-at-point))
                   (begin
                    (nth 0 completion))
                   (end
                    (nth 1 completion))
                   (table
                    (nth 2 completion))
                   (annotation
                    (plist-get
                     (nthcdr 3 completion)
                     :annotation-function)))
              (list
               begin end
               (buffer-substring-no-properties
                begin end)
               (all-completions
                "al" table)
               (funcall annotation
                        "alpha")
               (plist-get
                (nthcdr 3 completion)
                :exclusive))))
          (with-temp-buffer
            (insert "   ")
            (goto-char 2)
            (agtags--completion-at-point))))"##;
    let expect = expect![[r#"OK ((2 7 "alpha" ("alha" "alphabet") " Gtags" no) nil)"#]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_dynamic_completion_table_forwards_prefix_and_supports_completion_protocol_metadata() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'agtags--run-cached-global-to-list)
                    (lambda (arguments)
                      (push arguments calls)
                      '("alpha"
                        "alphabet"
                        "beta"))))
           (list
            (try-completion
             "al"
             agtags--completion-table)
            (all-completions
             "al"
             agtags--completion-table)
            (test-completion
             "alpha"
             agtags--completion-table)
            (completion-metadata
             "al"
             agtags--completion-table
             nil)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("alpha" ("alpha" "alphabet") t (metadata) (("-c" "al") ("-c" "al") ("-c" "alpha")))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_xref_parser_builds_file_locations_from_real_global_x_output_and_rejects_malformed_lines()
{
    let elisp_form = r##"(mapcar
         (lambda (line)
           (let ((xref
                  (agtags-xref--make-xref
                   line)))
             (when xref
               (let ((location
                      (xref-item-location
                       xref)))
                 (list
                  (xref-item-summary
                   xref)
                  (xref-file-location-file
                   location)
                  (xref-file-location-line
                   location)
                  (xref-file-location-column
                   location))))))
         '("main\t12\tsrc/main.c\tint main(void)"
           "helper 7 lib/helper.c static void helper()"
           "unicode_name 42 src/λ.c λ body"
           "missing fields"
           ""
           "symbol not-a-line file.c body"))"##;
    let expect = expect![[
        r#"OK (("int main(void)" "src/main.c" 12 0) ("static void helper()" "lib/helper.c" 7 0) ("λ body" "src/λ.c" 42 0) nil nil nil)"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_xref_find_symbol_builds_global_arguments_and_filters_bad_output_lines() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'agtags--run-global-to-list)
                    (lambda (arguments
                             &optional directory)
                      (push
                       (list arguments directory)
                       calls)
                      '("target 5 src/a.c definition body"
                        "malformed"
                        "target 18 src/b.c second body"))))
           (let ((results
                  (agtags-xref--find-symbol
                   "target"
                   "-d" "-i")))
             (list
              (mapcar
               (lambda (xref)
                 (let ((location
                        (xref-item-location
                         xref)))
                   (list
                    (xref-item-summary
                     xref)
                    (xref-file-location-file
                     location)
                    (xref-file-location-line
                     location))))
               results)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((("definition body" "src/a.c" 5) ("second body" "src/b.c" 18)) ((("-d" "-i" "-x" "-a" "target") nil)))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_xref_backend_activates_only_for_real_project_root_with_gtags_database() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-xref-backend"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (tag-file
                 (expand-file-name
                  "GTAGS" root)))
         (unwind-protect
             (progn
               (make-directory root t)
               (cl-letf (((symbol-function
                           'agtags--parse-root)
                          (lambda ()
                            (file-name-as-directory
                             root))))
                 (let ((before
                        (agtags-xref--backend)))
                   (write-region
                    "database" nil
                    tag-file nil 'silent)
                   (let ((active
                          (agtags-xref--backend)))
                     (delete-file tag-file)
                     (make-directory tag-file)
                     (list
                      before
                      active
                      (agtags-xref--backend))))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect!["OK (nil agtags nil)"];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_xref_methods_quote_definitions_references_and_preserve_apropos_patterns() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'agtags-xref--find-symbol)
                    (lambda (symbol
                             &rest arguments)
                      (push
                       (cons symbol arguments)
                       calls)
                      (list symbol
                            arguments))))
           (with-temp-buffer
             (insert
              (propertize
               "current-symbol"
               'face 'bold))
             (goto-char 5)
             (list
              (xref-backend-identifier-at-point
               'agtags)
              (eq
               (xref-backend-identifier-completion-table
                'agtags)
               agtags--completion-table)
              (xref-backend-definitions
               'agtags "-name+[x]")
              (xref-backend-references
               'agtags "reference")
              (xref-backend-apropos
               'agtags "-wild.*")
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ("current-symbol" t ("\\-name\\+\\[x]" #1=("-d")) ("reference" #2=("-r")) ("\\-wild.*" #3=("-g")) (("\\-name\\+\\[x]" . #1#) ("reference" . #2#) ("\\-wild.*" . #3#)))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_minor_mode_installs_runs_and_removes_buffer_local_save_xref_and_completion_hooks() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (add-hook
          'completion-at-point-functions
          #'ignore nil t)
         (agtags-mode 1)
         (let ((enabled
                (list
                 agtags-mode
                 (memq
                  'agtags--auto-update
                  before-save-hook)
                 (memq
                  'agtags-xref--backend
                  xref-backend-functions)
                 completion-at-point-functions
                 (local-variable-p
                  'before-save-hook)
                 (local-variable-p
                  'xref-backend-functions)
                 (local-variable-p
                  'completion-at-point-functions))))
           (agtags-mode -1)
           (list
            enabled
            agtags-mode
            (memq
             'agtags--auto-update
             before-save-hook)
            (memq
             'agtags-xref--backend
             xref-backend-functions)
            completion-at-point-functions)))"##;
    let expect = expect![
        "OK ((t (agtags--auto-update t) (agtags-xref--backend elisp--xref-backend t) (ignore elisp-completion-at-point t agtags--completion-at-point) t t t) nil nil nil (ignore elisp-completion-at-point t))"
    ];
    assert_agtags_parity(elisp_form, expect);
}
