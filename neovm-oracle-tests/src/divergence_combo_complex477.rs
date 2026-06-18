/// Batch 477: yaml, toml, json-mode, xml, nxml, markdown, rst, reStructuredText.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx477_yaml_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'yaml-mode)
  (list (fboundp 'yaml-mode) (boundp 'yaml-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_toml_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'toml-mode)
  (list (fboundp 'toml-mode) (boundp 'toml-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_json_mode_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'json-mode)
  (list (fboundp 'json-mode) (boundp 'json-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_nxml_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'nxml-mode)
  (list (fboundp 'nxml-mode) (boundp 'nxml-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_xml_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'xml-mode)
  (list (fboundp 'xml-mode) (boundp 'xml-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_markdown_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'markdown-mode)
  (list (fboundp 'markdown-mode) (boundp 'markdown-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_rst_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'rst)
  (list (fboundp 'rst-mode) (boundp 'rst-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_prolog_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'prolog)
  (list (fboundp 'prolog-mode) (boundp 'prolog-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_sql_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'sql)
  (list (fboundp 'sql-mode) (boundp 'sql-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_scheme_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cmuscheme)
  (list (fboundp 'scheme-mode) (boundp 'scheme-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_lisp_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'lisp-mode)
  (list (fboundp 'lisp-mode) (boundp 'lisp-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_racket_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'racket-mode)
  (list (fboundp 'racket-mode) (boundp 'racket-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_haskell_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'haskell-mode)
  (list (fboundp 'haskell-mode) (boundp 'haskell-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_ocaml_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ocaml-mode)
  (list (fboundp 'ocaml-mode) (boundp 'ocaml-mode-map)))
"##,
    );
}

#[test]
fn div_cx477_fsharp_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'fsharp-mode)
  (list (fboundp 'fsharp-mode) (boundp 'fsharp-mode-map)))
"##,
    );
}
