use expect_test::expect;

use super::{assert_zero_x_c_parity, assert_zero_x_c_signal_parity};

#[test]
fn zero_x_c_public_defaults_match_the_pinned_release() {
    let elisp_form = r##"(list
               0xc-strict
               0xc-padding
               0xc-clamp-ten
               0xc-clamp-hex
               0xc-max-base
               0xc-default-base
               0xc-extension
               (not
                (null
                 (custom-variable-p
                  '0xc-max-base))))"##;
    let expect = expect![[r#"OK (nil " _,." t t 16 10 ".." t)"#]];

    assert_zero_x_c_parity(elisp_form, expect);
}

#[test]
fn zero_x_c_number_to_string_handles_zero_and_multiple_output_bases() {
    let elisp_form = r##"(let ((0xc-max-base 16))
               (list
                (0xc-number-to-string 0 2)
                (0xc-number-to-string 255 2)
                (0xc-number-to-string 255 8)
                (0xc-number-to-string 255 10)
                (0xc-number-to-string 255 16)))"##;
    let expect = expect![[r#"OK ("" "11111111" "377" "255" "FF")"#]];

    assert_zero_x_c_parity(elisp_form, expect);
}

#[test]
fn zero_x_c_character_conversion_preserves_boundary_quirks() {
    let elisp_form = r##"(let ((0xc-max-base 16))
               (list
                (mapcar
                 (lambda (digit)
                   (0xc--char-to-string digit))
                 '(0 9 10 15))
                (0xc--char-to-string 2 2)
                (0xc--char-to-string 15 16)))"##;
    let expect = expect![[r#"OK (("0" "9" "A" "F") "2" "F")"#]];

    assert_zero_x_c_parity(elisp_form, expect);
}

#[test]
fn zero_x_c_character_conversion_reports_maximum_and_ascii_limits() {
    let elisp_form = r##"(let ((0xc-max-base 16))
               (mapcar
                (lambda (form)
                  (condition-case err
                      (eval form)
                    (error
                     (list
                      (car err)
                      (cdr err)))))
                '((0xc--char-to-string 1 17)
                  (0xc--char-to-string 18 16)
                  (0xc--char-to-string 36))))"##;
    let expect = expect![[
        r#"OK ((error ("That base is larger than the maximum allowed base: 16")) (error ("That character cannot fit in this base")) (error ("That character is too large to represent in ascii")))"#
    ]];

    assert_zero_x_c_parity(elisp_form, expect);
}

#[test]
fn zero_x_c_string_to_number_handles_hints_padding_extensions_and_explicit_base() {
    let elisp_form = r##"(let ((0xc-max-base 36)
                     (0xc-clamp-ten t)
                     (0xc-clamp-hex t)
                     (0xc-padding " _,."))
               (list
                (0xc-string-to-number "0xff")
                (0xc-string-to-number "'b1010")
                (0xc-string-to-number "5:41300")
                (0xc-string-to-number
                 "10100..")
                (0xc-string-to-number
                 "101" 2)
                (0xc-string-to-number
                 "zz" 36)))"##;
    let expect = expect!["OK (255 10 2700 160 5 1295)"];

    assert_zero_x_c_parity(elisp_form, expect);
}

#[test]
fn zero_x_c_padding_is_validated_before_it_is_stripped_from_base_inference() {
    let elisp_form = r##"(let ((0xc-max-base 36)
                     (0xc-padding " _,."))
               (0xc-string-to-number
                "1_000_000"))"##;
    let expect = expect![[r#"ERR (error "Number exceeds maximum allowed base: 36")"#]];

    assert_zero_x_c_signal_parity(elisp_form, expect);
}

#[test]
fn zero_x_c_string_to_number_rejects_non_numbers() {
    let elisp_form = r##"(0xc-string-to-number "12$34")"##;
    let expect = expect![[r#"ERR (error "Not a number")"#]];

    assert_zero_x_c_signal_parity(elisp_form, expect);
}

#[test]
fn zero_x_c_digit_and_reverse_helpers_cover_all_ascii_digits() {
    let elisp_form = r##"(let ((0xc-max-base 36))
               (list
                (mapcar
                 (lambda (char)
                   (0xc--digit-value
                    (char-to-string char)))
                 (append
                  (number-sequence ?0 ?9)
                  (number-sequence ?a ?z)
                  (number-sequence ?A ?Z)))
                (0xc--reverse-string
                 "0x10ff")
                (0xc--reverse-string
                 "racecar")
                (0xc--reverse-string "")))"##;
    let expect = expect![[
        r#"OK ((0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35) "ff01x0" "racecar" "")"#
    ]];

    assert_zero_x_c_parity(elisp_form, expect);
}

#[test]
fn zero_x_c_recursive_numeric_helper_uses_reversed_input_order() {
    let elisp_form = r##"(list
               (0xc--string-to-number "" 10)
               (0xc--string-to-number "321" 10)
               (0xc--string-to-number "FF" 16)
               (0xc--string-to-number "101" 2))"##;
    let expect = expect!["OK (0 123 255 5)"];

    assert_zero_x_c_parity(elisp_form, expect);
}
