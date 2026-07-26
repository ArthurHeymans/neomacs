use expect_test::expect;

use super::{assert_ace_isearch_parity, assert_ace_isearch_with_prelude_parity};

#[test]
fn ace_isearch_pop_mark_dispatches_to_ace_jump_only_for_the_ace_backend() {
    let elisp_form = r##"(progn
               (setq ace-isearch--ace-jump-or-avy 'ace-jump)
               (let (calls)
                 (cl-letf (((symbol-function 'ace-jump-mode-pop-mark)
                            (lambda ()
                              (push 'ace calls)
                              'ace-result))
                           ((symbol-function 'avy-pop-mark)
                            (lambda ()
                              (push 'avy calls)
                              'avy-result)))
                   (list
                    (ace-isearch-pop-mark)
                    (nreverse calls)))))"##;
    let expect = expect!["OK (ace-result (ace))"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_pop_mark_dispatches_to_avy_only_for_the_avy_backend() {
    let elisp_form = r##"(progn
               (setq ace-isearch--ace-jump-or-avy 'avy)
               (let (calls)
                 (cl-letf (((symbol-function 'ace-jump-mode-pop-mark)
                            (lambda ()
                              (push 'ace calls)
                              'ace-result))
                           ((symbol-function 'avy-pop-mark)
                            (lambda ()
                              (push 'avy calls)
                              'avy-result)))
                   (list
                    (ace-isearch-pop-mark)
                    (nreverse calls)))))"##;
    let expect = expect!["OK (avy-result (avy))"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_pop_mark_is_a_noop_for_nil_and_unknown_backends() {
    let elisp_form = r##"(mapcar
               (lambda (backend)
                 (setq ace-isearch--ace-jump-or-avy backend)
                 (ace-isearch-pop-mark))
               '(nil unknown))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_helm_occur_adapter_quotes_plain_queries_and_preserves_regex_queries() {
    let elisp_form = r##"(let (results)
               (dolist (regexp '(nil t))
                 (let ((isearch-string "a+b")
                       (isearch-regexp regexp)
                       calls)
                   (cl-letf (((symbol-function 'isearch-update-ring)
                              (lambda (&rest arguments)
                                (push (cons 'ring arguments) calls)))
                             ((symbol-function 'isearch-done)
                              (lambda (&rest arguments)
                                (push (cons 'done arguments) calls)
                                (error "ignored done")))
                             ((symbol-function 'helm-multi-occur-1)
                              (lambda (buffers query)
                                (push (list 'helm
                                            (mapcar #'buffer-name buffers)
                                            query)
                                      calls)
                                'helm-result)))
                     (push (list regexp
                                 (ace-isearch-helm-occur-from-isearch)
                                 (nreverse calls))
                           results))))
               (nreverse results))"##;
    let expect = expect![[
        r#"OK ((nil helm-result ((ring "a+b" nil) (done t t) (helm ("*scratch*") "a\\+b"))) (t helm-result ((ring "a+b" t) (done t t) (helm ("*scratch*") "a+b"))))"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_helm_swoop_adapter_quotes_plain_queries_and_uses_query_keyword() {
    let elisp_form = r##"(let ((isearch-string "a+b")
                   (isearch-regexp nil)
                   calls)
               (cl-letf (((symbol-function 'isearch-update-ring)
                          (lambda (&rest arguments)
                            (push (cons 'ring arguments) calls)))
                         ((symbol-function 'isearch-done)
                          (lambda (&rest arguments)
                            (push (cons 'done arguments) calls)
                            'done-result))
                         ((symbol-function 'helm-swoop)
                          (lambda (&rest arguments)
                            (push (cons 'helm arguments) calls)
                            'helm-result)))
                 (list
                  (ace-isearch-helm-swoop-from-isearch)
                  (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (helm-result ((ring "a+b" nil) (done t t) (helm :query "a\\+b")))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_swiper_adapter_preserves_regex_queries_and_call_order() {
    let elisp_form = r##"(let ((isearch-string "a+b")
                   (isearch-regexp t)
                   calls)
               (cl-letf (((symbol-function 'isearch-update-ring)
                          (lambda (&rest arguments)
                            (push (cons 'ring arguments) calls)))
                         ((symbol-function 'isearch-done)
                          (lambda (&rest arguments)
                            (push (cons 'done arguments) calls)
                            'done-result))
                         ((symbol-function 'swiper)
                          (lambda (query)
                            (push (list 'swiper query) calls)
                            'swiper-result)))
                 (list
                  (ace-isearch-swiper-from-isearch)
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (swiper-result ((ring "a+b" t) (done t t) (swiper "a+b")))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_consult_adapter_quotes_plain_queries_and_call_order() {
    let elisp_form = r##"(let ((isearch-string "a+b")
                   (isearch-regexp nil)
                   calls)
               (cl-letf (((symbol-function 'isearch-update-ring)
                          (lambda (&rest arguments)
                            (push (cons 'ring arguments) calls)))
                         ((symbol-function 'isearch-done)
                          (lambda (&rest arguments)
                            (push (cons 'done arguments) calls)
                            'done-result))
                         ((symbol-function 'consult-line)
                          (lambda (query)
                            (push (list 'consult query) calls)
                            'consult-result)))
                 (list
                  (ace-isearch-consult-line-from-isearch)
                  (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (consult-result ((ring "a+b" nil) (done t t) (consult "a\\+b")))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_adapter_clears_nonincremental_state_during_isearch_exit_and_restores_it() {
    let elisp_form = r##"(let ((isearch-string "query")
                   (isearch-regexp nil)
                   (search-nonincremental-instead t)
                   calls)
               (cl-letf (((symbol-function 'isearch-update-ring)
                          (lambda (&rest _arguments) nil))
                         ((symbol-function 'isearch-done)
                          (lambda (&rest arguments)
                            (push
                             (cons
                              search-nonincremental-instead
                              arguments)
                             calls)
                            (error "synthetic isearch exit")))
                         ((symbol-function 'swiper)
                          (lambda (query)
                            (push
                             (list
                              'swiper
                              query
                              search-nonincremental-instead)
                             calls)
                            'swiper-result)))
                 (list
                  (ace-isearch-swiper-from-isearch)
                  search-nonincremental-instead
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (swiper-result t ((nil t t) (swiper "query" t)))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jump_during_isearch_ace_backend_exits_then_jumps_in_window_scope() {
    let elisp_form = r##"(progn
               (setq ace-isearch--ace-jump-or-avy 'ace-jump)
               (let ((isearch-string "a+b")
                     (ace-isearch-input-length 6)
                     (ace-jump-mode-scope 'outer)
                     calls)
                 (cl-letf (((symbol-function 'isearch-exit)
                            (lambda ()
                              (push (list 'exit ace-jump-mode-scope) calls)))
                           ((symbol-function 'ace-jump-do)
                            (lambda (regexp)
                              (push
                               (list 'jump regexp ace-jump-mode-scope)
                               calls)
                              'jump-result))
                           ((symbol-function 'avy-isearch)
                            (lambda ()
                              (push 'avy calls))))
                   (list
                    (ace-isearch-jump-during-isearch)
                    ace-jump-mode-scope
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK (jump-result outer ((exit window) (jump "a\\+b" window)))"#]];
    assert_ace_isearch_with_prelude_parity(
        "(defvar ace-jump-mode-scope nil)\n(provide 'ace-jump-mode)",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_isearch_jump_during_isearch_avy_backend_disables_all_windows() {
    let elisp_form = r##"(progn
               (setq ace-isearch--ace-jump-or-avy 'avy)
               (let ((isearch-string "ab")
                     (ace-isearch-input-length 6)
                     (avy-all-windows 'outer)
                     calls)
                 (cl-letf (((symbol-function 'isearch-exit)
                            (lambda () (push 'exit calls)))
                           ((symbol-function 'ace-jump-do)
                            (lambda (_regexp) (push 'jump calls)))
                           ((symbol-function 'avy-isearch)
                            (lambda ()
                              (push (list 'avy avy-all-windows) calls)
                              'avy-result)))
                   (list
                    (ace-isearch-jump-during-isearch)
                    avy-all-windows
                    (nreverse calls)))))"##;
    let expect = expect!["OK (avy-result outer ((avy nil)))"];
    assert_ace_isearch_with_prelude_parity(
        "(defvar avy-all-windows nil)\n(provide 'avy)",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_isearch_jump_during_isearch_skips_long_queries_and_unknown_backends() {
    let elisp_form = r##"(let (calls)
               (cl-letf (((symbol-function 'isearch-exit)
                          (lambda () (push 'exit calls)))
                         ((symbol-function 'ace-jump-do)
                          (lambda (_regexp) (push 'jump calls)))
                         ((symbol-function 'avy-isearch)
                          (lambda () (push 'avy calls))))
                 (list
                  (let ((isearch-string "abcdef")
                        (ace-isearch-input-length 6))
                    (setq ace-isearch--ace-jump-or-avy 'ace-jump)
                    (ace-isearch-jump-during-isearch))
                  (let ((isearch-string "a")
                        (ace-isearch-input-length 6))
                    (setq ace-isearch--ace-jump-or-avy 'unknown)
                    (ace-isearch-jump-during-isearch))
                  (nreverse calls))))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_ace_isearch_parity(elisp_form, expect);
}
