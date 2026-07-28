use expect_test::expect;

use super::assert_auto_complete_nxml_parity;

#[test]
fn auto_complete_nxml_expand_tag_adds_attribute_space_for_open_element() {
    let elisp_form = r##"(with-temp-buffer
         (insert "<table")
         (cl-letf (((symbol-function 'rng-qname-p)
                    (lambda (name) (equal name "table")))
                   ((symbol-function 'rng-expand-qname)
                    (lambda (&rest _args) '(html-ns . "table")))
                   ((symbol-function 'rng-match-start-tag-open)
                    (lambda (_qname) t))
                   ((symbol-function 'rng-match-start-tag-close)
                    (lambda () nil)))
           (let ((rng-open-elements '(root)))
             (auto-complete-nxml-expand-tag)
             (list (buffer-string) (point)))))"##;
    let expect = expect![[r#"OK ("<table " 8)"#]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}

#[test]
fn auto_complete_nxml_expand_tag_handles_extra_strings_and_invalid_names() {
    let elisp_form = r##"(cl-letf (((symbol-function 'rng-qname-p)
                                    (lambda (_name) nil)))
         (mapcar
          (lambda (case)
            (with-temp-buffer
              (insert (car case))
              (setq rng-complete-extra-strings (cdr case))
              (auto-complete-nxml-expand-tag)
              (buffer-string)))
          '(("<!--" "<!--" "<![CDATA[")
            ("<unknown" "<!--" "<![CDATA["))))"##;
    let expect = expect![[r#"OK ("<!--" "<unknown")"#]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}

#[test]
fn auto_complete_nxml_expand_tag_respects_closed_schema_match_and_root_state() {
    let elisp_form = r##"(cl-letf (((symbol-function 'rng-qname-p)
                                    (lambda (_name) t))
                   ((symbol-function 'rng-expand-qname)
                    (lambda (&rest _args) '(ns . "root")))
                   ((symbol-function 'rng-match-start-tag-open)
                    (lambda (_qname) t)))
         (mapcar
          (lambda (case)
            (with-temp-buffer
              (insert "<root")
              (let ((rng-open-elements (car case)))
                (cl-letf (((symbol-function 'rng-match-start-tag-close)
                           (lambda () (cdr case))))
                  (auto-complete-nxml-expand-tag)
                  (list case (buffer-string))))))
          '(((parent) . t)
            ((parent) . nil)
            (nil . t))))"##;
    let expect =
        expect![[r#"OK ((((parent) . t) "<root") (((parent)) "<root ") ((nil . t) "<root "))"#]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}

#[test]
fn auto_complete_nxml_expand_xmlns_emits_all_nondefault_prefixed_namespaces() {
    let elisp_form = r##"(with-temp-buffer
         (insert "<root xmlns=\"urn:default")
         (let ((indent-tabs-mode nil))
           (cl-letf (((symbol-function 'rng-match-possible-namespace-uris)
                      (lambda () '(default-ns math-ns svg-ns unused-ns)))
                     ((symbol-function 'nxml-namespace-name)
                      (lambda (symbol)
                        (cdr (assq symbol
                                    '((default-ns . "urn:default")
                                      (math-ns . "urn:math")
                                      (svg-ns . "urn:svg")
                                      (unused-ns . "urn:unused"))))))
                     ((symbol-function 'auto-complete-nxml-get-prefix)
                      (lambda (namespace)
                        (cdr (assoc namespace
                                    '(("urn:math" . "m")
                                      ("urn:svg" . "svg")
                                      ("urn:unused" . "")))))))
             (auto-complete-nxml-expand-other-xmlns)
             (list (buffer-string) (point)))))"##;
    let expect = expect![[
        r#"OK ("<root xmlns=\"urn:default\" xmlns:m=\"urn:math\" xmlns:svg=\"urn:svg\"" 65)"#
    ]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}

#[test]
fn auto_complete_nxml_get_prefix_walks_schema_location_rules_in_order() {
    let elisp_form = r##"(let ((rng-schema-locating-files
                                '("first.xml" "second.xml" "third.xml")))
         (cl-letf (((symbol-function 'rng-get-parsed-schema-locating-file)
                    (lambda (file)
                      (cdr
                       (assoc
                        file
                        '(("first.xml"
                           (namespace (ns . "urn:other") (typeId . "other"))
                           (documentElement (typeId . "other") (prefix . "o")))
                          ("second.xml"
                           (namespace (ns . "urn:target") (typeId . "target"))
                           (documentElement (typeId . "target") (prefix . "t")))
                          ("third.xml"
                           (namespace (ns . "urn:target") (typeId . "late"))
                           (documentElement (typeId . "late") (prefix . "late")))))))))
           (list
            (auto-complete-nxml-get-prefix "urn:target")
            (auto-complete-nxml-get-prefix "urn:other")
            (auto-complete-nxml-get-prefix "urn:missing"))))"##;
    let expect = expect![[r#"OK ("t" "o" nil)"#]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}

#[test]
fn auto_complete_nxml_tag_source_action_expands_and_closes_end_tags() {
    let elisp_form = r##"(cl-letf (((symbol-function 'auto-complete-nxml-expand-tag)
                                    (lambda () (insert " "))))
         (mapcar
          (lambda (text)
            (with-temp-buffer
              (insert text)
              (funcall (cdr (assq 'action ac-source-nxml-tag)))
              (list text (buffer-string) (point))))
          '("</item" "</item>" "<item")))"##;
    let expect =
        expect![[r#"OK (("</item" "</item " 8) ("</item>" "</item> " 9) ("<item" "<item " 7))"#]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}

#[test]
fn auto_complete_nxml_attribute_source_action_builds_quotes_and_positions_point() {
    let elisp_form = r##"(let ((auto-complete-nxml-automatic-p nil))
         (mapcar
          (lambda (text)
            (with-temp-buffer
              (insert text)
              (funcall (cdr (assq 'action ac-source-nxml-attr)))
              (list text (buffer-string) (point) (char-after))))
          '("<node class" "<node class\"tail")))"##;
    let expect = expect![[
        r#"OK (("<node class" "<node class=\"\"" 14 34) ("<node class\"tail" "<node class\"tail=\"\"" 19 34))"#
    ]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}

#[test]
fn auto_complete_nxml_css_source_actions_chain_property_and_value_editing() {
    let elisp_form = r##"(let ((auto-complete-nxml-automatic-p nil))
         (with-temp-buffer
           (insert "<p style=\"color")
           (funcall (cdr (assq 'action ac-source-nxml-css)))
           (insert "red")
           (funcall (cdr (assq 'action ac-source-nxml-css-property)))
           (list (buffer-string) (point))))"##;
    let expect = expect![[r#"OK ("<p style=\"color: red;" 22)"#]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}

#[test]
fn auto_complete_nxml_tag_value_action_inserts_only_missing_matching_end_tag() {
    let elisp_form = r##"(mapcar
         (lambda (text)
           (with-temp-buffer
             (insert text)
             (funcall (cdr (assq 'action ac-source-nxml-tag-value-by-nxml)))
             (list text (buffer-string) (point))))
         '("<item>choice"
           "<item>choice</item>"
           "<item class=\"x\">choice"
           "plain choice"))"##;
    let expect = expect![[
        r#"OK (("<item>choice" "<item>choice</item>" 20) ("<item>choice</item>" "<item>choice</item><//item>" 28) ("<item class=\"x\">choice" "<item class=\"x\">choice</item>" 30) ("plain choice" "plain choice" 13))"#
    ]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}

#[test]
fn auto_complete_nxml_insert_command_and_toggle_drive_real_command_state() {
    let elisp_form = r##"(let ((auto-complete-nxml-automatic-p t)
             events)
         (cl-letf (((symbol-function 'self-insert-command)
                    (lambda (count)
                      (insert (make-string count ?x))
                      (push (list :insert count) events)))
                   ((symbol-function 'auto-complete-1)
                    (lambda (&rest arguments)
                      (push (cons :complete arguments) events))))
           (with-temp-buffer
             (auto-complete-nxml-ac-start-with-insert 3)
             (auto-complete-nxml-toggle-automatic)
             (let ((disabled-message (current-message)))
               (auto-complete-nxml-ac-start-with-insert 2)
               (auto-complete-nxml-toggle-automatic)
               (list
                (buffer-string)
                auto-complete-nxml-automatic-p
                disabled-message
                (current-message)
                (nreverse events))))))"##;
    let expect = expect![[
        r#"OK ("xxxxx" t nil nil ((:insert 3) (:complete :triggered trigger-key) (:insert 2)))"#
    ]];
    assert_auto_complete_nxml_parity(elisp_form, expect);
}
