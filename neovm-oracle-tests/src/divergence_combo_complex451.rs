//! Complex combo batch 451 — 15 subr-x/file-mode/backtrace probes:
//! string-lines, string-split, string-pad, string-chop-newline,
//! string-clean-whitespace, file-size-human-readable, file-modes-symbolic,
//! file-modes-number-to-symbolic, set-file-modes symbolic, backtrace-get-frames,
//! mapbacktrace, text-property-search-forward, thread-first with functions,
//! thread-last with functions, encode-time decoded-time extended.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx451_string_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-lines "hello\nworld\nthird")
        (string-lines "")))"##,
    );
}

#[test]
fn div_cx451_string_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(string-split "one two three" " " t)"##);
}

#[test]
fn div_cx451_string_pad() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-pad "hello" 8)
        (string-pad "hello" 8 nil t)
        (string-pad "hello" 3)))"##,
    );
}

#[test]
fn div_cx451_string_chop_newline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-chop-newline "hello\n")
        (string-chop-newline "hello")))"##,
    );
}

#[test]
fn div_cx451_string_clean_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (string-clean-whitespace "  hello   world  "))"##,
    );
}

#[test]
fn div_cx451_file_size_human() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (file-size-human-readable 1024)
      (file-size-human-readable 1048576)
      (file-size-human-readable 1073741824))"##,
    );
}

#[test]
fn div_cx451_file_modes_symbolic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (file-modes-symbolic-to-number "rw-r--r--")
      (file-modes-number-to-symbolic #o644)
      (file-modes-symbolic-to-number "rwxr-xr-x"))"##,
    );
}

#[test]
fn div_cx451_file_modes_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "neo-cx451-fm-")))
  (unwind-protect
      (progn (set-file-modes f #o600)
             (file-modes f))
    (delete-file f)))"##,
    );
}

#[test]
fn div_cx451_backtrace_get_frames() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((frames (backtrace-get-frames)))
  (list (vectorp frames)
        (> (length frames) 0)))"##,
    );
}

#[test]
fn div_cx451_mapbacktrace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((count 0))
  (mapbacktrace (lambda (&rest _) (setq count (1+ count))))
  count)"##,
    );
}

#[test]
fn div_cx451_text_property_search_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abc def ghi")
  (put-text-property 1 4 'face 'bold)
  (put-text-property 5 8 'face 'italic)
  (list (text-property-search-forward 'face 'bold)
        (text-property-search-forward 'face 'italic)))"##,
    );
}

#[test]
fn div_cx451_thread_first_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(thread-first 5
  (+ 3)
  (* 2)
  (- 4))"##,
    );
}

#[test]
fn div_cx451_thread_last_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(thread-last "hello world"
  (upcase)
  (split-string " "))"##,
    );
}

#[test]
fn div_cx451_encode_time_decoded_time_ext() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((dt (decode-time (encode-time 0 0 12 16 6 2024 nil))))
      (list dt (length dt)))
  (error (car e)))"##,
    );
}

#[test]
fn div_cx451_directory_empty_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((d (make-temp-file "neo-cx451-dep-" t)))
  (unwind-protect
      (file-directory-p d)
    (delete-directory d t)))"##,
    );
}
