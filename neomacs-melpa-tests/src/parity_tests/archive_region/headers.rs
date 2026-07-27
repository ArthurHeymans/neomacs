use expect_test::expect;

use super::assert_archive_region_parity;

#[test]
fn archive_region_add_header_uses_hash_fallback_fixed_date_and_nil_first_line_link() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "first archived line\n"
          "second archived line")
         (let ((comment-start
                nil)
               (comment-end
                ""))
           (cl-letf
               (((symbol-function
                  'format-time-string)
                 (lambda (format-string &rest _)
                   (list
                    format-string)
                   "[2024/02/03]")))
             (archive-region-add-header)))
         (list
          (buffer-string)
          (point)
          (line-number-at-pos)
          (archive-region-link-to-original)))"##;
    let expect = expect![[
        r##"OK ("# [2024/02/03]\n# (archive-region-pos nil)\nfirst archived line\nsecond archived line" 43 3 (archive-region-pos "# (archive-region-pos nil)"))"##
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_add_header_uses_real_emacs_lisp_comment_rules_and_previous_context() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(setq context 1)\n"
          "\n"
          "(message \"archive me\")\n"
          "(message \"too\")\n")
         (goto-char
          (point-min))
         (forward-line 2)
         (let ((start
                (point)))
           (narrow-to-region
            start
            (point-max))
           (cl-letf
               (((symbol-function
                  'format-time-string)
                 (lambda (&rest _)
                   "[2042/12/31]")))
             (archive-region-add-header))
           (list
            (buffer-string)
            (archive-region-link-to-original)
            (point-min)
            (point-max)
            (point))))"##;
    let expect = expect![[
        r#"OK (";; [2042/12/31]\n;; (archive-region-pos \"(setq context 1)\")\n(message \"archive me\")\n(message \"too\")\n" (archive-region-pos ";; (archive-region-pos \"(setq context 1)\")") 19 117 78)"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_add_header_respects_line_comment_start_end_and_padding_contracts() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (with-temp-buffer
             (insert
              "payload")
             (let ((comment-start
                    (nth 0 spec))
                   (comment-end
                    (nth 1 spec))
                   (comment-padding
                    (nth 2 spec)))
               (cl-letf
                   (((symbol-function
                      'format-time-string)
                     (lambda (&rest _)
                       "DATE"))
                    ((symbol-function
                      'archive-region-link-to-original)
                     (lambda ()
                       '(archive-region-pos
                         "context"))))
                 (archive-region-add-header))
               (list
                spec
                (buffer-string)))))
         '(("//" "" 1)
           ("--" "" 2)
           (";" "" 0)
           ("# " "" 1)))"##;
    let expect = expect![[
        r##"OK ((("//" "" 1) "// DATE\n// (archive-region-pos \"context\")\npayload") (("--" "" 2) "--  DATE\n--  (archive-region-pos \"context\")\npayload") ((";" "" 0) ";DATE\n;(archive-region-pos \"context\")\npayload") (("# " "" 1) "# DATE\n# (archive-region-pos \"context\")\npayload"))"##
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_add_header_forwards_custom_date_format_without_interpreting_result() {
    let elisp_form = r##"(let ((formats
                '("%Y-%m-%dT%H:%M:%S%z"
                  "week-%V"
                  "λ-%Y"))
               results)
         (dolist (format-string formats)
           (with-temp-buffer
             (insert
              "body")
             (let ((archive-region-date-format
                    format-string)
                   captured)
               (cl-letf
                   (((symbol-function
                      'format-time-string)
                     (lambda (received &rest arguments)
                       (setq captured
                             (cons
                              received
                              arguments))
                       (format
                        "<%s>"
                        received)))
                    ((symbol-function
                      'archive-region-link-to-original)
                     (lambda ()
                       '(archive-region-pos nil))))
                 (archive-region-add-header))
               (setq results
                     (append
                      results
                      (list
                       (list
                        format-string
                        captured
                        (buffer-string))))))))
         results)"##;
    let expect = expect![[
        r##"OK (("%Y-%m-%dT%H:%M:%S%z" ("%Y-%m-%dT%H:%M:%S%z") "# <%Y-%m-%dT%H:%M:%S%z>\n# (archive-region-pos nil)\nbody") ("week-%V" ("week-%V") "# <week-%V>\n# (archive-region-pos nil)\nbody") ("λ-%Y" ("λ-%Y") "# <λ-%Y>\n# (archive-region-pos nil)\nbody"))"##
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_add_header_preserves_payload_properties_and_marks_only_inserted_header_as_comment()
 {
    let elisp_form = r##"(with-temp-buffer
         (insert
          (propertize
           "payload"
           'archive-test-property
           :kept))
         (let ((comment-start
                "#")
               (comment-end
                ""))
           (cl-letf
               (((symbol-function
                  'format-time-string)
                 (lambda (&rest _)
                   "DATE"))
                ((symbol-function
                  'archive-region-link-to-original)
                 (lambda ()
                   '(archive-region-pos
                     "before"))))
             (archive-region-add-header)))
         (list
          (buffer-substring
           (point-min)
           (point-max))
          (get-text-property
           (point-min)
           'archive-test-property)
          (get-text-property
           (1-
            (point-max))
           'archive-test-property)
          (text-property-not-all
           (point-min)
           (point-max)
           'archive-test-property
           nil)
          (point)))"##;
    let expect = expect![[
        r##"OK (#("# DATE\n# (archive-region-pos \"before\")\npayload" 39 46 (archive-test-property :kept)) nil :kept 40 40)"##
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_add_header_repeated_calls_nest_new_headers_in_deterministic_order() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "payload")
         (let ((counter
                0)
               (comment-start
                "#")
               (comment-end
                ""))
           (cl-letf
               (((symbol-function
                  'format-time-string)
                 (lambda (&rest _)
                   (setq
                    counter
                    (1+
                     counter))
                   (format
                    "DATE-%d"
                    counter)))
                ((symbol-function
                  'archive-region-link-to-original)
                 (lambda ()
                   (list
                    'archive-region-pos
                    (format
                     "context-%d"
                     counter)))))
             (archive-region-add-header)
             (archive-region-add-header)))
         (list
          (buffer-string)
          (point)
          (count-lines
           (point-min)
           (point-max))))"##;
    let expect = expect![[
        r##"OK ("# DATE-2\n# (archive-region-pos \"context-2\")\n# DATE-1\n# (archive-region-pos \"context-1\")\npayload" 45 5)"##
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_header_contains_readable_round_trippable_navigation_form() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "payload")
         (let ((comment-start
                ";;")
               (comment-end
                ""))
           (cl-letf
               (((symbol-function
                  'format-time-string)
                 (lambda (&rest _)
                   "[2025/01/02]"))
                ((symbol-function
                  'archive-region-link-to-original)
                 (lambda ()
                   '(archive-region-pos
                     "previous \"quoted\" λ"))))
             (archive-region-add-header)))
         (let ((commented
                (buffer-string)))
           (goto-char
            (point-min))
           (forward-line 1)
           (let ((line
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position))))
             (string-match
              "\\`;;[ \t]*\\(.*\\)\\'"
              line)
             (let* ((printed
                     (match-string
                      1
                      line))
                    (form
                     (read printed)))
               (list
                commented
                printed
                form
                (prin1-to-string
                 form)
                (equal
                 form
                 '(archive-region-pos
                   "previous \"quoted\" λ")))))))"##;
    let expect = expect![[
        r#"OK (";; [2025/01/02]\n;; (archive-region-pos \"previous \\\"quoted\\\" λ\")\npayload" "(archive-region-pos \"previous \\\"quoted\\\" λ\")" (archive-region-pos "previous \"quoted\" λ") "(archive-region-pos \"previous \\\"quoted\\\" λ\")" t)"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}
