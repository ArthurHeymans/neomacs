use expect_test::expect;

use super::assert_anki_editor_view_parity;

#[test]
fn anki_editor_view_missing_note_messages_and_returns_protocol_nil() {
    let elisp_form = r##"(let (messages search-call)
         (cl-letf
             (((symbol-function
                'anki-editor-view--ripgrep-find-locations)
               (lambda (&rest arguments)
                 (setq search-call arguments)
                 nil))
              ((symbol-function 'message)
               (lambda (&rest arguments)
                 (push arguments messages)
                 (apply #'format arguments))))
           (list
            (anki-editor-view--open-anki-note
             '(:id 12345 :ignored "value"))
            search-call
            (nreverse messages))))"##;
    let expect =
        expect![[r#"OK (nil (":ANKI_NOTE_ID: 12345" ("~/org")) (("Anki note not found.")))"#]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_single_location_opens_real_org_heading_and_unfolds_context() {
    let elisp_form = r##"(let* ((file
                          (expand-file-name
                           "anki-editor-view-single.org"
                           temporary-file-directory))
               opened-buffer recenter-calls
               result)
         (unwind-protect
             (progn
               (with-temp-file file
                 (insert
                  "#+title: Cards\n"
                  "* Deck\n"
                  "** Survey the ruins\n"
                  ":PROPERTIES:\n"
                  ":ANKI_NOTE_ID: 4242\n"
                  ":END:\n"
                  "Visible card body.\n"
                  "*** Child detail\n"
                  "Nested body.\n"))
               (cl-letf
                   (((symbol-function
                      'anki-editor-view--ripgrep-find-locations)
                     (lambda (_search _directories)
                       (list
                        `((file . ,file)
                          (line . 5)))))
                    ((symbol-function
                      'recenter-top-bottom)
                     (lambda (&rest arguments)
                       (push arguments
                             recenter-calls))))
                 (setq result
                       (anki-editor-view--open-anki-note
                        '(:id 4242))
                       opened-buffer
                       (current-buffer))
                 (list
                  result
                  (file-name-nondirectory
                   buffer-file-name)
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position))
                  (org-at-heading-p)
                  (org-entry-get nil
                                 "ANKI_NOTE_ID")
                  (invisible-p
                   (save-excursion
                     (forward-line 1)
                     (point)))
                  (nreverse recenter-calls))))
           (when
               (buffer-live-p opened-buffer)
             (kill-buffer opened-buffer))
           (when
               (file-exists-p file)
             (delete-file file))))"##;
    let expect = expect![[
        r#"OK (nil "anki-editor-view-single.org" "** Survey the ruins" t "4242" nil (nil))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_multiple_locations_warns_and_opens_first_match() {
    let elisp_form = r##"(let* ((first
                          (expand-file-name
                           "anki-editor-view-first.org"
                           temporary-file-directory))
               (second
                (expand-file-name
                 "anki-editor-view-second.org"
                 temporary-file-directory))
               messages opened-buffer)
         (unwind-protect
             (progn
               (with-temp-file first
                 (insert
                  "* First card\n"
                  ":PROPERTIES:\n"
                  ":ANKI_NOTE_ID: 7\n"
                  ":END:\n"))
               (with-temp-file second
                 (insert
                  "* Second card\n"
                  ":PROPERTIES:\n"
                  ":ANKI_NOTE_ID: 7\n"
                  ":END:\n"))
               (cl-letf
                   (((symbol-function
                      'anki-editor-view--ripgrep-find-locations)
                     (lambda (_search _directories)
                       (list
                        `((file . ,first)
                          (line . 3))
                        `((file . ,second)
                          (line . 3)))))
                    ((symbol-function
                      'recenter-top-bottom)
                     (lambda (&rest _arguments)))
                    ((symbol-function 'message)
                     (lambda (&rest arguments)
                       (when
                           (string-prefix-p
                            "Warning:"
                            (car arguments))
                         (push arguments messages))
                       (apply #'format arguments))))
                 (anki-editor-view--open-anki-note
                  '(:id 7))
                 (setq opened-buffer
                       (current-buffer))
                 (list
                  (file-name-nondirectory
                   buffer-file-name)
                  (org-get-heading t t t t)
                  (nreverse messages))))
           (when
               (buffer-live-p opened-buffer)
             (kill-buffer opened-buffer))
           (mapc
            (lambda (file)
              (when
                  (file-exists-p file)
                (delete-file file)))
            (list first second))))"##;
    let expect = expect![[
        r#"OK ("anki-editor-view-first.org" #("First card" 0 10 (fontified nil)) (("Warning: Found more than one (%s) location of the Anki Note" 2)))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_protocol_entry_dispatches_real_handler_with_plist() {
    let elisp_form = r##"(let* ((entry
                          (seq-find
                           (lambda (candidate)
                             (equal
                              (plist-get
                               (cdr candidate)
                               :protocol)
                              "anki-editor-view"))
                           org-protocol-protocol-alist))
               (handler
                (plist-get
                 (cdr entry)
                 :function))
               call)
         (cl-letf
             (((symbol-function
                'anki-editor-view--ripgrep-find-locations)
               (lambda (&rest arguments)
                 (setq call arguments)
                 nil))
              ((symbol-function 'message)
               (lambda (&rest arguments)
                 (apply #'format arguments))))
           (list
            (funcall handler
                     '(:id "9001"
                       :url "ignored"))
            handler call)))"##;
    let expect =
        expect![[r#"OK (nil anki-editor-view--open-anki-note (":ANKI_NOTE_ID: 9001" ("~/org")))"#]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_open_note_builds_search_from_numeric_string_and_nil_ids() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'anki-editor-view--ripgrep-find-locations)
               (lambda (&rest arguments)
                 (push arguments calls)
                 nil))
              ((symbol-function 'message)
               (lambda (&rest arguments)
                 (apply #'format arguments))))
           (list
            (anki-editor-view--open-anki-note
             '(:id 101))
            (anki-editor-view--open-anki-note
             '(:id "0101"))
            (anki-editor-view--open-anki-note
             '(:url "missing-id"))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (nil nil nil ((":ANKI_NOTE_ID: 101" #1=("~/org")) (":ANKI_NOTE_ID: 0101" #1#) (":ANKI_NOTE_ID: nil" #1#)))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_invalid_line_before_first_heading_signals_org_error() {
    let elisp_form = r##"(let* ((file
                          (expand-file-name
                           "anki-editor-view-invalid.org"
                           temporary-file-directory))
               opened-buffer outcome)
         (unwind-protect
             (progn
               (with-temp-file file
                 (insert
                  "Preamble without heading.\n"
                  "* Later card\n"
                  ":ANKI_NOTE_ID: 11\n"))
               (cl-letf
                   (((symbol-function
                      'anki-editor-view--ripgrep-find-locations)
                     (lambda (_search _directories)
                       (list
                        `((file . ,file)
                          (line . 1))))))
                 (setq outcome
                       (condition-case error
                           (list
                            'value
                            (anki-editor-view--open-anki-note
                             '(:id 11)))
                         (error
                          (list
                           'error
                           (car error)
                           (cdr error))))
                       opened-buffer
                       (current-buffer))
                 outcome))
           (when
               (buffer-live-p opened-buffer)
             (kill-buffer opened-buffer))
           (when
               (file-exists-p file)
             (delete-file file))))"##;
    let expect = expect![[
        r#"OK (error user-error ("Before first headline at position 1 in buffer anki-editor-view-invalid.org"))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}
