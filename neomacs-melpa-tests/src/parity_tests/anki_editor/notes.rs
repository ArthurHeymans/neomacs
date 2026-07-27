use expect_test::expect;

use super::assert_anki_editor_parity;

#[test]
fn note_at_point_maps_real_org_properties_subheadings_inheritance_and_filtered_tags() {
    let elisp_form = r##"(with-temp-buffer
                      (org-mode)
                      (insert
                       "* Deck defaults\n:PROPERTIES:\n:ANKI_DECK: Study::Languages\n:END:\n")
                      (insert
                       "** Japanese particle :urgent:noexport:\n")
                      (insert
                       ":PROPERTIES:\n:ANKI_NOTE_TYPE: Basic\n:ANKI_TAGS: grammar local\n:ANKI_NOTE_ID: 1700000000001\n:ANKI_NOTE_HASH: old-hash\n:END:\n")
                      (insert
                       "*** Front\nWhat does は mark?\n")
                      (insert
                       "*** Back\nThe topic of a sentence.\n")
                      (goto-char
                       (point-min))
                      (re-search-forward
                       "^\\*\\* Japanese")
                      (beginning-of-line)
                      (let ((anki-editor--collection-data-updated
                             t)
                            (anki-editor--model-fields
                             '(("Basic"
                                "Front"
                                "Back")))
                            (anki-editor-ignored-org-tags
                             '("noexport"))
                            (anki-editor-org-tags-as-anki-tags
                             t))
                        (let ((note
                               (anki-editor-note-at-point)))
                          (list
                           (anki-editor-note-id note)
                           (anki-editor-note-model note)
                           (anki-editor-note-deck note)
                           (anki-editor-note-fields note)
                           (anki-editor-note-tags note)
                           (anki-editor-note-hash note)
                           (marker-position
                            (anki-editor-note-marker
                             note))))))"##;
    let expect = expect![[
        r#"OK ("1700000000001" "Basic" "Study::Languages" (("Back" . "The topic of a sentence.\n") ("Front" . "What does は mark?\n")) ("grammar" "local" "urgent") "old-hash" 65)"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn property_field_overrides_subheading_and_field_alias_maps_practical_note_shapes() {
    let elisp_form = r##"(let ((anki-editor--collection-data-updated
                           t)
                          (anki-editor--model-fields
                           '(("Basic"
                              "Front"
                              "Back"))))
                      (list
                       (with-temp-buffer
                         (org-mode)
                         (insert
                          "* Property card\n:PROPERTIES:\n:ANKI_NOTE_TYPE: Basic\n:ANKI_DECK: Study\n:ANKI_FIELD_FRONT: Property question\n:END:\n** Front\nIgnored subheading\n** Back\nVisible answer\n")
                         (goto-char
                          (point-min))
                         (let ((note
                                (anki-editor-note-at-point)))
                           (anki-editor-note-fields
                            note)))
                       (with-temp-buffer
                         (org-mode)
                         (insert
                          "* Alias card\n:PROPERTIES:\n:ANKI_NOTE_TYPE: Basic\n:ANKI_DECK: Study\n:END:\n** Question\nAliased question\n** Back\nAliased answer\n")
                         (goto-char
                          (point-min))
                         (let ((anki-editor-field-alias
                                '(("Basic"
                                   ("Question"
                                    . "Front")))))
                           (anki-editor-note-fields
                            (anki-editor-note-at-point))))
                       (with-temp-buffer
                         (org-mode)
                         (insert
                          "* Extra field\n:PROPERTIES:\n:ANKI_NOTE_TYPE: Basic\n:ANKI_DECK: Study\n:END:\n** Front\nQ\n** Back\nA\n** Foreign\nX\n")
                         (goto-char
                          (point-min))
                         (condition-case error-data
                             (anki-editor-note-at-point)
                           (error error-data)))))"##;
    let expect = expect![[
        r#"OK ((("Back" . "Visible answer\n") ("Front" . "Property question")) (("Back" . "Aliased answer\n") ("Front" . "Aliased question\n")) (user-error "Failed to map all named fields for note: Extra field. Extra fields: Foreign"))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn field_mapping_covers_complete_one_missing_prepend_two_missing_swap_and_overflow() {
    let elisp_form = r##"(with-temp-buffer
                      (org-mode)
                      (insert "* Context\n")
                      (goto-char
                       (point-min))
                      (let ((anki-editor--collection-data-updated
                             t)
                            (anki-editor--model-fields
                             '(("Basic"
                                "Front"
                                "Back")
                               ("Three"
                                "One"
                                "Two"
                                "Three"))))
                        (list
                         (anki-editor--map-fields
                          "Heading"
                          ""
                          '(("Front" . "Q")
                            ("Back" . "A"))
                          "Basic" 1 nil 0)
                         (anki-editor--map-fields
                          "Heading only"
                          ""
                          '(("Back" . "A"))
                          "Basic" 1 nil 0)
                         (let ((anki-editor-prepend-heading-format
                                "<h>%s</h>\n"))
                           (anki-editor--map-fields
                            "Heading"
                            "Body\n"
                            '(("Extra"
                               . "Details\n"))
                            "Basic" 1 t 0))
                         (anki-editor--map-fields
                          "Heading"
                          "Body"
                          nil
                          "Basic" 1 nil 0)
                         (anki-editor--map-fields
                          "Heading"
                          "Body"
                          nil
                          "Basic" 1 nil 1)
                         (condition-case error-data
                             (anki-editor--map-fields
                              "Heading"
                              ""
                              nil
                              "Three" 1 nil 0)
                           (error error-data)))))"##;
    let expect = expect![[
        r#"OK ((("Front" . "Q") ("Back" . "A")) (("Front" . "Heading only") ("Back" . "A")) (("Back" . "Body\n** Extra\n\nDetails\n\n\n") ("Front" . "Heading")) (("Back" . "Body") ("Front" . "Heading")) (("Front" . "Body") ("Back" . "Heading")) (user-error "Cannot map note fields: more than two fields missing"))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn multivalued_anki_tags_org_tags_filtering_and_validity_rules_match_real_org_entry() {
    let elisp_form = r##"(with-temp-buffer
                      (org-mode)
                      (insert
                       "* Parent :parent_tag:\n:PROPERTIES:\n:ANKI_TAGS: inherited one\\ two\n:END:\n")
                      (insert
                       "** Child :child_tag:noexport:\n:PROPERTIES:\n:ANKI_TAGS+: local local\\ tag\n:END:\n")
                      (goto-char
                       (point-min))
                      (re-search-forward
                       "^\\*\\* Child")
                      (beginning-of-line)
                      (let ((anki-editor-org-tags-as-anki-tags
                             t)
                            (anki-editor-ignored-org-tags
                             '("noexport")))
                        (list
                         (anki-editor--entry-get-multivalued-property-with-inheritance
                          nil
                          anki-editor-prop-tags)
                         (anki-editor--get-tags)
                         (let ((anki-editor-org-tags-as-anki-tags
                                nil))
                           (anki-editor--get-tags))
                         (mapcar
                          #'anki-editor-is-valid-org-tag
                          '("simple"
                            "under_score"
                            "@home"
                            "hash#tag"
                            "space tag"
                            "colon:tag"
                            "")))))"##;
    let expect = expect![[
        r#"OK (("inherited" "one\\" "two" "local" "local\\" "tag") ("inherited" "one\\" "two" "local" "local\\" "tag" "parent_tag" "child_tag" "noexport") ("inherited" "one\\" "two" "local" "local\\" "tag") (0 0 0 0 nil nil nil))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn tag_completion_hooks_merge_real_org_properties_cache_remote_tags_and_warn_on_invalid_values() {
    let elisp_form = r##"(with-temp-buffer
                      (org-mode)
                      (insert
                       "* Card\n:PROPERTIES:\n:ANKI_TAGS: alpha beta\n:ANKI_TAGS+: gamma\n:END:\n")
                      (goto-char (point-min))
                      (let (warnings
                            anki-editor--anki-tags-cache)
                        (cl-letf
                            (((symbol-function
                               'anki-editor--enable-tag-completion)
                              (lambda () t))
                             ((symbol-function
                               'anki-editor-all-tags)
                              (lambda ()
                                '("safe"
                                  "bad tag")))
                             ((symbol-function 'warn)
                              (lambda
                                  (format-string
                                   &rest arguments)
                                (push
                                 (apply
                                  #'format
                                  format-string
                                  arguments)
                                 warnings))))
                          (list
                           (anki-editor--concat-multivalued-property-value
                            anki-editor-prop-tags
                            "new tag")
                           (anki-editor--concat-multivalued-property-value
                            anki-editor-prop-tags-plus
                            "next tag")
                           (progn
                             (anki-editor--before-set-tags
                              nil nil)
                             anki-editor--anki-tags-cache)
                           (nreverse warnings)
                           (anki-editor--get-buffer-tags
                            (lambda ()
                              '(("local")
                                ("safe"))))
                           (progn
                             (setq
                              anki-editor--anki-tags-cache
                              nil
                              warnings nil)
                             (anki-editor--before-set-tags
                              nil t)
                             (list
                              anki-editor--anki-tags-cache
                              warnings))))))"##;
    let expect = expect![[
        r#"OK ("alpha beta new%20tag" "gamma next%20tag" ("safe" "bad tag") ("Some tags from Anki contain characters that are notvalid in Org tags.") (("local") ("safe") ("safe") ("bad tag")) (nil nil))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn format_and_prepend_heading_toggles_respect_inheritance_and_remove_redundant_properties() {
    let elisp_form = r##"(with-temp-buffer
                      (org-mode)
                      (insert
                       "* Parent\n:PROPERTIES:\n:ANKI_FORMAT: nil\n:ANKI_PREPEND_HEADING: t\n:END:\n** Child\nBody\n")
                      (goto-char
                       (point-min))
                      (re-search-forward
                       "^\\*\\* Child")
                      (beginning-of-line)
                      (let ((anki-editor-prepend-heading
                             nil))
                        (let ((before
                               (list
                                (anki-editor-entry-format)
                                (anki-editor-prepend-heading)
                                (org-entry-get
                                 nil
                                 anki-editor-prop-format)
                                (org-entry-get
                                 nil
                                 anki-editor-prop-prepend-heading))))
                          (anki-editor-toggle-format)
                          (anki-editor-toggle-prepend-heading)
                          (let ((after-first
                                 (list
                                  (anki-editor-entry-format)
                                  (anki-editor-prepend-heading)
                                  (org-entry-get
                                   nil
                                   anki-editor-prop-format)
                                  (org-entry-get
                                   nil
                                   anki-editor-prop-prepend-heading))))
                            (anki-editor-toggle-format)
                            (anki-editor-toggle-prepend-heading)
                            (list
                             before
                             after-first
                             (list
                              (anki-editor-entry-format)
                              (anki-editor-prepend-heading)
                              (org-entry-get
                               nil
                               anki-editor-prop-format)
                              (org-entry-get
                               nil
                               anki-editor-prop-prepend-heading)))))))"##;
    let expect = expect![[r#"OK ((nil t nil nil) (t nil "t" nil) (nil t nil nil))"#]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn cloze_transformation_handles_emphasis_hint_default_number_and_word_at_point() {
    let elisp_form = r##"(list
                      (with-temp-buffer
                        (org-mode)
                        (insert "/important/")
                        (anki-editor-cloze
                         (point-min)
                         (point-max)
                         2
                         "remember")
                        (buffer-string))
                      (with-temp-buffer
                        (org-mode)
                        (insert "plain text")
                        (anki-editor-cloze
                         (point-min)
                         (point-max)
                         nil
                         "")
                        (buffer-string))
                      (with-temp-buffer
                        (org-mode)
                        (insert "alpha beta gamma")
                        (goto-char 9)
                        (anki-editor-cloze-dwim
                         3
                         "middle")
                        (buffer-string))
                      (with-temp-buffer
                        (org-mode)
                        (condition-case error-data
                            (anki-editor-cloze-dwim
                             1 "")
                          (error error-data))))"##;
    let expect = expect![[
        r#"OK ("{{c2:: /important/::remember}}" "{{c1::plain text}}" "alpha {{c3::beta::middle}} gamma" (user-error "Nothing to create cloze from"))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn insert_note_skeleton_builds_real_org_subtree_properties_fields_and_point() {
    let elisp_form = r##"(list
                      (with-temp-buffer
                        (org-mode)
                        (insert
                         "* Existing\n:PROPERTIES:\n:ANKI_DECK: Study\n:END:\nBody\n")
                        (goto-char
                         (point-max))
                        (let ((anki-editor-insert-note-always-use-content
                               nil))
                          (anki-editor--insert-note-skeleton
                           nil
                           "Study"
                           "New card"
                           "Basic"
                           '("Front"
                             "Back")))
                        (list
                         (buffer-string)
                         (org-entry-get
                          nil
                          anki-editor-prop-note-type)
                         (org-entry-get
                          nil
                          anki-editor-prop-deck)
                         (org-get-heading
                          t t t t)
                         (point)))
                      (with-temp-buffer
                        (org-mode)
                        (insert "* Existing\n")
                        (goto-char
                         (point-max))
                        (let ((anki-editor-insert-note-always-use-content
                               t))
                          (anki-editor--insert-note-skeleton
                           nil
                           "Other"
                           ""
                           "Basic"
                           '("Front"
                             "Back"
                             "Extra")))
                        (list
                         (buffer-string)
                         (org-entry-get
                          nil
                          anki-editor-prop-note-type)
                         (org-entry-get
                          nil
                          anki-editor-prop-deck)
                         (point))))"##;
    let expect = expect![[
        r#"OK (("* Existing\n:PROPERTIES:\n:ANKI_DECK: Study\n:END:\nBody\n* New card\n:PROPERTIES:\n:ANKI_NOTE_TYPE: Basic\n:ANKI_DECK: Study\n:END:\n** Front\n** Back\n" nil nil "Front" 133) ("* Existing\n* \n:PROPERTIES:\n:ANKI_NOTE_TYPE: Basic\n:ANKI_DECK: Other\n:END:\n** Extra\n" "Basic" "Other" 14))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn process_note_selects_create_update_skip_and_force_update_with_real_org_properties() {
    let elisp_form = r##"(with-temp-buffer
                      (org-mode)
                      (insert
                       "* Card\n:PROPERTIES:\n:ANKI_FAILURE_REASON: stale\n:END:\n")
                      (goto-char
                       (point-min))
                      (let (events)
                        (cl-letf
                            (((symbol-function
                               'anki-editor--enqueue-create-note)
                              (lambda (note)
                                (push
                                 (list
                                  'create
                                  (anki-editor-note-id
                                   note))
                                 events)))
                             ((symbol-function
                               'anki-editor--enqueue-update-note)
                              (lambda (note)
                                (push
                                 (list
                                  'update
                                  (anki-editor-note-hash
                                   note))
                                 events))))
                          (let* ((new-note
                                  (make-anki-editor-note
                                   :model "Basic"
                                   :deck "Study"
                                   :fields
                                   '(("Front" . "Q")
                                     ("Back" . "A"))
                                   :tags nil
                                   :marker
                                   (point-marker)))
                                 (existing
                                  (make-anki-editor-note
                                   :id "42"
                                   :model "Basic"
                                   :deck "Study"
                                   :fields
                                   '(("Front" . "Q")
                                     ("Back" . "A"))
                                   :tags nil
                                   :marker
                                   (point-marker)))
                                 (matching-hash
                                  (anki-editor--calc-note-hash
                                   existing)))
                            (setf
                             (anki-editor-note-hash
                              existing)
                             matching-hash)
                            (let ((create
                                   (anki-editor--process-note
                                    new-note))
                                  (skip
                                   (anki-editor--process-note
                                    existing)))
                              (setf
                               (anki-editor-note-fields
                                existing)
                               '(("Front" . "Changed")
                                 ("Back" . "A")))
                              (let ((update
                                     (anki-editor--process-note
                                      existing))
                                    forced)
                                (setf
                                 (anki-editor-note-hash
                                  existing)
                                 (anki-editor--calc-note-hash
                                  existing))
                                (let ((anki-editor-force-update
                                       t))
                                  (setq forced
                                        (anki-editor--process-note
                                         existing)))
                                (funcall
                                 (anki-editor--make-set-note-failure-reason
                                  existing)
                                 "network unavailable")
                                (list
                                 create skip update forced
                                 (nreverse events)
                                 (org-entry-get
                                  nil
                                  anki-editor-prop-failure-reason))))))))"##;
    let expect = expect![[
        r#"OK (:create :skip :update :update ((create nil) (update "405d8d0be34ebabcd3ff91bacf1cf0dc") (update "405d8d0be34ebabcd3ff91bacf1cf0dc")) "network unavailable")"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}
