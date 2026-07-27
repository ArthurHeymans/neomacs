use expect_test::expect;

use super::assert_arxiv_citation_parity;

#[test]
fn citation_command_appends_the_same_generated_entry_to_multiple_existing_bibliographies() {
    let elisp_form = r##"(let* ((root
         (expand-file-name
          "arxiv-citation-write-contract"
          (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
       (first (expand-file-name "primary.bib" root))
       (second (expand-file-name "project.bib" root))
       (arxiv-citation-bibtex-files
        (list first second))
       calls)
  (when (file-directory-p root)
    (delete-directory root t))
  (make-directory root t)
  (with-temp-file first
    (insert "% Primary bibliography\n"))
  (with-temp-file second
    (insert
     "@Book{existing,\n"
     " title = {Existing Entry},\n"
     "}\n"))
  (cl-letf
      (((symbol-function
         'arxiv-citation-get-citation)
        (lambda (url)
          (push url calls)
          (concat
           "@Article{lovelace:2024:practical,\n"
           " title = {Practical Results},\n"
           "}\n"))))
    (unwind-protect
        (list
         (arxiv-citation
          "https://arxiv.org/abs/2402.12345")
         (nreverse calls)
         (mapcar
          (lambda (file)
            (with-temp-buffer
              (insert-file-contents file)
              (buffer-string)))
          (list first second)))
      (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (nil ("https://arxiv.org/abs/2402.12345") ("% Primary bibliography\n\n@Article{lovelace:2024:practical,\n title = {Practical Results},\n}\n\n" "@Book{existing,\n title = {Existing Entry},\n}\n\n@Article{lovelace:2024:practical,\n title = {Practical Results},\n}\n\n"))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn duplicate_bibliography_paths_append_duplicate_entries_in_declared_order() {
    let elisp_form = r##"(let* ((root
         (expand-file-name
          "arxiv-citation-duplicate-contract"
          (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
       (file (expand-file-name "all.bib" root))
       (arxiv-citation-bibtex-files
        (list file file)))
  (when (file-directory-p root)
    (delete-directory root t))
  (make-directory root t)
  (with-temp-file file
    (insert "% citations\n"))
  (cl-letf
      (((symbol-function
         'arxiv-citation-get-citation)
        (lambda (_url)
          "@Article{repeated,\n title = {Repeated},\n}")))
    (unwind-protect
        (progn
          (arxiv-citation
           "https://arxiv.org/abs/1111.22222")
          (with-temp-buffer
            (insert-file-contents file)
            (buffer-string)))
      (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK "% citations\n\n@Article{repeated,\n title = {Repeated},\n}\n\n@Article{repeated,\n title = {Repeated},\n}\n""#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn empty_bibliography_configuration_still_resolves_the_citation_once_without_writing_files() {
    let elisp_form = r##"(let ((arxiv-citation-bibtex-files nil)
       calls)
  (cl-letf
      (((symbol-function
         'arxiv-citation-get-citation)
        (lambda (url)
          (push url calls)
          "unused citation")))
    (list
     (arxiv-citation
      "https://arxiv.org/abs/2301.00001")
     (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil ("https://arxiv.org/abs/2301.00001"))"#]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn a_later_unwritable_bibliography_preserves_the_first_successful_append_before_signaling() {
    let elisp_form = r##"(let* ((root
         (expand-file-name
          "arxiv-citation-partial-write-contract"
          (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
       (first (expand-file-name "first.bib" root))
       (missing
        (expand-file-name
         "missing-parent/second.bib" root))
       (arxiv-citation-bibtex-files
        (list first missing)))
  (when (file-directory-p root)
    (delete-directory root t))
  (make-directory root t)
  (with-temp-file first
    (insert "% first\n"))
  (cl-letf
      (((symbol-function
         'arxiv-citation-get-citation)
        (lambda (_url)
          "@Article{partial,\n title = {Partial},\n}")))
    (unwind-protect
        (let ((outcome
               (condition-case error
                   (arxiv-citation
                    "https://arxiv.org/abs/2301.00002")
                 (error
                  (list
                   (car error)
                   (mapcar
                    (lambda (item)
                      (if (stringp item)
                          (replace-regexp-in-string
                           (regexp-quote root)
                           "<ROOT>"
                           item)
                        item))
                    (cdr error)))))))
          (list
           outcome
           (with-temp-buffer
             (insert-file-contents first)
             (buffer-string))
           (file-exists-p missing)))
      (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK ((file-missing ("Opening output file" "No such file or directory" "<ROOT>/missing-parent/second.bib")) "% first\n\n@Article{partial,\n title = {Partial},\n}\n" nil)"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn gui_prefers_http_primary_then_http_clipboard_and_ignores_non_url_selections() {
    let elisp_form = r##"(let (calls primary clipboard)
  (cl-letf
      (((symbol-function
         'gui-get-primary-selection)
        (lambda () primary))
       ((symbol-function 'gui-get-selection)
        (lambda (&rest arguments)
          (push (cons 'clipboard arguments) calls)
          clipboard))
       ((symbol-function 'arxiv-citation)
        (lambda (url)
          (push (list 'citation url) calls)
          (concat "INSERTED:" url))))
    (list
     (progn
       (setq primary
             "https://arxiv.org/abs/2401.00001"
             clipboard
             "https://arxiv.org/abs/2401.99999")
       (list
        (arxiv-citation-gui)
        (prog1 (nreverse calls)
          (setq calls nil))))
     (progn
       (setq primary "selected prose"
             clipboard "http://zbmath.org/?q=ai")
       (list
        (arxiv-citation-gui)
        (prog1 (nreverse calls)
          (setq calls nil))))
     (progn
       (setq primary nil
             clipboard "ftp://example.org/paper")
       (list
        (arxiv-citation-gui)
        (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (("INSERTED:https://arxiv.org/abs/2401.00001" ((clipboard CLIPBOARD) (citation "https://arxiv.org/abs/2401.00001"))) ("INSERTED:http://zbmath.org/?q=ai" ((clipboard CLIPBOARD) (citation "http://zbmath.org/?q=ai"))) (nil ((clipboard CLIPBOARD))))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}
