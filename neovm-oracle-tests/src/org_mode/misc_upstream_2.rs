//! Ported upstream ERT tests from org-mode's smaller test files - batch 2.
//!
//! Covers: test-org-pcomplete, test-org-protocol, test-org-colview,
//! test-org-capture, test-org-agenda.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ── Pcomplete: drawer ────────────────────────────────────────────────

#[test]
fn upstream_org_pcomplete_drawer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-pcomplete)
  (let ((org-mode-hook nil))
    (list
     ;; Complete PROPERTIES.
     (with-temp-buffer (org-mode) (insert "* Foo\n:")
       (goto-char (point-max))
       (pcomplete) (buffer-string))
     ;; Complete DRAWER.
     (with-temp-buffer (org-mode) (insert ":DRAWER:\nContents\n:END:\n* Foo\n:D")
       (goto-char (point-max))
       (pcomplete) (buffer-string)))))"##,
    );
}

// ── Pcomplete: entity ────────────────────────────────────────────────

#[test]
fn upstream_org_pcomplete_entity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-pcomplete)
  (let ((org-mode-hook nil))
    (list
     ;; Complete alpha.
     (with-temp-buffer (org-mode) (insert "\\alp")
       (goto-char (point-max))
       (pcomplete) (buffer-string))
     ;; Complete frac12.
     (with-temp-buffer (org-mode) (insert "\\frac1")
       (goto-char (point-max))
       (pcomplete) (buffer-string)))))"##,
    );
}

// ── Pcomplete: block ─────────────────────────────────────────────────

#[test]
fn upstream_org_pcomplete_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-pcomplete)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "#+begin_")
      (goto-char (point-max))
      (pcomplete) (buffer-string))))"##,
    );
}

// ── Protocol: parse-parameters ───────────────────────────────────────

#[test]
fn upstream_org_protocol_parse_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-protocol)
  (list
   ;; Plist parameters.
   (let ((data (org-protocol-parse-parameters '(:url "abc" :title "def") nil)))
     (list (plist-get data :url) (plist-get data :title)))
   ;; New-style URL parameters.
   (let ((data (org-protocol-parse-parameters "url=abc&title=def" t)))
     (list (plist-get data :url) (plist-get data :title)))
   ;; Complex URL parameters.
   (let* ((url (concat "template=p&"
                       "url=https%3A%2F%2Forgmode.org%2Forg.html%23capture-protocol&"
                       "title=The%20Org%20Manual&"
                       "body=9.4.2%20capture%20protocol"))
          (data (org-protocol-parse-parameters url t)))
     (list (plist-get data :template)
           (plist-get data :url)
           (plist-get data :title)
           (plist-get data :body)))
   ;; Old-style slash parameters.
   (let ((data (org-protocol-parse-parameters "abc/def" nil '(:url :title))))
     (list (plist-get data :url) (plist-get data :title)))))"##,
    );
}

// ── Colview: get-format ──────────────────────────────────────────────

#[test]
fn upstream_org_colview_get_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-colview)
  (let ((org-mode-hook nil))
    (list
     ;; Default format.
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min))
       (let ((org-columns-default-format "%A"))
         (org-columns-get-format)))
     ;; COLUMNS keyword.
     (with-temp-buffer (org-mode) (insert "#+COLUMNS: %B\n* H")
       (goto-char (point-min))
       (let ((org-columns-default-format "%A"))
         (org-columns-get-format)))
     ;; Property override.
     (with-temp-buffer (org-mode)
       (insert "#+COLUMNS: %B\n* H\n:PROPERTIES:\n:COLUMNS: %C\n:END:\n** S")
       (goto-char (point-max))
       (let ((org-columns-default-format "%A"))
         (org-columns-get-format)))
     ;; Optional argument.
     (with-temp-buffer (org-mode)
       (insert "#+COLUMNS: %B\n* H\n:PROPERTIES:\n:COLUMNS: %C\n:END:\n** S")
       (goto-char (point-max))
       (let ((org-columns-default-format "%A"))
         (org-columns-get-format "%D"))))))"##,
    );
}

// ── Colview: columns-width ───────────────────────────────────────────

#[test]
fn upstream_org_colview_columns_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-colview)
  (let ((org-mode-hook nil))
    (list
     ;; Width from format.
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min))
       (let ((org-columns-default-format "%9ITEM")) (org-columns))
       (aref org-columns-current-maxwidths 0))
     ;; Width from values.
     (with-temp-buffer (org-mode)
       (insert "* H\n:PROPERTIES:\n:P: X\n:END:\n** H2\n:PROPERTIES:\n:P: XX\n:END:")
       (goto-char (point-min))
       (let ((org-columns-default-format "%P")) (org-columns))
       (aref org-columns-current-maxwidths 0))
     ;; Title width.
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min))
       (let ((org-columns-default-format "%ITEM")) (org-columns))
       (aref org-columns-current-maxwidths 0))
     ;; Stars count for ITEM.
     (with-temp-buffer (org-mode) (insert "* Head")
       (goto-char (point-min))
       (let ((org-columns-default-format "%ITEM")) (org-columns))
       (aref org-columns-current-maxwidths 0)))))"##,
    );
}

