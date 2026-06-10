//! Oracle parity tests for `string-match`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use proptest::prelude::*;

use super::common::{ORACLE_PROP_CASES, assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prop_string_match_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle_hit, neovm_hit) = eval_oracle_and_neovm(r#"(string-match "b+" "abbb")"#);
    assert_ok_eq("1", &oracle_hit, &neovm_hit);

    let (oracle_miss, neovm_miss) = eval_oracle_and_neovm(r#"(string-match "z+" "abbb")"#);
    assert_ok_eq("nil", &oracle_miss, &neovm_miss);
}

#[test]
fn oracle_prop_string_match_wrong_type_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match 1 "abc")"#);
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

#[test]
fn oracle_prop_string_match_char_class_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match "[z-a]" "z")"#);
    assert_ok_eq("nil", &oracle, &neovm);

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match "[^z-a]" "x")"#);
    assert_ok_eq("0", &oracle, &neovm);

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match "[]a]+" "]aa")"#);
    assert_ok_eq("0", &oracle, &neovm);

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match "[[]+" "[[[")"#);
    assert_ok_eq("0", &oracle, &neovm);

    let (oracle, neovm) = eval_oracle_and_neovm(r#"(string-match "[\\]" "\\")"#);
    assert_ok_eq("0", &oracle, &neovm);
}

#[test]
fn oracle_prop_string_match_zero_length_at_start_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU `string-match` reports the zero-length match at START and leaves it
    // to callers that loop over matches to advance explicitly.
    let form = r#"(let ((line "a,b,c")
      (pos 0)
      (steps nil)
      (n 0))
    (while (< n 5)
      (string-match "\\([^,]*\\)" line pos)
    (setq steps
          (cons (list pos
                      (match-beginning 0)
                      (match-end 0)
                      (substring line (match-beginning 1) (match-end 1)))
                steps))
    (setq pos (match-end 0))
    (if (and (< pos (length line)) (= (aref line pos) ?,))
        (progn
          (setq pos (1+ pos))))
    (setq n (1+ n)))
  (nreverse steps))"#;
    let (oracle, neovm) = eval_oracle_and_neovm(form);
    assert_ok_eq(
        r#"((0 0 1 "a") (2 2 3 "b") (4 4 5 "c") (5 5 5 "") (5 5 5 ""))"#,
        &oracle,
        &neovm,
    );
}

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(ORACLE_PROP_CASES))]

    #[test]
    fn oracle_prop_string_match_index_for_simple_prefix(
        n in 0usize..20usize,
    ) {
        return_if_neovm_enable_oracle_proptest_not_set!(Ok(()));

        let haystack = format!("{}a", "b".repeat(n));
        let form = format!(r#"(string-match "a" "{}")"#, haystack);
        let expected = n.to_string();
        let (oracle, neovm) = eval_oracle_and_neovm(&form);
        assert_ok_eq(expected.as_str(), &oracle, &neovm);
    }
}
