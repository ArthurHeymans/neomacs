use expect_test::expect;

use super::assert_ac_sly_parity;

#[test]
fn ac_sly_setup_prepends_simple_or_fuzzy_source_and_is_idempotent() {
    let elisp_form = r##"(list
              (let ((ac-sources
                     '(existing)))
                (list
                 (set-up-sly-ac)
                 ac-sources
                 (set-up-sly-ac)
                 ac-sources))
              (let ((ac-sources
                     '(existing)))
                (list
                 (set-up-sly-ac
                  t)
                 ac-sources
                 (set-up-sly-ac
                  'non-nil)
                 ac-sources))
              (let ((ac-sources
                     '(ac-source-sly-simple
                       existing)))
                (list
                 (set-up-sly-ac
                  t)
                 ac-sources
                 (set-up-sly-ac)
                 ac-sources)))"##;
    let expect = expect![
        "OK ((#1=(ac-source-sly-simple existing) #1# #1# #1#) (#2=(ac-source-sly-fuzzy existing) #2# #2# #2#) (#3=(ac-source-sly-fuzzy ac-source-sly-simple existing) #3# #3# #3#))"
    ];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_setup_treats_only_nil_as_the_simple_source_selector() {
    let elisp_form = r##"(mapcar
              (lambda (fuzzy)
                (let (ac-sources)
                  (set-up-sly-ac
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
        r#"OK ((nil (ac-source-sly-simple)) (t (ac-source-sly-fuzzy)) (0 (ac-source-sly-fuzzy)) ("" (ac-source-sly-fuzzy)) (fuzzy (ac-source-sly-fuzzy)))"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_setup_changes_only_the_current_buffers_local_source_list() {
    let elisp_form = r##"(let ((default-before
                    (default-value
                     'ac-sources))
                   first-state
                   second-state)
               (with-temp-buffer
                 (setq-local
                  ac-sources
                  '(first))
                 (set-up-sly-ac)
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
                 (set-up-sly-ac
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
        expect!["OK (((ac-source-sly-simple first) t) ((ac-source-sly-fuzzy second) t) t)"];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_setup_interactive_invocation_selects_simple_without_prompting() {
    let elisp_form = r##"(let ((ac-sources
                    '(existing)))
               (list
                (call-interactively
                 #'set-up-sly-ac)
                ac-sources
                (interactive-form
                 'set-up-sly-ac)))"##;
    let expect = expect!["OK (#1=(ac-source-sly-simple existing) #1# (interactive nil))"];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_source_callbacks_honor_runtime_rebinding_and_match_policies() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-sly-init)
                     (lambda ()
                       (push
                        'init
                        calls)
                       'initialized))
                    ((symbol-function
                      'ac-source-sly-fuzzy-candidates)
                     (lambda ()
                       (push
                        'fuzzy-candidates
                        calls)
                       '(fuzzy)))
                    ((symbol-function
                      'ac-source-sly-simple-candidates)
                     (lambda ()
                       (push
                        'simple-candidates
                        calls)
                       '(simple)))
                    ((symbol-function
                      'sly-symbol-start-pos)
                     (lambda ()
                       (push
                        'prefix
                        calls)
                       7))
                    ((symbol-function
                      'ac-sly-documentation)
                     (lambda (candidate)
                       (push
                        (list
                         'document
                         candidate)
                        calls)
                       'documented))
                    ((symbol-function
                      'ac-source-sly-case-correcting-completions)
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
                     ac-source-sly-fuzzy)))
                  (funcall
                   (cdr
                    (assq
                     'candidates
                     ac-source-sly-fuzzy)))
                  (funcall
                   (cdr
                    (assq
                     'prefix
                     ac-source-sly-fuzzy)))
                  (funcall
                   (cdr
                    (assq
                     'document
                     ac-source-sly-fuzzy))
                   'candidate-a)
                  (funcall
                   (cdr
                    (assq
                     'match
                     ac-source-sly-fuzzy))
                   "prefix"
                   '(one two))
                  (funcall
                   (cdr
                    (assq
                     'candidates
                     ac-source-sly-simple)))
                  (funcall
                   (cdr
                    (assq
                     'match
                     ac-source-sly-simple))
                   "name"
                   '(three four))
                  (nreverse
                   calls))))"##;
    let expect = expect![[
        r#"OK (initialized (fuzzy) 7 documented (one two) (simple) matched (init fuzzy-candidates prefix (document candidate-a) simple-candidates (match "name" (three four))))"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}
