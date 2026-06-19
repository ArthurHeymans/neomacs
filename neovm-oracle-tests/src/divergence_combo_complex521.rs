/// Batch 521: read syntax, print syntax, read-from-string edge, print-circle.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx521_read_from_string_ints() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(mapcar #'car (list (read-from-string "42") (read-from-string "-7") (read-from-string "0x1A")))
"##,
    );
}

#[test]
fn div_cx521_read_from_string_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(mapcar #'car (list (read-from-string "hello") (read-from-string "hello-world") (read-from-string "|Hello World|")))
"##,
    );
}

#[test]
fn div_cx521_read_from_string_lists() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (read-from-string "(1 2 3)"))
"##,
    );
}

#[test]
fn div_cx521_read_from_string_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (read-from-string "[1 2 3]"))
"##,
    );
}

#[test]
fn div_cx521_print_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(mapcar #'prin1-to-string '(42 -7 3.14 1/3))
"##,
    );
}

#[test]
fn div_cx521_print_circle_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((print-circle t) (l (list 1 2 3)))
  (setcdr (cddr l) l)
  (prin1-to-string l))
"##,
    );
}

#[test]
fn div_cx521_print_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((print-length 2)) (prin1-to-string '(1 2 3 4 5)))
"##,
    );
}

#[test]
fn div_cx521_print_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((print-level 2)) (prin1-to-string '(((1 2) 3) 4)))
"##,
    );
}

#[test]
fn div_cx521_print_gensym() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((print-gensym t)) (prin1-to-string (make-symbol "xyz")))
"##,
    );
}

#[test]
fn div_cx521_print_escape_nonascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((print-escape-nonascii t)) (prin1-to-string "cafe\xe9"))
"##,
    );
}

#[test]
fn div_cx521_read_quote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (read-from-string "'(1 2 3)"))
"##,
    );
}

#[test]
fn div_cx521_read_backquote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (read-from-string "`(1 2 ,(+ 1 2))"))
"##,
    );
}

#[test]
fn div_cx521_read_hash_notation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (read-from-string "#(1 2 3)"))
"##,
    );
}

#[test]
fn div_cx521_read_char_literal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (read-from-string "?a"))
"##,
    );
}

#[test]
fn div_cx521_read_string_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (read-from-string "\"hello\nworld\""))
"##,
    );
}
