use expect_test::expect;

use super::assert_which_key_parity;

#[test]
fn which_key_upstream_sort_orders_match_for_uppercase_and_lowercase_priority() {
    let elisp_form = r##"(let ((keys '(("a" . "z")
                             ("A" . "Z")
                             ("b" . "y")
                             ("B" . "Y")
                             ("p" . "prefix")
                             ("SPC" . "x")
                             ("C-a" . "w"))))
               (list
                (let ((which-key-sort-uppercase-first t))
                  (mapcar #'car
                          (sort (copy-tree keys) #'which-key-key-order)))
                (let (which-key-sort-uppercase-first)
                  (mapcar #'car
                          (sort (copy-tree keys) #'which-key-key-order)))
                (let ((which-key-sort-uppercase-first t))
                  (mapcar #'car
                          (sort (copy-tree keys)
                                #'which-key-key-order-alpha)))
                (let (which-key-sort-uppercase-first)
                  (mapcar #'car
                          (sort (copy-tree keys)
                                #'which-key-key-order-alpha)))))"##;
    let expect = expect![[
        r#"OK (("SPC" "A" "B" "a" "b" "p" "C-a") ("SPC" "a" "b" "p" "A" "B" "C-a") ("SPC" "A" "a" "B" "b" "p" "C-a") ("SPC" "a" "A" "b" "B" "p" "C-a"))"#
    ]];

    assert_which_key_parity(elisp_form, expect);
}

#[test]
fn which_key_upstream_prefix_sort_orders_match_in_both_directions() {
    let elisp_form = r##"(let ((keys '(("a" . "z")
                             ("A" . "Z")
                             ("b" . "y")
                             ("B" . "Y")
                             ("p" . "prefix")
                             ("SPC" . "x")
                             ("C-a" . "w"))))
               (list
                (let ((which-key-sort-uppercase-first t))
                  (mapcar #'car
                          (sort (copy-tree keys)
                                #'which-key-prefix-then-key-order)))
                (let (which-key-sort-uppercase-first)
                  (mapcar #'car
                          (sort (copy-tree keys)
                                #'which-key-prefix-then-key-order)))
                (let ((which-key-sort-uppercase-first t))
                  (mapcar #'car
                          (sort (copy-tree keys)
                                #'which-key-prefix-then-key-order-reverse)))
                (let (which-key-sort-uppercase-first)
                  (mapcar #'car
                          (sort (copy-tree keys)
                                #'which-key-prefix-then-key-order-reverse)))))"##;
    let expect = expect![[
        r#"OK (("SPC" "A" "B" "a" "b" "C-a" "p") ("SPC" "a" "b" "A" "B" "C-a" "p") ("p" "SPC" "A" "B" "a" "b" "C-a") ("p" "SPC" "a" "b" "A" "B" "C-a"))"#
    ]];

    assert_which_key_parity(elisp_form, expect);
}

#[test]
fn which_key_description_order_is_case_insensitive_and_stable_for_ties() {
    let elisp_form = r##"(let ((keys '(("a" . "z")
                             ("A" . "Z")
                             ("b" . "y")
                             ("B" . "Y")
                             ("p" . "prefix")
                             ("SPC" . "x")
                             ("C-a" . "w"))))
               (list
                (let ((which-key-sort-uppercase-first t))
                  (mapcar #'car
                          (sort (copy-tree keys)
                                #'which-key-description-order)))
                (let (which-key-sort-uppercase-first)
                  (mapcar #'car
                          (sort (copy-tree keys)
                                #'which-key-description-order)))))"##;
    let expect =
        expect![[r#"OK (("p" "C-a" "SPC" "b" "B" "a" "A") ("p" "C-a" "SPC" "b" "B" "a" "A"))"#]];

    assert_which_key_parity(elisp_form, expect);
}

#[test]
fn which_key_key_order_handles_empty_ranges_specials_function_keys_and_modifiers() {
    let elisp_form = r##"(let ((keys '(("" . "empty")
                             ("z .. a" . "range")
                             ("TAB" . "tab")
                             ("RET" . "ret")
                             ("a" . "lower")
                             ("A" . "upper")
                             ("<f12>" . "f12")
                             ("<f2>" . "f2")
                             ("M-b" . "meta-b")
                             ("C-a" . "control-a")
                             ("long" . "other"))))
               (list
                (let ((which-key-sort-uppercase-first t))
                  (mapcar #'car
                          (sort (copy-tree keys) #'which-key-key-order)))
                (let (which-key-sort-uppercase-first)
                  (mapcar #'car
                          (sort (copy-tree keys)
                                #'which-key-key-order-alpha)))))"##;
    let expect = expect![[
        r#"OK (("" "RET" "TAB" "A" "a" "z .. a" "<f2>" "<f12>" "C-a" "M-b" "long") ("" "RET" "TAB" "a" "A" "z .. a" "<f2>" "<f12>" "C-a" "M-b" "long"))"#
    ]];

    assert_which_key_parity(elisp_form, expect);
}
