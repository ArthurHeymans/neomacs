use expect_test::expect;

use super::assert_anki_connect_parity;

#[test]
fn deck_names_existence_and_creation_preserve_list_tail_and_exact_request_params() {
    let elisp_form = r##"(let (requests)
                      (cl-letf
                          (((symbol-function
                             'anki-connect-request)
                            (lambda (action params)
                              (push
                               (list action params)
                               requests)
                              (cond
                               ((equal
                                 action
                                 "deckNames")
                                ["Default"
                                 "Study"
                                 "Study::Japanese"])
                               ((equal
                                 action
                                 "createDeck")
                                4815162342)))))
                        (list
                         (anki-connect-deck-names)
                         (anki-connect-deck-exists-p
                          "Study")
                         (anki-connect-deck-exists-p
                          "Missing")
                         (anki-connect-create-deck
                          "Study::Grammar")
                         (nreverse requests))))"##;
    let expect = expect![[
        r#"OK (("Default" "Study" "Study::Japanese") ("Study" "Study::Japanese") nil 4815162342 (("deckNames" nil) ("deckNames" nil) ("deckNames" nil) ("createDeck" (("deck" . "Study::Grammar")))))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn ensure_deck_builds_only_missing_hierarchy_levels_in_parent_first_order() {
    let elisp_form = r##"(let ((decks
                           '("Root"))
                          requests
                          creations)
                      (cl-letf
                          (((symbol-function 's-split)
                            (lambda
                                (separator string)
                              (split-string
                               string
                               (regexp-quote separator)
                               t)))
                           ((symbol-function 's-blank?)
                            (lambda (string)
                              (string= string "")))
                           ((symbol-function
                             'anki-connect-request)
                            (lambda (action params)
                              (push
                               (list action params)
                               requests)
                              (cond
                               ((equal
                                 action
                                 "deckNames")
                                (vconcat decks))
                               ((equal
                                 action
                                 "createDeck")
                                (let ((deck
                                       (cdr
                                        (assoc
                                         "deck"
                                         params))))
                                  (setq decks
                                        (append
                                         decks
                                         (list deck)))
                                  (push deck creations)
                                  (length decks)))))))
                        (list
                         (anki-connect-ensure-deck
                          "Root::Language::Verbs")
                         decks
                         (nreverse creations)
                         (nreverse requests))))"##;
    let expect = expect![[
        r#"OK (nil ("Root" "Root::Language" "Root::Language::Verbs") ("Root::Language" "Root::Language::Verbs") (("deckNames" nil) ("deckNames" nil) ("createDeck" (("deck" . "Root::Language"))) ("deckNames" nil) ("createDeck" (("deck" . "Root::Language::Verbs")))))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn ensure_deck_is_idempotent_when_every_hierarchy_level_already_exists() {
    let elisp_form = r##"(let ((decks
                           '("Root"
                             "Root::Language"
                             "Root::Language::Verbs"))
                          requests)
                      (cl-letf
                          (((symbol-function 's-split)
                            (lambda
                                (separator string)
                              (split-string
                               string
                               (regexp-quote separator)
                               t)))
                           ((symbol-function 's-blank?)
                            (lambda (string)
                              (string= string "")))
                           ((symbol-function
                             'anki-connect-request)
                            (lambda (action params)
                              (push
                               (list action params)
                               requests)
                              (if
                                  (equal
                                   action
                                   "deckNames")
                                  (vconcat decks)
                                (error
                                 "unexpected creation")))))
                        (list
                         (anki-connect-ensure-deck
                          "Root::Language::Verbs")
                         (nreverse requests)
                         decks)))"##;
    let expect = expect![[
        r#"OK (nil (("deckNames" nil) ("deckNames" nil) ("deckNames" nil)) ("Root" "Root::Language" "Root::Language::Verbs"))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn ensure_deck_propagates_creation_failure_and_stops_before_deeper_children() {
    let elisp_form = r##"(let ((decks
                           '("Root"))
                          requests)
                      (cl-letf
                          (((symbol-function 's-split)
                            (lambda
                                (separator string)
                              (split-string
                               string
                               (regexp-quote separator)
                               t)))
                           ((symbol-function 's-blank?)
                            (lambda (string)
                              (string= string "")))
                           ((symbol-function
                             'anki-connect-request)
                            (lambda (action params)
                              (push
                               (list action params)
                               requests)
                              (if
                                  (equal
                                   action
                                   "deckNames")
                                  (vconcat decks)
                                (error
                                 "Anki rejected %s"
                                 (cdr
                                  (assoc
                                   "deck"
                                   params)))))))
                        (list
                         (condition-case error-data
                             (anki-connect-ensure-deck
                              "Root::Language::Verbs")
                           (error error-data))
                         (nreverse requests))))"##;
    let expect = expect![[
        r#"OK ((error "Anki rejected Root::Language") (("deckNames" nil) ("deckNames" nil) ("createDeck" (("deck" . "Root::Language")))))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}
