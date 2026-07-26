use expect_test::expect;

use super::{assert_academic_phrases_parity, assert_academic_phrases_signal_parity};

#[test]
fn academic_phrases_prompt_categories_returns_every_fixture_title_without_keys() {
    let elisp_form = r##"(let ((phrases
                    (ht
                     (:alpha
                      (ht
                       (:title "Alpha")
                       (:items nil)))
                     (:beta
                      (ht
                       (:title "Beta")
                       (:items nil)))
                     (:gamma
                      (ht
                       (:title "Gamma")
                       (:items nil))))))
               (list
                (sort
                 (academic-phrases--prompt-categories
                  phrases)
                 #'string<)
                (academic-phrases--prompt-categories
                 (ht))))"##;
    let expect = expect![[r#"OK (("Alpha" "Beta" "Gamma") nil)"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_prompt_items_expands_all_placeholders_and_preserves_item_order_and_ids() {
    let elisp_form = r##"(let* ((first
                     (ht
                      (:id 7)
                      (:template "[{1}] and [{2}] plus [{3}]")
                      (:choices
                       '(("one"
                          "uno")
                         ("two")
                         ("three"
                          "tres")))))
                    (second
                     (ht
                      (:id "string-id")
                      (:template "No replacement")
                      (:choices
                       '(()))))
                    (items
                     (list
                      first
                      second))
                    (phrases
                     (ht
                      (:fixture
                       (ht
                        (:title "Fixture")
                        (:items items))))))
               (list
                (academic-phrases--prompt-items
                 :fixture
                 phrases)
                (eq
                 (academic-phrases--get-items
                  :fixture
                  phrases)
                 items)
                items))"##;
    let expect = expect![[
        r#"OK ((("[one/uno] and [two] plus [three/tres]" . 7) ("No replacement" . "string-id")) t (#s(hash-table test equal data (:id 7 :template "[{1}] and [{2}] plus [{3}]" :choices (("one" "uno") ("two") ("three" "tres")))) #s(hash-table test equal data (:id "string-id" :template "No replacement" :choices (nil)))))"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_get_cat_and_get_items_find_titles_and_return_live_objects() {
    let elisp_form = r##"(let* ((items-a
                     (list
                      'a))
                    (items-b
                     (list
                      'b))
                    (phrases
                     (ht
                      (:alpha
                       (ht
                        (:title "Alpha")
                        (:items items-a)))
                      (:beta
                       (ht
                        (:title "Beta")
                        (:items items-b))))))
               (list
                (academic-phrases--get-cat
                 "Alpha"
                 phrases)
                (academic-phrases--get-cat
                 "Beta"
                 phrases)
                (academic-phrases--get-cat
                 "Missing"
                 phrases)
                (eq
                 (academic-phrases--get-items
                  :alpha
                  phrases)
                 items-a)
                (eq
                 (academic-phrases--get-items
                  :beta
                  phrases)
                 items-b)
                items-a
                items-b))"##;
    let expect = expect!["OK (:alpha :beta nil t t (a) (b))"];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_filter_item_returns_the_first_equal_id_object_and_nil_for_missing() {
    let elisp_form = r##"(let* ((first
                     (ht
                      (:id 7)
                      (:template "first")
                      (:choices
                       '(()))))
                    (duplicate
                     (ht
                      (:id 7)
                      (:template "duplicate")
                      (:choices
                       '(()))))
                    (string-id
                     (ht
                      (:id "7")
                      (:template "string")
                      (:choices
                       '(()))))
                    (phrases
                     (ht
                      (:fixture
                       (ht
                        (:title "Fixture")
                        (:items
                         (list
                          first
                          duplicate
                          string-id)))))))
               (list
                (eq
                 (academic-phrases--filter-item
                  :fixture
                  7
                  phrases)
                 first)
                (eq
                 (academic-phrases--filter-item
                  :fixture
                  "7"
                  phrases)
                 string-id)
                (academic-phrases--filter-item
                 :fixture
                 8
                 phrases)
                first
                duplicate
                string-id))"##;
    let expect = expect![[
        r#"OK (t t nil #s(hash-table test equal data (:id 7 :template "first" :choices (nil))) #s(hash-table test equal data (:id 7 :template "duplicate" :choices (nil))) #s(hash-table test equal data (:id "7" :template "string" :choices (nil))))"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_optional_lookup_arguments_use_the_live_global_table() {
    let elisp_form = r##"(let* ((cat1-items
                     (academic-phrases--get-items
                      :cat1))
                    (cat57-items
                     (academic-phrases--get-items
                      :cat57))
                    (first
                     (academic-phrases--filter-item
                      :cat1
                      1))
                    (last
                     (academic-phrases--filter-item
                      :cat57
                      592)))
               (list
                (academic-phrases--get-cat
                 "Establishing why your topic X is important")
                (academic-phrases--get-cat
                 "Referring outside the paper")
                (length
                 cat1-items)
                (length
                 cat57-items)
                (eq
                 first
                 (car
                  cat1-items))
                (eq
                 last
                 (car
                  (last
                   cat57-items)))
                (ht-get
                 first
                 :template)
                (ht-get
                 last
                 :template)))"##;
    let expect = expect![[
        r#"OK (:cat1 :cat57 12 3 t t "X is the [{1}] cause of ..." "More details on this topic can be found in [Ref].")"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_get_items_signals_for_missing_categories() {
    let elisp_form = r##"(academic-phrases--get-items
              :missing
              (ht
               (:present
                (ht
                 (:title "Present")
                 (:items nil)))))"##;
    let expect = expect!["ERR (wrong-type-argument hash-table-p nil)"];

    assert_academic_phrases_signal_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_category_keyword_generation_covers_single_forward_empty_and_full_ranges() {
    let elisp_form = r##"(list
               (academic-phrases--gen-cats-keywords
                1
                1)
               (academic-phrases--gen-cats-keywords
                3
                7)
               (academic-phrases--gen-cats-keywords
                7
                3)
               (let ((all
                      (academic-phrases--gen-cats-keywords
                       1
                       57)))
                 (list
                  (length
                   all)
                  (car
                   all)
                  (car
                   (last
                    all))
                  (eq
                   (nth
                    28
                    all)
                   :cat29))))"##;
    let expect = expect!["OK ((:cat1) (:cat3 :cat4 :cat5 :cat6 :cat7) nil (57 :cat1 :cat57 t))"];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_category_keyword_generation_signals_for_non_numeric_bounds() {
    let elisp_form = r##"(academic-phrases--gen-cats-keywords
              'one
              3)"##;
    let expect = expect!["ERR (wrong-type-argument number-or-marker-p one)"];

    assert_academic_phrases_signal_parity(elisp_form, expect);
}
