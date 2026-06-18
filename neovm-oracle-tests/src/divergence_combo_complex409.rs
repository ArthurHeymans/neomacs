//! Complex combo batch 409 — 20 probes in new territory: window-point/start,
//! pos-visible-in-window-p, input-pending-p, recent-keys, this-command-keys,
//! keyboard-translate, translation-table, key-translation-map, locale-coding-system,
//! file-coding-system-alist, file-equal-p/file-in-directory-p, file-name-base/extension,
//! make-backup-file-name, find-backup-file-name, replace-regexp-in-string with
//! subexp count, sort-subr, sequential-command, describe-key-briefly,
//! help-buffer/help-setup-xref, apropos, and format-find-file.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// window-point / window-start with different window layouts.
#[test]
fn div_cx409_window_point_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "line1\nline2\nline3\nline4\nline5")
  (let ((w (selected-window)))
    (set-window-point w 3)
    (set-window-start w 2)
    (list (window-point w)
          (window-start w))))
"##,
    );
}

/// pos-visible-in-window-p with partially visible lines.
#[test]
fn div_cx409_pos_visible_window_partial() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "aaa bbb ccc ddd eee fff ggg hhh iii jjj")
  (list (pos-visible-in-window-p 1)
        (pos-visible-in-window-p (point-max))
        (pos-visible-in-window-p 5 nil t)))
"##,
    );
}

/// input-pending-p / recent-keys: keyboard state queries
/// in batch mode (should return nil/empty).
#[test]
fn div_cx409_input_pending_recent_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (input-pending-p)
      (length (recent-keys))
      (this-command-keys)
      (last-command-keys))
"##,
    );
}

/// keyboard-translate: translation table for key events.
#[test]
fn div_cx409_keyboard_translate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (keyboard-translate ?a ?b)
  (let ((kt (keyboard-translate ?a)))
    (list kt
          (if kt (char-equal kt ?b) nil))))
"##,
    );
}

/// locale-coding-system / file-coding-system-alist:
/// coding system configuration may differ.
#[test]
fn div_cx409_coding_system_config() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (locale-coding-system)
      (keyboard-coding-system)
      (file-coding-system-alist))
"##,
    );
}

/// file-equal-p / file-in-directory-p with temp files.
#[test]
fn div_cx409_file_equal_in_dir() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((d (make-temp-file "neo-cx409-dir-" t))
      (f1 (make-temp-file "neo-cx409-f1-"))
      (f2 (make-temp-file "neo-cx409-f2-")))
  (unwind-protect
      (list (file-equal-p f1 f1)
            (file-equal-p f1 f2)
            (file-in-directory-p f1 default-directory)
            (file-in-directory-p f1 d))
    (delete-file f1)
    (ignore-errors (delete-file f2))
    (ignore-errors (delete-directory d t))))
"##,
    );
}

/// file-name-base / file-name-extension / file-name-sans-extension.
#[test]
fn div_cx409_file_name_base_ext() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (file-name-base "foo.txt")
      (file-name-extension "foo.txt")
      (file-name-sans-extension "foo.txt")
      (file-name-base "/path/to/bar.tar.gz")
      (file-name-extension "/path/to/bar.tar.gz"))
"##,
    );
}

/// make-backup-file-name / find-backup-file-name.
#[test]
fn div_cx409_backup_file_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f "/tmp/neo-cx409-fixed-name.el"))
  (list (make-backup-file-name f)
        (file-name-extension (make-backup-file-name f))
        (file-name-base (make-backup-file-name f))))
"##,
    );
}

/// replace-regexp-in-string with subexp replacement and count.
#[test]
fn div_cx409_replace_regexp_subexp_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (list (replace-regexp-in-string "\\([a-z]+\\)" "\\1!" "hello world")
        (replace-regexp-in-string "\\([a-z]+\\)" "\\1!" "hello world" nil nil nil 1)
        (replace-regexp-in-string "a" "X" "aaa aaa" nil nil nil 2)))
"##,
    );
}

/// sort-subr with custom predicate: sorting buffer regions.
#[test]
fn div_cx409_sort_subr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "banana\napple\ncherry\ndate\n")
  (sort-subr nil 'forward-line 'end-of-line nil nil
             (lambda (a b) (string< (buffer-substring a b) (buffer-substring (car b) (cdr b)))))
  (buffer-string))
"##,
    );
}

/// describe-key-briefly: formatted key description.
#[test]
fn div_cx409_describe_key_briefly() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'help)
  (with-temp-buffer
    (let ((map (make-sparse-keymap)))
      (define-key map "a" 'forward-char)
      (list (describe-key-briefly "a" map)
            (key-description (kbd "C-c C-f"))))))
"##,
    );
}

/// help-buffer / help-setup-xref: help infrastructure.
#[test]
fn div_cx409_help_buffer_xref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'help-mode)
  (with-temp-buffer
    (help-setup-xref (list 'forward-char) (interactive-form 'forward-char))
    (list (help-buffer)
          (buffer-name (current-buffer)))))
"##,
    );
}

/// apropos: symbol searching may behave differently.
#[test]
fn div_cx409_apropos_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'apropos)
  (let ((buf (get-buffer-create "*Apropos*")))
    (apropos "forward-char")
    (prog1 (with-current-buffer buf
             (buffer-string))
      (kill-buffer buf))))
"##,
    );
}

/// format-find-file / format-insert-file: format conversion
/// on file read.
#[test]
fn div_cx409_format_find_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-file "neo-cx409-fmt-")))
  (with-temp-file f (insert "test content"))
  (unwind-protect
      (with-temp-buffer
        (format-find-file f '(""))
        (buffer-string))
    (delete-file f)))
"##,
    );
}

/// save-buffer / basic-save-buffer: save operations.
#[test]
fn div_cx409_save_buffer_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-file "neo-cx409-sv-")))
  (unwind-protect
      (with-temp-file f
        (insert "original")
        (list (buffer-modified-p)
              (buffer-file-name)))
    (delete-file f)))
"##,
    );
}

/// window-vscroll / set-window-vscroll: vertical scroll.
#[test]
fn div_cx409_window_vscroll() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert (make-string 100 ?a))
  (let ((w (selected-window)))
    (list (window-vscroll w)
          (set-window-vscroll w 10.0)
          (window-vscroll w))))
"##,
    );
}

/// compare-window-configurations: structural equality (same config).
#[test]
fn div_cx409_compare_window_config() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((c1 (current-window-configuration))
      (c2 (current-window-configuration)))
  (compare-window-configurations c1 c2))
"##,
    );
}

/// force-window-update / redisplay: redisplay triggers.
#[test]
fn div_cx409_force_window_update() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "test")
  (list (force-window-update (selected-window))
        (redisplay t)))
"##,
    );
}

/// translation-table / set-translation-table:
/// character translation tables.
#[test]
fn div_cx409_translation_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((tt (make-translation-table)))
  (list (char-table-p tt)
        (condition-case e (set-translation-table tt) (error (car e)))))
"##,
    );
}

/// sit-for with zero seconds: yields to process I/O.
#[test]
fn div_cx409_sit_for_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (sit-for 0)
      (sit-for 0.01))
"##,
    );
}