// ── Colview: columns-scope ───────────────────────────────────────────

#[test]
fn upstream_org_colview_columns_scope() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-colview)
  (let ((org-mode-hook nil))
    (list
     ;; Before first headline: view all.
     (with-temp-buffer (org-mode) (insert "Top\n* H1\n** H2\n* H3")
       (goto-char (point-min))
       (let ((org-columns-default-format "%ITEM")) (org-columns))
       (org-map-entries
        (lambda () (get-char-property (point) 'org-columns-value))))
     ;; With prefix arg: view all.
     (with-temp-buffer (org-mode)
       (insert "* H1\n** H2\n:PROPERTIES:\n:COLUMNS: %ITEM\n:END:\n*** H3\n* H4")
       (goto-char (point-min))
       (forward-line 3)
       (let ((org-columns-default-format "%ITEM")) (org-columns t))
       (org-map-entries
        (lambda () (get-char-property (point) 'org-columns-value)))))))"##,
    );
}

// ── Capture: fill-template ───────────────────────────────────────────

#[test]
fn upstream_org_capture_fill_template() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-capture)
  (let ((org-store-link-plist nil))
    (list
     ;; %(sexp) placeholder.
     (org-capture-fill-template "%(concat \"success\" \"!\")")
     ;; %<...> placeholder.
     (org-capture-fill-template "%<%Y>")
     ;; %t placeholder.
     (org-capture-fill-template "%t")
     ;; %u placeholder.
     (org-capture-fill-template "%u")
     ;; %i placeholder.
     (org-capture-fill-template "%i" "success!")
     ;; %-escaping.
     (org-capture-fill-template "\\%i" "success!")
     ;; Multiple placeholders.
     (org-capture-fill-template "%i %i" "ok"))))"##,
    );
}

// ── Agenda: org-agenda-list ──────────────────────────────────────────

#[test]
fn upstream_org_agenda_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-agenda)
  (let ((org-agenda-span 'day)
        (org-agenda-files nil)
        (org-agenda-sticky nil))
    (org-agenda-list)
    (set-buffer org-agenda-buffer-name)
    (count-lines (point-min) (point-max))))"##,
    );
}

// ── Agenda: org-agenda-skip ──────────────────────────────────────────

#[test]
fn upstream_org_agenda_skip_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-agenda)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* TODO A\n* DONE B\n* TODO C\n* DONE D")
      (goto-char (point-min))
      (let ((org-agenda-skip-function
             (lambda () (org-agenda-skip-entry-if 'todo '("DONE")))))
        (org-agenda-skip-entry-if 'todo '("DONE"))
        (point)))))"##,
    );
}

// ── Lint: add-checker extended ───────────────────────────────────────

#[test]
fn upstream_org_lint_add_checker_extended() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-lint--checkers nil))
    (org-lint-add-checker 'test-check "Test checker" #'ignore)
    (list
     ;; Checker added.
     (length org-lint--checkers)
     ;; Duplicate not added.
     (progn (org-lint-add-checker 'test-check "Other" #'ignore)
            (length org-lint--checkers))
     ;; Second checker.
     (progn (org-lint-add-checker 'test-check2 "Another" #'ignore)
            (length org-lint--checkers)))))"##,
    );
}

// ── Lint: deprecated blocks extended ─────────────────────────────────

#[test]
fn upstream_org_lint_deprecated_blocks_extended() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (list
     ;; LaTeX block.
     (with-temp-buffer (org-mode)
       (insert "#+begin_latex\n...\n#+end_latex")
       (goto-char (point-min))
       (org-lint '(deprecated-export-blocks)))
     ;; HTML block.
     (with-temp-buffer (org-mode)
       (insert "#+BEGIN_HTML\n<p>Text</p>\n#+END_HTML")
       (goto-char (point-min))
       (org-lint '(deprecated-export-blocks)))
     ;; No deprecated block.
     (with-temp-buffer (org-mode)
       (insert "#+begin_export html\n<p>Text</p>\n#+end_export")
       (goto-char (point-min))
       (org-lint '(deprecated-export-blocks))))))"##,
    );
}

// ── Lint: missing language extended ──────────────────────────────────

#[test]
fn upstream_org_lint_missing_language_extended() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-lint)
  (let ((org-mode-hook nil))
    (list
     ;; Missing language.
     (with-temp-buffer (org-mode)
       (insert "#+begin_src\n...\n#+end_src")
       (goto-char (point-min))
       (org-lint '(missing-language-in-src-block)))
     ;; Has language.
     (with-temp-buffer (org-mode)
       (insert "#+begin_src emacs-lisp\n...\n#+end_src")
       (goto-char (point-min))
       (org-lint '(missing-language-in-src-block))))))"##,
    );
}
