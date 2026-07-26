use super::assert_ace_jump_helm_line_parity;
use expect_test::expect;

#[test]
fn ace_jump_helm_line_dispatch_is_nil_when_avy_dispatch_is_unbound() {
    let elisp_form = r##"(let ((was-bound
                    (boundp 'avy-dispatch-alist))
                   (saved
                    (and
                     (boundp 'avy-dispatch-alist)
                     avy-dispatch-alist)))
               (unwind-protect
                   (progn
                     (makunbound 'avy-dispatch-alist)
                     (ace-jump-helm-line--get-dispatch-alist))
                 (when was-bound
                   (setq avy-dispatch-alist saved))))"##;
    let expect = expect!["OK nil"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_nil_default_omits_move_only_and_ignores_nil_keys() {
    let elisp_form = r##"(let ((avy-dispatch-alist 'outer)
                   (ace-jump-helm-line-default-action nil)
                   (ace-jump-helm-line-persistent-key ?p)
                   (ace-jump-helm-line-select-key nil)
                   (ace-jump-helm-line-move-only-key ?m))
               (list
                (ace-jump-helm-line--get-dispatch-alist)
                avy-dispatch-alist
                ace-jump-helm-line-default-action))"##;
    let expect = expect!["OK (((112 . ace-jump-helm-line-action-persistent)) outer nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_persistent_default_dispatches_select_and_move_only() {
    let elisp_form = r##"(let ((avy-dispatch-alist nil)
                   (ace-jump-helm-line-default-action 'persistent)
                   (ace-jump-helm-line-persistent-key ?p)
                   (ace-jump-helm-line-select-key ?s)
                   (ace-jump-helm-line-move-only-key ?m))
               (ace-jump-helm-line--get-dispatch-alist))"##;
    let expect = expect![
        "OK ((109 . ace-jump-helm-line-action-move-only) (115 . ace-jump-helm-line-action-select))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_select_default_dispatches_persistent_and_move_only() {
    let elisp_form = r##"(let ((avy-dispatch-alist 'unrelated)
                   (ace-jump-helm-line-default-action 'select)
                   (ace-jump-helm-line-persistent-key ?p)
                   (ace-jump-helm-line-select-key ?s)
                   (ace-jump-helm-line-move-only-key ?m))
               (ace-jump-helm-line--get-dispatch-alist))"##;
    let expect = expect![
        "OK ((109 . ace-jump-helm-line-action-move-only) (112 . ace-jump-helm-line-action-persistent))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_move_only_default_dispatches_persistent_and_select() {
    let elisp_form = r##"(let ((avy-dispatch-alist nil)
                   (ace-jump-helm-line-default-action 'move-only)
                   (ace-jump-helm-line-persistent-key ?p)
                   (ace-jump-helm-line-select-key ?s)
                   (ace-jump-helm-line-move-only-key ?m))
               (ace-jump-helm-line--get-dispatch-alist))"##;
    let expect = expect![
        "OK ((115 . ace-jump-helm-line-action-select) (112 . ace-jump-helm-line-action-persistent))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_unknown_default_keeps_all_three_dispatch_actions() {
    let elisp_form = r##"(let ((avy-dispatch-alist nil)
                   (ace-jump-helm-line-default-action 'unknown)
                   (ace-jump-helm-line-persistent-key 1)
                   (ace-jump-helm-line-select-key 2)
                   (ace-jump-helm-line-move-only-key 3))
               (ace-jump-helm-line--get-dispatch-alist))"##;
    let expect = expect![
        "OK ((3 . ace-jump-helm-line-action-move-only) (2 . ace-jump-helm-line-action-select) (1 . ace-jump-helm-line-action-persistent))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_dispatch_returns_a_fresh_list_without_mutating_avy_dispatch() {
    let elisp_form = r##"(let ((avy-dispatch-alist '((?x . outer-action)))
                   (ace-jump-helm-line-default-action 'select)
                   (ace-jump-helm-line-persistent-key ?p)
                   (ace-jump-helm-line-select-key ?s)
                   (ace-jump-helm-line-move-only-key ?m))
               (let ((first
                      (ace-jump-helm-line--get-dispatch-alist))
                     (second
                      (ace-jump-helm-line--get-dispatch-alist)))
                 (list
                  first
                  second
                  (eq first second)
                  avy-dispatch-alist)))"##;
    let expect = expect![
        "OK (((109 . ace-jump-helm-line-action-move-only) (112 . ace-jump-helm-line-action-persistent)) ((109 . ace-jump-helm-line-action-move-only) (112 . ace-jump-helm-line-action-persistent)) nil ((120 . outer-action)))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
