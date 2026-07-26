use expect_test::expect;

use super::assert_ac_slime_parity;

#[test]
fn ac_slime_setup_prepends_simple_or_fuzzy_source_and_is_idempotent() {
    let elisp_form = r##"(list
              (let ((ac-sources
                     '(existing)))
                (list
                 (set-up-slime-ac)
                 ac-sources
                 (set-up-slime-ac)
                 ac-sources))
              (let ((ac-sources
                     '(existing)))
                (list
                 (set-up-slime-ac
                  t)
                 ac-sources
                 (set-up-slime-ac
                  'non-nil)
                 ac-sources))
              (let ((ac-sources
                     '(ac-source-slime-simple
                       existing)))
                (list
                 (set-up-slime-ac
                  t)
                 ac-sources
                 (set-up-slime-ac)
                 ac-sources)))"##;
    let expect = expect![
        "OK ((#1=(ac-source-slime-simple existing) #1# #1# #1#) (#2=(ac-source-slime-fuzzy existing) #2# #2# #2#) (#3=(ac-source-slime-fuzzy ac-source-slime-simple existing) #3# #3# #3#))"
    ];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_setup_treats_only_nil_as_the_simple_source_selector() {
    let elisp_form = r##"(mapcar
              (lambda (fuzzy)
                (let (ac-sources)
                  (set-up-slime-ac
                   fuzzy)
                  (list
                   fuzzy
                   ac-sources)))
              '(nil
                t
                0
                ""
                fuzzy))"##;
    let expect = expect![[
        r#"OK ((nil (ac-source-slime-simple)) (t (ac-source-slime-fuzzy)) (0 (ac-source-slime-fuzzy)) ("" (ac-source-slime-fuzzy)) (fuzzy (ac-source-slime-fuzzy)))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_setup_changes_only_the_current_buffers_local_source_list() {
    let elisp_form = r##"(let ((default-before
                    (default-value
                     'ac-sources))
                   first-state
                   second-state)
               (with-temp-buffer
                 (setq-local
                  ac-sources
                  '(first))
                 (set-up-slime-ac)
                 (setq
                  first-state
                  (list
                   ac-sources
                   (local-variable-p
                    'ac-sources))))
               (with-temp-buffer
                 (setq-local
                  ac-sources
                  '(second))
                 (set-up-slime-ac
                  t)
                 (setq
                  second-state
                  (list
                   ac-sources
                   (local-variable-p
                    'ac-sources))))
               (list
                first-state
                second-state
                (equal
                 default-before
                 (default-value
                  'ac-sources))))"##;
    let expect =
        expect!["OK (((ac-source-slime-simple first) t) ((ac-source-slime-fuzzy second) t) t)"];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_setup_interactive_invocation_selects_simple_without_prompting() {
    let elisp_form = r##"(let ((ac-sources
                    '(existing)))
               (list
                (call-interactively
                 #'set-up-slime-ac)
                ac-sources
                (interactive-form
                 'set-up-slime-ac)))"##;
    let expect = expect!["OK (#1=(ac-source-slime-simple existing) #1# (interactive nil))"];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_source_callbacks_honor_runtime_rebinding_and_match_policies() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-slime-init)
                     (lambda ()
                       (push
                        'init
                        calls)
                       'initialized))
                    ((symbol-function
                      'ac-source-slime-fuzzy-candidates)
                     (lambda ()
                       (push
                        'fuzzy-candidates
                        calls)
                       '(fuzzy)))
                    ((symbol-function
                      'ac-source-slime-simple-candidates)
                     (lambda ()
                       (push
                        'simple-candidates
                        calls)
                       '(simple)))
                    ((symbol-function
                      'slime-symbol-start-pos)
                     (lambda ()
                       (push
                        'prefix
                        calls)
                       7))
                    ((symbol-function
                      'ac-slime-documentation)
                     (lambda (candidate)
                       (push
                        (list
                         'document
                         candidate)
                        calls)
                       'documented))
                    ((symbol-function
                      'ac-source-slime-case-correcting-completions)
                     (lambda (name collection)
                       (push
                        (list
                         'match
                         name
                         collection)
                        calls)
                       'matched)))
                 (list
                  (funcall
                   (cdr
                    (assq
                     'init
                     ac-source-slime-fuzzy)))
                  (funcall
                   (cdr
                    (assq
                     'candidates
                     ac-source-slime-fuzzy)))
                  (funcall
                   (cdr
                    (assq
                     'prefix
                     ac-source-slime-fuzzy)))
                  (funcall
                   (cdr
                    (assq
                     'document
                     ac-source-slime-fuzzy))
                   'candidate-a)
                  (funcall
                   (cdr
                    (assq
                     'match
                     ac-source-slime-fuzzy))
                   "prefix"
                   '(one two))
                  (funcall
                   (cdr
                    (assq
                     'candidates
                     ac-source-slime-simple)))
                  (funcall
                   (cdr
                    (assq
                     'match
                     ac-source-slime-simple))
                   "name"
                   '(three four))
                  (nreverse
                   calls))))"##;
    let expect = expect![[
        r#"OK (initialized (fuzzy) 7 documented (one two) (simple) matched (init fuzzy-candidates prefix (document candidate-a) simple-candidates (match "name" (three four))))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}
