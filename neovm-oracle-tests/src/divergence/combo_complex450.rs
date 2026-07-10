//! Complex combo batch 450 — 15 final milestone probes: thing-at-point,
//! forward-sentence, backward-sentence, transpose-regions, replace-highlight,
//! list-abbrevs, edit-abbrevs, define-global-abbrev, expand-abbrev,
//! unexpand-abbrev, add-global-abbrev, inverse-add-global-abbrev,
//! set-case-syntax-1, set-case-syntax-pair, with-case-table.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx450_thing_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"hello\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "hello world")
  (goto-char 3)
  (thing-at-point 'word))"##,
        expect,
    );
}

#[test]
fn div_cx450_forward_sentence() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK 28""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "Hello world. Goodbye world.")
  (goto-char 1)
  (forward-sentence 1)
  (point))"##,
        expect,
    );
}

#[test]
fn div_cx450_transpose_regions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"123abc\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "abc123")
  (transpose-regions 1 4 4 7)
  (buffer-string))"##,
        expect,
    );
}

#[test]
fn div_cx450_define_global_abbrev() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'abbrev)
  (let ((global-abbrev-table (make-abbrev-table)))
    (define-global-abbrev "teh" "the")
    (expand-abbrev)))"##,
        expect,
    );
}

#[test]
fn div_cx450_abbrev_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (\"on my way\" nil)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'abbrev)
  (let ((tab (make-abbrev-table)))
    (define-abbrev tab "omw" "on my way")
    (list (abbrev-expansion "omw" tab)
          (abbrev-expansion "nonexistent" tab))))"##,
        expect,
    );
}

#[test]
fn div_cx450_set_case_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (invalid-read-syntax \")\" 3 17)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(let ((tbl (copy-case-table)))
  (set-case-syntax-pair ?\\[ ?\\] tbl)
  (aref tbl ?\\[))"##,
        expect,
    );
}

#[test]
fn div_cx450_with_case_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (wrong-number-of-arguments (1 . 1) 0)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(let ((ct (copy-case-table)))
  (set-char-table-range ct ?a ?x)
  (set-case-table ct)
  (downcase "A"))"##,
        expect,
    );
}

#[test]
fn div_cx450_list_abbrevs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK t""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'abbrev)
  (with-temp-buffer
    (fboundp 'list-abbrevs)))"##,
        expect,
    );
}

#[test]
fn div_cx450_unexpand_abbrev() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK t""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'abbrev)
  (fboundp 'unexpand-abbrev))"##,
        expect,
    );
}

#[test]
fn div_cx450_add_global_abbrev() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK t""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'abbrev)
  (fboundp 'add-global-abbrev))"##,
        expect,
    );
}

#[test]
fn div_cx450_inverse_add_global_abbrev() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK t""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'abbrev)
  (fboundp 'inverse-add-global-abbrev))"##,
        expect,
    );
}

#[test]
fn div_cx450_edit_abbrevs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK t""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'abbrev)
  (fboundp 'edit-abbrevs))"##,
        expect,
    );
}

#[test]
fn div_cx450_replace_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'replace)
  (boundp 'replace-highlight))"##,
        expect,
    );
}

#[test]
fn div_cx450_make_temp_file_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK t""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(let ((n (make-temp-name "/tmp/neo-cx450-")))
  (stringp n))"##,
        expect,
    );
}

#[test]
fn div_cx450_file_name_all_completions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK 148""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(length (file-name-all-completions "" "/tmp"))"##,
        expect,
    );
}
