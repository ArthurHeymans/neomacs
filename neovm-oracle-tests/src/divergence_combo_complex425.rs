//! Complex combo batch 425 — 19 probes into esoteric/unusual areas:
//! format-spec deeper, char-fold-to-regexp deeper, regexp-opt with
//! paren/shy, key-valid-p edge cases, isearch-filter-predicate,
//! dired-mark/unmark, time-stamp, format-spec with modifiers,
//! ewoc/elib widget, tq/task-queue, atimer-run-at-time,
//! itimer/idle-timer, substitute-env-vars, file-name-case-insensitive-p,
//! file-ownership-preserved-p, system-users/system-groups,
//! memory-limit, gc-status-features, and emacs-pid.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// format-spec with modifiers and character specs.
#[test]
fn div_cx425_format_spec_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'format-spec)
  (let ((spec (format-spec-make ?a "hello" ?b "world")))
    (list (format-spec "%a %b" spec)
          (format-spec "%a" spec))))
"##,
    );
}

/// char-fold-to-regexp with multibyte and ASCII equivalents.
#[test]
fn div_cx425_char_fold_to_regexp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (char-fold-to-regexp "cafe")
      (char-fold-to-regexp "a")
      (char-fold-to-regexp "12"))
"##,
    );
}

/// regexp-opt with paren and shy-group options.
#[test]
fn div_cx425_regexp_opt_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (regexp-opt '("hello" "hello-world") 'paren)
      (regexp-opt '("abc" "def") 'shy))
"##,
    );
}

/// key-valid-p with various edge case inputs.
#[test]
fn div_cx425_key_valid_p_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (key-valid-p "a")
      (key-valid-p "C-c C-x M-a")
      (key-valid-p "")
      (key-valid-p "mouse-1")
      (key-valid-p "C-1"))
"##,
    );
}

/// substitute-env-vars with multibyte.
#[test]
fn div_cx425_substitute_env_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((process-environment (cons "MY_VAR=café世界" process-environment)))
  (list (substitute-env-vars "$MY_VAR")
        (substitute-env-vars "prefix-$MY_VAR-suffix")
        (substitute-env-vars "no-env")))
"##,
    );
}

/// file-name-case-insensitive-p / file-ownership-preserved-p.
#[test]
fn div_cx425_file_case_ownership() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (file-name-case-insensitive-p "/")
      (file-ownership-preserved-p "/tmp"))
"##,
    );
}

/// system-users / system-groups: user/group database queries.
#[test]
fn div_cx425_system_users_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (system-users) (error (car e)))
      (condition-case e (system-groups) (error (car e))))
"##,
    );
}

/// memory-limit / gc-status-features / emacs-pid.
#[test]
fn div_cx425_memory_gc_status() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (memory-limit)
      (gc-status-features)
      (emacs-pid))
"##,
    );
}

/// file-newest-backup / find-backup-file-name deeper.
#[test]
fn div_cx425_backup_file_deeper() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f "/tmp/neo-cx425-fixed.el"))
  (list (file-newest-backup f)
        (backup-file-name-p (concat f "~"))
        (make-backup-file-name f)))
"##,
    );
}

/// atimer: run-at-time with 0 delay in batch.
#[test]
fn div_cx425_atimer_run_at_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((fired nil))
  (run-at-time 0 nil (lambda () (setq fired t)))
  (sit-for 0.1)
  fired)
"##,
    );
}

/// dired-mark/unmark operations.
#[test]
fn div_cx425_dired_mark_unmark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(require 'dired)
(let ((tmpdir (make-temp-file "neo-cx425-dm-" t)))
  (with-temp-file (expand-file-name "a.txt" tmpdir) (insert "x"))
  (with-temp-file (expand-file-name "b.txt" tmpdir) (insert "y"))
  (unwind-protect
      (with-temp-buffer
        (dired tmpdir)
        (dired-mark 1)
        (dired-unmark 1)
        (length (dired-get-marked-files)))
    (delete-directory tmpdir t)))
"##,
    );
}

/// time-stamp: automatic time stamp formatting.
#[test]
fn div_cx425_time_stamp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'time-stamp)
  (list (stringp (time-stamp-string))
        (boundp 'time-stamp-format)))
"##,
    );
}

/// log-file-suffixes / byte-compile-dest-file.
#[test]
fn div_cx425_log_byte_dest() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'bytecomp)
  (condition-case e
      (byte-compile-dest-file "/tmp/test.el")
    (error (car e))))
"##,
    );
}

/// file-name-split / file-name-canonicalize.
#[test]
fn div_cx425_file_name_split_canon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (file-name-split "/a/b/c.txt")
      (file-name-split "a/b")
      (condition-case e (file-name-canonicalize "/tmp/../tmp/.") (error (car e))))
"##,
    );
}

/// read-char-from-minibuffer / read-char-by-name deeper.
#[test]
fn div_cx425_read_char_from_minibuffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (read-char-by-name "test: " t)
  (error (car e)))
"##,
    );
}

/// global-substring / substring-no-properties with multibyte.
#[test]
fn div_cx425_substring_no_props_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (substring-no-properties "café世界" 1 4)
      (substring "café世界" 1 4))
"##,
    );
}

/// integer-or-marker-p / number-or-marker-p / natnump.
#[test]
fn div_cx425_number_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (integer-or-marker-p 5)
      (integer-or-marker-p (make-marker))
      (number-or-marker-p 5.5)
      (natnump -1)
      (natnump 0)
      (natnump 1))
"##,
    );
}
