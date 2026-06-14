//! Source-audit divergences: fileio / dired / marker relocation.
//!
//! From GNU src/fileio.c, src/dired.c, src/alloc.c(marker) vs neovm-core
//! fileio.rs, dired.rs, edit_transaction.rs: expand-file-name path collapse,
//! directory-files dot handling, file-attributes representation, and marker
//! relocation using bytepos vs charpos (multibyte-sensitive).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- expand-file-name (no filesystem needed) --------------------------------

#[test]
fn div_af_expand_dotdot_at_root() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(expand-file-name "../.." "/")"##);
}

#[test]
fn div_af_expand_dotdot_segment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(expand-file-name "a/../b" "/tmp")"##);
}

#[test]
fn div_af_expand_double_slash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(expand-file-name "a//b" "/tmp")"##);
}

#[test]
fn div_af_expand_trailing_dotdot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(expand-file-name "a/.." "/tmp")"##);
}

#[test]
fn div_af_expand_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(expand-file-name "" "/tmp")"##);
}

#[test]
fn div_af_directory_file_name_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(directory-file-name "")"##);
}

#[test]
fn div_af_directory_file_name_trailing_slash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(directory-file-name "/tmp/")"##);
}

#[test]
fn div_af_name_as_directory_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (file-name-as-directory "/tmp")
      (directory-file-name (file-name-as-directory "/tmp"))
      (file-name-as-directory "x"))
"##,
    );
}

#[test]
fn div_af_split_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (file-name-nondirectory "/a/b/c.txt")
      (file-name-directory "/a/b/c.txt")
      (file-name-sans-versions "c.txt~")
      (file-name-extension "/a/b.tar.gz")
      (file-name-base "/a/b/c.txt"))
"##,
    );
}

// --- directory-files (temp dir) ---------------------------------------------

#[test]
fn div_af_directory_files_dot_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((dir (make-temp-name "/tmp/neo-af-")))
  (make-directory dir)
  (unwind-protect
      (length (directory-files dir))
    (ignore-errors (delete-directory dir t))))
"##,
    );
}

#[test]
fn div_af_directory_files_sorted() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((dir (make-temp-name "/tmp/neo-af2-")))
  (make-directory dir)
  (unwind-protect
      (sort (directory-files dir) (function string<))
    (ignore-errors (delete-directory dir t))))
"##,
    );
}

#[test]
fn div_af_directory_files_with_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((dir (make-temp-name "/tmp/neo-af3-")))
  (make-directory dir)
  (unwind-protect
      (length (directory-files dir nil nil t 2))
    (ignore-errors (delete-directory dir t))))
"##,
    );
}

// --- marker relocation over multibyte (bytepos vs charpos) ------------------

#[test]
fn div_af_marker_delete_multibyte_in_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "ab世cd")
  (let ((m (set-marker (make-marker) 4)))
    (delete-region 3 4)
    (marker-position m)))
"##,
    );
}

#[test]
fn div_af_marker_at_multibyte_boundary_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "ab世")
  (let ((m (set-marker (make-marker) 3)))
    (set-marker-insertion-type m t)
    (goto-char 3)
    (insert "X")
    (marker-position m)))
"##,
    );
}

#[test]
fn div_af_marker_relocate_before_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abc世界")
  (let ((m (set-marker (make-marker) 6))
        (m2 (set-marker (make-marker) 4)))
    (delete-region 1 3)
    (list (marker-position m) (marker-position m2))))
"##,
    );
}

#[test]
fn div_af_file_attributes_small_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-name "/tmp/neo-fa-")))
  (write-region "hello" nil f nil 0)
  (unwind-protect
      ;; size field (element 7) only — deterministic
      (nth 7 (file-attributes f))
    (ignore-errors (delete-file f))))
"##,
    );
}
