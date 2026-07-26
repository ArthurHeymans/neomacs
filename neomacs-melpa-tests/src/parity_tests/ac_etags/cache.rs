use expect_test::expect;

use super::{assert_ac_etags_parity, assert_ac_etags_signal_parity};

#[test]
fn ac_etags_cache_candidates_passes_exact_arguments_and_retains_result_identity() {
    let elisp_form = r##"(let ((ac-etags--completion-cache
                    (make-hash-table :test 'equal))
                   (table
                    '(fixture completion table))
                   (candidates
                    (list "alpha" "alpine"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'tags-completion-table)
                     (lambda ()
                       (push '(tags-completion-table)
                             calls)
                       table))
                    ((symbol-function
                      'all-completions)
                     (lambda (prefix actual-table
                              &rest optional)
                       (push
                        (list
                         'all-completions
                         prefix
                         actual-table
                         optional)
                        calls)
                       candidates)))
                 (let ((result
                        (ac-etags--cache-candidates
                         "al")))
                   (list
                    result
                    (eq result candidates)
                    (gethash
                     "al"
                     ac-etags--completion-cache)
                    (eq
                     result
                     (gethash
                      (copy-sequence "al")
                      ac-etags--completion-cache))
                    (nreverse calls)
                    (hash-table-count
                     ac-etags--completion-cache)))))"##;
    let expect = expect![[
        r#"OK (#1=("alpha" "alpine") t #1# t ((tags-completion-table) (all-completions "al" (fixture completion table) nil)) 1)"#
    ]];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_cache_candidates_does_not_cache_empty_results() {
    let elisp_form = r##"(let ((ac-etags--completion-cache
                    (make-hash-table :test 'equal))
                   calls)
               (puthash
                "existing"
                'preserved
                ac-etags--completion-cache)
               (cl-letf
                   (((symbol-function
                      'tags-completion-table)
                     (lambda ()
                       (push 'table calls)
                       'fixture-table))
                    ((symbol-function
                      'all-completions)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       nil)))
                 (list
                  (ac-etags--cache-candidates
                   "missing")
                  (gethash
                   "missing"
                   ac-etags--completion-cache
                   'absent)
                  (gethash
                   "existing"
                   ac-etags--completion-cache)
                  (hash-table-count
                   ac-etags--completion-cache)
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil absent preserved 1 (table ("missing" fixture-table)))"#]];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_cache_candidates_demotes_tag_table_errors_without_mutating_cache() {
    let elisp_form = r##"(let ((ac-etags--completion-cache
                    (make-hash-table :test 'equal))
                   messages)
               (cl-letf
                   (((symbol-function
                      'tags-completion-table)
                     (lambda ()
                       (error
                        "fixture tag failure")))
                    ((symbol-function
                      'message)
                     (lambda (format-string
                              &rest arguments)
                       (push
                        (apply
                         #'format-message
                         format-string
                         arguments)
                        messages))))
                 (list
                  (ac-etags--cache-candidates
                   "fixture")
                  (nreverse messages)
                  (hash-table-count
                   ac-etags--completion-cache)
                  (gethash
                   "fixture"
                   ac-etags--completion-cache
                   'absent))))"##;
    let expect = expect![[r#"OK (nil ("(error fixture tag failure)") 0 absent)"#]];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_cache_candidates_does_not_demote_all_completions_errors() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'tags-completion-table)
                 (lambda ()
                   'fixture-table))
                ((symbol-function
                  'all-completions)
                 (lambda (&rest _arguments)
                   (signal
                    'error
                    '("fixture completion failure")))))
               (ac-etags--cache-candidates
                "fixture"))"##;
    let expect = expect![[r#"ERR (error "fixture completion failure")"#]];

    assert_ac_etags_signal_parity(elisp_form, expect);
}

#[test]
fn ac_etags_candidates_gates_on_tag_tables_and_uses_live_prefix_cache_keys() {
    let elisp_form = r##"(let ((ac-etags--completion-cache
                    (make-hash-table :test 'equal))
                   calls)
               (puthash
                "cached"
                '("cached-value")
                ac-etags--completion-cache)
               (cl-letf
                   (((symbol-function
                      'ac-etags--cache-candidates)
                     (lambda (prefix)
                       (push prefix calls)
                       (list
                        "generated"
                        prefix))))
                 (list
                  (let ((tags-table-list nil)
                        (ac-prefix "missing"))
                    (ac-etags--candidates))
                  (let ((tags-table-list
                         '("TAGS"))
                        (ac-prefix "cached"))
                    (ac-etags--candidates))
                  (let ((tags-table-list
                         '("TAGS"))
                        (ac-prefix "first"))
                    (ac-etags--candidates))
                  (let ((tags-table-list
                         '("TAGS"))
                        (ac-prefix "second"))
                    (ac-etags--candidates))
                  (nreverse calls)
                  (hash-table-count
                   ac-etags--completion-cache))))"##;
    let expect = expect![[
        r#"OK (nil ("cached-value") ("generated" "first") ("generated" "second") ("first" "second") 1)"#
    ]];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_clear_cache_empties_the_same_hash_table_and_is_idempotent() {
    let elisp_form = r##"(let ((ac-etags--completion-cache
                    (make-hash-table :test 'equal)))
               (puthash
                "one" 'first
                ac-etags--completion-cache)
               (puthash
                "two" 'second
                ac-etags--completion-cache)
               (let ((identity
                      ac-etags--completion-cache))
                 (list
                  (hash-table-count identity)
                  (ac-etags-clear-cache)
                  (eq
                   identity
                   ac-etags--completion-cache)
                  (hash-table-count identity)
                  (ac-etags-clear-cache)
                  (hash-table-count identity)
                  (gethash
                   "one" identity 'absent)
                  (interactive-form
                   #'ac-etags-clear-cache))))"##;
    let expect = expect!["OK (2 #1=#s(hash-table test equal) t 0 #1# 0 absent (interactive nil))"];

    assert_ac_etags_parity(elisp_form, expect);
}
