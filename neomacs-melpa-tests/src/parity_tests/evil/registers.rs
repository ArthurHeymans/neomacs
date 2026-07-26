use expect_test::expect;

use super::{assert_evil_parity, assert_evil_signal_parity};

#[test]
fn evil_named_registers_replace_lowercase_and_append_uppercase_values() {
    let elisp_form = r##"(let ((register-alist nil))
               (evil-set-register ?a "alpha")
               (evil-set-register ?A "-beta")
               (evil-set-register ?b [98 121 116 101])
               (evil-set-register ?B [115])
               (list
                (evil-get-register ?a)
                (evil-get-register ?A)
                (evil-get-register ?b)
                (evil-get-register ?B)
                register-alist))"##;
    let expect = expect![[
        r#"OK ("alpha-beta" "alpha-beta" #1=[98 121 116 101 115] #1# ((98 . #1#) (97 . "alpha-beta")))"#
    ]];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_special_registers_cover_unnamed_numbered_small_delete_and_black_hole() {
    let elisp_form = r##"(let ((kill-ring nil)
                    (kill-ring-yank-pointer nil)
                    (evil-last-small-deletion nil))
               (evil-set-register ?\" "unnamed")
               (evil-set-register ?- "small")
               (evil-set-register ?_ "discarded")
               (evil-set-register ?1 "number-one")
               (list
                (evil-get-register ?\")
                (evil-get-register ?1)
                (evil-get-register ?-)
                (evil-get-register ?_)
                kill-ring
                evil-last-small-deletion))"##;
    let expect = expect![[r#"OK ("number-one" "number-one" "small" "" ("number-one") "small")"#]];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_get_register_noerror_distinguishes_empty_from_black_hole() {
    let elisp_form = r##"(let ((register-alist nil)
                    (kill-ring nil)
                    (evil-last-small-deletion nil))
               (list
                (evil-get-register ?q t)
                (evil-get-register ?_ t)
                (evil-get-register ?- t)))"##;
    let expect = expect![[r#"OK (nil "" nil)"#]];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_register_list_filters_names_and_sorts_character_registers() {
    let elisp_form = r##"(let ((register-alist
                     '((122 . "zed")
                       (97 . "alpha")
                       (symbolic . "ignored")
                       ("string" . "ignored")))
                    (kill-ring nil)
                    (evil-last-small-deletion nil)
                    (evil-last-insertion "inserted")
                    (evil-last-=-register-input "1+1")
                    (evil-ex-history nil)
                    (evil-ex-search-history nil))
               (cl-remove-if
                (lambda (entry)
                  (null (cdr entry)))
                (evil-register-list)))"##;
    let expect = expect![[r#"OK ((46 . "inserted") (61 . "1+1") (97 . "alpha") (122 . "zed"))"#]];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_set_register_rejects_read_only_and_non_character_registers() {
    let elisp_form = r##"(evil-set-register ?: "forbidden")"##;
    let expect = expect![[r#"ERR (user-error "Can’t modify read-only register")"#]];

    assert_evil_signal_parity(elisp_form, expect);
}

#[test]
fn evil_get_register_rejects_an_empty_register_without_noerror() {
    let elisp_form = r##"(let ((register-alist nil))
               (evil-get-register ?q))"##;
    let expect = expect![[r#"ERR (user-error "Register ‘q’ is empty")"#]];

    assert_evil_signal_parity(elisp_form, expect);
}
