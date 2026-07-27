use expect_test::expect;

use super::assert_asn1_mode_parity;

#[test]
fn abbrev_generation_covers_every_keyword_with_stable_unique_shortcuts() {
    let elisp_form = r##"(progn
          (clear-abbrev-table asn1-mode-abbrev-table)
          (asn1-mode-abbrev-table)
          (let (pairs)
            (mapatoms
             (lambda (symbol)
               (let ((expansion
                      (abbrev-expansion
                       (symbol-name symbol)
                       asn1-mode-abbrev-table)))
                 (when expansion
                   (push (cons (symbol-name symbol) expansion)
                         pairs))))
             asn1-mode-abbrev-table)
            (setq pairs
                  (sort pairs
                        (lambda (left right)
                          (string< (car left) (car right)))))
            (list
             (length pairs)
             (seq-take pairs 12)
             (seq-take (reverse pairs) 12)
             (= (length pairs)
                (length
                 (delete-dups
                  (mapcar #'car (copy-sequence pairs))))))))"##;
    let expect = expect![[
        r#"OK (94 (("a" . "ALL") ("ab" . "ABSENT") ("ap" . "APPLICATION") ("as" . "ABSTRACT-SYNTAX") ("au" . "AUTOMATIC") ("b" . "BY") ("be" . "BEGIN") ("bi" . "BIT") ("bm" . "BMPString") ("bo" . "BOOLEAN") ("c" . "CLASS") ("ch" . "CHOICE")) (("ws" . "WITH SYNTAX") ("w" . "WITH") ("vi" . "VideotexString") ("v" . "VisibleString") ("utf" . "UTF8String") ("ut" . "UTCTime") ("univ" . "UniversalString") ("uni" . "UNIVERSAL") ("un" . "UNIQUE") ("u" . "UNION") ("tr" . "TRUE") ("tod" . "TIME-OF-DAY")) t)"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn representative_abbreviations_resolve_collisions_and_expand_to_exact_keywords() {
    let elisp_form = r##"(progn
          (asn1-mode-abbrev-table)
          (let (pairs)
            (mapatoms
             (lambda (symbol)
               (let ((expansion
                      (abbrev-expansion
                       (symbol-name symbol)
                       asn1-mode-abbrev-table)))
                 (when expansion
                   (push (cons expansion (symbol-name symbol))
                         pairs))))
             asn1-mode-abbrev-table)
            (mapcar
             (lambda (keyword)
               (cons keyword (cdr (assoc keyword pairs))))
             '("BEGIN" "BIT" "BMPString" "OBJECT"
               "OBJECT IDENTIFIER" "OCTET" "OID-IRI"
               "RELATIVE-OID" "WITH" "WITH SYNTAX"))))"##;
    let expect = expect![[
        r#"OK (("BEGIN" . "be") ("BIT" . "bi") ("BMPString" . "bm") ("OBJECT" . "ob") ("OBJECT IDENTIFIER" . "oid") ("OCTET" . "oc") ("OID-IRI" . "oi") ("RELATIVE-OID" . "ro") ("WITH" . "w") ("WITH SYNTAX" . "ws"))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn generated_abbreviation_expands_in_a_real_asn1_editing_buffer() {
    let elisp_form = r##"(progn
          (asn1-mode-abbrev-table)
          (let (shortcut)
            (mapatoms
             (lambda (symbol)
               (when (equal
                      (abbrev-expansion
                       (symbol-name symbol)
                       asn1-mode-abbrev-table)
                      "OBJECT IDENTIFIER")
                 (setq shortcut (symbol-name symbol))))
             asn1-mode-abbrev-table)
            (with-temp-buffer
              (asn1-mode)
              (abbrev-mode 1)
              (insert shortcut)
              (let ((expanded (expand-abbrev)))
                (list shortcut expanded
                      (buffer-string)
                      last-abbrev-text
                      last-abbrev-location)))))"##;
    let expect = expect![[r#"OK ("oid" oid "OBJECT IDENTIFIER" "oid" 1)"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn repeated_abbrev_generation_accumulates_new_collision_shortcuts_like_upstream() {
    let elisp_form = r##"(let (snapshots)
          (dotimes (_ 3)
            (asn1-mode-abbrev-table)
            (let (pairs)
              (mapatoms
               (lambda (symbol)
                 (let ((expansion
                        (abbrev-expansion
                         (symbol-name symbol)
                         asn1-mode-abbrev-table)))
                   (when expansion
                     (push (cons (symbol-name symbol) expansion)
                           pairs))))
               asn1-mode-abbrev-table)
              (push (sort pairs
                          (lambda (left right)
                            (string< (car left) (car right))))
                    snapshots)))
          (list
           (mapcar #'length snapshots)
           (and (equal (nth 0 snapshots) (nth 1 snapshots))
                (equal (nth 1 snapshots) (nth 2 snapshots)))))"##;
    let expect = expect!["OK ((272 186 94) nil)"];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn outline_level_maps_real_section_headers_to_dot_depth_plus_one() {
    let elisp_form = r##"(with-temp-buffer
          (insert "-- 1 Module\n")
          (insert "-- 1.2 Types\n")
          (insert "-- 1.2.3 Values\n")
          (insert "-- 10.4.2.9 Deep\n")
          (asn1-mode)
          (goto-char (point-min))
          (let (levels)
            (while (re-search-forward outline-regexp nil t)
              (push
               (list
                (match-string-no-properties 0)
                (funcall outline-level))
               levels))
            (nreverse levels)))"##;
    let expect = expect![[r#"OK (("-- 1 " 1) ("-- 1.2 " 2) ("-- 1.2.3 " 3) ("-- 10.4.2.9 " 4))"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn debug_logger_appends_formatted_messages_to_the_trace_buffer_in_order() {
    let elisp_form = r##"(progn
          (when (get-buffer "*trace-output*")
            (kill-buffer "*trace-output*"))
          (asn1-mode-debug "token=%s point=%d" "BEGIN" 17)
          (asn1-mode-debug "rule=%S" '(:before . "END"))
          (with-current-buffer "*trace-output*"
            (list
             (buffer-string)
             (line-number-at-pos (point-max))
             buffer-read-only)))"##;
    let expect = expect![[r#"OK ("token=BEGIN point=17\nrule=(:before . \"END\")\n" 3 nil)"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn debug_toggle_traces_and_untraces_the_smie_rule_with_exact_messages() {
    let elisp_form = r##"(let ((asn1-mode-debug nil)
              calls)
          (cl-letf (((symbol-function 'trace-function)
                     (lambda (symbol)
                       (push (list :trace symbol) calls)))
                    ((symbol-function 'untrace-function)
                     (lambda (symbol)
                       (push (list :untrace symbol) calls)))
                    ((symbol-function 'message)
                     (lambda (format-string &rest arguments)
                       (push
                        (list :message
                              (apply #'format format-string arguments))
                        calls))))
            (asn1-mode-toggle-debug)
            (let ((first-state asn1-mode-debug))
              (asn1-mode-toggle-debug)
              (list first-state asn1-mode-debug
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (t nil ((:message "asn1-mode-debug is t") (:trace asn1-mode-smie-rules) (:message "asn1-mode-debug is nil") (:untrace asn1-mode-smie-rules)))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn common_setup_is_buffer_local_and_does_not_leak_editor_state() {
    let elisp_form = r##"(let ((first (generate-new-buffer " *asn1-first*"))
              (second (generate-new-buffer " *asn1-second*")))
          (unwind-protect
              (progn
                (with-current-buffer first
                  (asn1-mode)
                  (setq-local tab-width 9)
                  (setq-local comment-start ";;"))
                (with-current-buffer second
                  (gdmo-mode))
                (list
                 (with-current-buffer first
                   (list major-mode tab-width comment-start
                         smie-forward-token-function))
                 (with-current-buffer second
                   (list major-mode tab-width comment-start
                         smie-forward-token-function))
                 (default-value 'tab-width)))
            (kill-buffer first)
            (kill-buffer second)))"##;
    let expect = expect![[
        r#"OK ((asn1-mode 9 ";;" asn1-mode-forward-token) (gdmo-mode 4 "--" asn1-mode-forward-token) 8)"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn smie_rule_handles_context_free_list_element_xml_and_end_cases() {
    let elisp_form = r##"(let ((smie-indent-basic 4))
          (list
           (asn1-mode-smie-rules :list-intro "anything")
           (asn1-mode-smie-rules :elem "anything")
           (asn1-mode-smie-rules :after "_XML_OPENER")
           (asn1-mode-smie-rules :before "END")
           (asn1-mode-smie-rules :after ";")))"##;
    let expect = expect!["OK (t 0 4 nil nil)"];
    assert_asn1_mode_parity(elisp_form, expect);
}
