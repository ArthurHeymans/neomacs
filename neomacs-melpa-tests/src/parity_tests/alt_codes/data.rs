use expect_test::expect;

use super::assert_alt_codes_parity;

#[test]
fn alt_codes_table_has_complete_exact_windows_code_corpus() {
    let elisp_form = r##"(list
         (length alt-codes--list)
         (car alt-codes--list)
         (nth 31 alt-codes--list)
         (nth 188 alt-codes--list)
         (nth 254 alt-codes--list)
         (nth 255 alt-codes--list)
         (car (last alt-codes--list))
         (secure-hash
          'sha256
          (prin1-to-string alt-codes--list))
         (= (length alt-codes--list)
            (length
             (delete-dups
              (mapcar #'car
                      (copy-tree alt-codes--list))))))"##;
    let expect = expect![[
        r#"OK (383 ("1" "☺") ("32" "spc") ("189" "") ("255" "spc") ("0128" "€") ("0255" "ÿ") "84d621beb21f5e17bc9fe7b37869e4ea66c78a24be5388bbd62ef783ec871bb3" t)"#
    ]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_lookup_covers_ascii_symbols_extended_glyphs_and_leading_zero_codes() {
    let elisp_form = r##"(mapcar
         (lambda (code)
           (list code
                 (alt-codes--get-symbol code)
                 (string-to-list
                  (alt-codes--get-symbol code))))
         '("1" "32" "33" "65" "90"
           "97" "126" "127" "128"
           "189" "224" "255"
           "0128" "0145" "0153"
           "0160" "0173" "0255"))"##;
    let expect = expect![[
        r#"OK (("1" "☺" (9786)) ("32" "spc" (115 112 99)) ("33" "!" (33)) ("65" "A" (65)) ("90" "Z" (90)) ("97" "a" (97)) ("126" "~" (126)) ("127" "⌂" (8962)) ("128" "Ç" (199)) ("189" "" nil) ("224" "α" (945)) ("255" "spc" (115 112 99)) ("0128" "€" (8364)) ("0145" "‘" (8216)) ("0153" "™" (8482)) ("0160" "spc" (115 112 99)) ("0173" "" nil) ("0255" "ÿ" (255)))"#
    ]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_lookup_returns_nil_for_malformed_or_unknown_codes() {
    let elisp_form = r##"(mapcar
         (lambda (code)
           (list code
                 (alt-codes--get-symbol code)))
         '("" "0" "00" "0001" "256"
           "999" "1280" " 65" "65 "
           "A" "M-kp-1"))"##;
    let expect = expect![[
        r#"OK (("" nil) ("0" nil) ("00" nil) ("0001" nil) ("256" nil) ("999" nil) ("1280" nil) (" 65" nil) ("65 " nil) ("A" nil) ("M-kp-1" nil))"#
    ]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_lookup_matches_declared_entries_across_every_table_region() {
    let elisp_form = r##"(mapcar
         (lambda (index)
           (let ((entry
                  (nth index alt-codes--list)))
             (list
              index
              entry
              (alt-codes--get-symbol
               (car entry))
              (equal
               (cadr entry)
               (alt-codes--get-symbol
                (car entry))))))
         '(0 127 188 254 255 382))"##;
    let expect = expect![[
        r#"OK ((0 ("1" "☺") "☺" t) (127 ("128" "Ç") "Ç" t) (188 ("189" "") "" t) (254 ("255" "spc") "spc" t) (255 ("0128" "€") "€" t) (382 ("0255" "ÿ") "ÿ" t))"#
    ]];
    assert_alt_codes_parity(elisp_form, expect);
}
