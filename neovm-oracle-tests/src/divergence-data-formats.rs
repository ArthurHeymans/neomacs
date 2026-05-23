//! Divergence tests: json, xml, csv parsing deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_json_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'json-parse-string)
  (fboundp 'json-parse-buffer)
  (fboundp 'json-serialize)
  (featurep 'json)) "#,
    );
}

#[test]
fn divergence_json_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((obj '((foo . 1) (bar . 2) (baz . [3 4 5])))
        (json-str (json-serialize obj)))
  (list (stringp json-str)
        (plistp (json-parse-string json-str :object-type 'plist))
        (listp (json-parse-string json-str :object-type 'alist)))) "#,
    );
}

#[test]
fn divergence_json_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'json-insert)
  (fboundp 'json-read)
  (fboundp 'json-read-from-string)
  (boundp 'json-object-type)
  (member json-object-type '(hash-table alist plist))) "#,
    );
}

#[test]
fn divergence_xml_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'xml-parse-region)
  (fboundp 'xml-parse-string)
  (fboundp 'libxml-parse-xml-region)
  (fboundp 'libxml-parse-html-region)
  (featurep 'xml)) "#,
    );
}

#[test]
fn divergence_dom_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'dom-tag)
  (fboundp 'dom-attributes)
  (fboundp 'dom-children)
  (fboundp 'dom-parent)
  (fboundp 'dom-remove-node)
  (fboundp 'dom-append-child)
  (featurep 'dom)) "#,
    );
}

#[test]
fn divergence_csv() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'csv-parse-buffer)
  (fboundp 'csv-parse-string)
  (featurep 'csv)) "#,
    );
}

#[test]
fn divergence_tsv() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'tsv-mode)
  (fboundp 'align)
  (featurep 'align)) "#,
    );
}

#[test]
fn divergence_yaml() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'yaml-parse-string)
  (fboundp 'yaml-parse-file)
  (featurep 'yaml)) "#,
    );
}

#[test]
fn divergence_toml() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'toml-parse-string)
  (fboundp 'toml-parse-file)
  (featurep 'toml)) "#,
    );
}

#[test]
fn divergence_rpc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'jsonrpc-request)
  (fboundp 'jsonrpc-notify)
  (featurep 'jsonrpc)) "#,
    );
}
