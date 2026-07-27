use expect_test::expect;

use super::{
    assert_arxiv_mode_parity, assert_arxiv_mode_query_parity, assert_arxiv_mode_signal_parity,
};

#[test]
fn parse_query_data_normalizes_real_words_quotes_and_surrounding_whitespace() {
    let elisp_form = r##"(mapcar
         #'arxiv-parse-query-data
         '("  quantum   gravity  "
           "\"large language model\" alignment"
           "single"
           "   "
           "a  \"b c\"   d"))"##;
    let expect = expect![[
        r#"OK ("quantum+gravity" "%22large+language+model%22+alignment" "single" "" "a+%22b+c%22+d")"#
    ]];
    assert_arxiv_mode_query_parity(elisp_form, expect);
}

#[test]
fn extract_pdf_walks_real_xml_link_nodes_and_handles_absence() {
    let elisp_form = r##"(list
         (arxiv-extract-pdf
          '((link ((href . "https://arxiv.org/abs/2401.1")
                   (title . "alternate")))
            (link ((href . "https://arxiv.org/pdf/2401.1")
                   (title . "pdf")))
            (link ((href . "https://example.test/source")
                   (title . "source")))))
         (arxiv-extract-pdf
          '((link ((href . "https://example.test/no-pdf")
                   (title . "alternate")))))
         (arxiv-extract-pdf nil))"##;
    let expect = expect![[r#"OK ("https://arxiv.org/pdf/2401.1" nil nil)"#]];
    assert_arxiv_mode_query_parity(elisp_form, expect);
}

#[test]
fn api_url_serializes_every_supported_field_boolean_and_sorting_choice() {
    let elisp_form = r##"(let ((arxiv-url "https://api.example/query")
               (arxiv-entries-per-fetch 25)
               (arxiv-query-data-list
                '((all t " quantum  gravity ")
                  (id t "2401.01234")
                  (time nil "[202401010000 TO 202402010000]")
                  (title t "\"dark matter\"")
                  (author nil "Ada Lovelace")
                  (abstract t "neural fields")
                  (comment nil "conference")
                  (journal t "Nature Physics")
                  (category t "cs.LG")))
               (arxiv-query-sorting
                '(:sortby relevance :sortorder ascending)))
         (arxiv-get-api-url 50))"##;
    let expect = expect![[
        r#"OK "http://export.arxiv.org/api/query?search_query=all:quantum+gravity+AND+id:2401.01234+ANDNOT+submittedDate:[202401010000+TO+202402010000]+AND+ti:%22dark+matter%22+ANDNOT+au:Ada+Lovelace+AND+abs:neural+fields+ANDNOT+co:conference+AND+jr:Nature+Physics+AND+cat:cs.LG&sortBy=relevance&sortOrder=ascending&start=50&max_results=25""#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn api_url_defaults_start_and_omits_sorting_when_unset() {
    let elisp_form = r##"(let ((arxiv-url "http://export.example/api")
               (arxiv-entries-per-fetch 7)
               (arxiv-query-data-list
                '((author t "Grace Hopper")))
               (arxiv-query-sorting nil))
         (list (arxiv-get-api-url)
               arxiv-query-data-list
               arxiv-query-sorting))"##;
    let expect = expect![[
        r#"OK ("http://export.arxiv.org/api/query?search_query=au:Grace+Hopper&start=0&max_results=7" ((author t "Grace Hopper")) nil)"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn date_url_covers_default_and_ascending_pagination() {
    let elisp_form = r##"(let ((arxiv-url "https://api.example/query")
               (arxiv-entries-per-fetch 100))
         (list
          (arxiv-geturl-date "202401010000" "202401312359"
                             "cs.LG")
          (arxiv-geturl-date "202401010000" "202401312359"
                             "math.NT" 200 t)))"##;
    let expect = expect![[
        r#"OK ("http://export.arxiv.org/api/query?search_query=submittedDate:[202401010000+TO+202401312359]+AND+cat:cs.LG*&sortBy=submittedDate&sortOrder=descending&start=0&max_results=100" "http://export.arxiv.org/api/query?search_query=submittedDate:[202401010000+TO+202401312359]+AND+cat:math.NT*&sortBy=submittedDate&sortOrder=ascending&start=200&max_results=100")"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn xml_context_reads_direct_text_from_real_parser_nodes() {
    let elisp_form = r##"(with-temp-buffer
         (insert "<entry><title>  A title  </title><id>2401.1</id></entry>")
         (let ((node (car (xml-parse-region (point-min) (point-max)))))
           (list (arxiv-getxml-context node 'title)
                 (arxiv-getxml-context node 'id)
                 (arxiv-getxml-context node 'missing))))"##;
    let expect = expect![[r#"OK ("  A title  " "2401.1" nil)"#]];
    assert_arxiv_mode_query_parity(elisp_form, expect);
}

#[test]
fn parse_api_transforms_a_realistic_atom_feed_into_complete_article_records() {
    let elisp_form = r##"(let ((xml
          "<?xml version=\"1.0\"?>
<feed>
 <opensearch:totalResults>2</opensearch:totalResults>
 <opensearch:startIndex>0</opensearch:startIndex>
 <opensearch:itemsPerPage>2</opensearch:itemsPerPage>
 <entry>
  <id>http://arxiv.org/abs/2401.01234</id>
  <updated>2024-01-03T04:05:06Z</updated>
  <published>2024-01-02T03:04:05Z</published>
  <title>  Practical
    Parity Testing </title>
  <summary> First line.
Second line. </summary>
  <author><name>Ada Lovelace</name></author>
  <author><name>Grace Hopper</name></author>
  <arxiv:doi>10.1000/parity</arxiv:doi>
  <arxiv:comment>12 pages</arxiv:comment>
  <arxiv:journal_ref>Journal of Tests 7</arxiv:journal_ref>
  <category term=\"cs.SE\"/>
  <category term=\"cs.LG\"/>
  <link href=\"http://arxiv.org/abs/2401.01234\" title=\"alternate\"/>
  <link href=\"http://arxiv.org/pdf/2401.01234\" title=\"pdf\"/>
 </entry>
 <entry>
  <id>http://arxiv.org/abs/2401.09999</id>
  <updated>2024-02-03T00:00:00Z</updated>
  <published>2024-02-01T00:00:00Z</published>
  <title>Minimal Entry</title>
  <summary>No optional metadata.</summary>
  <author><name>Lin Test</name></author>
  <category term=\"math.NT\"/>
  <link href=\"http://arxiv.org/pdf/2401.09999\" title=\"pdf\"/>
 </entry>
</feed>"))
         (cl-letf (((symbol-function 'url-retrieve-synchronously)
                    (lambda (url)
                      (let ((buffer
                             (generate-new-buffer
                              " *arxiv-api-fixture*")))
                        (with-current-buffer buffer
                          (insert xml)
                          (goto-char (point-min))
                          (setq-local fixture-url url))
                        buffer))))
           (let ((entries
                  (arxiv-parse-api
                   "https://api.example/query?fixture=1")))
             (list entries
                   arxiv-query-total-results
                   arxiv-query-results-min
                   arxiv-query-results-max))))"##;
    let expect = expect![[
        r#"OK ((((title . " Practical Parity Testing ") (author "Ada Lovelace" "Grace Hopper") (abstract . " First line. Second line. ") (url . "http://arxiv.org/abs/2401.01234") (id . "2401.01234") (date . "2024-01-02 03:04:05 ") (updated . "2024-01-03 04:05:06 ") (doi . "10.1000/parity") (comment . "12 pages") (journal . "Journal of Tests 7") (categories "cs.SE" "cs.LG") (pdf . "http://arxiv.org/pdf/2401.01234")) ((title . "Minimal Entry") (author "Lin Test") (abstract . "No optional metadata.") (url . "http://arxiv.org/abs/2401.09999") (id . "2401.09999") (date . "2024-02-01 00:00:00 ") (updated . "2024-02-03 00:00:00 ") (doi) (comment) (journal) (categories "math.NT") (pdf . "http://arxiv.org/pdf/2401.09999"))) 2 1 2)"#
    ]];
    assert_arxiv_mode_query_parity(elisp_form, expect);
}

#[test]
fn parse_api_caps_reported_result_max_to_total_results() {
    let elisp_form = r##"(let ((xml
          "<feed>
 <opensearch:totalResults>3</opensearch:totalResults>
 <opensearch:startIndex>2</opensearch:startIndex>
 <opensearch:itemsPerPage>100</opensearch:itemsPerPage>
</feed>"))
         (cl-letf (((symbol-function 'url-retrieve-synchronously)
                    (lambda (_url)
                      (let ((buffer (generate-new-buffer
                                     " *arxiv-empty-page*")))
                        (with-current-buffer buffer
                          (insert xml)
                          (goto-char (point-min)))
                        buffer))))
           (list (arxiv-parse-api "fixture:")
                 arxiv-query-total-results
                 arxiv-query-results-min
                 arxiv-query-results-max)))"##;
    let expect = expect!["OK (nil 3 3 3)"];
    assert_arxiv_mode_query_parity(elisp_form, expect);
}

#[test]
fn query_rejects_non_increasing_dates_with_the_exact_user_error() {
    let elisp_form = r##"(arxiv-query "cs.LG" "202402010000"
                     "202402010000")"##;
    let expect = expect![[r#"ERR (user-error "Incorrect date specification")"#]];
    assert_arxiv_mode_signal_parity(elisp_form, expect);
}

#[test]
fn query_builds_the_date_url_and_forwards_parsed_results() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arxiv-parse-api)
                    (lambda (url)
                      (push url calls)
                      '(((id . "result"))))))
           (list
            (arxiv-query "cs.LG" "202401010000"
                         "202402010000" 25 t)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((((id . "result"))) ("http://export.arxiv.org/api/query?search_query=submittedDate:[202401010000+TO+202402010000]+AND+cat:cs.LG*&sortBy=submittedDate&sortOrder=ascending&start=25&max_results=100"))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn category_sort_is_stable_and_places_primary_category_before_cross_lists() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((id . "cross-1")
                   (categories . ("math.NT" "cs.LG")))
                  ((id . "main-1")
                   (categories . ("cs.LG" "math.OC")))
                  ((id . "cross-2")
                   (categories . ("stat.ML" "cs.LG")))
                  ((id . "main-2")
                   (categories . ("cs.LG"))))))
         (arxiv-query-sort-cat "cs.LG")
         (mapcar (lambda (entry)
                   (alist-get 'id entry))
                 arxiv-entry-list))"##;
    let expect = expect![[r#"OK ("main-1" "main-2" "cross-1" "cross-2")"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn query_general_forwards_start_through_url_builder_and_parser() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arxiv-get-api-url)
                    (lambda (&optional start)
                      (push (list :url start) calls)
                      (format "fixture:%s" start)))
                   ((symbol-function 'arxiv-parse-api)
                    (lambda (url)
                      (push (list :parse url) calls)
                      '(:parsed))))
           (list (arxiv-query-general 75)
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK ((:parsed) ((:url 75) (:parse "fixture:75")))"#]];
    assert_arxiv_mode_query_parity(elisp_form, expect);
}
