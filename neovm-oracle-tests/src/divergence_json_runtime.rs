//! json-serialize / json-parse-string parity: objects/arrays, object-type
//! alist/plist/hash, special values, numbers, roundtrips, plus the unibyte
//! vs multibyte result-string divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn json_alist_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(json-serialize '((a . 1)) :false-object :false :null-object :null)"##,
    );
}

#[test]
fn json_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (json-parse-string "3.14") (json-parse-string "42")
        (json-parse-string "-17") (json-parse-string "1e3")
        (json-serialize '((x . 3.5) (y . 100))))"##,
    );
}

#[test]
fn json_parse_object_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (json-parse-string "{\"a\":1,\"b\":[2,3]}")
        (json-parse-string "{\"a\":1}" :object-type 'alist)
        (json-parse-string "{\"a\":1}" :object-type 'plist))"##,
    );
}

#[test]
fn json_parse_special() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (json-parse-string "true") (json-parse-string "false")
        (json-parse-string "null") (json-parse-string "null" :null-object 'NIL)
        (json-parse-string "[]") (json-parse-string "{}"))"##,
    );
}

#[test]
fn json_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s "{\"k\":[1,2,{\"n\":true}],\"s\":\"v\"}"))
  (string= s (json-serialize (json-parse-string s))))"##,
    );
}

#[test]
fn json_serialize_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (json-serialize '((a . 1) (b . "x") (c . t) (d . :false) (e . :null)))
        (json-serialize [1 2 3])
        (json-serialize (make-hash-table)))"##,
    );
}

#[test]
fn json_serialize_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(json-serialize '((name . "test") (nums . [1 2 3]) (obj . ((k . "v")))))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: json-serialize returns a multibyte string in neomacs but a unibyte UTF-8 string in GNU (multibyte-string-p t vs nil; length differs from string-bytes for non-ASCII output)."]
fn divergence_json_serialize_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s (json-serialize ["é" "⚡"])))
  (list (multibyte-string-p s)
        (string-bytes s)
        (length s)))"##,
    );
}
