/// Batch 475: gud, gdb, etags, ebrowse, ccmode, cflow, emerge, compile deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx475_gud_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'gud)
  (list (boundp 'gud-mode-map) (fboundp 'gud-gdb)))
"##,
    );
}

#[test]
fn div_cx475_gdb_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'gdb-mi)
  (list (fboundp 'gdb) (boundp 'gdb-mode-map)))
"##,
    );
}

#[test]
fn div_cx475_etags_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'etags)
  (list (fboundp 'find-tag) (fboundp 'tags-search)))
"##,
    );
}

#[test]
fn div_cx475_ebrowse_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ebrowse)
  (list (fboundp 'ebrowse) (boundp 'ebrowse-version)))
"##,
    );
}

#[test]
fn div_cx475_ccmode_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cc-mode)
  (list (fboundp 'c-mode) (boundp 'c-mode-map)))
"##,
    );
}

#[test]
fn div_cx475_cflow_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cflow)
  (list (fboundp 'cflow) (boundp 'cflow-version)))
"##,
    );
}

#[test]
fn div_cx475_emerge_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'emerge)
  (list (fboundp 'emerge-files) (boundp 'emerge-version)))
"##,
    );
}

#[test]
fn div_cx475_compile_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'compile)
  (list (fboundp 'compile) (boundp 'compilation-buffer-name-function)))
"##,
    );
}

#[test]
fn div_cx475_grep_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'grep)
  (list (fboundp 'grep) (fboundp 'lgrep) (boundp 'grep-find-command)))
"##,
    );
}

#[test]
fn div_cx475_python_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'python)
  (list (fboundp 'run-python) (boundp 'python-mode-map)))
"##,
    );
}

#[test]
fn div_cx475_ruby_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ruby-mode)
  (list (fboundp 'ruby-mode) (boundp 'ruby-mode-map)))
"##,
    );
}

#[test]
fn div_cx475_perl_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cperl-mode)
  (list (fboundp 'cperl-mode) (boundp 'cperl-mode-map)))
"##,
    );
}

#[test]
fn div_cx475_tcl_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'tcl)
  (list (fboundp 'tcl-mode) (boundp 'tcl-mode-map)))
"##,
    );
}

#[test]
fn div_cx475_fortran_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'fortran)
  (list (fboundp 'fortran-mode) (boundp 'fortran-mode-map)))
"##,
    );
}

#[test]
fn div_cx475_pascal_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'pascal)
  (list (fboundp 'pascal-mode) (boundp 'pascal-mode-map)))
"##,
    );
}
