use expect_test::expect;

use super::assert_ac_html_parity;

#[test]
fn ac_slim_inside_ruby_code_recognizes_indented_dash_and_equals_lines() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (mapcar
                (lambda (text)
                  (with-temp-buffer
                    (insert text)
                    (goto-char
                     (point-max))
                    (ac-slim-inside-ruby-code)))
                '("- ruby_call"
                  "  = render"
                  "\t- nested"
                  "div = text"
                  "  p content"
                  "")))"##;
    let expect = expect!["OK (t t t nil nil nil)"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_slim_line_leading_spaces_counts_tabs_and_spaces_and_preserves_point() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (mapcar
                (lambda (text)
                  (with-temp-buffer
                    (insert text)
                    (goto-char
                     (point-max))
                    (let ((before
                           (point)))
                      (list
                       (ac-slim--line-leading-spaces)
                       (point)
                       before))))
                '("plain"
                  "   spaced"
                  "\t \tmixed"
                  "    "
                  "first\n  second")))"##;
    let expect = expect!["OK ((0 6 6) (3 10 10) (3 9 9) (4 5 5) (2 15 15))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_slim_line_classifiers_cover_block_indicators_and_empty_lines() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (mapcar
                (lambda (text)
                  (with-temp-buffer
                    (insert text)
                    (goto-char
                     (point-max))
                    (list
                     (ac-slim--line-is-block-indicator)
                     (ac-slim--line-is-empty))))
                '("ruby:"
                  "  javascript: "
                  "\tcoffee:"
                  "div ruby:"
                  "  div"
                  "   "
                  "")))"##;
    let expect = expect!["OK ((t nil) (t nil) (t nil) (t nil) (nil nil) (nil t) (nil t))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_slim_inside_non_slim_block_walks_indentation_and_skips_blank_lines() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (cl-labels
                   ((probe
                     (text)
                     (with-temp-buffer
                       (insert text)
                       (goto-char
                        (point-max))
                       (ac-slim-inside-non-slim-block))))
                 (mapcar
                  #'probe
                  '("ruby:\n  value"
                    "javascript:\n\n  nested\n    deeper"
                    "coffee:\n  first\nnext"
                    "div\n  span\n    text"
                    "  ruby:"
                    "plain"))))"##;
    let expect = expect!["OK (t t nil nil t nil)"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_slim_tag_prefix_handles_root_nested_colon_and_suppressed_code_contexts() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (cl-labels
                   ((probe
                     (text)
                     (with-temp-buffer
                       (insert text)
                       (goto-char
                        (point-max))
                       (list
                        (ac-slim-tag-prefix)
                        (point)))))
                 (mapcar
                  #'probe
                  '("di"
                    "  spa"
                    "li: a"
                    "- ruby"
                    "ruby:\n  code"
                    "div\n  spa"))))"##;
    let expect = expect!["OK ((1 3) (3 6) (5 6) (nil 7) (nil 13) (7 10))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_slim_attr_and_value_prefixes_distinguish_names_from_quoted_values() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (cl-labels
                   ((probe
                     (text)
                     (with-temp-buffer
                       (insert text)
                       (goto-char
                        (point-max))
                       (list
                        (ac-slim-attr-prefix)
                        (ac-slim-attrv-prefix)
                        (point)))))
                 (mapcar
                  #'probe
                  '("a hr"
                    "a href=\""
                    "a href=\"one"
                    "a class='one two"
                    "- call attr=\"value"
                    "javascript:\n  node attr=\"value"))))"##;
    let expect =
        expect!["OK ((3 nil 5) (nil 9 9) (nil 9 12) (nil 14 17) (nil nil 19) (nil nil 31))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_slim_class_and_id_prefixes_find_the_last_shorthand_component() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (cl-labels
                   ((probe
                     (text)
                     (with-temp-buffer
                       (insert text)
                       (goto-char
                        (point-max))
                       (list
                        (ac-slim-class-prefix)
                        (ac-slim-id-prefix)
                        (point)))))
                 (mapcar
                  #'probe
                  '(".bu"
                    "div.card.bu"
                    "#he"
                    "div.card#he"
                    "li: a.link.ac"
                    "- .ruby"
                    "ruby:\n  .code"))))"##;
    let expect = expect![
        "OK ((2 nil 4) (10 nil 12) (nil 2 4) (5 10 12) (12 nil 14) (nil nil 8) (nil nil 14))"
    ];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_slim_current_tag_ports_upstream_scan_and_uses_div_for_shorthand() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (let ((upstream
                      (with-temp-buffer
                        (insert
                         "html\n  head\n    title This Title\n  body\n    div")
                        (goto-char 30)
                        (ac-slim-current-tag))))
                 (list
                  upstream
                  (with-temp-buffer
                    (insert ".card")
                    (goto-char
                     (point-max))
                    (ac-slim-current-tag))
                  (with-temp-buffer
                    (insert "li: a.link")
                    (goto-char
                     (point-max))
                    (ac-slim-current-tag))
                  (with-temp-buffer
                    (insert "  custom-tag content")
                    (goto-char
                     (point-max))
                    (ac-slim-current-tag)))))"##;
    let expect = expect![[r#"OK ("title" "div" "li" "custom")"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_slim_current_attr_returns_nearest_hyphenated_assignment_and_preserves_point() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (mapcar
                (lambda (text)
                  (with-temp-buffer
                    (insert text)
                    (goto-char
                     (point-max))
                    (let ((before
                           (point)))
                      (list
                       (ac-slim-current-attr)
                       (point)
                       before))))
                '("a href=\"value"
                  "div data-role = \"card"
                  "input type=\"text\" aria-label=\"name")))"##;
    let expect = expect![[r#"OK (("href" 14 14) ("data-role" 22 22) ("aria-label" 35 35))"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_slim_setup_selects_slim_callbacks_per_buffer() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (with-temp-buffer
                 (setq
                  ac-html-current-tag-function
                  'old-tag
                  ac-html-current-attr-function
                  'old-attr)
                 (list
                  (ac-slim-setup)
                  ac-html-current-tag-function
                  ac-html-current-attr-function
                  (local-variable-p
                   'ac-html-current-tag-function)
                  (local-variable-p
                   'ac-html-current-attr-function))))"##;
    let expect = expect!["OK (ac-slim-current-attr ac-slim-current-tag ac-slim-current-attr t t)"];

    assert_ac_html_parity(elisp_form, expect);
}
