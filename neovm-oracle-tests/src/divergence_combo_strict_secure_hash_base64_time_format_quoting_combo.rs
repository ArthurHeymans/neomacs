//! Strict combo oracle probes, batch 146: deterministic crypto/encoding/time
//! surfaces where any divergence is a real bug. secure-hash md5/sha1/sha224/
//! sha384/sha512, base64 encode/decode round-trips incl. multibyte + binary,
//! format-time-string over a FIXED decoded time with explicit TZ rule,
//! number-sequence edge cases (float step, negative, singleton), and string
//! quoting helpers (split-string OMIT-NULLS, combine-and-quote-strings,
//! shell-quote-argument).
//! Uses assert_oracle_parity_expect format.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_secure_hash_algorithm_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(let ((s "The quick brown fox jumps over the lazy dog"))
  (list (secure-hash 'md5 s)
        (secure-hash 'sha1 s)
        (secure-hash 'sha224 s)
        (secure-hash 'sha256 s)
        (secure-hash 'sha384 s)
        (secure-hash 'sha512 s)
        (secure-hash 'sha1 "")
        (secure-hash 'sha1 "abc" nil nil 'binary)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_base64_roundtrip_multibyte_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(let ((samples '("" "f" "fo" "foo" "foob" "fooba" "foobar"
                 "héllo wörld" "日本語のテスト" "a\\u0000b")))
  (append
   (mapcar (lambda (s) (base64-encode-string s)) samples)
   (mapcar (lambda (s) (base64-decode-string (base64-encode-string s))) samples)
   (list (base64-encode-string "foo" t)
         (length (base64-encode-string "foo"))
         (base64-decode-string "Zm9vYmFy"))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_format_time_string_fixed_time_explicit_tz() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(let ((tz-saved (getenv "TZ")))
  (unwind-protect
      (progn
        (set-time-zone-rule "UTC0")
        (let ((fixed (encode-time 30 45 12 15 3 2025 nil -1 nil)))
          (list (format-time-string "%Y-%m-%d %H:%M:%S" fixed)
                (format-time-string "%a %b %d %I:%M:%S %p %Z" fixed)
                (format-time-string "%j %U %V %w %u" fixed)
                (format-time-string "%s" fixed)
                (format-time-string "%FT%T%z" fixed)
                (decode-time fixed)
                (float-time fixed)
                ;; 1-second-before-epoch-boundary day math
                (format-time-string "%Y-%m-%d" (encode-time 0 0 0 1 1 1970 nil -1 nil)))))
    (set-time-zone-rule tz-saved)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_number_sequence_edge_float_negative_singleton() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(list (number-sequence 1 5)
      (number-sequence 1 10 2)
      (number-sequence 10 1 -3)
      (number-sequence 0 1 0.25)
      (number-sequence 5 5)
      (number-sequence 5 5 2)
      (number-sequence 1 0)
      (length (number-sequence 1 1000))
      (mapcar #'identity (number-sequence 0.0 1.0 0.5))
      (number-sequence -2 2))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_string_quoting_split_combine_shell() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(list (split-string "  a  b   c  ")
      (split-string "  a  b   c  " " " t)
      (split-string "a,,b,,c" ",")
      (split-string "a,,b,,c" "," t)
      (split-string "one two\tthree\nfour" "[ \t\n]+")
      (combine-and-quote-strings '("a" "b c" "d\"e"))
      (combine-and-quote-strings '("simple" "with space" "quote\"") " ")
      (shell-quote-argument "simple")
      (shell-quote-argument "with $shell; injection")
      (shell-quote-argument "a'b`c"))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
