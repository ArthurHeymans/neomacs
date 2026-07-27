use expect_test::expect;

use super::assert_anki_editor_parity;

#[test]
fn package_defaults_property_protocol_constants_and_media_extensions_match() {
    let elisp_form = r##"(list
                      (featurep 'anki-editor)
                      anki-editor-api-version
                      (list
                       anki-editor-export-note-fields-on-push
                       anki-editor-break-consecutive-braces-in-latex
                       anki-editor-allow-duplicates
                       anki-editor-org-tags-as-anki-tags
                       anki-editor-protected-tags
                       anki-editor-ignored-org-tags
                       anki-editor-api-host
                       anki-editor-api-port
                       anki-editor-latex-style
                       anki-editor-include-default-style
                       anki-editor-html-head
                       anki-editor-note-match
                       anki-editor-prepend-heading
                       anki-editor-prepend-heading-format
                       anki-editor-insert-note-always-use-content
                       anki-editor-default-note-type
                       anki-editor-gui-browse-ensure-foreground
                       anki-editor-latex-display-math-div
                       anki-editor-swap-two-fields
                       anki-editor-field-alias
                       anki-editor-force-update)
                      (list
                       anki-editor-prop-note-type
                       anki-editor-prop-note-id
                       anki-editor-prop-note-hash
                       anki-editor-prop-deck
                       anki-editor-prop-format
                       anki-editor-prop-prepend-heading
                       anki-editor-prop-field-prefix
                       anki-editor-prop-tags
                       anki-editor-prop-tags-plus
                       anki-editor-prop-failure-reason
                       anki-editor-prop-default-note-type
                       anki-editor-prop-swap-two-fields
                       anki-editor-prop-no-subheading-fields
                       anki-editor-org-tag-regexp)
                      (list
                       anki-editor--api-active-queue
                       anki-editor--api-request-queue-1
                       anki-editor--api-request-queue-2
                       anki-editor--collection-data-updated
                       anki-editor--model-names
                       anki-editor--model-fields
                       anki-editor--anki-tags-cache
                       anki-editor--note-markers
                       anki-editor--ox-anki-html-backend
                       anki-editor--ox-export-ext-plist
                       anki-editor--style-start
                       anki-editor--style-end)
                      anki-editor--audio-extensions
                      anki-editor--native-latex-delimiters
                      anki-editor--mathjax-delimiters)"##;
    let expect = expect![[
        r#"OK (t 6 (t nil nil t ("marked" "leech") ("export" "noexport") "127.0.0.1" "8765" builtin t nil nil nil "/%s/\n\n" nil "Basic" t nil nil nil nil) ("ANKI_NOTE_TYPE" "ANKI_NOTE_ID" "ANKI_NOTE_HASH" "ANKI_DECK" "ANKI_FORMAT" "ANKI_PREPEND_HEADING" "ANKI_FIELD_" "ANKI_TAGS" "ANKI_TAGS+" "ANKI_FAILURE_REASON" "ANKI_DEFAULT_NOTE_TYPE" "ANKI_SWAP_TWO_FIELDS" "ANKI_NO_SUBHEADING_FIELDS" "^\\([[:alnum:]_@#%]+\\)+$") (1 nil nil nil nil nil nil nil #s(org-export-backend anki-html html ((latex-fragment . anki-editor--ox-latex) (latex-environment . anki-editor--ox-latex) (link . anki-editor--ox-html-link-transcoder)) nil nil nil nil) (:with-toc nil :with-properties nil :with-planning nil :anki-editor-mode t) "</style>\n<!-- {{ Emacs Org-mode -->" "<!-- Emacs Org-mode }} -->\n<style>") (".mp3" ".3gp" ".flac" ".m4a" ".oga" ".ogg" ".opus" ".spx" ".wav") (("^\\$\\$" "[$$]" "\\$\\$$" "[/$$]") ("^\\$" "[$]" "\\$$" "[/$]") ("^\\\\(" "[$]" "\\\\)$" "[/$]") ("^\\\\\\[" "[$$]" "\\\\]$" "[/$$]")) (("^\\$\\$" "\\[" "\\$\\$$" "\\]") ("^\\$" "\\(" "\\$$" "\\)")))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn complete_core_callable_surface_has_exact_arglists_and_command_status() {
    let elisp_form = r##"(mapcar
                      (lambda (function)
                        (list
                         function
                         (fboundp function)
                         (copy-tree
                          (help-function-arglist
                           function t))
                         (commandp function)))
                      '(anki-editor--fetch
                        anki-editor-api-call
                        anki-editor-api-call-result
                        anki-editor-api--make-queued-request
                        anki-editor-api--get-active-queue
                        anki-editor-api--push-active-queue
                        anki-editor-api--toggle-active-queue
                        anki-editor-api-enqueue-request
                        anki-editor-api-dispatch-queue
                        anki-editor-api--note
                        anki-editor-api--store-media-file
                        anki-editor--latex-div-beg
                        anki-editor--latex-div-end
                        anki-editor--translate-latex-fragment
                        anki-editor--translate-latex-env
                        anki-editor--ox-latex
                        anki-editor--ox-html-link-transcoder
                        anki-editor--ox-html-link
                        anki-editor--export-string
                        anki-editor--export-fields
                        make-anki-editor-note
                        copy-anki-editor-note
                        anki-editor-note-p
                        anki-editor-note-id
                        anki-editor-note-model
                        anki-editor-note-deck
                        anki-editor-note-fields
                        anki-editor-note-tags
                        anki-editor-note-hash
                        anki-editor-note-marker
                        anki-editor--with-collection-data-updated
                        anki-editor-map-note-entries
                        anki-editor--insert-note-skeleton
                        anki-editor--process-note
                        anki-editor--make-set-note-failure-reason
                        anki-editor--enqueue-create-note
                        anki-editor--enqueue-update-note
                        anki-editor--calc-note-hash
                        anki-editor--set-note-id
                        anki-editor--set-note-hash
                        anki-editor--set-failure-reason
                        anki-editor--clear-failure-reason
                        anki-editor--get-allowed-values-for-property
                        anki-editor-is-valid-org-tag
                        anki-editor-all-tags
                        anki-editor-deck-names
                        anki-editor-note-types
                        anki-editor-entry-format
                        anki-editor-toggle-format
                        anki-editor-prepend-heading
                        anki-editor-toggle-prepend-heading
                        anki-editor-note-at-point
                        anki-editor--expand-attachment-links
                        anki-editor--get-tags
                        anki-editor--entry-get-multivalued-property-with-inheritance
                        anki-editor--skip-drawer
                        anki-editor--build-fields
                        anki-editor--property-fields
                        anki-editor--note-contents-before-subheading
                        anki-editor--map-fields
                        anki-editor--concat-fields
                        anki-editor-mode
                        anki-editor-setup-minor-mode
                        anki-editor-teardown-minor-mode
                        anki-editor--enable-tag-completion
                        anki-editor--before-set-tags
                        anki-editor--get-buffer-tags
                        anki-editor--concat-multivalued-property-value
                        anki-editor--collect-note-marker
                        anki-editor--draw-progress-bar
                        anki-editor-push-notes
                        anki-editor--goto-nearest-note-type
                        anki-editor-push-note-at-point
                        anki-editor-push-new-notes
                        anki-editor-retry-failed-notes
                        anki-editor-force-push-notes
                        anki-editor-delete-note-at-point
                        anki-editor-insert-note
                        anki-editor-insert-default-note
                        anki-editor-set-note-type
                        anki-editor-set-deck
                        anki-editor-set-default-note-type
                        anki-editor-cloze-region
                        anki-editor-cloze-dwim
                        anki-editor-cloze
                        anki-editor-export-subtree-to-html
                        anki-editor-convert-region-to-html
                        anki-editor-api-check
                        anki-editor-sync-collection
                        anki-editor-gui-browse
                        anki-editor-gui-add-cards
                        anki-editor-find-notes
                        anki-editor-copy-styles
                        anki-editor-remove-styles))"##;
    let expect = expect![
        "OK ((anki-editor--fetch t (url &rest settings) nil) (anki-editor-api-call t (action &rest params) nil) (anki-editor-api-call-result t (&rest args) nil) (anki-editor-api--make-queued-request t (request success error) nil) (anki-editor-api--get-active-queue t nil nil) (anki-editor-api--push-active-queue t (request) nil) (anki-editor-api--toggle-active-queue t nil nil) (anki-editor-api-enqueue-request t (action params &rest callbacks) nil) (anki-editor-api-dispatch-queue t nil nil) (anki-editor-api--note t (note) nil) (anki-editor-api--store-media-file t (path) nil) (anki-editor--latex-div-beg t nil nil) (anki-editor--latex-div-end t nil nil) (anki-editor--translate-latex-fragment t (latex-code) nil) (anki-editor--translate-latex-env t (latex-code) nil) (anki-editor--ox-latex t (latex _contents _info) nil) (anki-editor--ox-html-link-transcoder t (link desc info) nil) (anki-editor--ox-html-link t (oldfun link desc info) nil) (anki-editor--export-string t (src) nil) (anki-editor--export-fields t (fields) nil) (make-anki-editor-note t (&rest --cl-rest--) nil) (copy-anki-editor-note t (arg) nil) (anki-editor-note-p t (x) nil) (anki-editor-note-id t (x) nil) (anki-editor-note-model t (x) nil) (anki-editor-note-deck t (x) nil) (anki-editor-note-fields t (x) nil) (anki-editor-note-tags t (x) nil) (anki-editor-note-hash t (x) nil) (anki-editor-note-marker t (x) nil) (anki-editor--with-collection-data-updated t (&rest body) nil) (anki-editor-map-note-entries t (func &optional match scope &rest skip) nil) (anki-editor--insert-note-skeleton t (prefix deck heading type fields) nil) (anki-editor--process-note t (note) nil) (anki-editor--make-set-note-failure-reason t (note) nil) (anki-editor--enqueue-create-note t (note) nil) (anki-editor--enqueue-update-note t (note) nil) (anki-editor--calc-note-hash t (note) nil) (anki-editor--set-note-id t (id) nil) (anki-editor--set-note-hash t (hash) nil) (anki-editor--set-failure-reason t (reason) nil) (anki-editor--clear-failure-reason t nil nil) (anki-editor--get-allowed-values-for-property t (property) nil) (anki-editor-is-valid-org-tag t (tag) nil) (anki-editor-all-tags t nil nil) (anki-editor-deck-names t nil nil) (anki-editor-note-types t nil nil) (anki-editor-entry-format t nil nil) (anki-editor-toggle-format t nil t) (anki-editor-prepend-heading t nil nil) (anki-editor-toggle-prepend-heading t nil t) (anki-editor-note-at-point t nil nil) (anki-editor--expand-attachment-links t (fields) nil) (anki-editor--get-tags t nil nil) (anki-editor--entry-get-multivalued-property-with-inheritance t (pom property) nil) (anki-editor--skip-drawer t (element) nil) (anki-editor--build-fields t nil nil) (anki-editor--property-fields t (fields) nil) (anki-editor--note-contents-before-subheading t nil nil) (anki-editor--map-fields t (heading content-before-subheading subheading-fields note-type level prepend-heading field-swap) nil) (anki-editor--concat-fields t (field-names field-alist level) nil) (anki-editor-mode t (&optional arg) t) (anki-editor-setup-minor-mode t nil nil) (anki-editor-teardown-minor-mode t nil nil) (anki-editor--enable-tag-completion t nil nil) (anki-editor--before-set-tags t (&optional _ just-align) nil) (anki-editor--get-buffer-tags t (oldfun) nil) (anki-editor--concat-multivalued-property-value t (prop value) nil) (anki-editor--collect-note-marker t nil nil) (anki-editor--draw-progress-bar t (title count total &rest --cl-rest--) nil) (anki-editor-push-notes t (&optional scope match &rest skip) t) (anki-editor--goto-nearest-note-type t nil nil) (anki-editor-push-note-at-point t nil t) (anki-editor-push-new-notes t (&optional scope) t) (anki-editor-retry-failed-notes t (&optional scope) t) (anki-editor-force-push-notes t (&optional scope) t) (anki-editor-delete-note-at-point t (&optional prefix) t) (anki-editor-insert-note t (&optional prefix note-type) t) (anki-editor-insert-default-note t (&optional prefix) t) (anki-editor-set-note-type t (&optional prefix note-type) t) (anki-editor-set-deck t (&optional prefix note-deck) t) (anki-editor-set-default-note-type t (&optional prefix) t) (anki-editor-cloze-region t (&optional arg hint) t) (anki-editor-cloze-dwim t (&optional arg hint) t) (anki-editor-cloze t (begin end arg hint) nil) (anki-editor-export-subtree-to-html t nil t) (anki-editor-convert-region-to-html t nil t) (anki-editor-api-check t nil t) (anki-editor-sync-collection t nil t) (anki-editor-gui-browse t (&optional query) t) (anki-editor-gui-add-cards t nil t) (anki-editor-find-notes t (&optional query) t) (anki-editor-copy-styles t nil t) (anki-editor-remove-styles t nil t))"
    ];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn note_struct_constructor_copy_mutation_and_hash_are_deterministic() {
    let elisp_form = r##"(let* ((note
                            (make-anki-editor-note
                             :id "42"
                             :model "Basic"
                             :deck "Study"
                             :fields
                             '(("Front" . "Question")
                               ("Back" . "Answer"))
                             :tags
                             '("review" "priority")
                             :hash "old"))
                           (copy
                            (copy-anki-editor-note
                             note))
                           (before
                            (anki-editor--calc-note-hash
                             note)))
                      (setf
                       (anki-editor-note-deck copy)
                       "Study::Changed")
                      (list
                       (anki-editor-note-p note)
                       (equal note copy)
                       (list
                        (anki-editor-note-id note)
                        (anki-editor-note-model note)
                        (anki-editor-note-deck note)
                        (anki-editor-note-fields note)
                        (anki-editor-note-tags note)
                        (anki-editor-note-hash note)
                        (anki-editor-note-marker note))
                       before
                       (anki-editor--calc-note-hash
                        note)
                       (anki-editor--calc-note-hash
                        copy)))"##;
    let expect = expect![[
        r#"OK (t nil ("42" "Basic" "Study" (("Front" . "Question") ("Back" . "Answer")) ("review" "priority") "old" nil) "73219c68fcf73ce329c1c091226f0665" "73219c68fcf73ce329c1c091226f0665" "8bb8fb447b3eda57128d7a0643f250a9")"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}
