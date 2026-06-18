use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx454_split_string_omit_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (split-string "a|b|c" "|" t)
      (split-string "a||b|c" "|" t)
      (split-string "a||b|c" "|")
      (split-string "café|世界" "|" t))"##,
    );
}

#[test]
fn div_cx454_mapconcat_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (mapconcat #'identity '("a" "b" "c") ", ")
      (mapconcat #'number-to-string '(1 2 3) "-"))"##,
    );
}

#[test]
fn div_cx454_assoc_string_casefold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((al '(("Foo" . 1) ("bar" . 2))))
  (list (assoc-string "foo" al t)
        (assoc-string "BAR" al t)
        (assoc-string "baz" al t)))"##,
    );
}

#[test]
fn div_cx454_cl_position_find_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((lst '((:a 1) (:b 2) (:a 3))))
  (list (cl-position :a lst :key #'car)
        (cl-find :a lst :key #'car)
        (cl-count :a lst :key #'car)))"##,
    );
}

#[test]
fn div_cx454_cl_delete_duplicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (cl-delete-duplicates '(1 2 1 3 2 4) :test #'=)
      (cl-delete-duplicates '("a" "b" "a" "c") :test #'equal))"##,
    );
}

#[test]
fn div_cx454_seq_group_by() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'seq)
  (seq-group-by #'cl-evenp '(1 2 3 4 5 6)))"##,
    );
}

#[test]
fn div_cx454_seq_min_max() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'seq)
  (list (seq-min '(3 1 4 1 5)) (seq-max '(3 1 4 1 5))))"##,
    );
}

#[test]
fn div_cx454_cl_reduce_some_every() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (cl-reduce #'+ '(1 2 3 4))
      (cl-some #'oddp '(2 4 6))
      (cl-every #'numberp '(1 2 3)))"##,
    );
}

#[test]
fn div_cx454_bufferpos_filepos() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "hello\nworld\n")
  (list (bufferpos-to-filepos 3)
        (filepos-to-bufferpos 3)))"##,
    );
}

#[test]
fn div_cx454_string_to_syntax_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-to-syntax " ")
      (string-to-syntax "w")
      (string-to-syntax ".")
      (string-to-syntax "(")
      (string-to-syntax ")"))"##,
    );
}

#[test]
fn div_cx454_seq_into() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'seq)
  (list (seq-into '(1 2 3) 'vector)
        (seq-into [1 2 3] 'list)))"##,
    );
}

#[test]
fn div_cx454_match_data_full() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "hello world foo")
  (string-match "\\([a-z]+\\) \\([a-z]+\\)" "hello world")
  (list (match-data) (match-data t)))"##,
    );
}

#[test]
fn div_cx454_string_as_multibyte_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "cafe"))
  (list (string-as-unibyte s)
        (string-as-multibyte (string-as-unibyte s))
        (equal s (string-as-multibyte (string-as-unibyte s)))))"##,
    );
}

#[test]
fn div_cx454_format_time_string_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((t1 (encode-time 0 0 12 16 6 2024 nil)))
  (list (format-time-string "%Y-%m-%d %H:%M:%S" t1)
        (format-time-string "%A, %B %d %Y" t1)))"##,
    );
}

#[test]
fn div_cx454_window_config_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((c (current-window-configuration)))
  (list (window-configuration-p c)
        (framep (window-configuration-frame c))))"##,
    );
}
