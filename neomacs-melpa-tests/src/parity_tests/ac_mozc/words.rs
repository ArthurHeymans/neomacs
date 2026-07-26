use expect_test::expect;

use super::{assert_ac_mozc_parity, assert_ac_mozc_signal_parity};

#[test]
fn ac_mozc_remove_non_ascii_splits_each_word_drops_empty_runs_and_preserves_input() {
    let elisp_form = r##"(let ((words
                    '("alpha日本beta"
                      "日本"
                      ""
                      "a-b_c 123"
                      "λx"
                      "plain")))
               (list
                (ac-mozc-remove-non-ascii-character
                 words)
                words))"##;
    let expect = expect![[
        r#"OK (("alpha" "beta" "a-b_c 123" "x" "plain") ("alpha日本beta" "日本" "" "a-b_c 123" "λx" "plain"))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_partial_match_uses_word_boundaries_live_case_folding_and_unquoted_regex_input() {
    let elisp_form = r##"(let ((collection
                    '("alpha"
                      "alphabet"
                      "x alpha"
                      "xalpha"
                      "Alpha"
                      "beta-alpha"
                      "alp"
                      "a.p")))
               (list
                (let ((case-fold-search
                       t))
                  (ac-mozc-partial-match
                   "alpha"
                   collection))
                (let ((case-fold-search
                       nil))
                  (ac-mozc-partial-match
                   "alpha"
                   collection))
                (ac-mozc-partial-match
                 "a.p"
                 collection)
                (ac-mozc-partial-match
                 ""
                 collection)
                collection))"##;
    let expect = expect![[
        r#"OK (("alpha" "alphabet" "x alpha" "Alpha" "beta-alpha") ("alpha" "alphabet" "x alpha" "beta-alpha") ("alpha" "alphabet" "x alpha" "Alpha" "beta-alpha" "alp" "a.p") #1=("alpha" "alphabet" "x alpha" "xalpha" "Alpha" "beta-alpha" "alp" "a.p") #1#)"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_partial_match_preserves_the_callers_existing_match_data() {
    let elisp_form = r##"(progn
               (string-match
                "\\(seed\\)"
                "seed")
               (let ((before
                      (match-data)))
                 (list
                  (ac-mozc-partial-match
                   "alp"
                   '("alpha"
                     "beta"))
                  before
                  (match-data)
                  (equal
                   before
                   (match-data)))))"##;
    let expect = expect![[r#"OK (("alpha") (0 4 0 4) (0 4 0 4) t)"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_partial_match_propagates_an_invalid_user_regexp() {
    let elisp_form = r##"(ac-mozc-partial-match
               "["
               '("alpha"))"##;
    let expect = expect![[r#"ERR (invalid-regexp "Unmatched [ or [^")"#]];

    assert_ac_mozc_signal_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_ascii_word_candidates_dynamically_selects_matcher_forwards_predicate_and_filters() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-word-candidates)
                     (lambda (&optional predicate)
                       (push
                        (list
                         predicate
                         ac-match-function)
                        calls)
                       '("alpha日本beta"
                         "日本"
                         "gamma-delta"
                         "λx"))))
                 (list
                  (ac-mozc-word-candidates-ascii-only
                   'fixture-buffer-predicate)
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("alpha" "beta" "gamma-delta" "x") ((fixture-buffer-predicate ac-mozc-partial-match)))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_ascii_source_buffer_predicate_compares_each_buffer_mode_to_current_derivation() {
    let elisp_form = r##"(let* ((candidate-form
                      (cdr
                       (assq
                        'candidates
                        ac-source-ascii-words-in-same-mode-buffers)))
                     (predicate
                      (nth 1
                           candidate-form))
                     (fundamental
                      (generate-new-buffer
                       " *ac-mozc fundamental*"))
                     (text
                      (generate-new-buffer
                       " *ac-mozc text*"))
                     (lisp
                      (generate-new-buffer
                       " *ac-mozc lisp*")))
               (unwind-protect
                   (progn
                     (with-current-buffer
                         fundamental
                       (fundamental-mode))
                     (with-current-buffer
                         text
                       (text-mode))
                     (with-current-buffer
                         lisp
                       (emacs-lisp-mode))
                     (with-temp-buffer
                       (text-mode)
                       (list
                        candidate-form
                        (funcall
                         predicate
                         fundamental)
                        (funcall
                         predicate
                         text)
                        (funcall
                         predicate
                         lisp))))
                 (kill-buffer
                  fundamental)
                 (kill-buffer
                  text)
                 (kill-buffer
                  lisp)))"##;
    let expect = expect![[
        r#"OK ((ac-mozc-word-candidates-ascii-only (lambda (buffer) (derived-mode-p (buffer-local-value 'major-mode buffer)))) nil text-mode nil)"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}
