use expect_test::expect;

use super::{assert_ac_emmet_parity, assert_unshimmed_ac_emmet_signal_parity};

#[test]
fn ac_emmet_unshimmed_source_signals_for_the_unrequired_legacy_loop_macro() {
    let elisp_form = r##"(list
               (featurep 'ac-emmet)
               (hash-table-p
                ac-emmet-html-snippets-hash)
               (length
                ac-emmet-html-snippets-keys))"##;
    let expect = expect!["ERR (void-function loop)"];

    assert_unshimmed_ac_emmet_signal_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_exact_pin_dependencies_features_faces_and_defaults_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-emmet package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-emmet auto-complete emmet-mode))
                (facep 'ac-emmet-candidate-face)
                (get
                 'ac-emmet-candidate-face
                 'face-defface-spec)
                (facep 'ac-emmet-selection-face)
                (get
                 'ac-emmet-selection-face
                 'face-defface-spec)
                ac-emmet-source-defaults))"##;
    let expect = expect![[
        r#"OK (ac-emmet "20131015.1558" ((emmet-mode (1 0 2)) (auto-complete (1 4))) (t t t) [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit ac-candidate-face))) [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit ac-selection-face))) ((candidate-face . ac-emmet-candidate-face) (selection-face . ac-emmet-selection-face) (symbol . "a") (requires . 1) (action lambda nil (call-interactively 'emmet-expand-line))))"#
    ]];

    assert_ac_emmet_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_hashes_and_candidate_snapshots_match_emmet_data_shapes() {
    let elisp_form = r##"(list
               (mapcar
                #'hash-table-p
                (list
                 ac-emmet-html-snippets-hash
                 ac-emmet-html-aliases-hash
                 ac-emmet-css-snippets-hash))
               (mapcar
                #'hash-table-count
                (list
                 ac-emmet-html-snippets-hash
                 ac-emmet-html-aliases-hash
                 ac-emmet-css-snippets-hash))
               (mapcar
                #'length
                (list
                 ac-emmet-html-snippets-keys
                 ac-emmet-html-aliases-keys
                 ac-emmet-css-snippets-keys))
               (mapcar
                (lambda (pair)
                  (let ((hash (car pair))
                        (keys (cdr pair)))
                    (list
                     (= (hash-table-count hash)
                        (length keys))
                     (length
                      (delete-dups
                       (copy-sequence keys)))
                     (cl-every
                      (lambda (key)
                        (not
                         (eq
                          (gethash
                           key hash
                           :missing)
                          :missing)))
                      keys))))
                (list
                 (cons
                  ac-emmet-html-snippets-hash
                  ac-emmet-html-snippets-keys)
                 (cons
                  ac-emmet-html-aliases-hash
                  ac-emmet-html-aliases-keys)
                 (cons
                  ac-emmet-css-snippets-hash
                  ac-emmet-css-snippets-keys))))"##;
    let expect = expect!["OK ((t t t) (9 112 641) (9 112 641) ((t 9 t) (t 112 t) (t 641 t)))"];

    assert_ac_emmet_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_sources_prepend_unique_candidates_and_share_default_tail() {
    let elisp_form = r##"(let ((sources
                    (list
                     ac-source-emmet-html-snippets
                     ac-source-emmet-html-aliases
                     ac-source-emmet-css-snippets)))
               (list
                sources
                (mapcar
                 (lambda (source)
                   (list
                    (cdr
                     (assq
                      'candidates source))
                    (functionp
                     (cdr
                      (assq
                       'document source)))
                    (eq
                     (nthcdr 2 source)
                     ac-emmet-source-defaults)))
                 sources)
                (eq
                 (nthcdr
                  2 ac-source-emmet-html-snippets)
                 (nthcdr
                  2 ac-source-emmet-html-aliases))
                (eq
                 (nthcdr
                  2 ac-source-emmet-html-aliases)
                 (nthcdr
                  2 ac-source-emmet-css-snippets))))"##;
    let expect = expect![[
        r#"OK ((((candidates . ac-emmet-html-snippets-keys) (document lambda (s) (ac-emmet-document s ac-emmet-html-snippets-hash)) . #1=((candidate-face . ac-emmet-candidate-face) (selection-face . ac-emmet-selection-face) (symbol . "a") (requires . 1) (action lambda nil (call-interactively 'emmet-expand-line)))) ((candidates . ac-emmet-html-aliases-keys) (document lambda (s) (ac-emmet-document s ac-emmet-html-aliases-hash)) . #1#) ((candidates . ac-emmet-css-snippets-keys) (document lambda (s) (ac-emmet-document s ac-emmet-css-snippets-hash)) . #1#)) ((ac-emmet-html-snippets-keys t nil) (ac-emmet-html-aliases-keys t nil) (ac-emmet-css-snippets-keys t nil)) t t)"#
    ]];

    assert_ac_emmet_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_source_action_calls_emmet_expand_line_interactively() {
    let elisp_form = r##"(let ((action
                    (cdr
                     (assq
                      'action
                      ac-emmet-source-defaults)))
                   calls)
               (cl-letf
                   (((symbol-function
                      'call-interactively)
                     (lambda
                         (function
                          &optional record-flag
                          keys)
                       (push
                        (list
                         function
                         record-flag
                         keys)
                        calls)
                       'expanded)))
                 (list
                  (funcall action)
                  (nreverse calls))))"##;
    let expect = expect!["OK (expanded ((emmet-expand-line nil nil)))"];

    assert_ac_emmet_parity(elisp_form, expect);
}
