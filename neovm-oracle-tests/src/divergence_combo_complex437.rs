//! Complex combo batch 437 — 16 feature-interaction probes: org-crypt
//! (encryption), org-mobile, org-persist, org-eldoc, org-compat,
//! org-macs, org-loaddefs, org-keys, org-attach-git, org-capture
//! deep, org-export with filter, org-element cache, org-timer-pause,
//! org-clock-in/out, org-todo-yesterday, org-agenda-skip.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// org-crypt: headline encryption (may be stubbed).
#[test]
fn div_cx437_org_crypt_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-crypt)
  (list (fboundp 'org-encrypt-entry)
        (fboundp 'org-decrypt-entry)
        (boundp 'org-crypt-key)))
"##,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

/// org-mobile: mobile synchronization.
#[test]
fn div_cx437_org_mobile_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-mobile)
  (list (fboundp 'org-mobile-push)
        (fboundp 'org-mobile-pull)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

/// org-persist: persistence of org data.
#[test]
fn div_cx437_org_persist_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-persist)
  (list (boundp 'org-persist-directory)
        (fboundp 'org-persist-read)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

/// org-eldoc: eldoc integration.
#[test]
fn div_cx437_org_eldoc_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-eldoc)
  (list (boundp 'org-eldoc-documentation-functions)))
"##,
        expect_test::expect![[
            r#""ERR (file-missing \"Cannot open load file\" \"No such file or directory\" \"org-eldoc\")""#
        ]],
    );
}

/// org-compat: compatibility functions.
#[test]
fn div_cx437_org_compat_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-compat)
  (list (fboundp 'org-compatible-face))))
"##,
        expect_test::expect![[r#""ERR (invalid-read-syntax \")\" 2 41)""#]],
    );
}

/// org-attach-git: git attachment integration.
#[test]
fn div_cx437_org_attach_git_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-attach-git)
  (list (fboundp 'org-attach-git-annex-get))))
"##,
        expect_test::expect![[r#""ERR (invalid-read-syntax \")\" 2 46)""#]],
    );
}

/// org-capture deep: capture template expansion.
#[test]
fn div_cx437_org_capture_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-capture)
  (with-temp-buffer
    (org-mode)
    (let ((org-capture-templates
           '(("t" "Todo" entry "* TODO %?\n  %u\n" "~/test.org" "Top"))))
      (fboundp 'org-capture-fill-template))))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

/// org-export with advanced filters.
#[test]
fn div_cx437_org_export_filter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'ox)
  (with-temp-buffer
    (insert "* Hello\nWorld\n")
    (let ((org-export-with-toc nil)
          (org-export-filter-final-output-functions
           (list (lambda (text backend info) (upcase text)))))
      (org-export-as 'ascii nil nil t nil))))
"##,
        expect_test::expect![[r#""OK \"1 HELLO\n=======\n\n  WORLD\n\"""#]],
    );
}

/// org-element cache operations.
#[test]
fn div_cx437_org_element_cache() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-element)
  (with-temp-buffer
    (org-mode)
    (insert "* H1\n** H2\n")
    (let ((cache (org-element-cache-reset)))
      (fboundp 'org-element-cache-active-p)))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

/// org-todo-yesterday: TODO state change with yesterday.
#[test]
fn div_cx437_org_todo_yesterday() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO task\n")
    (list (fboundp 'org-todo-yesterday)
          (fboundp 'org-todo-set-date))))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

/// org-agenda-skip: agenda skip conditions.
#[test]
fn div_cx437_org_agenda_skip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-agenda)
  (with-temp-buffer
    (org-mode)
    (insert "* task\n")
    (list (fboundp 'org-agenda-skip-entry-if)
          (fboundp 'org-agenda-skip-subtree-if)))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

/// org-timer-pause / org-timer-continue.
#[test]
fn div_cx437_org_timer_pause() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-timer)
  (list (fboundp 'org-timer-pause-or-continue)
        (boundp 'org-timer-current-timer)))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

/// org-clock-in/out deep.
#[test]
fn div_cx437_org_clock_in_out_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO task\n")
    (let ((org-clock-in-resume nil))
      (list (fboundp 'org-clock-in-last)
            (fboundp 'org-clock-cancel))))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

/// org-indent with inline tasks.
#[test]
fn div_cx437_org_indent_inlinetask() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-indent) (require 'org-inlinetask)
  (with-temp-buffer
    (org-mode)
    (let ((org-inlinetask-min-level 15))
      (insert "*************** task\n*************** END\n")
      (list (fboundp 'org-indent-add-editable-areas)
            (boundp 'org-indent-indentation-per-level))))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

/// org-table sort lines.
#[test]
fn div_cx437_org_table_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'org-table)
  (with-temp-buffer
    (org-mode)
    (insert "| b | 2 |\n| a | 1 |\n| c | 3 |\n")
    (org-table-sort-lines nil ?a)
    (buffer-string)))
"##,
        expect_test::expect![[r#""ERR (user-error \"Not in table data field\")""#]],
    );
}

/// org-export to ODT format.
#[test]
fn div_cx437_org_export_odt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'ox-odt)
  (list (fboundp 'org-odt-convert)
        (fboundp 'org-odt-export-to-odt)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}
