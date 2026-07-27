use expect_test::expect;

use super::assert_apel_source_parity;

#[test]
fn alist_crud_mutates_existing_cells_and_preserves_unrelated_entries() {
    let elisp_form = r##"(let* ((alist (list (cons "alpha" 1)
                                      (cons "beta" 2)))
                           (alpha-cell (car alist)))
                      (setq alist (put-alist "alpha" 10 alist))
                      (setq alist (put-alist "gamma" 3 alist))
                      (let ((same-cell (eq alpha-cell (cdr alist))))
                        (setq alist (del-alist "beta" alist))
                        (list alist same-cell alpha-cell)))"##;
    let expect = expect![[r#"OK ((("gamma" . 3) #1=("alpha" . 10)) nil #1#)"#]];
    assert_apel_source_parity("alist.el", elisp_form, expect);
}

#[test]
fn symbol_backed_alist_workflow_sets_modifies_and_removes_values() {
    let elisp_form = r##"(let ((inventory '((apple . 2) (pear . 3))))
                      (set-alist 'inventory 'apple 5)
                      (set-modified-alist
                       'inventory '((pear . 7) (orange . 11)))
                      (remove-alist 'inventory 'apple)
                      (list inventory
                            (modify-alist
                             '((pear . 21) (banana . 4))
                             inventory)
                            inventory))"##;
    let expect = expect!["OK (#1=((apple . 2) (pear . 21)) ((banana . 4) . #1#) #1#)"];
    assert_apel_source_parity("alist.el", elisp_form, expect);
}

