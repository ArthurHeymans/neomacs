use expect_test::expect;

use super::assert_ac_html_parity;

#[test]
fn ac_jade_current_tag_ports_upstream_scan_and_preserves_point() {
    let elisp_form = r##"(progn
               (require 'ac-jade)
               (let ((upstream
                      (with-temp-buffer
                        (insert
                         "html\n  head\n    title= This Title\n  body\n    div")
                        (goto-char 30)
                        (let ((before
                               (point)))
                          (list
                           (ac-jade-current-tag)
                           (point)
                           before)))))
                 (list
                  upstream
                  (with-temp-buffer
                    (insert "  article.card")
                    (goto-char
                     (point-max))
                    (ac-jade-current-tag))
                  (with-temp-buffer
                    (insert "main\n  custom-tag")
                    (goto-char
                     (point-max))
                    (ac-jade-current-tag)))))"##;
    let expect = expect![[r#"OK (("title" 30 30) "article" "custom")"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_jade_current_attr_uses_nearest_assignment_and_keeps_the_live_point() {
    let elisp_form = r##"(progn
               (require 'ac-jade)
               (mapcar
                (lambda (text)
                  (with-temp-buffer
                    (insert text)
                    (goto-char
                     (point-max))
                    (let ((before
                           (point)))
                      (list
                       (ac-jade-current-attr)
                       (point)
                       before))))
                '("a(href=\"value"
                  "div(data-role = \"card"
                  "input(type=\"text\", aria-label=\"name")))"##;
    let expect = expect![[r#"OK (("href" 14 14) ("data-role" 22 22) ("aria-label" 36 36))"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_jade_attr_value_prefix_tracks_the_last_space_delimited_value() {
    let elisp_form = r##"(progn
               (require 'ac-jade)
               (mapcar
                (lambda (text)
                  (with-temp-buffer
                    (insert text)
                    (goto-char
                     (point-max))
                    (list
                     (ac-jade-attrv-prefix)
                     (point))))
                '("a(href=\""
                  "a(class=\"one"
                  "a(class=\"one two"
                  "a(class='one two"
                  "a(href = \"path with spa")))"##;
    let expect = expect!["OK ((9 6) (10 7) (14 7) (nil 17) (21 6))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_jade_setup_and_sources_keep_literal_prefix_regexps() {
    let elisp_form = r##"(progn
               (require 'ac-jade)
               (with-temp-buffer
                 (list
                  (ac-jade-setup)
                  ac-html-current-tag-function
                  ac-html-current-attr-function
                  (cdr
                   (assq
                    'prefix
                    ac-source-jade-tag))
                  (cdr
                   (assq
                    'prefix
                    ac-source-jade-attr))
                  (cdr
                   (assq
                    'prefix
                    ac-source-jade-attrv)))))"##;
    let expect = expect![[
        r#"OK (ac-jade-current-attr ac-jade-current-tag ac-jade-current-attr "^[\11 ]*\\(.*\\)" "\\(?:,\\|(\\)[ ]*\\(.*\\)" ac-jade-attrv-prefix)"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_haml_current_tag_ports_upstream_scan_and_defaults_shorthand_to_div() {
    let elisp_form = r##"(progn
               (require 'ac-haml)
               (let ((upstream
                      (with-temp-buffer
                        (insert
                         "%html\n  %head\n    %title This Title\n  %body\n    %div")
                        (goto-char 28)
                        (ac-haml-current-tag))))
                 (list
                  upstream
                  (with-temp-buffer
                    (insert ".card")
                    (goto-char
                     (point-max))
                    (ac-haml-current-tag))
                  (with-temp-buffer
                    (insert "  %article.card")
                    (goto-char
                     (point-max))
                    (ac-haml-current-tag))
                  (with-temp-buffer
                    (insert "#hero")
                    (goto-char
                     (point-max))
                    (ac-haml-current-tag)))))"##;
    let expect = expect![[r#"OK ("title" "div" "article" "div")"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_haml_attr_prefix_returns_only_colon_prefixed_token_contents() {
    let elisp_form = r##"(progn
               (require 'ac-haml)
               (mapcar
                (lambda (text)
                  (with-temp-buffer
                    (insert text)
                    (goto-char
                     (point-max))
                    (list
                     (ac-haml-attr-prefix)
                     (point))))
                '("%a :hr"
                  "%a :href :ti"
                  "%a plain"
                  "- ruby :attr"
                  "ruby:\n  :attr"
                  "%div{:key => \"value\"}")))"##;
    let expect = expect!["OK ((5 7) (11 13) (nil 9) (nil 13) (nil 14) (nil 22))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_haml_attr_value_prefix_handles_hash_rocket_and_space_delimited_values() {
    let elisp_form = r##"(progn
               (require 'ac-haml)
               (mapcar
                (lambda (text)
                  (with-temp-buffer
                    (insert text)
                    (goto-char
                     (point-max))
                    (list
                     (ac-haml-attrv-prefix)
                     (point))))
                '("%a href=\""
                  "%a href=\"path"
                  "%a class=\"one two"
                  "%a href => \"path with spa"
                  "%a href='single"
                  "%a no-value")))"##;
    let expect = expect!["OK ((10 7) (10 7) (15 8) (23 7) (nil 16) (nil 12))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_haml_class_and_id_prefixes_share_slim_rules_and_code_suppression() {
    let elisp_form = r##"(progn
               (require 'ac-haml)
               (mapcar
                (lambda (text)
                  (with-temp-buffer
                    (insert text)
                    (goto-char
                     (point-max))
                    (list
                     (ac-haml-class-prefix)
                     (ac-haml-id-prefix)
                     (point))))
                '("%div.card.bu"
                  "%div.card#he"
                  ".bu"
                  "#he"
                  "- %div.card"
                  "ruby:\n  %div.card")))"##;
    let expect =
        expect!["OK ((nil nil 13) (nil nil 13) (2 nil 4) (nil 2 4) (nil nil 12) (nil nil 18))"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_haml_current_attr_and_setup_select_exact_callbacks() {
    let elisp_form = r##"(progn
               (require 'ac-haml)
               (with-temp-buffer
                 (insert
                  "%input type=\"text\" aria-label=\"name")
                 (goto-char
                  (point-max))
                 (let ((before
                        (point))
                       (attribute
                        (ac-haml-current-attr)))
                   (list
                    attribute
                    (point)
                    before
                    (ac-haml-setup)
                    ac-html-current-tag-function
                    ac-html-current-attr-function))))"##;
    let expect = expect![[
        r#"OK ("aria-label" 36 36 ac-haml-current-attr ac-haml-current-tag ac-haml-current-attr)"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}
