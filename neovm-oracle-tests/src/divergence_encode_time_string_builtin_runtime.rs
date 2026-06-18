//! encode-time field normalization (day 32 / month 13 / day 0 with dst=-1),
//! and string builtins: string-limit, string-clean-whitespace, string-fill,
//! string-replace (overlapping/empty), string-pad/chop edges, string-to-vector/
//! list + vconcat/concat/append mixing, regexp-quote/regexp-opt, string-search
//! edges (empty/oob-start/multibyte).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn encode_time_month_overflow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (let ((d (decode-time (encode-time (list 0 0 0 1 13 2024 nil -1 0)) 0)))
  (list (nth 4 d) (nth 5 d))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn encode_time_negative_field() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (let ((d (decode-time (encode-time (list 0 0 0 0 6 2024 nil -1 0)) 0)))
  (list (nth 3 d) (nth 4 d))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn encode_time_normalize() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (let ((d (decode-time (encode-time (list 0 0 0 32 1 2024 nil -1 0)) 0)))
  (list (nth 3 d) (nth 4 d))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn regexp_quote_opt() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (regexp-quote "a.b*c+") (regexp-opt '("foo" "bar" "baz"))
        (regexp-opt '("cat" "car" "card") 'words))"##,
    );
}

#[test]
fn string_clean_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (list (string-clean-whitespace "  a   b  c  ")
        (string-fill "one two three four five" 10)) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn string_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (list (string-limit "hello world" 5) (string-limit "hi" 10)
        (string-limit "hello" 3 t)) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn string_pad_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-pad "abc" 2) (string-pad "" 3 ?x) (string-pad "ab" 5 ?- t)
        (string-chop-newline "x\n\n"))"##,
    );
}

#[test]
fn string_replace_overlap() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-replace "aa" "b" "aaaa") (string-replace "aba" "x" "ababa")
        (string-replace "" "x" "ab"))"##,
    );
}

#[test]
fn string_search_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-search "" "abc") (string-search "abc" "") (string-search "c" "abc" 3)
        (string-search "日" "a日b") (string-search "x" "abc"))"##,
    );
}

#[test]
fn string_to_vector_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-to-vector "abc") (string-to-list "héllo")
        (vconcat "ab" [1 2]) (concat [?a ?b] "cd") (append "xy" nil))"##,
    );
}