#[test]
fn vector_association_distinguishes_equal_keys_and_missing_values() {
    let elisp_form = r##"(let ((records (list ["alpha" 1] ["beta" 2]
                                       ["alpha" 3] [alpha 4])))
                      (list (vassoc "alpha" records)
                            (vassoc (copy-sequence "beta") records)
                            (vassoc 'alpha records)
                            (vassoc "missing" records)))"##;
    let expect = expect![[r#"OK (["alpha" 1] ["beta" 2] [alpha 4] nil)"#]];
    assert_apel_source_parity("alist.el", elisp_form, expect);
}

#[test]
fn typed_field_unification_handles_exact_wildcard_custom_and_failure_cases() {
    let elisp_form = r##"(progn
                      (fset 'field-unifier-for-ci
                            (lambda (a b)
                              (and (string-equal (downcase (cadr a))
                                                 (downcase (cadr b)))
                                   (list nil
                                         (list 'ci (downcase (cadr a)))
                                         nil))))
                      (list (field-unifier-for-default '(kind x) '(kind x))
                            (field-unifier-for-default '(kind) '(kind fallback))
                            (field-unifier-for-default '(kind left) '(kind))
                            (field-unifier-for-default '(kind x) '(kind y))
                            (field-unify '(ci "HELLO") '(ci "hello"))
                            (field-unify '(unknown 4) '(unknown 4))
                            (field-unify '(unknown 4) '(unknown 5))))"##;
    let expect = expect![[
        r#"OK ((nil (kind x) nil) (nil (kind fallback) nil) (nil (kind left) nil) nil (nil (ci "hello") nil) (nil (unknown 4) nil) nil)"#
    ]];
    assert_apel_source_parity("atype.el", elisp_form, expect);
}

#[test]
fn associative_type_unification_tracks_consumed_and_remaining_constraints() {
    let elisp_form = r##"(list
                      (assoc-unify '((kind note) (lang en))
                                   '((lang en) (kind note) (extra t)))
                      (assoc-unify '((kind note) (lang))
                                   '((lang ja) (kind note) (extra t)))
                      (assoc-unify '((kind note) (lang en))
                                   '((lang ja) (kind note)))
                      (get-unified-alist
                       '(((kind note) (lang en))
                         ((kind task) (lang en)))
                       '((lang en) (kind note) (extra t))))"##;
    let expect = expect![
        "OK ((nil ((kind note) (lang en) (extra t)) nil) (nil ((kind note) (lang ja) (extra t)) nil) nil ((kind note) (lang en) (extra t)))"
    ];
    assert_apel_source_parity("atype.el", elisp_form, expect);
}

#[test]
fn associative_type_crud_supports_ignore_remove_replace_and_insert_policies() {
    let elisp_form = r##"(progn
                      (setq apel-atype-table
                            '(((kind note) (lang en))
                              ((kind task) (lang en))))
                      (list
                       (delete-atype apel-atype-table '((kind note)))
                       (progn
                         (setq apel-atype-copy (copy-tree apel-atype-table))
                         (remove-atype 'apel-atype-copy '((kind task)))
                         apel-atype-copy)
                       (replace-atype
                        (copy-tree apel-atype-table)
                        '((kind note))
                        '((kind note) (lang ja)))
                       (progn
                         (setq apel-atype-copy (copy-tree apel-atype-table))
                         (set-atype 'apel-atype-copy '((kind note) (lang ja))
                                    'ignore '(lang))
                         apel-atype-copy)
                       (progn
                         (setq apel-atype-copy (copy-tree apel-atype-table))
                         (set-atype 'apel-atype-copy '((kind note) (lang ja))
                                    'remove '((kind note)))
                         apel-atype-copy)
                       (progn
                         (setq apel-atype-copy (copy-tree apel-atype-table))
                         (set-atype 'apel-atype-copy '((kind note) (lang ja))
                                    'replacement)
                         apel-atype-copy)
                       (progn
                         (setq apel-atype-copy (copy-tree apel-atype-table))
                         (set-atype 'apel-atype-copy '((kind event) (lang en))
                                    'replacement)
                         apel-atype-copy)))"##;
    let expect = expect![
        "OK ((((kind task) (lang en))) (((kind note) (lang en))) (((kind note) (lang ja)) ((kind task) (lang en))) (((kind note) (lang ja)) ((kind task) (lang en))) (((kind note) (lang ja)) ((kind task) (lang en))) (((kind note) (lang ja)) ((kind note) (lang en)) ((kind task) (lang en))) (((kind event) (lang en)) ((kind note) (lang en)) ((kind task) (lang en))))"
    ];
    assert_apel_source_parity("atype.el", elisp_form, expect);
}

#[test]
fn calist_package_defines_custom_matchers_and_switches_namespaces() {
    let elisp_form = r##"(let ((package
                           (make-calist-package 'apel-test-mail)))
                      (in-calist-package 'apel-test-mail)
                        (define-calist-field-match-method
                          'folder
                          (lambda (calist field-type field-value)
                            (let ((actual (cdr (assq field-type calist))))
                              (and actual
                                   (string-prefix-p field-value actual)
                                   calist))))
                        (list (vectorp package)
                              (length package)
                              (eq package
                                  (find-calist-package 'apel-test-mail))
                              (eq package calist-field-match-method-obarray)
                              (functionp (calist-field-match-method 'folder))
                              (calist-field-match
                               '((folder . "inbox")) 'folder "in")
                              (calist-field-match
                               '((priority . 2)) 'priority 2)
                              (calist-field-match
                               '((priority . 2)) 'priority 3)))"##;
    let expect = expect![[r#"OK (t 7 t t t ((folder . "inbox")) ((priority . 2)) nil)"#]];
    assert_apel_source_parity("calist.el", elisp_form, expect);
}

#[test]
fn condition_tree_build_match_and_partial_match_model_real_message_routing() {
    let elisp_form = r##"(let ((calist-package-alist nil))
                      (make-calist-package 'mail nil)
                      (use-calist-package 'mail)
                      (let ((tree nil))
                        (setq tree
                              (ctree-add-calist-strictly
                               tree '((folder . "inbox") (priority . high))))
                        (setq tree
                              (ctree-add-calist-with-default
                               tree '((folder . "inbox") (priority . medium))))
                        (setq tree
                              (ctree-add-calist-with-default
                               tree '((folder . "archive") (priority . low))))
                        (list tree
                              (ctree-match-calist
                               tree '((folder . "inbox") (priority . high)))
                              (ctree-match-calist
                               tree '((folder . "inbox") (priority . medium)))
                              (ctree-match-calist
                               tree '((folder . "archive") (priority . low)))
                              (ctree-match-calist-partially
                               tree '((folder . "inbox")))
                              (ctree-find-calist
                               tree '((folder . "inbox")) t))))"##;
    let expect = expect![[
        r#"OK ((folder (t) ("archive" priority (low)) ("inbox" priority (t) (medium) (high))) ((folder . "inbox") (priority . high)) ((folder . "inbox") (priority . medium)) ((folder . "archive") (priority . low)) ((priority . medium) (folder . "inbox")) (#1=((folder . "inbox")) ((priority . medium) . #1#) ((priority . high) . #1#) ((priority . t) . #1#)))"#
    ]];
    assert_apel_source_parity("calist.el", elisp_form, expect);
}

#[test]
fn condition_tree_strict_and_default_updates_resolve_overlapping_rules() {
    let elisp_form = r##"(let ((calist-package-alist nil))
                      (make-calist-package 'routing nil)
                      (use-calist-package 'routing)
                      (progn
                        (setq apel-routing-tree nil)
                        (ctree-set-calist-strictly
                         'apel-routing-tree
                         '((kind . mail) (state . unread)))
                        (ctree-set-calist-with-default
                         'apel-routing-tree
                         '((kind . mail) (state . read)))
                        (ctree-set-calist-strictly
                         'apel-routing-tree
                         '((kind . note) (state . unread)))
                        (ctree-set-calist-with-default
                         'apel-routing-tree
                         '((kind . note) (state . archived)))
                        (list apel-routing-tree
                              (ctree-match-calist
                               apel-routing-tree
                               '((kind . mail) (state . unread)))
                              (ctree-match-calist
                               apel-routing-tree
                               '((kind . mail) (state . read)))
                              (ctree-match-calist
                               apel-routing-tree
                               '((kind . note) (state . archived))))))"##;
    let expect = expect![
        "OK ((kind (note state (t) (archived) (unread)) (mail state (t) (read) (unread))) ((kind . mail) (state . unread)) ((kind . mail) (state . read)) ((kind . note) (state . archived)))"
    ];
    assert_apel_source_parity("calist.el", elisp_form, expect);
}
