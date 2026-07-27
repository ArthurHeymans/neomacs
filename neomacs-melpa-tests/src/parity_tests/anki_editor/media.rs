use expect_test::expect;

use super::assert_anki_editor_parity;

#[test]
fn raw_org_html_and_latex_export_cover_builtin_mathjax_div_and_brace_modes() {
    let elisp_form = r##"(with-temp-buffer
                  (org-mode)
                  (insert "* Export context\n")
                  (goto-char (point-max))
                  (list
                      (list
                       (let ((anki-editor-latex-display-math-div
                              "display-math"))
                         (list
                          (anki-editor--latex-div-beg)
                          (anki-editor--latex-div-end)))
                       (let ((anki-editor-latex-display-math-div
                              nil))
                         (list
                          (anki-editor--latex-div-beg)
                          (anki-editor--latex-div-end))))
                      (mapcar
                       #'anki-editor--export-string
                       '("# raw content"
                         "# raw  content"
                         "# raw\ncontent"
                         "# raw"
                         ""
                         "A *bold* and /italic/ answer."))
                      (let ((anki-editor-latex-style
                             'builtin)
                            (anki-editor-latex-display-math-div
                             "display-math"))
                        (mapcar
                         #'anki-editor--translate-latex-fragment
                         '("$x+y$"
                           "$$x^2$$"
                           "\\(a+b\\)"
                           "\\[c+d\\]")))
                      (let ((anki-editor-latex-style
                             'mathjax))
                        (list
                         (anki-editor--translate-latex-fragment
                          "$x+y$")
                         (anki-editor--translate-latex-fragment
                          "$$x^2$$")
                         (anki-editor--translate-latex-env
                          "\\begin{align}\na&=b\n\\end{align}")))
                      (let ((anki-editor-latex-style
                             'builtin)
                            (anki-editor-latex-display-math-div
                             "math-block"))
                        (anki-editor--translate-latex-env
                         "\\begin{equation}\na < b\n\\end{equation}"))
                      (let ((anki-editor-break-consecutive-braces-in-latex
                             t))
                        (anki-editor--ox-latex
                         '(latex-fragment
                           (:value "$x^{2}}$"))
                         nil nil))))"##;
    let expect = expect![[
        r#"OK ((("<div class=\"display-math\">" "</div>") ("" "")) ("content" "content" "content" "" "" "<p>\nA <b>bold</b> and <i>italic</i> answer.</p>\n") ("[$]x+y[/$]" "<p><div class=\"display-math\">[$$]x^2[/$$]</div></p>" "[$]a+b[/$]" "<p><div class=\"display-math\">[$$]c+d[/$$]</div></p>") ("\\(x+y\\)" "<p>\\[x^2\\]</p>" "\\[<br>\\begin{align}<br>a&amp;=b<br>\\end{align}\\]") "<div class=\"math-block\">[latex]<br>\\begin{equation}<br>a &lt; b<br>\\end{equation}[/latex]</div>" "[$]x^{2} } [/$]")"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn store_media_hashes_real_bytes_retrieves_before_uploading_and_reuses_existing_name() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (directory
                            (expand-file-name
                             "anki-media"
                             sandbox))
                           (path
                            (expand-file-name
                             "pixel.gif"
                             directory))
                           (retrieve-results
                            '(:json-false
                              "R0lGODlh"))
                           calls messages)
                      (make-directory directory t)
                      (with-temp-file path
                        (set-buffer-multibyte nil)
                        (insert "GIF89a"))
                      (cl-letf
                          (((symbol-function
                             'anki-editor-api-call-result)
                            (lambda (action &rest params)
                              (push
                               (cons action params)
                               calls)
                              (pcase action
                                ('retrieveMediaFile
                                 (pop retrieve-results))
                                ('storeMediaFile
                                 (plist-get
                                  params
                                  :filename)))))
                           ((symbol-function 'message)
                            (lambda
                                (format-string
                                 &rest arguments)
                              (push
                               (apply
                                #'format
                                format-string
                                arguments)
                               messages))))
                        (list
                         (secure-hash
                          'sha1 "GIF89a")
                         (anki-editor-api--store-media-file
                          path)
                         (anki-editor-api--store-media-file
                          path)
                         (nreverse calls)
                         (nreverse messages)
                         retrieve-results)))"##;
    let expect = expect![[
        r#"OK ("25c9b37ae36a0a08318d4dca7ca57ea98d776821" "pixel-25c9b37ae36a0a08318d4dca7ca57ea98d776821.gif" "pixel-25c9b37ae36a0a08318d4dca7ca57ea98d776821.gif" ((retrieveMediaFile :filename "pixel-25c9b37ae36a0a08318d4dca7ca57ea98d776821.gif") (storeMediaFile :filename "pixel-25c9b37ae36a0a08318d4dca7ca57ea98d776821.gif" :data "R0lGODlh") (retrieveMediaFile :filename "pixel-25c9b37ae36a0a08318d4dca7ca57ea98d776821.gif")) ("Storing media file [ORACLE-SANDBOX]/anki-media/pixel.gif to Anki, this might take a while") nil)"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn org_export_stores_real_image_audio_and_document_links_with_correct_html() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (directory
                            (file-name-as-directory
                             (expand-file-name
                              "export-media"
                              sandbox)))
                           stored)
                      (make-directory directory t)
                      (dolist
                          (file
                           '("picture.gif"
                             "sound.mp3"
                             "manual.pdf"))
                        (with-temp-file
                            (expand-file-name
                             file directory)
                          (insert
                           (concat
                            "contents:"
                            file))))
                      (cl-letf
                          (((symbol-function
                             'anki-editor-api--store-media-file)
                            (lambda (path)
                              (push path stored)
                              (concat
                               "anki-"
                               (file-name-nondirectory
                                path)))))
                        (with-temp-buffer
                          (org-mode)
                          (insert "* Export context\n")
                          (goto-char (point-max))
                          (let ((default-directory
                                 directory))
                            (list
                             (replace-regexp-in-string
                              "id=\"org[[:xdigit:]]+\""
                              "id=\"org-ID\""
                              (anki-editor--export-string
                               "[[file:picture.gif]]"))
                             (anki-editor--export-string
                              "[[file:sound.mp3]]")
                             (anki-editor--export-string
                              "[[file:manual.pdf][Read manual]]")
                             (nreverse stored))))))"##;
    let expect = expect![[
        r#"OK ("\n<div id=\"org-ID\" class=\"figure\">\n<p><img src=\"anki-picture.gif\" alt=\"anki-picture.gif\" /></p>\n</div>\n" "<p>\n[sound:anki-sound.mp3]</p>\n" "<p>\n<a href=\"anki-manual.pdf\">Read manual</a></p>\n" ("[ORACLE-SANDBOX]/export-media/picture.gif" "[ORACLE-SANDBOX]/export-media/sound.mp3" "[ORACLE-SANDBOX]/export-media/manual.pdf"))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn attachment_links_expand_from_real_org_dir_before_note_field_mapping() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (project
                            (file-name-as-directory
                             (expand-file-name
                              "attachment-note"
                              sandbox)))
                           (attachment-directory
                            (expand-file-name
                             "media"
                             project))
                           (attachment
                            (expand-file-name
                             "diagram.gif"
                             attachment-directory)))
                      (make-directory
                       attachment-directory t)
                      (with-temp-file attachment
                        (insert "GIF89a"))
                      (with-temp-buffer
                        (setq buffer-file-name
                              (expand-file-name
                               "cards.org"
                               project))
                        (org-mode)
                        (insert
                         "* Diagram card\n:PROPERTIES:\n:ANKI_NOTE_TYPE: Basic\n:ANKI_DECK: Study\n:DIR: media\n:END:\n")
                        (insert
                         "[[attachment:diagram.gif][Diagram]]\n")
                        (goto-char
                         (point-min))
                        (let ((org-attach-dir-relative
                               t)
                              (anki-editor--collection-data-updated
                               t)
                              (anki-editor--model-fields
                               '(("Basic"
                                  "Front"
                                  "Back"))))
                          (let* ((note
                                  (anki-editor-note-at-point))
                                 (fields
                                  (anki-editor-note-fields
                                   note)))
                            (list
                             fields
                             (string-match-p
                              (regexp-quote
                               (concat
                                "file:"
                                attachment))
                              (cdr
                               (assoc
                                "Back"
                                fields))))))))"##;
    let expect = expect![[
        r#"OK ((("Back" . "[[attachment:diagram.gif][Diagram]]\n") ("Front" . "Diagram card")) nil)"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn copying_and_removing_styles_issue_exact_model_requests_with_fixed_timestamp() {
    let elisp_form = r##"(let (calls messages
                          (styles
                           '(("Basic"
                              . "before</style>\n<!-- {{ Emacs Org-mode -->old<!-- Emacs Org-mode }} -->\n<style>after")
                             ("Cloze"
                              . "plain-css"))))
                      (cl-letf
                          (((symbol-function
                             'anki-editor-note-types)
                            (lambda ()
                              '("Basic"
                                "Cloze")))
                           ((symbol-function
                             'current-time-string)
                            (lambda ()
                              "Mon Jan  2 03:04:05 2006"))
                           ((symbol-function
                             'anki-editor-api-call-result)
                            (lambda (action &rest params)
                              (push
                               (cons action params)
                               calls)
                              (pcase action
                                ('modelStyling
                                 (list
                                  (cons
                                   'css
                                   (cdr
                                    (assoc
                                     (plist-get
                                      params
                                      :modelName)
                                     styles)))))
                                ('updateModelStyling
                                 nil))))
                           ((symbol-function 'message)
                            (lambda
                                (format-string
                                 &rest arguments)
                              (push
                               (apply
                                #'format
                                format-string
                                arguments)
                               messages))))
                        (let ((anki-editor-include-default-style
                               nil)
                              (anki-editor-html-head
                               "<style>.custom { color: red; }</style>"))
                          (anki-editor-copy-styles))
                        (let ((after-copy
                               (list
                                (nreverse calls)
                                (nreverse messages))))
                          (setq calls nil
                                messages nil)
                          (anki-editor-remove-styles)
                          (list
                           after-copy
                           (nreverse calls)
                           (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK ((((modelStyling :modelName "Basic") (updateModelStyling :model (:name "Basic" :css "</style>\n<!-- {{ Emacs Org-mode -->\n<!-- Updated: Mon Jan  2 03:04:05 2006 -->\n<style>.custom { color: red; }</style>\n<!-- Emacs Org-mode }} -->\n<style>\n\nbeforeafter")) (modelStyling :modelName "Cloze") (updateModelStyling :model (:name "Cloze" :css "</style>\n<!-- {{ Emacs Org-mode -->\n<!-- Updated: Mon Jan  2 03:04:05 2006 -->\n<style>.custom { color: red; }</style>\n<!-- Emacs Org-mode }} -->\n<style>\n\nplain-css"))) ("Updating styles for \"Basic\"..." "Updating styles for \"Cloze\"..." "Updating styles...Done")) ((modelStyling :modelName "Basic") (updateModelStyling :model (:name "Basic" :css "beforeafter")) (modelStyling :modelName "Cloze")) ("Resetting styles for \"Basic\"..." "Resetting styles...Done"))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}
