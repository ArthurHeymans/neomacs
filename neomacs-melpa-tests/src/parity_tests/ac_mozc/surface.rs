use expect_test::expect;

use super::assert_ac_mozc_parity;

#[test]
fn ac_mozc_exact_pin_dependencies_features_group_custom_and_internal_state_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-mozc
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-mozc
                   cl-lib
                   mozc
                   auto-complete))
                (get
                 'ac-mozc
                 'group-documentation)
                (assq
                 'ac-mozc
                 (get
                  'auto-complete
                  'custom-group))
                (get
                 'ac-mozc
                 'custom-prefix)
                (mapcar
                 (lambda (variable)
                   (list
                    variable
                    (symbol-value variable)
                    (get variable
                         'standard-value)
                    (get variable
                         'variable-documentation)
                    (get variable
                         'custom-type)
                    (get variable
                         'custom-group)))
                 '(ac-mozc-remove-space
                   ac-mozc-preedit
                   ac-mozc-candidates
                   ac-mozc-ac-point
                   ac-mozc-sending))))"##;
    let expect = expect![[
        r#"OK (ac-mozc "20150227.1619" ((cl-lib (0 5)) (auto-complete (1 4)) (mozc (0))) (t t t t) "Auto-complete sources for Japanese input using Mozc." (ac-mozc custom-group) "ac-mozc-" ((ac-mozc-remove-space t (t) "Non-nil if a space between two alphanumeric strings should be removed.\nWhen a translated Japanese word is selected and it follows an\nalphanumeric string and a space, the space in between is removed. To\nstop this behavior, set this variable to nil." boolean nil) (ac-mozc-preedit nil nil nil nil nil) (ac-mozc-candidates nil nil nil nil nil) (ac-mozc-ac-point nil nil nil nil nil) (ac-mozc-sending nil nil nil nil nil)))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_complete_function_surface_arities_interactivity_and_documentation_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (help-function-arglist
                   function t)
                  (interactive-form
                   function)
                  (documentation
                   function t)
                  (let ((definition
                         (symbol-function
                          function)))
                    (cond
                     ((symbolp definition)
                      definition)
                     ((byte-code-function-p
                       definition)
                      'byte-code)
                     (t 'interpreted)))))
               '(ac-mozc-prefix
                 ac-mozc-action
                 ac-mozc-match
                 ac-mozc-send-word
                 ac-mozc-handle-event
                 ac-mozc-all-candidate-words-to-candidates
                 ac-mozc-pick-preedit
                 ac-mozc-pick-candidates
                 ac-mozc-kana-p
                 ac-mozc-remove-non-ascii-character
                 ac-mozc-partial-match
                 ac-mozc-word-candidates-ascii-only))"##;
    let expect = expect![[
        r#"OK ((ac-mozc-prefix nil nil nil interpreted) (ac-mozc-action nil nil nil interpreted) (ac-mozc-match (ac-prefix candidates) nil nil interpreted) (ac-mozc-send-word (word) nil nil interpreted) (ac-mozc-handle-event (event) nil nil interpreted) (ac-mozc-all-candidate-words-to-candidates (all-candidate-words) nil nil interpreted) (ac-mozc-pick-preedit (preedit) nil nil interpreted) (ac-mozc-pick-candidates (candidates) nil nil interpreted) (ac-mozc-kana-p (str) nil nil interpreted) (ac-mozc-remove-non-ascii-character (words) nil nil interpreted) (ac-mozc-partial-match (string collection) nil nil interpreted) (ac-mozc-word-candidates-ascii-only (&optional buffer-pred) nil nil interpreted))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_sources_and_active_cleanup_advice_match_exactly() {
    let elisp_form = r##"(list
               ac-source-mozc
               ac-source-ascii-words-in-same-mode-buffers
               (not
                (null
                 (ad-is-advised
                  'ac-cleanup)))
               (not
                (null
                 (ad-find-advice
                  'ac-cleanup
                  'before
                  'ac-mozc-before-cleanup-advice))))"##;
    let expect = expect![[
        r#"OK (((match . ac-mozc-match) (prefix . ac-mozc-prefix) (symbol . "M") (action . ac-mozc-action)) ((prefix . ac-mozc-prefix) (init . ac-update-word-index) (candidates ac-mozc-word-candidates-ascii-only (lambda (buffer) (derived-mode-p (buffer-local-value 'major-mode buffer))))) t t)"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_cleanup_advice_captures_the_live_auto_complete_point_before_cleanup() {
    let elisp_form = r##"(let ((ac-point
                    42)
                   (ac-mozc-ac-point
                    'unset))
               (list
                (ac-cleanup)
                ac-point
                ac-mozc-ac-point))"##;
    let expect = expect![[r#"OK (nil nil 42)"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_ascii_source_init_indirection_invokes_the_live_word_index_updater() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-update-word-index)
                     (lambda ()
                       (push
                        'updated
                        calls)
                       'index-result)))
                 (list
                  (funcall
                   (cdr
                    (assq
                     'init
                     ac-source-ascii-words-in-same-mode-buffers)))
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (index-result (updated))"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}
