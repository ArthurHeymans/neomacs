//! Ported upstream ERT tests from org-mode's test-org-lint.el (9.7.11).

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ── Lint: add-checker ────────────────────────────────────────────────

#[test]
fn upstream_org_lint_add_checker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (list
   ;; Valid checker.
   (let ((org-lint--checkers nil))
     (org-lint-add-checker 'check "check" #'ignore)
     (length org-lint--checkers))
   ;; Duplicate name: not added twice.
   (let ((org-lint--checkers nil))
     (org-lint-add-checker 'check "check" #'ignore)
     (org-lint-add-checker 'check "other check" #'ignore)
     (length org-lint--checkers))))"##,
    );
}

// ── Lint: duplicate-custom-id ────────────────────────────────────────

#[test]
fn upstream_org_lint_duplicate_custom_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (list
     ;; Duplicate: detected.
     (with-temp-buffer (org-mode)
       (insert "* H1\n:PROPERTIES:\n:CUSTOM_ID: foo\n:END:\n\n* H2\n:PROPERTIES:\n:CUSTOM_ID: foo\n:END:")
       (goto-char (point-min))
       (org-lint '(duplicate-custom-id)))
     ;; No duplicate.
     (with-temp-buffer (org-mode)
       (insert "* H1\n:PROPERTIES:\n:CUSTOM_ID: foo\n:END:\n\n* H2\n:PROPERTIES:\n:CUSTOM_ID: bar\n:END:")
       (goto-char (point-min))
       (org-lint '(duplicate-custom-id))))))"##,
    );
}

// ── Lint: duplicate-name ─────────────────────────────────────────────

#[test]
fn upstream_org_lint_duplicate_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (list
     ;; Duplicate: detected.
     (with-temp-buffer (org-mode)
       (insert "#+name: foo\nParagraph1\n\n#+name: foo\nParagraph 2")
       (goto-char (point-min))
       (org-lint '(duplicate-name)))
     ;; No duplicate.
     (with-temp-buffer (org-mode)
       (insert "#+name: foo\nParagraph1\n\n#+name: bar\nParagraph 2")
       (goto-char (point-min))
       (org-lint '(duplicate-name))))))"##,
    );
}

// ── Lint: duplicate-target ───────────────────────────────────────────

#[test]
fn upstream_org_lint_duplicate_target() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (list
     ;; Duplicate: detected.
     (with-temp-buffer (org-mode)
       (insert "<<foo>> <<foo>>")
       (goto-char (point-min))
       (org-lint '(duplicate-target)))
     ;; No duplicate.
     (with-temp-buffer (org-mode)
       (insert "<<foo>> <<bar>>")
       (goto-char (point-min))
       (org-lint '(duplicate-target))))))"##,
    );
}

// ── Lint: duplicate-footnote-definition ──────────────────────────────

#[test]
fn upstream_org_lint_duplicate_footnote_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (list
     ;; Duplicate: detected.
     (with-temp-buffer (org-mode)
       (insert "[fn:1] Definition 1\n\n[fn:1] Definition 2")
       (goto-char (point-min))
       (org-lint '(duplicate-footnote-definition)))
     ;; No duplicate.
     (with-temp-buffer (org-mode)
       (insert "[fn:1] Definition 1\n\n[fn:2] Definition 2")
       (goto-char (point-min))
       (org-lint '(duplicate-footnote-definition))))))"##,
    );
}

// ── Lint: orphaned-affiliated-keywords ───────────────────────────────

#[test]
fn upstream_org_lint_orphaned_affiliated_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+name: foo")
      (goto-char (point-min))
      (org-lint '(orphaned-affiliated-keywords)))))"##,
    );
}

// ── Lint: deprecated-export-blocks ───────────────────────────────────

#[test]
fn upstream_org_lint_deprecated_export_blocks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+begin_latex\n...\n#+end_latex")
      (goto-char (point-min))
      (org-lint '(deprecated-export-blocks)))))"##,
    );
}

// ── Lint: deprecated-header-syntax ───────────────────────────────────

#[test]
fn upstream_org_lint_deprecated_header_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (list
     ;; Keyword.
     (with-temp-buffer (org-mode)
       (insert "#+property: cache yes")
       (goto-char (point-min))
       (org-lint '(deprecated-header-syntax)))
     ;; Property drawer.
     (with-temp-buffer (org-mode)
       (insert "* H\n:PROPERTIES:\n:cache: yes\n:END:")
       (goto-char (point-min))
       (org-lint '(deprecated-header-syntax))))))"##,
    );
}

// ── Lint: missing-language-in-src-block ──────────────────────────────

#[test]
fn upstream_org_lint_missing_language_in_src_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+begin_src\n...\n#+end_src")
      (goto-char (point-min))
      (org-lint '(missing-language-in-src-block)))))"##,
    );
}

// ── Lint: missing-backend-in-export-block ────────────────────────────

#[test]
fn upstream_org_lint_missing_backend_in_export_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+begin_export\n...\n#+end_export")
      (goto-char (point-min))
      (org-lint '(missing-backend-in-export-block)))))"##,
    );
}

// ── Lint: special block with no name ─────────────────────────────────

#[test]
fn upstream_org_lint_special_block_no_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_SPECIAL*\nContents\n#+END_SPECIAL*")
      (goto-char (point-min))
      (org-lint '(special-block-with-parameters)))))"##,
    );
}

// ── Lint: obsolete-syntax ────────────────────────────────────────────

#[test]
fn upstream_org_lint_obsolete_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (list
     ;; Old export blocks.
     (with-temp-buffer (org-mode)
       (insert "#+BEGIN_HTML\n<p>Text</p>\n#+END_HTML")
       (goto-char (point-min))
       (org-lint '(deprecated-export-blocks)))
     ;; Old LaTeX block.
     (with-temp-buffer (org-mode)
       (insert "#+BEGIN_LaTeX\n\\textbf{Text}\n#+END_LaTeX")
       (goto-char (point-min))
        (org-lint '(deprecated-export-blocks))))))"##,
    );
}

#[test]
fn org_lint_multi_checker_report_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+NAME: dup\n#+NAME: dup\n")
      (insert "#+begin_src\nmissing language\n#+end_src\n")
      (insert "[fn:missing]\n")
      (insert "#+BEGIN_HTML\n<p>raw</p>\n#+END_HTML\n")
      (insert "* TODO Task\nSCHEDULED: <2026-05-27 Wed>\nDEADLINE: <2026-05-26 Tue>\n")
      (let* ((ast (org-element-parse-buffer))
             (dup-name (org-lint-duplicate-name ast))
             (no-lang (org-lint-missing-language-in-src-block ast))
             (undef-fn (org-lint-undefined-footnote-reference ast))
             (deprecated (org-lint-deprecated-export-blocks ast))
             (sched-after-deadline
              (condition-case nil
                  (org-lint-scheduled-after-deadline ast)
                (error nil))))
        (list (mapcar (lambda (r) (list (car r) (nth 1 r))) dup-name)
              (mapcar (lambda (r) (list (car r) (nth 1 r))) no-lang)
              (mapcar (lambda (r) (list (car r) (nth 1 r))) undef-fn)
              (mapcar (lambda (r) (list (car r) (nth 1 r))) deprecated)
              (length dup-name)
              (length no-lang)
              (length undef-fn))))))"##,
    );
}
