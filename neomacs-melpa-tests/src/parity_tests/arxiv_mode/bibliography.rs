use expect_test::expect;

use super::assert_arxiv_mode_parity;

#[test]
fn bibtex_string_formats_real_authors_title_abstract_year_and_arxiv_fields() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((title . "Practical Editor Parity")
                   (id . "2401.01234")
                   (author . ("Ada Lovelace" "Grace Brewster Hopper"))
                   (abstract . "  A deterministic abstract.  ")
                   (date . "2024-01-02 03:04:05 ")
                   (url . "https://arxiv.org/abs/2401.01234")
                   (journal)
                   (doi))))
               (arxiv-current-entry 0)
               (bibtex-autokey-names 1)
               (bibtex-autokey-year-length 4)
               (bibtex-autokey-titlewords 0))
         (arxiv-export-bibtex-to-string))"##;
    let expect = expect![[
        r#"OK "@article{lovelace2024,\ntitle = {Practical Editor Parity},\nauthor = {Lovelace, Ada and Hopper, Grace Brewster},\nabstract = {A deterministic abstract.},\narchivePrefix = {arXiv},\neprint = {2401.01234},\nurl = {https://arxiv.org/abs/2401.01234},\nyear = {2024}\n}""#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn bibtex_string_includes_doi_journal_and_expanded_local_pdf_link() {
    let elisp_form = r##"(let* ((pdf
                 (expand-file-name "papers/2402.00001.pdf"
                                   temporary-file-directory))
                (arxiv-entry-list
                 '(((title . "Linked Research")
                    (id . "2402.00001")
                    (author . ("Lin Test"))
                    (abstract . "Summary")
                    (date . "2023-12-31 23:59:59 ")
                    (url . "https://arxiv.org/abs/2402.00001")
                    (journal . "Journal of Links")
                    (doi . "10.1000/linked"))))
                (arxiv-current-entry 0)
                (result
                 (arxiv-export-bibtex-to-string pdf)))
         (list
          (replace-regexp-in-string
           (regexp-quote temporary-file-directory)
           "<TMP>/"
           result)
          (string-match-p "doi = {10.1000/linked}" result)
          (string-match-p "journal = {Journal of Links}" result)
          (string-match-p "file = {:" result)))"##;
    let expect = expect![[
        r#"OK ("@article{test23:_linked_resear,\ntitle = {Linked Research},\nauthor = {Test, Lin},\nabstract = {Summary},\narchivePrefix = {arXiv},\neprint = {2402.00001},\nurl = {https://arxiv.org/abs/2402.00001},\nyear = {2023},\ndoi = {10.1000/linked},\njournal = {Journal of Links},\nfile = {:<TMP>/papers/2402.00001.pdf:pdf}\n}" 208 232 262)"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn bibtex_name_conversion_handles_particles_middle_names_and_multiple_authors() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((title . "Names in Practice")
                   (id . "2403.1")
                   (author . ("Ludwig van Beethoven"
                              "Mary Jane Watson"
                              "Cher"))
                   (abstract . "Names")
                   (date . "2022-03-04 ")
                   (url . "https://arxiv.org/abs/2403.1"))))
               (arxiv-current-entry 0))
         (condition-case error
             (list :ok
                   (arxiv-export-bibtex-to-string))
           (error (list :error error))))"##;
    let expect = expect![[r#"OK (:error (args-out-of-range "Cher" 10 16))"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn export_bibtex_appends_one_entry_to_existing_file_and_reports_destination() {
    let elisp_form = r##"(let* ((file
                 (expand-file-name "arxiv-mode-library.bib"
                                   temporary-file-directory))
                (arxiv-default-bibliography file)
                (arxiv-entry-list
                 '(((title . "Appended Paper")
                    (id . "2404.2")
                    (author . ("Ada Lovelace"))
                    (abstract . "Append safely")
                    (date . "2024-04-02 ")
                    (url . "https://arxiv.org/abs/2404.2"))))
                (arxiv-current-entry 0)
                messages)
         (with-temp-file file
           (insert "@comment{existing}\n"))
         (cl-letf (((symbol-function 'read-file-name)
                    (lambda (&rest _args) file))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args)
                            messages))))
           (arxiv-export-bibtex)
           (unwind-protect
               (list
                (with-temp-buffer
                  (insert-file-contents file)
                  (replace-regexp-in-string
                   (regexp-quote temporary-file-directory)
                   "<TMP>/"
                   (buffer-string)))
                (mapcar
                 (lambda (message)
                   (replace-regexp-in-string
                    (regexp-quote temporary-file-directory)
                    "<TMP>/"
                    message))
                 (nreverse messages)))
             (delete-file file))))"##;
    let expect = expect![[
        r#"OK ("@comment{existing}\n@article{lovelace24:_appen_paper,\ntitle = {Appended Paper},\nauthor = {Lovelace, Ada},\nabstract = {Append safely},\narchivePrefix = {arXiv},\neprint = {2404.2},\nurl = {https://arxiv.org/abs/2404.2},\nyear = {2024}\n}\n" ("Written bibTeX entry to <TMP>/arxiv-mode-library.bib."))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn download_pdf_derives_default_filename_and_copies_exact_url_without_prompt_when_confirmed() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((pdf . "https://arxiv.org/pdf/2405.01234"))))
               (arxiv-current-entry 0)
               (arxiv-default-download-folder
                temporary-file-directory)
               calls)
         (cl-letf (((symbol-function 'read-file-name)
                    (lambda (prompt directory default
                             mustmatch initial &rest rest)
                      (push (list :read prompt
                                  (file-name-nondirectory
                                   (directory-file-name
                                    directory))
                                  default mustmatch initial rest)
                            calls)
                      directory))
                   ((symbol-function 'url-copy-file)
                    (lambda (url file overwrite)
                      (push (list :copy url
                                  (file-name-nondirectory file)
                                  overwrite)
                            calls))))
           (let ((result (arxiv-download-pdf t)))
             (list (file-name-nondirectory result)
                   (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ("2405.01234.pdf" ((:read "save pdf as: " "tmp" nil nil "2405.01234.pdf" nil) (:copy "https://arxiv.org/pdf/2405.01234" "2405.01234.pdf" 1)))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn interactive_download_opens_saved_pdf_only_for_yes_answers() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((pdf . "https://arxiv.org/pdf/yes-paper"))))
               (arxiv-current-entry 0)
               (arxiv-default-download-folder
                temporary-file-directory)
               (arxiv-pdf-open-function 'fixture-open)
               calls)
         (cl-letf (((symbol-function 'read-file-name)
                    (lambda (&rest _args)
                      (expand-file-name "chosen.pdf"
                                        temporary-file-directory)))
                   ((symbol-function 'url-copy-file)
                    (lambda (url file overwrite)
                      (push (list :copy url
                                  (file-name-nondirectory file)
                                  overwrite)
                            calls)))
                   ((symbol-function 'read-char-exclusive)
                    (lambda (&rest _args) ?Y))
                   ((symbol-function 'fixture-open)
                    (lambda (file)
                      (push (list :open
                                  (file-name-nondirectory file))
                            calls))))
           (list (file-name-nondirectory
                  (arxiv-download-pdf nil))
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("chosen.pdf" ((:copy "https://arxiv.org/pdf/yes-paper" "chosen.pdf" 1) (:open "chosen.pdf")))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn export_bibtex_to_buffer_replaces_previous_content_and_enters_bibtex_mode() {
    let elisp_form = r##"(let ((arxiv-abstract-window (selected-window))
               calls)
         (cl-letf (((symbol-function 'arxiv-export-bibtex-to-string)
                    (lambda (&optional pdfpath)
                      (push (list :generate pdfpath) calls)
                      "@article{fixture,\ntitle={Parity}\n}"))
                   ((symbol-function 'select-window)
                    (lambda (window &rest _)
                      (push (list :select (windowp window))
                            calls)
                      window))
                   ((symbol-function 'pop-to-buffer)
                    (lambda (buffer &optional action norecord)
                      (push (list :pop buffer action norecord)
                            calls)
                      (set-buffer (get-buffer-create buffer))
                      (current-buffer))))
           (with-current-buffer
               (get-buffer-create "*arXiv-bibTeX*")
             (erase-buffer)
             (insert "stale"))
           (arxiv-export-bibtex-to-buffer "paper.pdf")
           (unwind-protect
               (list (buffer-name)
                     major-mode
                     buffer-read-only
                     (buffer-string)
                     (nreverse calls))
             (kill-buffer "*arXiv-bibTeX*"))))"##;
    let expect = expect![[
        r#"OK ("*arXiv-bibTeX*" bibtex-mode nil "@article{fixture,\ntitle={Parity}\n}" ((:generate "paper.pdf") (:select t) (:pop "*arXiv-bibTeX*" (display-buffer-below-selected) nil)))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn download_and_export_composition_passes_downloaded_path_to_bibliography() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arxiv-download-pdf)
                    (lambda (&optional confirm)
                      (push (list :download confirm) calls)
                      "/workspace/papers/result.pdf"))
                   ((symbol-function 'arxiv-export-bibtex)
                    (lambda (&optional pdfpath)
                      (push (list :export pdfpath) calls)
                      'written)))
           (list (arxiv-download-pdf-export-bibtex)
                 (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (written ((:download t) (:export "/workspace/papers/result.pdf")))"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}
