//! Oracle parity tests for GNU conversion and formatting edge semantics.
//!
//! GNU implements character/string conversion and `format` in `src/editfns.c`,
//! and numeric string conversion in `src/data.c`. These tests focus on exact
//! edge behavior and error payloads rather than broad formatting examples.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_char_string_conversion_edges_and_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (char-to-string ?é)
 (char-to-string #x3fffff)
 (string-bytes (char-to-string #x3fffff))
 (string-to-char "éx")
 (string-to-char "")
 (condition-case err
     (char-to-string -1)
   (error (list (car err) (cdr err))))
 (condition-case err
     (char-to-string #x400000)
   (error (list (car err) (cdr err))))
 (condition-case err
     (string-to-char 42)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_string_to_number_base_and_error_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (string-to-number " \t+ffrest" 16)
 (string-to-number "\n42")
 (string-to-number "0x10")
 (string-to-number "0x10" 16)
 (string-to-number "1e2" 16)
 (string-to-number "nan")
 (string-to-number "inf")
 (condition-case err
     (string-to-number 'bad)
   (error (list (car err) (cdr err))))
 (condition-case err
     (string-to-number "10" 1)
   (error (list (car err) (cdr err))))
 (condition-case err
     (string-to-number "10" 17)
   (error (list (car err) (cdr err))))
 (condition-case err
     (string-to-number "10" 2.0)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_number_to_string_float_and_type_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (number-to-string 1000000000000000000000001)
 (number-to-string -0.0)
 (number-to-string (/ 0.0 0.0))
 (number-to-string (/ 1.0 0.0))
 (number-to-string (/ -1.0 0.0))
 (condition-case err
     (number-to-string "42")
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_format_numbered_and_unnumbered_arguments() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (format "%2$s %1$s %2$s" "one" "two")
 (format "%2$04d %1$#x" 255 7)
 (format "%% %2$s" "unused" "used")
 (condition-case err
     (format "%2$s %s" "one" "two")
   (error (list (car err) (cdr err))))
 (condition-case err
     (format "%0$s" "zero")
   (error (list (car err) (cdr err))))
 (condition-case err
     (format "%2$s" "only-one")
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_format_precision_properties_and_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((fmt (propertize "[%.3s|%S]" 'face 'bold))
       (out (format fmt "abcdef" "x")))
  (list
   out
   (text-properties-at 0 out)
   (format "%#b %#B %#o %#x %#X" 10 10 10 255 255)
   (format "%+.0f % .0f %#.0f" 3.4 3.4 3.4)
   (condition-case err
       (format "%d" "not-number")
     (error (list (car err) (cdr err))))
   (condition-case err
       (format "%c" #x400000)
     (error (list (car err) (cdr err))))
   (condition-case err
       (format "%")
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_format_argument_text_property_propagation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU `styled_format` in src/editfns.c records each substituted span
    // and then copies text properties from both the format string and string
    // arguments, extending argument properties across right padding.
    let form = r#"
(list
 (let ((s (format "%s" (propertize "abc" 'face 'bold))))
   (list s
         (text-properties-at 0 s)
         (text-properties-at 1 s)
         (text-properties-at 2 s)))
 (let ((s (format "[%5s]" (propertize "abc" 'face 'bold))))
   (list s
         (mapcar (lambda (i) (text-properties-at i s))
                 (number-sequence 0 (1- (length s))))))
 (let ((s (format "%-5s" (propertize "abc" 'face 'bold))))
   (list s
         (mapcar (lambda (i) (text-properties-at i s))
                 (number-sequence 0 (1- (length s))))))
 (let ((s (format "%.2s" (propertize "abcdef" 'face 'bold))))
   (list s
         (mapcar (lambda (i) (text-properties-at i s))
                 (number-sequence 0 (1- (length s))))))
 (let* ((fmt (propertize "[%s]" 'face 'italic))
        (arg (propertize "abc" 'face 'bold))
        (s (format fmt arg)))
   (list s
         (mapcar (lambda (i) (text-properties-at i s))
                 (number-sequence 0 (1- (length s)))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
