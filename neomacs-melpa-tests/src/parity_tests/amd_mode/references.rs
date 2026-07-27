use expect_test::expect;

use super::assert_amd_mode_parity;

#[test]
fn xref_candidate_parses_colons_absolute_path_line_and_truncates_long_match() {
    let elisp_form = r##"
(let ((root (amd-test-project "xref-candidate")))
  (let ((default-directory root))
    (mapcar
     (lambda (raw)
       (condition-case error-data
           (amd--xref-candidate "foo" raw)
         (error
          (cons (car error-data)
                (cdr error-data)))))
     (list
      "src/a.js:17: define(['foo'], function(foo) {})"
      (concat
       "src/long.js:23: "
       (make-string 105 ?x)
       ":tail")))))
"##;
    let expect = expect![[
        r#"OK (((file . "[ORACLE-SANDBOX]/xref-candidate/src/a.js") (line . 17) (symbol . "foo") (match . "define(['foo'], function(foo) {})")) (wrong-type-argument number-or-marker-p "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx:tail"))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn false_positive_filter_requires_word_boundary_and_closing_quote() {
    let elisp_form = r##"
(mapcar
 (lambda (match)
   (amd--xref-false-positive
    (list (cons 'match match))
    "foo"))
 '("define(['foo'], function(foo) {})"
   "define(['path/foo\"], function(foo) {})"
   "define(['foobar'], function(foobar) {})"
   "plain foo text"
   "define(['foo-bar'], function(fooBar) {})"))
"##;
    let expect = expect!["OK (nil nil t t t)"];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn find_references_runs_real_fake_ag_with_ignores_and_filters_candidates() {
    let elisp_form = r##"
(let* ((root (amd-test-project "find-references"))
       (source (amd-test-write root "src/foo.js" ""))
       (log-file
        (amd-test-configure-ag
         root
         "src/a.js:4: define(['foo'], function(foo) {})\nsrc/b.js:9: define(['foobar'], function(foobar) {})\nsrc/c.js:12: define(['path/foo\"], function(foo) {})\n")))
  (let ((default-directory root)
        (amd-ag-arguments '("--js" "--numbers"))
        (amd-ag-ignored-dirs '("vendor" "build"))
        (amd-ag-ignored-files '("*.min.js" "bundle.js")))
    (list
     (amd--find-references source)
     (amd-test-read log-file))))
"##;
    let expect = expect![[
        r#"OK ((((file . "[ORACLE-SANDBOX]/find-references/src/c.js") (line . 12) (symbol . "foo") (match . "define(['path/foo\"], function(foo) {})")) ((file . "[ORACLE-SANDBOX]/find-references/src/a.js") (line . 4) (symbol . "foo") (match . "define(['foo'], function(foo) {})"))) "<--js>\n<--numbers>\n<--ignore-dir>\n<vendor>\n<--ignore-dir>\n<build>\n<--ignore>\n<*.min.js>\n<--ignore>\n<bundle.js>\n<define\\([^])]+['|\"](.*/)?foo['|\"]>\n")"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn xref_search_shows_real_xref_items_or_reports_no_reference() {
    let elisp_form = r##"
(let* ((root (amd-test-project "xref-show"))
       (candidate
        (list
         (cons 'file
               (amd-test-write root "src/a.js" "line"))
         (cons 'line 7)
         (cons 'symbol "foo")
         (cons 'match "define(['foo'])")))
       events)
  (cl-letf
      (((symbol-function 'amd--find-references)
        (lambda (file)
          (if (string= file "none.js")
              nil
            (list candidate))))
       ((symbol-function 'xref--show-xrefs)
        (lambda (xrefs display-action)
          (push
           (list
            (mapcar
             (lambda (xref)
               (let ((location
                      (xref-item-location xref)))
                 (list
                  (xref-item-summary xref)
                  (xref-file-location-file location)
                  (xref-file-location-line location)
                  (xref-file-location-column location))))
             xrefs)
            display-action)
           events)
          'shown))
       ((symbol-function 'message)
        (lambda (&rest arguments)
          (push arguments events)
          (apply #'format-message arguments))))
    (list
     (amd--xref-search-references "some.js")
     (amd--xref-search-references "none.js")
     (nreverse events))))
"##;
    let expect = expect![[
        r#"OK (shown "No reference found" (((("define(['foo'])" "[ORACLE-SANDBOX]/xref-show/src/a.js" 7 0)) nil) ("No reference found")))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn replace_references_accepts_string_or_function_and_rewrites_all_matches() {
    let elisp_form = r##"
(let* ((root (amd-test-project "replace-references"))
       (first
        (amd-test-write
         root "src/first.js"
         "define(['old/foo', \"old/foo\"], function(foo) {});\n"))
       (second
        (amd-test-write
         root "src/second.js"
         "define(['old/foo'], function(foo) {});\n"))
       (regexp "\\(define([^)]*['\"]\\)old/foo\\(['\"]\\)"))
  (amd--replace-references-in-file regexp "new/foo" first)
  (amd--replace-references-in-file
   regexp (lambda () "computed/foo") second)
  (list
   (with-current-buffer (find-file-noselect first)
     (buffer-string))
   (with-current-buffer (find-file-noselect second)
     (buffer-string))))
"##;
    let expect = expect![[
        r#"OK ("define(['old/foo', \"new/foo], function(foo) {});\n" "define(['computed/foo], function(foo) {});\n")"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn replace_all_updates_only_javascript_and_tolerates_missing_file() {
    let elisp_form = r##"
(let* ((root (amd-test-project "replace-all"))
       (js
        (amd-test-write
         root "src/a.js"
         "define(['old/foo'], function(foo) {});\n"))
       (text
        (amd-test-write
         root "src/a.txt"
         "define(['old/foo'], function(foo) {});\n"))
       (missing (expand-file-name "missing.js" root))
       (target-buffer
        (amd-test-open root "src/new/foo.js" "define([]);"))
       messages)
  (let ((default-directory root))
    (cl-letf
        (((symbol-function 'amd--module)
          (lambda (_) "new/foo"))
         ((symbol-function 'message)
          (lambda (&rest arguments)
            (push arguments messages))))
      (amd--replace-all-file-references
       "\\(define([^)]*['\"]\\)old/foo\\(['\"]\\)"
       target-buffer
       (list js text missing))
      (list
       (with-current-buffer
           (find-file-noselect js)
         (buffer-string))
       (with-current-buffer
           (find-file-noselect text)
         (buffer-string))
       (nreverse messages)))))
"##;
    let expect = expect![[
        r#"OK ("define(['new/foo], function(foo) {});\n" "define(['old/foo'], function(foo) {});\n" nil)"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}
