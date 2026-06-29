/// Batch 499: fill-individual-paragraphs, fill-region, fill-nobreak, longlines.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx499_fill_individual() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "short\n\nlonger paragraph text here\n")
  (fill-individual-paragraphs (point-min) (point-max))
  (buffer-string))
"##,
        expect_test::expect![[r#""OK \"short\n\nlonger paragraph text here\n\"""#]],
    );
}

#[test]
fn div_cx499_fill_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "aaa bbb ccc ddd eee fff ggg hhh")
  (fill-region (point-min) (point-max) nil 'left)
  (buffer-string))
"##,
        expect_test::expect![[r#""OK \"aaa bbb ccc ddd eee fff ggg hhh\"""#]],
    );
}

#[test]
fn div_cx499_fill_nobreak() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "aaa bbb ccc ddd")
  (fboundp 'fill-nobreak-p))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx499_longlines_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'longlines)
  (with-temp-buffer
    (insert "aaa bbb ccc ddd eee fff ggg hhh iii jjj")
    (longlines-mode 1)
    (buffer-string)))
"##,
        expect_test::expect![[r#""OK \"aaa bbb ccc ddd eee fff ggg hhh iii jjj\"""#]],
    );
}

#[test]
fn div_cx499_sort_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "c\na\nb\n")
  (sort-lines nil (point-min) (point-max))
  (buffer-string))
"##,
        expect_test::expect![[r#""OK \"a\nb\nc\n\"""#]],
    );
}

#[test]
fn div_cx499_sort_paragraphs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "para b\n\npara a\n\npara c\n")
  (sort-paragraphs nil (point-min) (point-max))
  (buffer-string))
"##,
        expect_test::expect![[r#""OK \"para a\n\npara b\n\npara c\n\"""#]],
    );
}

#[test]
fn div_cx499_sort_numeric_fields() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "b 10\na 2\nc 1\n")
  (sort-numeric-fields 2 (point-min) (point-max))
  (buffer-string))
"##,
        expect_test::expect![[r#""OK \"c 1\na 2\nb 10\n\"""#]],
    );
}

#[test]
fn div_cx499_sort_regexp_fields() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "b-10\na-2\nc-1\n")
  (sort-regexp-fields nil "-\\([0-9]+\\)$" "\\1" (point-min) (point-max))
  (buffer-string))
"##,
        expect_test::expect![[r#""OK \"b-1\na-10\nc-2\n\"""#]],
    );
}

#[test]
fn div_cx499_check_doctor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'doctor)
  (list (boundp 'doctor-doctors) (fboundp 'doctor)))
"##,
        expect_test::expect![[r#""OK (nil t)""#]],
    );
}

#[test]
fn div_cx499_check_blackbox() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'blackbox)
  (list (fboundp 'blackbox) (boundp 'bb-version)))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx499_check_mpuz() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'mpuz)
  (list (fboundp 'mpuz) (boundp 'mpuz-version)))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx499_check_gomoku() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'gomoku)
  (list (fboundp 'gomoku) (boundp 'gomoku-version)))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx499_check_bubbles() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'bubbles)
  (list (fboundp 'bubbles) (boundp 'bubbles-version)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx499_check_landmark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'landmark)
  (list (fboundp 'landmark) (boundp 'landmark-version)))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx499_misc_games() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (fboundp 'solitaire) (fboundp 'snake) (fboundp 'tetris))
"##,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}
