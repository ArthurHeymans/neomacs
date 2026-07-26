use expect_test::expect;

use super::assert_ac_ispell_parity;

#[test]
fn ac_ispell_lookup_candidates_forwards_pattern_dictionary_and_caches_result_identity() {
    let elisp_form = r##"(let ((ac-ispell--cache
                    (make-ring 3))
                   (ispell-complete-word-dict
                    "fixture.dict")
                   (result
                    '("alpha" "alpine"))
                   events)
               (cl-letf
                   (((symbol-function
                      'fixture-lookup)
                     (lambda (&rest arguments)
                       (push arguments events)
                       result)))
                 (let ((returned
                        (ac-ispell--lookup-candidates
                         'fixture-lookup
                         "al")))
                   (list
                    returned
                    (eq returned result)
                    (ring-elements
                     ac-ispell--cache)
                    (nreverse events)))))"##;
    let expect =
        expect![[r#"OK (#1=("alpha" "alpine") t (("al" . #1#)) (("al*" "fixture.dict")))"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_candidates_cache_nil_results_but_repeat_the_lookup() {
    let elisp_form = r##"(let ((ac-ispell--cache
                    (make-ring 3))
                   (ispell-complete-word-dict
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'ispell-lookup-words)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       nil)))
                 (let ((ac-prefix
                        "word"))
                   (list
                    (ac-ispell--candidates)
                    (ac-ispell--candidates)
                    (nreverse calls)
                    (ring-elements
                     ac-ispell--cache)))))"##;
    let expect = expect![[r#"OK (nil nil (("word*" nil) ("word*" nil)) (("word") ("word")))"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_candidate_cache_evicts_the_oldest_entry_at_capacity() {
    let elisp_form = r##"(let ((ac-ispell--cache
                    (make-ring 3))
                   (ispell-complete-word-dict
                    nil))
               (cl-letf
                   (((symbol-function
                      'fixture-lookup)
                     (lambda (pattern _dictionary)
                       (list pattern))))
                 (mapc
                  (lambda (prefix)
                    (ac-ispell--lookup-candidates
                     'fixture-lookup prefix))
                  '("a" "b" "c" "d"))
                 (mapcar
                  #'car
                  (ring-elements
                   ac-ispell--cache))))"##;
    let expect = expect![[r#"OK ("d" "c" "b")"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_lookup_cache_prefers_the_newest_matching_prefix_without_mutation() {
    let elisp_form = r##"(let ((ac-ispell--cache
                    (make-ring 5))
                   (a-result
                    '("a-result"))
                   (ab-result
                    '("ab-result")))
               (ring-insert
                ac-ispell--cache
                (cons "a" a-result))
               (ring-insert
                ac-ispell--cache
                (cons "ab" ab-result))
               (let ((before
                      (ring-elements
                       ac-ispell--cache))
                     (returned
                      (ac-ispell--lookup-cache
                       "abc")))
                 (list
                  returned
                  (eq returned ab-result)
                  (ring-elements
                   ac-ispell--cache)
                  (equal
                   before
                   (ring-elements
                    ac-ispell--cache)))))"##;
    let expect = expect![[r#"OK (#1=("ab-result") t (("ab" . #1#) ("a" "a-result")) t)"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_lookup_cache_treats_cached_prefixes_as_regexps() {
    let elisp_form = r##"(let ((ac-ispell--cache
                    (make-ring 4)))
               (ring-insert
                ac-ispell--cache
                '("literal["
                  "invalid-regexp"))
               (ring-insert
                ac-ispell--cache
                '("a."
                  "regexp-match"))
               (list
                (ac-ispell--lookup-cache
                 "abacus")
                (condition-case error-data
                    (ac-ispell--lookup-cache
                     "literal[value")
                  (error
                   (cons
                    :error error-data)))))"##;
    let expect = expect![[r#"OK (("regexp-match") (:error invalid-regexp "Unmatched [ or [^"))"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_lookup_cache_returns_nil_for_empty_or_nonmatching_caches() {
    let elisp_form = r##"(let ((ac-ispell--cache
                    (make-ring 3)))
               (let ((empty
                      (ac-ispell--lookup-cache
                       "word")))
                 (ring-insert
                  ac-ispell--cache
                  '("long-prefix"
                    "candidate"))
                 (list
                  empty
                  (ac-ispell--lookup-cache
                   "short")
                  (ring-elements
                   ac-ispell--cache))))"##;
    let expect = expect![[r#"OK (nil nil (("long-prefix" "candidate")))"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}
