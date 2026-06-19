/// Batch 522: further print/read edge cases - read syntax errors, prin1 edge.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx522_read_bad_character() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (read-from-string "#\\zZzZz") (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx522_read_unmatched_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (read-from-string "(1 2 3") (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx522_read_unmatched_bracket() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (read-from-string "[1 2 3") (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx522_read_unmatched_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (read-from-string "\"abc") (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx522_read_invalid_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (read-from-string "\"\\z\"" ) (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx522_print_char_escapes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((print-escape-newlines t) (print-escape-nonascii t))
  (prin1-to-string "hello\nworld\xe9"))
"##,
    );
}

#[test]
fn div_cx522_print_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(prin1-to-string [1 2 3])
"##,
    );
}

#[test]
fn div_cx522_print_bool_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(prin1-to-string (bool-vector t nil t))
"##,
    );
}

#[test]
fn div_cx522_print_char_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ct (make-char-table 'syntax-table ?w)))
  (prin1-to-string ct))
"##,
    );
}

#[test]
fn div_cx522_read_propertized_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (read-from-string "#(\"hello\" 0 5 (face bold))"))
"##,
    );
}

#[test]
fn div_cx522_read_bad_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (read-from-string "1.0.0") (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx522_read_bad_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (read-from-string "12a") (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx522_print_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (prin1-to-string 'HELLO) (prin1-to-string 'Hello) (prin1-to-string 'hello))
"##,
    );
}

#[test]
fn div_cx522_print_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(prin1-to-string :test-keyword)
"##,
    );
}

#[test]
fn div_cx522_read_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (read-from-string "nil"))
"##,
    );
}
