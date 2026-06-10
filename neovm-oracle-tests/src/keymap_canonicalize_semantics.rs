//! Oracle parity tests for GNU `subr.el' `keymap-canonicalize'.

use super::common::assert_oracle_parity;

#[test]
fn oracle_keymap_canonicalize_resolves_parent_and_nil_shadowing() {
    let form = r#"
(let ((parent (make-sparse-keymap))
      (child (make-sparse-keymap)))
  (define-key parent [?a] 'parent-a)
  (define-key parent [?b] 'parent-b)
  (define-key parent [?d] 'parent-d)
  (define-key child [?a] nil)
  (define-key child [?b] 'child-b)
  (define-key child [?c] 'child-c)
  (set-keymap-parent child parent)
  (let ((canon (keymap-canonicalize child)))
    (list
     (keymapp canon)
     (keymap-parent canon)
     (lookup-key child [?a])
     (lookup-key canon [?a])
     (lookup-key canon [?b])
     (lookup-key canon [?c])
     (lookup-key canon [?d]))))"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_keymap_canonicalize_merges_duplicate_prefix_keymaps() {
    let form = r#"
(let ((parent (make-sparse-keymap))
      (child (make-sparse-keymap)))
  (define-key parent [?x ?a] 'parent-x-a)
  (define-key parent [?x ?b] 'parent-x-b)
  (define-key child [?x ?b] 'child-x-b)
  (define-key child [?x ?c] 'child-x-c)
  (set-keymap-parent child parent)
  (let* ((canon (keymap-canonicalize child))
         (prefix (lookup-key canon [?x])))
    (list
     (keymapp prefix)
     (lookup-key canon [?x ?a])
     (lookup-key canon [?x ?b])
     (lookup-key canon [?x ?c])
     (lookup-key child [?x ?a])
     (lookup-key child [?x ?b])
     (lookup-key child [?x ?c]))))"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_keymap_canonicalize_preserves_prompt_and_collapses_redefinitions() {
    let form = r#"
(let ((m (make-sparse-keymap "Menu")))
  (define-key m [?a] 'first-a)
  (define-key m [?b] 'first-b)
  (define-key m [?a] 'second-a)
  (let ((canon (keymap-canonicalize m))
        (seen nil))
    (map-keymap (lambda (key binding)
                  (push (cons key binding) seen))
                canon)
    (list
     (keymap-prompt canon)
     (lookup-key canon [?a])
     (lookup-key canon [?b])
     (length (delq nil (mapcar (lambda (entry)
                                  (and (eq (car entry) ?a) entry))
                                seen))))))"#;
    assert_oracle_parity(form);
}
