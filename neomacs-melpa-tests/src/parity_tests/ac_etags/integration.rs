use expect_test::expect;

use super::assert_ac_etags_parity;

#[test]
fn ac_etags_source_completes_against_a_real_sandbox_local_tags_table() {
    let elisp_form = r##"(let* ((root
                     (make-temp-file
                      "ac-etags-table-" t))
                    (tags-file
                     (expand-file-name
                      "TAGS" root))
                    (tags-table-list
                     (list tags-file))
                    (tags-file-name nil)
                    (tags-completion-table nil)
                    (tags-table-computed-list nil)
                    (tags-table-computed-list-for nil)
                    (ac-etags--completion-cache
                     (make-hash-table :test 'equal)))
               (unwind-protect
                   (progn
                     (with-temp-file tags-file
                       (insert
                        "\f\nfixture.c,120\n"
                        "int alpha(void) {\177alpha\0011,0\n"
                        "int alphabet(void) {\177alphabet\0012,20\n"
                        "int alpine(void) {\177alpine\0013,45\n"
                        "int beta(void) {\177beta\0014,68\n"))
                     (ac-etags-setup)
                     (let* ((ac-prefix "al")
                            (candidate-function
                             (cdr
                              (assq
                               'candidates
                               ac-source-etags)))
                            (first
                             (funcall
                              candidate-function))
                            (second
                             (funcall
                              candidate-function)))
                       (list
                        candidate-function
                        first
                        (eq first second)
                        (gethash
                         "al"
                         ac-etags--completion-cache)
                        (hash-table-count
                         ac-etags--completion-cache))))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK (ac-etags--candidates #1=("alpha" "alphabet" "alpine") t #1# 1)"#]];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_clear_cache_exposes_a_replaced_real_tags_table() {
    let elisp_form = r##"(let* ((root
                     (make-temp-file
                      "ac-etags-switch-" t))
                    (first-tags
                     (expand-file-name
                      "FIRST-TAGS" root))
                    (second-tags
                     (expand-file-name
                      "SECOND-TAGS" root))
                    (tags-file-name nil)
                    (tags-completion-table nil)
                    (tags-table-computed-list nil)
                    (tags-table-computed-list-for nil)
                    (ac-etags--completion-cache
                     (make-hash-table :test 'equal))
                    (ac-prefix "same"))
               (unwind-protect
                   (progn
                     (with-temp-file first-tags
                       (insert
                        "\f\nfirst.c,40\n"
                        "int same_first;\177same_first\0011,0\n"))
                     (with-temp-file second-tags
                       (insert
                        "\f\nsecond.c,40\n"
                        "int same_second;\177same_second\0011,0\n"))
                     (setq tags-table-list
                           (list first-tags))
                     (let ((first
                            (ac-etags--candidates)))
                       (setq tags-table-list
                             (list second-tags)
                             tags-file-name nil
                             tags-completion-table nil
                             tags-table-computed-list nil
                             tags-table-computed-list-for nil)
                       (let ((stale
                              (ac-etags--candidates)))
                         (ac-etags-clear-cache)
                         (let ((fresh
                                (ac-etags--candidates)))
                           (list
                            first
                            stale
                            (eq first stale)
                            fresh
                            (hash-table-count
                             ac-etags--completion-cache))))))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK (#1=("same_first") #1# t ("same_second") 1)"#]];

    assert_ac_etags_parity(elisp_form, expect);
}
