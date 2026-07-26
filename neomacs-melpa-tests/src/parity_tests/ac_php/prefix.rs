use expect_test::expect;

use super::assert_ac_php_parity;

#[test]
fn ac_php_prefix_finds_php_identifiers_variables_and_qualified_names() {
    let elisp_form = r##"(mapcar
               (lambda (text)
                 (with-temp-buffer
                   (insert text)
                   (let ((before
                          (point))
                         (prefix
                          (ac-php-prefix)))
                     (list
                      text
                      before
                      prefix
                      (point)
                      (buffer-substring-no-properties
                       prefix
                       before)))))
               '("plain"
                 "$variable_2"
                 "\\Acme\\Service"
                 "prefix value"
                 "123name"
                 "$café"))"##;
    let expect = expect![[
        r#"OK (("plain" 6 1 6 "plain") ("$variable_2" 12 1 12 "$variable_2") ("\\Acme\\Service" 14 1 14 "\\Acme\\Service") ("prefix value" 13 8 13 "value") ("123name" 8 1 8 "123name") ("$café" 6 6 6 ""))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_prefix_starts_after_member_and_static_access_operators() {
    let elisp_form = r##"(mapcar
               (lambda (text)
                 (with-temp-buffer
                   (insert text)
                   (let ((before
                          (point))
                         (prefix
                          (ac-php-prefix)))
                     (list
                      text
                      before
                      prefix
                      (point)))))
               '("$object->"
                 "ClassName::"
                 "->"
                 "::"))"##;
    let expect = expect![[
        r#"OK (("$object->" 10 10 10) ("ClassName::" 12 12 12) ("->" 3 3 3) ("::" 3 3 3))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_prefix_handles_empty_punctuation_and_line_boundaries_without_moving_point() {
    let elisp_form = r##"(mapcar
               (lambda (text)
                 (with-temp-buffer
                   (insert text)
                   (let ((before
                          (point))
                         (prefix
                          (ac-php-prefix)))
                     (list
                      text
                      before
                      prefix
                      (point)
                      (and
                       (<= prefix before)
                       (buffer-substring-no-properties
                        prefix
                        before))))))
               '(""
                 "!"
                 "alpha beta!"
                 "old_name\n$new_name"
                 "a-b"
                 "value>"
                 "value:"))"##;
    let expect = expect![[
        r#"OK (("" 1 1 1 "") ("!" 2 2 2 "") ("alpha beta!" 12 12 12 "") ("old_name\n$new_name" 19 10 19 "$new_name") ("a-b" 4 3 4 "b") ("value>" 7 7 7 "") ("value:" 7 7 7 ""))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_prefix_uses_text_immediately_before_an_interior_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "before $target after")
               (search-backward
                " after")
               (let ((before
                      (point))
                     (prefix
                      (ac-php-prefix)))
                 (list
                  before
                  prefix
                  (point)
                  (buffer-substring-no-properties
                   prefix
                   before)
                  (buffer-string))))"##;
    let expect = expect![[r#"OK (15 8 15 "$target" "before $target after")"#]];

    assert_ac_php_parity(elisp_form, expect);
}
