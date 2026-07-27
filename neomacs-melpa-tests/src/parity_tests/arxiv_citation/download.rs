use expect_test::expect;

use super::assert_arxiv_citation_parity;

#[test]
fn overwrite_enabled_downloads_exact_pdf_url_to_normalized_name_then_opens_expanded_path() {
    let elisp_form = r##"(let ((arxiv-citation-library
        "/research/preprints")
       (arxiv-citation-overwrite-file t)
       calls)
  (cl-letf
      (((symbol-function
         'arxiv-citation-get-details)
        (lambda (link)
          (push (list 'details link) calls)
          '(:authors
            ("Lovelace, Ada"
             "Turing, Alan")
            :title
            "Practical_AI: Systems and Proofs")))
       ((symbol-function 'url-copy-file)
        (lambda (&rest arguments)
          (push (cons 'copy arguments) calls)
          'copied))
       ((symbol-function
         'arxiv-citation-test-open)
        (lambda (file)
          (push (list 'open file) calls)
          'opened)))
    (let ((arxiv-citation-open-pdf-function
           #'arxiv-citation-test-open))
      (list
       (arxiv-citation-download-and-open
        "https://arxiv.org/abs/2402.12345")
       (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (opened ((details "https://arxiv.org/pdf/2402.12345.pdf") (copy "https://arxiv.org/pdf/2402.12345.pdf" "/research/preprints/lovelace-turing_practical-ai.pdf" t) (open "/research/preprints/lovelace-turing_practical-ai.pdf")))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn batch_mode_without_overwrite_never_copies_existing_or_missing_file_but_still_opens_it() {
    let elisp_form = r##"(let ((arxiv-citation-library
        "/research/preprints")
       (arxiv-citation-overwrite-file nil)
       exists
       calls)
  (cl-letf
      (((symbol-function
         'arxiv-citation-get-details)
        (lambda (_link)
          '(:authors ("Hopper, Grace")
            :title "Compilers and Cobol")))
       ((symbol-function 'file-exists-p)
        (lambda (file)
          (push (list 'exists file exists) calls)
          exists))
       ((symbol-function 'url-copy-file)
        (lambda (&rest arguments)
          (push (cons 'copy arguments) calls)
          'copied))
       ((symbol-function
         'arxiv-citation-test-open)
        (lambda (file)
          (push (list 'open file) calls)
          'opened)))
    (let ((arxiv-citation-open-pdf-function
           #'arxiv-citation-test-open))
      (list
       noninteractive
       (progn
         (setq exists nil)
         (arxiv-citation-download-and-open
          "https://arxiv.org/abs/2301.00001"))
       (prog1
           (nreverse calls)
         (setq calls nil))
       (progn
         (setq exists t)
         (arxiv-citation-download-and-open
          "https://arxiv.org/pdf/2301.00001.pdf"))
       (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (t opened ((exists "/research/preprints/hopper_compilers-and-cobol.pdf" nil) (open "/research/preprints/hopper_compilers-and-cobol.pdf")) opened ((exists "/research/preprints/hopper_compilers-and-cobol.pdf" t) (open "/research/preprints/hopper_compilers-and-cobol.pdf")))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn interactive_existing_file_uses_integer_confirmation_flag_and_ignores_copy_errors() {
    let elisp_form = r##"(let ((arxiv-citation-library
        "/research/preprints")
       (arxiv-citation-overwrite-file nil)
       (noninteractive nil)
       fail-copy
       calls)
  (cl-letf
      (((symbol-function
         'arxiv-citation-get-details)
        (lambda (_link)
          '(:authors ("Church, Alonzo")
            :title "A Formulation of Logic")))
       ((symbol-function 'file-exists-p)
        (lambda (file)
          (push (list 'exists file) calls)
          t))
       ((symbol-function 'url-copy-file)
        (lambda (&rest arguments)
          (push (cons 'copy arguments) calls)
          (if fail-copy
              (error "simulated refusal")
            'copied)))
       ((symbol-function
         'arxiv-citation-test-open)
        (lambda (file)
          (push (list 'open file) calls)
          'opened)))
    (let ((arxiv-citation-open-pdf-function
           #'arxiv-citation-test-open))
      (list
       (arxiv-citation-download-and-open
        "https://arxiv.org/abs/2302.00002")
       (prog1
           (nreverse calls)
         (setq calls nil
               fail-copy t))
       (arxiv-citation-download-and-open
        "https://arxiv.org/abs/2302.00003")
       (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (opened ((exists "/research/preprints/church_a-formulation-of-logic.pdf") (copy "https://arxiv.org/pdf/2302.00002.pdf" "/research/preprints/church_a-formulation-of-logic.pdf" 42) (open "/research/preprints/church_a-formulation-of-logic.pdf")) opened ((exists "/research/preprints/church_a-formulation-of-logic.pdf") (copy "https://arxiv.org/pdf/2302.00003.pdf" "/research/preprints/church_a-formulation-of-logic.pdf" 42) (open "/research/preprints/church_a-formulation-of-logic.pdf")))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn elfeed_integration_reads_current_entry_link_then_delegates_to_download_workflow() {
    let elisp_form = r##"(let (calls)
  (setq elfeed-show-entry
        '(:id "entry-42"))
  (provide 'elfeed)
  (cl-letf
      (((symbol-function 'elfeed-entry-link)
        (lambda (entry)
          (push (list 'entry-link entry) calls)
          "https://arxiv.org/abs/2403.00042"))
       ((symbol-function
         'arxiv-citation-download-and-open)
        (lambda (url)
          (push (list 'download url) calls)
          'opened)))
    (list
     (arxiv-citation-elfeed)
     (nreverse calls)
     (featurep 'elfeed))))"##;
    let expect = expect![[
        r#"OK (opened ((entry-link (:id "entry-42")) (download "https://arxiv.org/abs/2403.00042")) t)"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}
