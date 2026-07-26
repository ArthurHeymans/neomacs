use expect_test::expect;

use super::{assert_academic_phrases_parity, assert_academic_phrases_signal_parity};

#[test]
fn academic_phrases_ht_get_star_walks_nested_tables_and_handles_final_missing_keys() {
    let elisp_form = r##"(let* ((leaf
                     (ht
                      (:value "found")
                      (:nil-value nil)))
                    (middle
                     (ht
                      (:leaf leaf)))
                    (root
                     (ht
                      (:middle middle)
                      (nil "nil-key"))))
               (list
                (academic-phrases--ht-get*
                 root
                 :middle
                 :leaf
                 :value)
                (academic-phrases--ht-get*
                 root
                 :middle
                 :leaf
                 :nil-value)
                (academic-phrases--ht-get*
                 root
                 :middle
                 :leaf
                 :missing)
                (academic-phrases--ht-get*
                 root
                 nil)
                (academic-phrases--ht-get*
                 root)))"##;
    let expect = expect![[r#"OK ("found" nil nil "nil-key" "nil-key")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_ht_get_star_signals_when_an_intermediate_value_is_not_a_table() {
    let elisp_form = r##"(academic-phrases--ht-get*
              (ht
               (:middle "not-a-table"))
              :middle
              :leaf)"##;
    let expect = expect![[r#"ERR (wrong-type-argument hash-table-p "not-a-table")"#]];

    assert_academic_phrases_signal_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_ht_select_keys_preserves_test_values_identity_and_requested_membership() {
    let elisp_form = r##"(let* ((first
                     (list
                      'first))
                    (second
                     (list
                      'second))
                    (table
                     (make-hash-table
                      :test
                      'eq))
                    result)
               (puthash
                :first
                first
                table)
               (puthash
                :second
                second
                table)
               (puthash
                :third
                3
                table)
               (setq
                result
                (academic-phrases--ht-select-keys
                 table
                 '(:second
                   :missing
                   :first
                   :second)))
               (list
                (hash-table-test
                 result)
                (hash-table-count
                 result)
                (eq
                 (gethash
                  :first
                  result)
                 first)
                (eq
                 (gethash
                  :second
                  result)
                 second)
                (gethash
                 :third
                 result
                 'absent)
                (gethash
                 :missing
                 result
                 'absent)
                (hash-table-count
                 table)))"##;
    let expect = expect!["OK (eq 2 t t absent absent 3)"];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_ht_select_keys_omits_a_present_value_equal_to_its_internal_sentinel() {
    let elisp_form = r##"(let ((table
               (ht
                     (:kept 'value)
                     (:sentinel 'key-not-found))))
               (let ((result
                      (academic-phrases--ht-select-keys
                       table
                       '(:sentinel
                         :kept))))
                 (list
                  (hash-table-count
                   result)
                  (gethash
                   :sentinel
                   result
                   'absent)
                  (gethash
                   :kept
                   result
                   'absent)
                  (gethash
                   :sentinel
                   table
                   'absent))))"##;
    let expect = expect!["OK (1 absent value key-not-found)"];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_ht_select_keys_accepts_empty_requests_and_rejects_non_tables() {
    let elisp_form = r##"(let* ((table
                     (ht
                      (:one 1)))
                    (result
                     (academic-phrases--ht-select-keys
                      table
                      nil)))
               (list
                (hash-table-p
                 result)
                (hash-table-test
                 result)
                (hash-table-count
                 result)
                (hash-table-count
                 table)))"##;
    let expect = expect!["OK (t equal 0 1)"];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_ht_select_keys_rejects_non_hash_tables() {
    let elisp_form = r##"(academic-phrases--ht-select-keys
              'not-a-table
              '(:one))"##;
    let expect = expect!["ERR (wrong-type-argument hash-table-p not-a-table)"];

    assert_academic_phrases_signal_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_replace_placeholders_joins_three_choice_groups_and_all_occurrences() {
    let elisp_form = r##"(academic-phrases--replace-placeholders
              "[{1}] then {2}, {1}, and [{3}]"
              '(("alpha"
                 "beta")
                ("middle")
                ("omega"
                 "final")))"##;
    let expect = expect![[r#"OK "[alpha/beta] then middle, alpha/beta, and [omega/final]""#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_replace_placeholders_erases_missing_groups_and_keeps_unused_choices() {
    let elisp_form = r##"(list
               (academic-phrases--replace-placeholders
                "A:{1}|B:{2}|C:{3}"
                '(()
                  nil
                  ()))
               (academic-phrases--replace-placeholders
                "No placeholders"
                '(("unused")
                  ("also unused")
                  ("still unused")))
               (academic-phrases--replace-placeholders
                "{1}-{2}-{3}"
                '(("x"))))"##;
    let expect = expect![[r#"OK ("A:|B:|C:" "No placeholders" "x--")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_replace_placeholders_rejects_non_string_templates() {
    let elisp_form = r##"(academic-phrases--replace-placeholders
              'not-a-string
              '(("choice")))"##;
    let expect = expect!["ERR (wrong-type-argument sequencep not-a-string)"];

    assert_academic_phrases_signal_parity(elisp_form, expect);
}
