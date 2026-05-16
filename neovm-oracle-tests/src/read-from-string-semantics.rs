//! Oracle parity tests for GNU `read-from-string` edge semantics.
//!
//! GNU implements this in `src/lread.c` by validating START/END as substring
//! bounds, reading one object, and returning `(OBJECT . FINAL-STRING-INDEX)`.
//! These tests pin exact error payloads because callers can observe them with
//! `condition-case`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_read_from_string_start_end_positions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (read-from-string "  αβ 42 tail" 2)
 (read-from-string "xx (a b) yy" 3 8)
 (read-from-string "one two three" -9)
 (read-from-string "one two three" -9 -6)
 (condition-case err
     (read-from-string "abc" 4)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_read_from_string_malformed_error_payloads() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r###"
(let ((cases
       '("(1 2 3"
         "\"unterminated"
         "[1 2"
         "#<buffer foo>"
         "#1=#1#"
         "#1#"
         "#@5abc")))
  (mapcar
   (lambda (input)
     (condition-case err
         (read-from-string input)
       (error (list input (car err) (cdr err)))))
   cases))
"###;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_read_from_string_hash_skip_lazy_string_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r###"
(let ((inputs '("#@"
                "#@x"
                "#@0x"
                "#@01abc"
                "#@4data42"
                "#@5abc"))
      (zero-zero "#@00abc"))
  (list
   (mapcar
    (lambda (input)
      (condition-case err
          (read-from-string input)
        (error (list input (car err) (cdr err)))))
    inputs)
   (read-from-string zero-zero)))
"###;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_read_from_string_empty_and_whitespace_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((cases '("" "   " "\n\t ")))
  (mapcar
   (lambda (input)
     (condition-case err
         (read-from-string input)
       (error (list input (car err) (cdr err)))))
   cases))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_read_from_string_reader_macro_positions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r###"
(list
 (read-from-string "'foo rest")
 (read-from-string "#'car rest")
 (read-from-string "`(a ,b ,@c) tail")
 (read-from-string "#:uninterned tail")
 (let ((r (read-from-string "#:same #:same")))
   (list (car r) (cdr r) (eq (car r) (car (read-from-string "#:same"))))))
"###;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_read_from_string_preserves_text_properties_on_read_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((src (copy-sequence "\"abc\" tail")))
  (put-text-property 1 4 'face 'bold src)
  (let ((r (read-from-string src)))
    (list r
          (text-properties-at 0 (car r))
          (text-properties-at 1 (car r))
          (substring-no-properties (car r)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
