use expect_test::expect;

use super::assert_arxiv_citation_parity;

#[test]
fn arxiv_id_parses_modern_old_pdf_query_case_and_malformed_url_variants_exactly() {
    let elisp_form = r##"(list
 (mapcar
  (lambda (url)
    (cons url
          (arxiv-citation-arXiv-id url)))
  '("https://arxiv.org/abs/2201.01234"
    "https://arxiv.org/pdf/2201.01234.pdf"
    "http://arxiv.org/abs/0704.0001v2"
    "https://arxiv.org/pdf/hep-th/9901001.pdf"
    "https://arxiv.org/abs/math.GT/0309136"
    "https://notarxiv.org/abs/2401.99999"
    "https://arxiv.org/ABS/2501.00001"
    "https://example.org/papers/2201.01234"
    "https://arxiv.org/abs/"))
 (let ((case-fold-search nil))
   (mapcar
    #'arxiv-citation-arXiv-id
    '("https://arxiv.org/ABS/2501.00001"
      "https://arxiv.org/abs/2501.00001"))))"##;
    let expect = expect![[
        r#"OK ((("https://arxiv.org/abs/2201.01234" . "2201.01234") ("https://arxiv.org/pdf/2201.01234.pdf" . "2201.01234") ("http://arxiv.org/abs/0704.0001v2" . "0704.0001") ("https://arxiv.org/pdf/hep-th/9901001.pdf" . "hep-th/9901001") ("https://arxiv.org/abs/math.GT/0309136" . "") ("https://notarxiv.org/abs/2401.99999" . "2401.99999") ("https://arxiv.org/ABS/2501.00001" . "2501.00001") ("https://example.org/papers/2201.01234") ("https://arxiv.org/abs/" . "")) (nil "2501.00001"))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn pdf_link_converts_abs_urls_preserves_existing_pdf_substrings_and_handles_non_arxiv_text() {
    let elisp_form = r##"(mapcar
 (lambda (url)
   (cons url
         (arxiv-citation-pdf-link url)))
 '("https://arxiv.org/abs/2201.01234"
   "https://arxiv.org/abs/2201.01234?download=1"
   "https://arxiv.org/pdf/2201.01234.pdf"
   "https://arxiv.org/pdf/2201.01234.pdf?download=1"
   "https://example.org/abs/paper"
   "https://example.org/view?format=.pdf"
   ""))"##;
    let expect = expect![[
        r#"OK (("https://arxiv.org/abs/2201.01234" . "https://arxiv.org/pdf/2201.01234.pdf") ("https://arxiv.org/abs/2201.01234?download=1" . "https://arxiv.org/pdf/2201.01234?download=1.pdf") ("https://arxiv.org/pdf/2201.01234.pdf" . "https://arxiv.org/pdf/2201.01234.pdf") ("https://arxiv.org/pdf/2201.01234.pdf?download=1" . "https://arxiv.org/pdf/2201.01234.pdf?download=1") ("https://example.org/abs/paper" . "https://example.org/pdf/paper.pdf") ("https://example.org/view?format=.pdf" . "https://example.org/view?format=.pdf") ("" . ".pdf"))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn parse_uses_real_libxml_for_http_prefixed_atom_xml_and_html_documents() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (insert
    "HTTP/1.1 200 OK\nContent-Type: application/atom+xml\n\n"
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
    "<feed>"
    "<entry>"
    "<title>Practical Parsing</title>"
    "<author><name>Ada Lovelace</name></author>"
   "<category term=\"cs.PL\"/>"
   "</entry>"
   "</feed>")
   (let* ((tree (arxiv-citation-parse :xml))
          (entry (alist-get 'entry tree))
          (author
           (seq-find
            (lambda (item)
              (and
               (consp item)
               (eq (car item) 'author)))
            entry))
          (category
           (seq-find
            (lambda (item)
              (and
               (consp item)
               (eq (car item) 'category)))
            entry)))
     (list
      (car tree)
      (cadr (alist-get 'title entry))
      (caddr (caddr author))
      (alist-get
       'term
       (cadr category)))))
 (with-temp-buffer
   (insert
    "HTTP/1.1 200 OK\nContent-Type: text/html\n\n"
    "<!doctype html>"
    "<html lang=\"en\">"
    "<head><title>Zentralblatt Result</title></head>"
    "<body><div id=\"result\"><span>Document Zbl 1234.56789</span></div></body>"
    "</html>")
   (let* ((tree (arxiv-citation-parse :html))
          (head (alist-get 'head tree))
          (body (alist-get 'body tree))
          (div (alist-get 'div body))
          (span (alist-get 'span div)))
     (list
     (car tree)
      (alist-get 'lang (cadr tree))
      (cadr (alist-get 'title head))
      (alist-get 'id (car div))
      (cadr span)))))"##;
    let expect = expect![[
        r#"OK ((feed "Practical Parsing" "Ada Lovelace" "cs.PL") (html "en" "Zentralblatt Result" "result" "Document Zbl 1234.56789"))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn parse_invalid_method_and_missing_document_marker_expose_exact_errors_and_point_state() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (insert (cdr case))
     (condition-case error
         (list
          (arxiv-citation-parse (car case))
          (point))
       (error
        (list
         (car error)
         (cdr error)
         (point))))))
 '((:yaml . "---\ntitle: paper")
   (:xml . "<feed><entry/></feed>")
   (:html . "<html><body>missing-space-marker</body></html>")))"##;
    let expect = expect![[
        r#"OK ((wrong-type-argument (stringp nil) 1) (search-failed ("<?xml ") 1) (search-failed ("<html ") 1))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn get_details_fetches_exact_api_url_then_real_xml_dash_and_s_pipeline_normalizes_metadata() {
    let elisp_form = r##"(let (calls response)
  (cl-letf
      (((symbol-function 'url-retrieve-synchronously)
        (lambda (url &rest arguments)
          (push (cons url arguments) calls)
          (setq response
                (generate-new-buffer
                 " *arxiv-details-response*"))
          (with-current-buffer response
            (insert
             "HTTP/1.1 200 OK\nContent-Type: application/atom+xml\n\n"
             "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
             "<feed><entry>"
             "<title>Categorical AI: \nAn XML Study</title>"
             "<author><name>Ada Lovelace</name></author>"
             "<author><name>Ludwig van Beethoven</name></author>"
             "<author><name>Émilie du Châtelet</name></author>"
             "<published>2024-02-29T18:30:00Z</published>"
             "<category term=\"cs.AI\"/>"
             "<category term=\"math.CT\"/>"
             "</entry></feed>"))
          response)))
    (unwind-protect
        (list
         (arxiv-citation-get-details
          "https://arxiv.org/abs/2402.12345")
         (nreverse calls)
         (and
          (buffer-live-p response)
          (with-current-buffer response
            case-fold-search)))
      (when (buffer-live-p response)
        (kill-buffer response)))))"##;
    let expect = expect![[
        r#"OK ((:id "2402.12345" :authors ("Lovelace, Ada" "Beethoven, Ludwigvan" "Châtelet, Émiliedu") :title "{C}ategorical {A}{I}: {A}n {X}{M}{L} {S}tudy" :year "2024" :categories ("cs.AI" "math.CT")) (("http://export.arxiv.org/api/query?id_list=2402.12345" t t)) nil)"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn get_details_braces_error_title_before_sentinel_check_so_the_error_entry_survives() {
    let elisp_form = r##"(let (response)
  (cl-letf
      (((symbol-function 'url-retrieve-synchronously)
        (lambda (&rest _arguments)
          (setq response
                (generate-new-buffer
                 " *arxiv-error-response*"))
          (with-current-buffer response
            (insert
             "HTTP/1.1 200 OK\n\n"
             "<?xml version=\"1.0\"?>"
             "<feed><entry>"
             "<title>Error</title>"
             "<published>2024-01-01T00:00:00Z</published>"
             "</entry></feed>"))
          response)))
    (unwind-protect
        (arxiv-citation-get-details
         "https://arxiv.org/abs/9999.99999")
      (when (buffer-live-p response)
        (kill-buffer response)))))"##;
    let expect = expect![[
        r#"OK (:id "9999.99999" :authors nil :title "{E}rror" :year "2024" :categories nil)"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}
