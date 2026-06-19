/// Batch 542: misc edge cases - %S on various objects, format on circular, etc.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx542_format_S_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%S" 'hello) (format "%S" 42) (format "%S" "hello"))
"##,
    );
}

#[test]
fn div_cx542_format_S_nil_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%S" nil) (format "%S" t))
"##,
    );
}

#[test]
fn div_cx542_format_S_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%S" '(a b c)) (format "%S" '(a . b)))
"##,
    );
}

#[test]
fn div_cx542_format_S_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%S" [1 2 3]) (format "%S" [:a :b]))
"##,
    );
}

#[test]
fn div_cx542_format_S_hash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ht (make-hash-table)))
  (puthash 'a 1 ht)
  (format "%S" ht))
"##,
    );
}

#[test]
fn div_cx542_format_S_bool_vec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(format "%S" (bool-vector t nil t))
"##,
    );
}

#[test]
fn div_cx542_format_S_char_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ct (make-char-table 'syntax-table ?w)))
  (format "%S" ct))
"##,
    );
}

#[test]
fn div_cx542_format_percent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(format "100%% complete")
"##,
    );
}

#[test]
fn div_cx542_format_multiline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(format "line1\nline2\nline3")
"##,
    );
}

#[test]
fn div_cx542_format_unicode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(format "cafe\u00e9 world")
"##,
    );
}

#[test]
fn div_cx542_propertize_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(propertize "hello" 'face 'bold)
"##,
    );
}

#[test]
fn div_cx542_propertize_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(propertize "hello" 'face 'bold 'mouse-face 'highlight 'help-echo "help")
"##,
    );
}

#[test]
fn div_cx542_propertize_read_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let* ((s (propertize "text" 'face 'bold))
       (p (prin1-to-string s))
       (r (car (read-from-string p))))
  (list (equal s r) (text-properties-at 0 r)))
"##,
    );
}

#[test]
fn div_cx542_format_S_obarray_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((obs (obarray-default)))
  (format "%S" obs))
"##,
    );
}

#[test]
fn div_cx542_format_S_window_buffer_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(format "%S" (current-buffer))
"##,
    );
}
