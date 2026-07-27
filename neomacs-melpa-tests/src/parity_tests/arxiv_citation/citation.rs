use expect_test::expect;

use super::assert_arxiv_citation_parity;

#[test]
fn generate_autokey_uses_real_bibtex_mode_and_package_specific_separator_configuration() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "@Article{,\n"
   " author = {Lovelace, Ada and Turing, Alan},\n"
   " title = {Practical Category Theory for Programs},\n"
   " year = {2024},\n"
   "}\n")
  (let ((before (buffer-string))
        (before-point (point)))
    (goto-char (point-min))
    (list
     (arxiv-citation-generate-autokey)
     major-mode
     bibtex-autokey-year-title-separator
     bibtex-autokey-titleword-separator
     before
     (buffer-string)
     before-point
     (point))))"##;
    let expect = expect![[
        r#"OK ("lovelace24:pract-categ-theor-progr" bibtex-mode ":" "-" "@Article{,\n author = {Lovelace, Ada and Turing, Alan},\n title = {Practical Category Theory for Programs},\n year = {2024},\n}\n" "@Article{,\n author = {Lovelace, Ada and Turing, Alan},\n title = {Practical Category Theory for Programs},\n year = {2024},\n}\n" 125 1)"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn arxiv_citation_formats_realistic_metadata_and_generates_a_bibtex_key_from_the_entry() {
    let elisp_form = r##"(cl-letf
    (((symbol-function 'arxiv-citation-get-details)
      (lambda (url)
        (list
         :source-url url
         :id "2402.12345"
         :authors
         '("Lovelace, Ada"
           "Turing, Alan M."
           "Hopper, Grace")
         :title
         "{P}ractical {C}ategory {T}heory for {P}rograms"
         :year "2024"
         :categories
         '("cs.PL" "math.CT" "cs.AI")))))
  (arxiv-citation-get-arxiv-citation
   "https://arxiv.org/abs/2402.12345"))"##;
    let expect = expect![[
        r#"OK "@Article{lovelace24:pract-categ-theor-progr,\n author        = {Lovelace, Ada and Turing, Alan M. and Hopper, Grace},\n journal       = {arXiv e-prints},\n title         = {{P}ractical {C}ategory {T}heory for {P}rograms},\n year          = {2024},\n eprint        = {2402.12345},\n eprintclass   = {cs.PL},\n eprinttype    = {arXiv},\n keywords      = {cs.PL, math.CT, cs.AI},\n}""#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn arxiv_citation_preserves_empty_authors_categories_and_title_fields_through_bibtex_generation() {
    let elisp_form = r##"(cl-letf
    (((symbol-function 'arxiv-citation-get-details)
      (lambda (_url)
        '(:id "0000.00000"
          :authors nil
          :title ""
          :year ""
          :categories nil))))
  (condition-case error
      (arxiv-citation-get-arxiv-citation
       "https://arxiv.org/abs/0000.00000")
    (error
     (list (car error) (cdr error)))))"##;
    let expect = expect![[r#"OK (user-error ("Year or date field ‘’ invalid"))"#]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn zbmath_citation_strips_http_headers_replaces_remote_key_and_aligns_fields_with_spaces() {
    let elisp_form = r##"(let (calls response)
  (cl-letf
      (((symbol-function 'url-retrieve-synchronously)
        (lambda (url &rest arguments)
          (push (cons url arguments) calls)
          (setq response
                (generate-new-buffer
                 " *zbmath-bibtex-response*"))
          (with-current-buffer response
            (insert
             "HTTP/1.1 200 OK\nContent-Type: text/plain\n\n"
             "@Article{ZBL_REMOTE_KEY,\n"
             "author = {Lovelace, Ada and Turing, Alan},\n"
             "title = {Practical Category Theory},\n"
             "journal = {Journal of Useful Results},\n"
             "year = {2024},\n"
             "}\n"))
          response)))
    (unwind-protect
        (list
         (arxiv-citation-get-zbmath-citation
          "https://zbmath.org/bibtex/1234.56789.bib")
         (nreverse calls)
         (and
          (buffer-live-p response)
          (with-current-buffer response
            (list
             major-mode
             indent-tabs-mode
             align-default-spacing))))
      (when (buffer-live-p response)
        (kill-buffer response)))))"##;
    let expect = expect![[
        r#"OK ("@Article{lovelace24:pract-categ-theor,\nauthor       = {Lovelace, Ada and Turing, Alan},\ntitle        = {Practical Category Theory},\njournal      = {Journal of Useful Results},\nyear         = {2024},\n}\n" (("https://zbmath.org/bibtex/1234.56789.bib" t t)) (bibtex-mode nil 5))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn citation_lookup_routes_arxiv_results_with_zbl_ids_to_the_exact_zbmath_bibtex_url() {
    let elisp_form = r##"(let (network-calls citation-calls response)
  (cl-letf
      (((symbol-function 'url-retrieve-synchronously)
        (lambda (url &rest arguments)
          (push (cons url arguments) network-calls)
          (setq response
                (generate-new-buffer
                 " *zbmath-search-response*"))
          response))
       ((symbol-function 'arxiv-citation-parse)
        (lambda (method)
          (list
           'html nil
           (list
            'search nil nil
            (list
             'result nil
             "Document Zbl 1234.56789")))))
       ((symbol-function
         'arxiv-citation-get-zbmath-citation)
        (lambda (url)
          (push (list 'zbmath url)
                citation-calls)
          "ZBMATH-CITATION"))
       ((symbol-function
         'arxiv-citation-get-arxiv-citation)
        (lambda (url)
          (push (list 'arxiv url)
                citation-calls)
          "ARXIV-CITATION")))
    (unwind-protect
        (list
         (arxiv-citation-get-citation
          "https://arxiv.org/abs/2402.12345")
         (nreverse network-calls)
         (nreverse citation-calls))
      (when (buffer-live-p response)
        (kill-buffer response)))))"##;
    let expect = expect![[
        r#"OK ("ZBMATH-CITATION" (("https://zbmath.org/?q=arXiv:2402.12345" t t)) ((zbmath "https://zbmath.org/bibtex/1234.56789.bib")))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn citation_lookup_falls_back_to_arxiv_when_zbmath_has_no_document_id() {
    let elisp_form = r##"(let (network-calls citation-calls response)
  (cl-letf
      (((symbol-function 'url-retrieve-synchronously)
        (lambda (url &rest arguments)
          (push (cons url arguments) network-calls)
          (setq response
                (generate-new-buffer
                 " *zbmath-empty-search-response*"))
          response))
       ((symbol-function 'arxiv-citation-parse)
        (lambda (_method)
          (list
           'html nil
           (list
            'search nil nil
            (list
             'result nil
             "No documents matched")))))
       ((symbol-function
         'arxiv-citation-get-zbmath-citation)
        (lambda (url)
          (push (list 'zbmath url)
                citation-calls)
          "ZBMATH-CITATION"))
       ((symbol-function
         'arxiv-citation-get-arxiv-citation)
        (lambda (url)
          (push (list 'arxiv url)
                citation-calls)
          "ARXIV-CITATION")))
    (unwind-protect
        (list
         (arxiv-citation-get-citation
          "https://arxiv.org/pdf/hep-th/9901001.pdf")
         (nreverse network-calls)
         (nreverse citation-calls))
      (when (buffer-live-p response)
        (kill-buffer response)))))"##;
    let expect = expect![[
        r#"OK ("ARXIV-CITATION" (("https://zbmath.org/?q=arXiv:hep-th/9901001" t t)) ((arxiv "https://arxiv.org/pdf/hep-th/9901001.pdf")))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn direct_zbmath_and_unrecognized_urls_preserve_the_dispatchers_exact_network_arguments() {
    let elisp_form = r##"(let (network-calls arxiv-calls responses)
  (cl-letf
      (((symbol-function 'url-retrieve-synchronously)
        (lambda (url &rest arguments)
          (push (cons url arguments) network-calls)
          (let ((buffer
                 (generate-new-buffer
                  " *citation-dispatch-response*")))
            (push buffer responses)
            buffer)))
       ((symbol-function 'arxiv-citation-parse)
        (lambda (_method)
          (list
           'html nil
           (list
            'search nil nil
            (list 'result nil "Nothing")))))
       ((symbol-function
         'arxiv-citation-get-arxiv-citation)
        (lambda (url)
          (push url arxiv-calls)
          (concat "ARXIV:" url))))
    (unwind-protect
        (list
         (arxiv-citation-get-citation
          "https://zbmath.org/?q=ti:semantics")
         (arxiv-citation-get-citation
          "https://example.org/paper")
         (nreverse network-calls)
         (nreverse arxiv-calls))
      (mapc
       (lambda (buffer)
         (when (buffer-live-p buffer)
           (kill-buffer buffer)))
       responses))))"##;
    let expect = expect![[
        r#"OK ("ARXIV:https://zbmath.org/?q=ti:semantics" "ARXIV:https://example.org/paper" (("https://zbmath.org/?q=ti:semantics" t t) (nil t t)) ("https://zbmath.org/?q=ti:semantics" "https://example.org/paper"))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}
