//! Divergence tests: network, threading, json, xml stubs.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_json_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(require 'json)
(list
  (json-parse-string "{\"a\": 1, \"b\": [2, 3]}")
  (json-serialize '((a . 1) (b . [2 3]))))"#,
        expect_test::expect![[
            r#""OK (#s(hash-table test equal data (\"a\" 1 \"b\" [2 3])) \"{\\\"a\\\":1,\\\"b\\\":[2,3]}\")""#
        ]],
    );
}

#[test]
fn divergence_json_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(require 'json)
(list
  (json-serialize 'null)
  (json-serialize t)
  (json-serialize 42)
  (json-serialize "hello"))"#,
        expect_test::expect![[r#""ERR (wrong-type-argument json-value-p null)""#]],
    );
}

#[test]
fn divergence_xml_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(require 'xml)
(let ((tree (xml-parse-string "<root><item>hello</item></root>")))
  (list (consp tree)
        (caar tree)
        (length tree)))"#,
        expect_test::expect![[r#""ERR (wrong-number-of-arguments (0 . 0) 1)""#]],
    );
}

#[test]
fn divergence_thread_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (featurep 'threads)
  (fboundp 'make-thread)
  (fboundp 'thread-join)
  (fboundp 'thread-signal)
  (fboundp 'current-thread)
  (fboundp 'all-threads))"#,
        expect_test::expect![[r#""OK (t t t t t t)""#]],
    );
}

#[test]
fn divergence_mutex_condition_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'make-mutex)
  (fboundp 'mutex-lock)
  (fboundp 'mutex-unlock)
  (fboundp 'make-condition-variable)
  (fboundp 'condition-wait)
  (fboundp 'condition-notify))"#,
        expect_test::expect![[r#""OK (t t t t t t)""#]],
    );
}

#[test]
fn divergence_network_interface_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'network-interface-list)
  (fboundp 'network-interface-info)
  (fboundp 'format-network-address))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_dns_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'dns-query)
  (fboundp 'dns-lookup)
  (fboundp 'dns-lookup-host))"#,
        expect_test::expect![[r#""OK (t nil t)""#]],
    );
}

#[test]
fn divergence_gnutls() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'gnutls-available-p)
  (fboundp 'gnutls-boot)
  (featurep 'gnutls))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_url_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'url-retrieve)
  (fboundp 'url-retrieve-synchronously)
  (featurep 'url))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}

#[test]
fn divergence_auth_source() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'auth-source-search)
  (fboundp 'auth-source-forget)
  (featurep 'auth-source))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
}
