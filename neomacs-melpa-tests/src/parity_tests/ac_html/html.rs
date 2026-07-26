use expect_test::expect;

use super::{assert_ac_html_parity, assert_ac_html_signal_parity};

#[test]
fn ac_html_inside_attr_value_distinguishes_quotes_boundaries_and_plain_attributes() {
    let elisp_form = r##"(cl-labels
               ((probe
                 (text)
                 (with-temp-buffer
                   (insert text)
                   (goto-char
                    (point-max))
                   (let ((before
                          (point)))
                     (list
                      (ac-html--inside-attrv)
                      (point)
                      before)))))
               (mapcar
                #'probe
                '("<a href=\""
                  "<a href=\"one two"
                  "<a href='single"
                  "<a href=\"closed\">"
                  "<a href=noquote"
                  "<a data-x = \n \"β")))"##;
    let expect = expect!["OK ((t 10 10) (t 17 17) (t 16 16) (nil 18 18) (nil 16 16) (t 17 17))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_tag_prefix_covers_empty_partial_nested_and_attribute_value_contexts() {
    let elisp_form = r##"(cl-labels
               ((probe
                 (text)
                 (with-temp-buffer
                   (insert text)
                   (goto-char
                    (point-max))
                   (list
                    (ac-html-tag-prefix)
                    (point)))))
               (mapcar
                #'probe
                '("<"
                  "<di"
                  "<section><sp"
                  "<a href=\"<fake"
                  "plain text"
                  "<a title='value")))"##;
    let expect = expect!["OK ((2 2) (2 4) (11 13) (nil 15) (0 11) (nil 16))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_attr_prefix_covers_spacing_partial_values_and_inside_value_suppression() {
    let elisp_form = r##"(cl-labels
               ((probe
                 (text)
                 (with-temp-buffer
                   (insert text)
                   (goto-char
                    (point-max))
                   (list
                    (ac-html-attr-prefix)
                    (point)))))
               (mapcar
                #'probe
                '("<a "
                  "<a hr"
                  "<input disabled ty"
                  "<a href=\"partial"
                  "<div\n  cla"
                  "no tag")))"##;
    let expect = expect!["OK ((4 4) (4 6) (17 19) (nil 17) (8 11) (0 7))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_value_prefix_returns_the_last_space_delimited_component() {
    let elisp_form = r##"(cl-labels
               ((probe
                 (text)
                 (with-temp-buffer
                   (insert text)
                   (goto-char
                    (point-max))
                   (list
                    (ac-html-value-prefix)
                    (point)))))
               (mapcar
                #'probe
                '("<div class=\""
                  "<div class=\"one"
                  "<div class=\"one two"
                  "<div class=\"one two three"
                  "<div class='one two"
                  "<input type=\"text\"")))"##;
    let expect = expect!["OK ((13 10) (13 10) (17 10) (21 10) (nil 20) (14 11))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_current_tag_ports_upstream_scan_and_handles_multiline_markup() {
    let elisp_form = r##"(cl-labels
               ((probe
                 (text position)
                 (with-temp-buffer
                   (insert text)
                   (goto-char position)
                   (let ((before
                          (point)))
                     (list
                      (ac-html-current-tag)
                      (point)
                      before)))))
               (list
                (probe
                 "<html><head lang=\"\"\n\n lang=\"\"></head></html>"
                 20)
                (probe
                 "<main>\n  <article\n    class=\"card\""
                 36)
                (probe
                 "<div><span data-x=\"value\">text"
                 30)))"##;
    let expect = expect![[r#"OK (("head" 20 20) ("article" 35 35) ("span" 30 30))"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_current_attr_handles_hyphens_whitespace_and_nearest_assignment() {
    let elisp_form = r##"(cl-labels
               ((probe
                 (text)
                 (with-temp-buffer
                   (insert text)
                   (goto-char
                    (point-max))
                   (let ((before
                          (point)))
                     (list
                      (ac-html-current-attr)
                      (point)
                      before)))))
               (mapcar
                #'probe
                '("<a href=\"value"
                  "<div data-long-name = \"value"
                  "<input type=\"text\" aria-label=\"name"
                  "<div\nclass = \"one two")))"##;
    let expect = expect![[
        r#"OK (("href" 15 15) ("data-long-name" 29 29) ("aria-label" 36 36) ("class" 22 22))"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_current_tag_signals_when_no_open_tag_with_trailing_space_exists() {
    let elisp_form = r##"(with-temp-buffer
               (insert "plain text")
               (goto-char
                (point-max))
               (ac-html-current-tag))"##;
    let expect = expect!["ERR (args-out-of-range (:buffer nil) 0 0)"];

    assert_ac_html_signal_parity(elisp_form, expect);
}

#[test]
fn ac_html_comment_detection_stub_returns_its_documentation_string() {
    let elisp_form = r##"(mapcar
               (lambda (text)
                 (with-temp-buffer
                   (insert text)
                   (goto-char
                    (point-max))
                   (ac-html--inside-comment)))
               '("<!-- open"
                 "<!-- closed -->"
                 "plain"))"##;
    let expect = expect![[
        r#"OK ("Return t if cursor inside comment.\nNot implemented yet." "Return t if cursor inside comment.\nNot implemented yet." "Return t if cursor inside comment.\nNot implemented yet.")"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}
