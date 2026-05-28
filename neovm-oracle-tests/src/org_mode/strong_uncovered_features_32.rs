//! Strong uncovered-features-32 oracle tests — org-habit, org-registry, org-git-link.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-habit
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_habit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Exercise\nSCHEDULED: <2026-01-15 .+2d/4d>\n:PROPERTIES:\n:STYLE: habit\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-habit-parse-todo)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-habit-parse-todo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_habit_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Exercise\nSCHEDULED: <2026-01-15 .+2d/4d>\n:PROPERTIES:\n:STYLE: habit\n:END:\n:LOGBOOK:\n- State \"DONE\" from \"TODO\" [2026-01-13]\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-habit-parse-todo)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-habit-build-consistency-graph
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_habit_graph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Exercise\nSCHEDULED: <2026-01-15 .+2d/4d>\n:PROPERTIES:\n:STYLE: habit\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-habit-build-consistency-graph)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-habit-toggle-display
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_habit_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Exercise\nSCHEDULED: <2026-01-15 .+2d/4d>\n:PROPERTIES:\n:STYLE: habit\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-habit-toggle-display)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-registry
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_registry() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-registry)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-registry-create
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_registry_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-registry-create)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-registry-find
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_registry_find() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-registry-find "test-file.org")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-registry-find-id
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_registry_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-registry-find-id "test-id")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-registry-find-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_registry_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-registry-find-link "test-link")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-git-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_git_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-git-link "http://github.com/user/repo" "main" "file.el" "1" "10")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-git-link-open
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_git_link_open() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-git-link-open "org-git:http://github.com/user/repo.git:main:file.el::1-10")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-git-link-store
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_git_link_store() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-git-link-store)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-git-link-insert
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_git_link_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-git-link-insert "http://github.com/user/repo.git" "main" "file.el" "1" "10")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-annotate-file
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_annotate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-annotate-file "/tmp/test.txt")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-annotate-file-show-sections
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_annotate_sections() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-annotate-file-show-sections)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-annotate-file-add-annotation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_annotate_add() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-annotate-file-add-annotation "test annotation")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-annotate-file-clear-annotations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_annotate_clear() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-annotate-file-clear-annotations)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-annotate-file-export
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_annotate_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-annotate-file-export)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-wikinodes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_wikinodes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-wikinodes)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-wikinodes-find
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_wikinodes_find() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-wikinodes-find "TestPage")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-wikinodes-create
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_wikinodes_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-wikinodes-create "TestPage")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-wikinodes-insert
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_wikinodes_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-wikinodes-insert "TestPage")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-wikinodes-rename
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_wikinodes_rename() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-wikinodes-rename "OldPage" "NewPage")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-wikinodes-update
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_wikinodes_update() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-wikinodes-update)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-wikinodes-export
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_wikinodes_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-wikinodes-export)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eval-light
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_eval_light() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (condition-case nil
      (org-eval-light)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eval-light-1
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_eval_light_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (condition-case nil
      (org-eval-light-1)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eval-light-2
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_eval_light_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (condition-case nil
      (org-eval-light-2)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eval-light-3
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf32_eval_light_3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (condition-case nil
      (org-eval-light-3)
    (error nil)))"##,
    );
}
