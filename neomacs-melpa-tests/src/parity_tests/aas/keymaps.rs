use expect_test::expect;

use super::{assert_aas_parity, assert_aas_signal_parity};

#[test]
fn aas_prefix_map_definition_creates_callable_extended_bindings_and_nil_removes_them() {
    let elisp_form = r##"(let ((map (make-sparse-keymap)))
               (aas-define-prefix-map-snippet
                map "ab" "expanded")
               (let ((binding (lookup-key map "ab")))
                 (list
                  (keymapp (lookup-key map "a"))
                  (functionp binding)
                  (with-temp-buffer
                    (insert "ab")
                    (setq-local
                     aas-global-condition-hook
                     (list #'aas--key-is-fully-typed?))
                    (list
                     (funcall binding)
                     (buffer-string)))
                  (progn
                    (aas-define-prefix-map-snippet
                     map "ab" nil)
                    (lookup-key map "ab")))))"##;
    let expect = expect![[r#"OK (t t (t "expanded") nil)"#]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_prefix_map_definition_accepts_functions_yas_and_tempel_forms() {
    let elisp_form = r##"(let ((map (make-sparse-keymap)))
               (aas-define-prefix-map-snippet
                map "f" #'forward-char)
               (aas-define-prefix-map-snippet
                map "y" '(yas "body"))
               (aas-define-prefix-map-snippet
                map "t" '(tempel "body"))
               (list
                (mapcar
                 (lambda (key)
                   (functionp (lookup-key map key)))
                 '("f" "y" "t"))
                (mapcar
                 (lambda (key)
                   (key-description
                    (where-is-internal
                     (lookup-key map key)
                     map t)))
                 '("f" "y" "t"))))"##;
    let expect = expect![[r#"OK ((t t t) ("f" "y" "t"))"#]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_prefix_map_definition_rejects_an_invalid_expansion_with_exact_error() {
    let elisp_form = r##"(aas-define-prefix-map-snippet
              (make-sparse-keymap)
              "x"
              '(unsupported "body"))"##;
    let expect = expect![[
        r#"ERR (error "Expansion must be either a string, function, tempel/yas form, or nil")"#
    ]];

    assert_aas_signal_parity(elisp_form, expect);
}

#[test]
fn aas_prefix_map_definition_rejects_an_invalid_condition_with_exact_error() {
    let elisp_form = r##"(aas-define-prefix-map-snippet
              (make-sparse-keymap)
              "x"
              "body"
              'not-a-function)"##;
    let expect = expect![[r#"ERR (error "Condition must be either nil or a function")"#]];

    assert_aas_signal_parity(elisp_form, expect);
}

#[test]
fn aas_set_snippets_reuses_named_map_applies_condition_scopes_and_ignores_descriptions() {
    let elisp_form = r##"(progn
               (aas-set-snippets
                   'neomacs-aas-map
                 :cond #'bolp
                 "aa" "at-bol"
                 "bb" "also-at-bol"
                 :cond nil
                 :expansion-desc "Human description"
                 "cc" "anywhere")
               (let ((first
                      (gethash
                       'neomacs-aas-map
                       aas-keymaps)))
                 (aas-set-snippets
                     'neomacs-aas-map
                   "dd" "later")
                 (let ((second
                        (gethash
                         'neomacs-aas-map
                         aas-keymaps)))
                   (list
                    (eq first second)
                    (mapcar
                     (lambda (key)
                       (functionp
                        (lookup-key second key)))
                     '("aa" "bb" "cc" "dd"))
                    (with-temp-buffer
                      (insert "xaa")
                      (setq-local
                       aas-global-condition-hook
                       (list #'aas--key-is-fully-typed?))
                      (list
                       (funcall
                        (lookup-key second "aa"))
                       (buffer-string)))
                    (with-temp-buffer
                      (insert "xcc")
                      (setq-local
                       aas-global-condition-hook
                       (list #'aas--key-is-fully-typed?))
                      (list
                       (funcall
                        (lookup-key second "cc"))
                       (buffer-string)))))))"##;
    let expect = expect![[r#"OK (t (t t t t) (nil "xaa") (t "xanywhere"))"#]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_set_snippets_rejects_unknown_keywords_with_exact_error() {
    let elisp_form = r##"(aas-set-snippets
                'neomacs-aas-map
              :unknown "value"
              "x" "body")"##;
    let expect = expect![[r#"ERR (error "Unknown keyword: :unknown")"#]];

    assert_aas_signal_parity(elisp_form, expect);
}

#[test]
fn aas_post_self_insert_tracks_prefixes_resets_after_expansion_and_drops_dead_ends() {
    let elisp_form = r##"(progn
               (aas-set-snippets
                   'neomacs-aas-typing
                 "abc" "EXPANDED")
               (with-temp-buffer
                 (aas-activate-keymap
                  'neomacs-aas-typing)
                 (setq-local
                  aas-global-condition-hook
                  (list #'aas--key-is-fully-typed?))
                 (let ((typed '("a" "x" "a" "b" "c"))
                       states)
                   (dolist (key typed)
                     (insert key)
                     (cl-letf
                         (((symbol-function
                            'this-command-keys)
                           (lambda () key)))
                       (aas-post-self-insert-hook))
                     (push
                      (list
                       (buffer-string)
                       (length
                        aas--current-prefix-maps))
                      states))
                   (list
                    (nreverse states)
                    (buffer-string)
                    aas--current-prefix-maps))))"##;
    let expect = expect![[
        r#"OK ((("a" 2) ("ax" 1) ("axa" 2) ("axab" 2) ("axEXPANDED" 1)) "axEXPANDED" (nil))"#
    ]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_post_self_insert_falls_back_to_a_shorter_overlapping_snippet_after_condition_failure() {
    let elisp_form = r##"(progn
               (aas-set-snippets
                   'neomacs-aas-overlap
                 :cond (lambda () nil)
                 "ab" "LONG"
                 :cond nil
                 "b" "SHORT")
               (with-temp-buffer
                 (aas-activate-keymap
                  'neomacs-aas-overlap)
                 (setq-local
                  aas-global-condition-hook
                  (list #'aas--key-is-fully-typed?))
                 (dolist (key '("a" "b"))
                   (insert key)
                   (cl-letf
                       (((symbol-function
                          'this-command-keys)
                         (lambda () key)))
                     (aas-post-self-insert-hook)))
                 (list
                  (buffer-string)
                  aas--current-prefix-maps)))"##;
    let expect = expect![[r#"OK ("aSHORT" (nil))"#]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_activation_precedence_duplicates_missing_maps_and_deactivation_are_exact() {
    let elisp_form = r##"(progn
               (aas-set-snippets
                   'neomacs-aas-first
                 "x" "FIRST")
               (aas-set-snippets
                   'neomacs-aas-second
                 "x" "SECOND")
               (with-temp-buffer
                 (let ((missing
                        (aas-activate-keymap
                         'neomacs-aas-missing)))
                   (aas-activate-keymap
                    'neomacs-aas-first)
                   (aas-activate-keymap
                    'neomacs-aas-second)
                   (aas-activate-keymap
                    'neomacs-aas-second)
                   (let ((before
                          (list
                           missing
                           (copy-sequence
                            aas-active-keymaps)
                           (keymapp aas--prefix-map)
                           (eq
                            (lookup-key
                             aas--prefix-map "x")
                            (lookup-key
                             (gethash
                              'neomacs-aas-second
                              aas-keymaps)
                             "x")))))
                     (let ((result
                            (aas-deactivate-keymap
                             'neomacs-aas-second)))
                       (list
                        before
                        (keymapp result)
                        (copy-sequence
                         aas-active-keymaps)
                        (eq
                         (lookup-key
                          aas--prefix-map "x")
                         (lookup-key
                          (gethash
                           'neomacs-aas-first
                           aas-keymaps)
                          "x"))))))))"##;
    let expect =
        expect!["OK ((nil (neomacs-aas-second neomacs-aas-first) t t) t (neomacs-aas-first) t)"];

    assert_aas_parity(elisp_form, expect);
}
