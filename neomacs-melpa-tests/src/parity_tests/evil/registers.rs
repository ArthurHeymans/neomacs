use expect_test::expect;

use super::{ParityBatchCase, assert_evil_batch};

fn evil_named_registers_replace_lowercase_and_append_uppercase_values() -> ParityBatchCase {
    ParityBatchCase::new(
        "evil_named_registers_replace_lowercase_and_append_uppercase_values",
        r##"(let ((register-alist nil))
               (evil-set-register ?a "alpha")
               (evil-set-register ?A "-beta")
               (evil-set-register ?b [98 121 116 101])
               (evil-set-register ?B [115])
               (list
                (evil-get-register ?a)
                (evil-get-register ?A)
                (evil-get-register ?b)
                (evil-get-register ?B)
                register-alist))"##,
        true,
        expect![[
            r#"OK ("alpha-beta" "alpha-beta" #1=[98 121 116 101 115] #1# ((98 . #1#) (97 . "alpha-beta")))"#
        ]],
    )
}

fn evil_special_registers_cover_unnamed_numbered_small_delete_and_black_hole() -> ParityBatchCase {
    ParityBatchCase::new(
        "evil_special_registers_cover_unnamed_numbered_small_delete_and_black_hole",
        r##"(let ((kill-ring nil)
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
                evil-last-small-deletion))"##,
        true,
        expect![[r#"OK ("number-one" "number-one" "small" "" ("number-one") "small")"#]],
    )
}

fn evil_get_register_noerror_distinguishes_empty_from_black_hole() -> ParityBatchCase {
    ParityBatchCase::new(
        "evil_get_register_noerror_distinguishes_empty_from_black_hole",
        r##"(let ((register-alist nil)
                    (kill-ring nil)
                    (evil-last-small-deletion nil))
               (list
                (evil-get-register ?q t)
                (evil-get-register ?_ t)
                (evil-get-register ?- t)))"##,
        true,
        expect![[r#"OK (nil "" nil)"#]],
    )
}

fn evil_register_list_filters_names_and_sorts_character_registers() -> ParityBatchCase {
    ParityBatchCase::new(
        "evil_register_list_filters_names_and_sorts_character_registers",
        r##"(let ((register-alist
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
                (evil-register-list)))"##,
        true,
        expect![[r#"OK ((46 . "inserted") (61 . "1+1") (97 . "alpha") (122 . "zed"))"#]],
    )
}

fn evil_set_register_rejects_read_only_and_non_character_registers() -> ParityBatchCase {
    ParityBatchCase::new(
        "evil_set_register_rejects_read_only_and_non_character_registers",
        r##"(evil-set-register ?: "forbidden")"##,
        false,
        expect![[r#"ERR (user-error "Can’t modify read-only register")"#]],
    )
}

fn evil_get_register_rejects_an_empty_register_without_noerror() -> ParityBatchCase {
    ParityBatchCase::new(
        "evil_get_register_rejects_an_empty_register_without_noerror",
        r##"(let ((register-alist nil))
               (evil-get-register ?q))"##,
        false,
        expect![[r#"ERR (user-error "Register ‘q’ is empty")"#]],
    )
}

#[test]
fn registers_public_surface_batch() {
    let cases: Vec<ParityBatchCase> = vec![
        evil_named_registers_replace_lowercase_and_append_uppercase_values(),
        evil_special_registers_cover_unnamed_numbered_small_delete_and_black_hole(),
        evil_get_register_noerror_distinguishes_empty_from_black_hole(),
        evil_register_list_filters_names_and_sorts_character_registers(),
        evil_set_register_rejects_read_only_and_non_character_registers(),
        evil_get_register_rejects_an_empty_register_without_noerror(),
    ];
    assert_evil_batch(&cases);
}
