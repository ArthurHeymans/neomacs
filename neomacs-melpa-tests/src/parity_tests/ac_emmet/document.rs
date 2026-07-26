use expect_test::expect;

use super::assert_ac_emmet_parity;

#[test]
fn ac_emmet_document_returns_strings_calls_functions_and_preserves_nil() {
    let elisp_form = r##"(let ((hash
                    (make-hash-table
                     :test 'equal))
                   calls)
               (puthash
                "literal"
                "<div>${child}</div>"
                hash)
               (puthash
                "computed"
                (lambda (input)
                  (push input calls)
                  (concat
                   "generated:" input))
                hash)
               (puthash "empty" nil hash)
               (list
                (ac-emmet-document
                 "literal" hash)
                (ac-emmet-document
                 "computed" hash)
                (ac-emmet-document
                 "empty" hash)
                (ac-emmet-document
                 "missing" hash)
                (nreverse calls)))"##;
    let expect = expect![[r#"OK ("<div>${child}</div>" "generated:" nil nil (""))"#]];

    assert_ac_emmet_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_document_functions_receive_exactly_one_empty_string() {
    let elisp_form = r##"(let ((hash
                    (make-hash-table
                     :test 'equal)))
               (puthash
                "arity"
                (lambda (&rest args)
                  args)
                hash)
               (ac-emmet-document
                "arity" hash))"##;
    let expect = expect![[r#"OK ("")"#]];

    assert_ac_emmet_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_source_document_callbacks_read_the_corresponding_live_hash() {
    let elisp_form = r##"(let ((html-key
                    "neomacs-html-fixture")
                   (alias-key
                    "neomacs-alias-fixture")
                   (css-key
                    "neomacs-css-fixture"))
               (unwind-protect
                   (progn
                     (puthash
                      html-key
                      "<fixture-html>"
                      ac-emmet-html-snippets-hash)
                     (puthash
                      alias-key
                      "fixture-alias"
                      ac-emmet-html-aliases-hash)
                     (puthash
                      css-key
                      "fixture:css"
                      ac-emmet-css-snippets-hash)
                     (list
                      (funcall
                       (cdr
                        (assq
                         'document
                         ac-source-emmet-html-snippets))
                       html-key)
                      (funcall
                       (cdr
                        (assq
                         'document
                         ac-source-emmet-html-aliases))
                       alias-key)
                      (funcall
                       (cdr
                        (assq
                         'document
                         ac-source-emmet-css-snippets))
                       css-key)))
                 (remhash
                  html-key
                  ac-emmet-html-snippets-hash)
                 (remhash
                  alias-key
                  ac-emmet-html-aliases-hash)
                 (remhash
                  css-key
                  ac-emmet-css-snippets-hash)))"##;
    let expect = expect![[r#"OK ("<fixture-html>" "fixture-alias" "fixture:css")"#]];

    assert_ac_emmet_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_candidate_key_lists_are_load_time_snapshots_of_live_hashes() {
    let elisp_form = r##"(let ((key
                    "neomacs-snapshot-fixture"))
               (unwind-protect
                   (progn
                     (puthash
                      key
                      "fixture"
                      ac-emmet-html-snippets-hash)
                     (list
                      (member
                       key
                       ac-emmet-html-snippets-keys)
                      (gethash
                       key
                       ac-emmet-html-snippets-hash)
                      (funcall
                       (cdr
                        (assq
                         'document
                         ac-source-emmet-html-snippets))
                       key)))
                 (remhash
                  key
                  ac-emmet-html-snippets-hash)))"##;
    let expect = expect![[r#"OK (nil "fixture" "fixture")"#]];

    assert_ac_emmet_parity(elisp_form, expect);
}
