/// Batch 542: misc edge cases - %S on various objects, format on circular, etc.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx542_format_S_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (format "%S" 'hello) (format "%S" 42) (format "%S" "hello"))
"##,
        expect_test::expect![[r#""OK (\"hello\" \"42\" \"\\\"hello\\\"\")""#]],
    );
}

#[test]
fn div_cx542_format_S_nil_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (format "%S" nil) (format "%S" t))
"##,
        expect_test::expect![[r#""OK (\"nil\" \"t\")""#]],
    );
}

#[test]
fn div_cx542_format_S_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (format "%S" '(a b c)) (format "%S" '(a . b)))
"##,
        expect_test::expect![[r#""OK (\"(a b c)\" \"(a . b)\")""#]],
    );
}

#[test]
fn div_cx542_format_S_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (format "%S" [1 2 3]) (format "%S" [:a :b]))
"##,
        expect_test::expect![[r#""OK (\"[1 2 3]\" \"[:a :b]\")""#]],
    );
}

#[test]
fn div_cx542_format_S_hash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((ht (make-hash-table)))
  (puthash 'a 1 ht)
  (format "%S" ht))
"##,
        expect_test::expect![[r##""OK \"#s(hash-table data (a 1))\"""##]],
    );
}

#[test]
fn div_cx542_format_S_bool_vec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(format "%S" (bool-vector t nil t))
"##,
        expect_test::expect![[r##""OK \"#&3\\\"\u{5}\\\"\"""##]],
    );
}

#[test]
fn div_cx542_format_S_char_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((ct (make-char-table 'syntax-table ?w)))
  (format "%S" ct))
"##,
        expect_test::expect![[
            r##""OK \"#^[119 nil syntax-table 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119 119]\"""##
        ]],
    );
}

#[test]
fn div_cx542_format_percent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(format "100%% complete")
"##,
        expect_test::expect![[r#""OK \"100% complete\"""#]],
    );
}

#[test]
fn div_cx542_format_multiline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(format "line1\nline2\nline3")
"##,
        expect_test::expect![[r#""OK \"line1\nline2\nline3\"""#]],
    );
}

#[test]
fn div_cx542_format_unicode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(format "cafe\u00e9 world")
"##,
        expect_test::expect![[r#""OK \"cafeé world\"""#]],
    );
}

#[test]
fn div_cx542_propertize_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(propertize "hello" 'face 'bold)
"##,
        expect_test::expect![[r#""OK #(\"hello\" 0 5 (face bold))""#]],
    );
}

#[test]
fn div_cx542_propertize_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(propertize "hello" 'face 'bold 'mouse-face 'highlight 'help-echo "help")
"##,
        expect_test::expect![[
            r#""OK #(\"hello\" 0 5 (face bold mouse-face highlight help-echo \"help\"))""#
        ]],
    );
}

#[test]
fn div_cx542_propertize_read_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let* ((s (propertize "text" 'face 'bold))
       (p (prin1-to-string s))
       (r (car (read-from-string p))))
  (list (equal s r) (text-properties-at 0 r)))
"##,
        expect_test::expect![[r#""OK (t (face bold))""#]],
    );
}

#[test]
fn div_cx542_format_S_obarray_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (obarray-default)))
  (format "%S" obs))
"##,
        expect_test::expect![[r#""ERR (void-function obarray-default)""#]],
    );
}

#[test]
fn div_cx542_format_S_window_buffer_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(format "%S" (current-buffer))
"##,
        expect_test::expect![[r##""OK \"#<buffer  *neovm-oracle-stdout*>\"""##]],
    );
}
