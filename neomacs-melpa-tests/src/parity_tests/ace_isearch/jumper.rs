use expect_test::expect;

use super::assert_ace_isearch_parity;

#[test]
fn ace_isearch_jumper_one_character_path_finishes_rewinds_and_calls_the_selected_backend() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abcdef")
               (goto-char 6)
               (let ((isearch-string "x")
                     (isearch-opoint 3)
                     (isearch-regexp nil)
                     (search-default-mode nil)
                     (isearch-success t)
                     (isearch--current-buffer nil)
                     (ace-isearch-jump-based-on-one-char t)
                     (ace-isearch-function 'test-jump)
                     (ace-isearch-use-jump t)
                     (ace-isearch-jump-delay 0.25)
                     (ace-isearch-input-length 6)
                     (ace-isearch-on-evil-mode nil)
                     calls)
                 (cl-letf (((symbol-function
                            'ace-isearch--isearch-regexp-function)
                            (lambda () nil))
                           ((symbol-function 'sit-for)
                            (lambda (delay)
                              (push (list 'sit delay) calls)
                              t))
                           ((symbol-function 'isearch-done)
                            (lambda (&rest arguments)
                              (push (cons 'done arguments) calls)))
                           ((symbol-function 'window-start)
                            (lambda () 1))
                           ((symbol-function 'window-end)
                            (lambda (&rest _arguments) 7))
                           ((symbol-function 'test-jump)
                            (lambda (character)
                              (push (list 'jump character (point)) calls)
                              'jump-result)))
                   (list
                    (ace-isearch--jumper-function)
                    (point)
                    isearch-string
                    (equal isearch--current-buffer (buffer-name))
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK ("" 3 "" t ((sit 0.25) (done t t) (jump 120 3)))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_two_character_path_passes_both_character_codes() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abcdef")
               (let ((isearch-string "Az")
                     (isearch-opoint 2)
                     (isearch-regexp nil)
                     (search-default-mode nil)
                     (isearch-success t)
                     (isearch--current-buffer nil)
                     (ace-isearch-jump-based-on-one-char nil)
                     (ace-isearch-2-function 'test-jump-two)
                     (ace-isearch-use-jump t)
                     (ace-isearch-jump-delay 0.3)
                     (ace-isearch-input-length 6)
                     (ace-isearch-on-evil-mode nil)
                     calls)
                 (cl-letf (((symbol-function
                            'ace-isearch--isearch-regexp-function)
                            (lambda () nil))
                           ((symbol-function 'sit-for)
                            (lambda (_delay) t))
                           ((symbol-function 'isearch-done)
                            (lambda (&rest arguments)
                              (push (cons 'done arguments) calls)))
                           ((symbol-function 'window-start)
                            (lambda () 1))
                           ((symbol-function 'window-end)
                            (lambda (&rest _arguments) 7))
                           ((symbol-function 'test-jump-two)
                            (lambda (first second)
                              (push (list 'jump first second (point)) calls)
                              'jump-result)))
                   (list
                    (ace-isearch--jumper-function)
                    (point)
                    isearch-string
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK ("" 2 "" ((done t t) (jump 65 122 2)))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_printing_char_policy_only_jumps_for_the_printing_command() {
    let elisp_form = r##"(let (results)
               (dolist (command '(isearch-printing-char isearch-delete-char))
                 (with-temp-buffer
                   (insert "abc")
                   (let ((isearch-string "a")
                         (isearch-opoint 1)
                         (isearch-regexp nil)
                         (search-default-mode nil)
                         (isearch-success t)
                         (isearch--current-buffer nil)
                         (this-command command)
                         (ace-isearch-jump-based-on-one-char t)
                         (ace-isearch-function 'test-jump)
                         (ace-isearch-use-jump 'printing-char)
                         (ace-isearch-jump-delay 0)
                         (ace-isearch-input-length 6)
                         calls)
                     (cl-letf (((symbol-function
                                'ace-isearch--isearch-regexp-function)
                                (lambda () nil))
                               ((symbol-function 'sit-for)
                                (lambda (_delay) t))
                               ((symbol-function 'isearch-done)
                                (lambda (&rest _arguments)
                                  (push 'done calls)))
                               ((symbol-function 'window-start)
                                (lambda () 1))
                               ((symbol-function 'window-end)
                                (lambda (&rest _arguments) 4))
                               ((symbol-function 'test-jump)
                                (lambda (character)
                                  (push (list 'jump character) calls))))
                       (push (list command
                                   (ace-isearch--jumper-function)
                                   isearch-string
                                   (nreverse calls))
                             results)))))
               (nreverse results))"##;
    let expect = expect![[
        r#"OK ((isearch-printing-char "" "" (done (jump 97))) (isearch-delete-char nil "a" nil))"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_disabled_jump_skips_function_validation_and_delay() {
    let elisp_form = r##"(let ((isearch-string "a")
                   (isearch-regexp nil)
                   (search-default-mode nil)
                   (isearch-success t)
                   (ace-isearch-jump-based-on-one-char t)
                   (ace-isearch-function nil)
                   (ace-isearch-use-jump nil)
                   (ace-isearch-input-length 6)
                   calls)
               (cl-letf (((symbol-function 'sit-for)
                          (lambda (_delay)
                            (push 'sit calls)
                            t))
                         ((symbol-function 'isearch-done)
                          (lambda (&rest _arguments)
                            (push 'done calls))))
                 (list
                  (ace-isearch--jumper-function)
                  (nreverse calls))))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_false_delay_keeps_isearch_active() {
    let elisp_form = r##"(let ((isearch-string "a")
                   (isearch-regexp nil)
                   (search-default-mode nil)
                   (isearch-success t)
                   (ace-isearch-jump-based-on-one-char t)
                   (ace-isearch-function 'test-jump)
                   (ace-isearch-use-jump t)
                   (ace-isearch-input-length 6)
                   calls)
               (cl-letf (((symbol-function
                           'ace-isearch--isearch-regexp-function)
                           (lambda () nil))
                         ((symbol-function 'sit-for)
                          (lambda (delay)
                            (push (list 'sit delay) calls)
                            nil))
                         ((symbol-function 'test-jump)
                          (lambda (_character)
                            (push 'jump calls))))
                 (list
                  (ace-isearch--jumper-function)
                  isearch-string
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil "a" ((sit 0.3)))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_regexp_search_requires_evil_override() {
    let elisp_form = r##"(let (results)
               (dolist (evil '(nil t))
                 (with-temp-buffer
                   (insert "abc")
                   (let ((isearch-string "a")
                         (isearch-opoint 1)
                         (isearch-regexp t)
                         (search-default-mode nil)
                         (isearch-success t)
                         (isearch--current-buffer nil)
                         (ace-isearch-on-evil-mode evil)
                         (ace-isearch-jump-based-on-one-char t)
                         (ace-isearch-function 'test-jump)
                         (ace-isearch-use-jump t)
                         (ace-isearch-input-length 6)
                         calls)
                     (cl-letf (((symbol-function
                                'ace-isearch--isearch-regexp-function)
                                (lambda () nil))
                               ((symbol-function 'sit-for)
                                (lambda (_delay)
                                  (push 'sit calls)
                                  t))
                               ((symbol-function 'isearch-done)
                                (lambda (&rest _arguments)
                                  (push 'done calls)))
                               ((symbol-function 'window-start)
                                (lambda () 1))
                               ((symbol-function 'window-end)
                                (lambda (&rest _arguments) 4))
                               ((symbol-function 'test-jump)
                                (lambda (character)
                                  (push (list 'jump character) calls))))
                       (push (list evil
                                   (ace-isearch--jumper-function)
                                   isearch-string
                                   (nreverse calls))
                             results)))))
               (nreverse results))"##;
    let expect = expect![[r#"OK ((nil nil "a" nil) (t "" "" (sit done (jump 97))))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_regexp_function_requires_non_nil_search_default_mode() {
    let elisp_form = r##"(let (results)
               (dolist (default-mode '(nil word))
                 (with-temp-buffer
                   (insert "abc")
                   (let ((isearch-string "a")
                         (isearch-opoint 1)
                         (isearch-regexp nil)
                         (search-default-mode default-mode)
                         (isearch-success t)
                         (isearch--current-buffer nil)
                         (ace-isearch-on-evil-mode nil)
                         (ace-isearch-jump-based-on-one-char t)
                         (ace-isearch-function 'test-jump)
                         (ace-isearch-use-jump t)
                         (ace-isearch-input-length 6)
                         calls)
                     (cl-letf (((symbol-function
                                'ace-isearch--isearch-regexp-function)
                                (lambda () 'regexp-function))
                               ((symbol-function 'sit-for)
                                (lambda (_delay)
                                  (push 'sit calls)
                                  t))
                               ((symbol-function 'isearch-done)
                                (lambda (&rest _arguments)
                                  (push 'done calls)))
                               ((symbol-function 'window-start)
                                (lambda () 1))
                               ((symbol-function 'window-end)
                                (lambda (&rest _arguments) 4))
                               ((symbol-function 'test-jump)
                                (lambda (character)
                                  (push (list 'jump character) calls))))
                       (push (list default-mode
                                   (ace-isearch--jumper-function)
                                   isearch-string
                                   (nreverse calls))
                             results)))))
               (nreverse results))"##;
    let expect = expect![[r#"OK ((nil nil "a" nil) (word "" "" (sit done (jump 97))))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_outside_visible_window_emits_the_exact_notice_before_jump() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abcdef")
               (let ((isearch-string "q")
                     (isearch-opoint 6)
                     (isearch-regexp nil)
                     (search-default-mode nil)
                     (isearch-success t)
                     (isearch--current-buffer nil)
                     (ace-isearch-jump-based-on-one-char t)
                     (ace-isearch-function 'test-jump)
                     (ace-isearch-use-jump t)
                     (ace-isearch-jump-delay 0)
                     (ace-isearch-input-length 6)
                     calls)
                 (cl-letf (((symbol-function
                            'ace-isearch--isearch-regexp-function)
                            (lambda () nil))
                           ((symbol-function 'sit-for)
                            (lambda (_delay) t))
                           ((symbol-function 'isearch-done)
                            (lambda (&rest _arguments) nil))
                           ((symbol-function 'window-start)
                            (lambda () 1))
                           ((symbol-function 'window-end)
                            (lambda (&rest _arguments) 4))
                           ((symbol-function 'message)
                            (lambda (&rest arguments)
                              (push (cons 'message arguments) calls)))
                           ((symbol-function 'test-jump)
                            (lambda (character)
                              (push (list 'jump character) calls))))
                   (list
                    (ace-isearch--jumper-function)
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ("" ((message "Notice: Character '%s' could not be found in the \"selected visible window\"." "q") (jump 113)))"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_failed_intermediate_search_calls_enabled_fallback() {
    let elisp_form = r##"(let ((isearch-string "abc")
                   (isearch-regexp nil)
                   (isearch-success nil)
                   (ace-isearch-jump-based-on-one-char t)
                   (ace-isearch-input-length 6)
                   (ace-isearch-jump-delay 0.4)
                   (ace-isearch-use-fallback-function t)
                   (ace-isearch-fallback-function 'test-fallback)
                   calls)
               (cl-letf (((symbol-function 'sit-for)
                          (lambda (delay)
                            (push (list 'sit delay) calls)
                            t))
                         ((symbol-function 'test-fallback)
                          (lambda ()
                            (push 'fallback calls)
                            'fallback-result)))
                 (list
                  (ace-isearch--jumper-function)
                  isearch-string
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (fallback-result "abc" ((sit 0.4) fallback))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_fallback_policy_success_and_delay_gate_dispatch() {
    let elisp_form = r##"(let (results)
               (dolist (configuration
                        '((nil nil t)
                          (t t t)
                          (t nil nil)))
                 (pcase-let ((`(,enabled ,success ,delay-result)
                              configuration))
                   (let ((isearch-string "abc")
                         (isearch-regexp nil)
                         (isearch-success success)
                         (ace-isearch-jump-based-on-one-char t)
                         (ace-isearch-input-length 6)
                         (ace-isearch-use-fallback-function enabled)
                         (ace-isearch-fallback-function 'test-fallback)
                         calls)
                     (cl-letf (((symbol-function 'sit-for)
                                (lambda (_delay)
                                  (push 'sit calls)
                                  delay-result))
                               ((symbol-function 'test-fallback)
                                (lambda ()
                                  (push 'fallback calls))))
                       (push (list configuration
                                   (ace-isearch--jumper-function)
                                   (nreverse calls))
                             results)))))
               (nreverse results))"##;
    let expect = expect!["OK (((nil nil t) nil (sit)) ((t t t) nil nil) ((t nil nil) nil (sit)))"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_long_query_finishes_and_calls_enabled_transition() {
    let elisp_form = r##"(let ((isearch-string "abcdef")
                   (isearch-regexp nil)
                   (isearch-success t)
                   (isearch--current-buffer nil)
                   (ace-isearch-jump-based-on-one-char t)
                   (ace-isearch-input-length 6)
                   (ace-isearch-on-evil-mode nil)
                   (ace-isearch-use-function-from-isearch t)
                   (ace-isearch-function-from-isearch 'test-transition)
                   (ace-isearch-func-delay 0.75)
                   calls)
               (cl-letf (((symbol-function 'sit-for)
                          (lambda (delay)
                            (push (list 'sit delay) calls)
                            t))
                         ((symbol-function 'isearch-done)
                          (lambda (&rest arguments)
                            (push (cons 'done arguments) calls)))
                         ((symbol-function 'test-transition)
                          (lambda ()
                            (push 'transition calls)
                            'transition-result)))
                 (list
                  (ace-isearch--jumper-function)
                  isearch-string
                  (equal isearch--current-buffer (buffer-name))
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK ("" "" t ((sit 0.75) (done t t) transition))"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_long_query_respects_transition_policy_regexp_and_delay() {
    let elisp_form = r##"(let (results)
               (dolist (configuration
                        '((nil nil nil t)
                          (t t nil t)
                          (t nil nil nil)))
                 (pcase-let ((`(,enabled ,regexp ,evil ,delay-result)
                              configuration))
                   (let ((isearch-string "abcdef")
                         (isearch-regexp regexp)
                         (isearch-success t)
                         (ace-isearch-input-length 6)
                         (ace-isearch-on-evil-mode evil)
                         (ace-isearch-use-function-from-isearch enabled)
                         (ace-isearch-function-from-isearch 'test-transition)
                         calls)
                     (cl-letf (((symbol-function 'sit-for)
                                (lambda (_delay)
                                  (push 'sit calls)
                                  delay-result))
                               ((symbol-function 'isearch-done)
                                (lambda (&rest _arguments)
                                  (push 'done calls)))
                               ((symbol-function 'test-transition)
                                (lambda ()
                                  (push 'transition calls))))
                       (push (list configuration
                                   (ace-isearch--jumper-function)
                                   (nreverse calls))
                             results)))))
               (nreverse results))"##;
    let expect =
        expect!["OK (((nil nil nil t) nil nil) ((t t nil t) nil nil) ((t nil nil nil) nil (sit)))"];
    assert_ace_isearch_parity(elisp_form, expect);
}
