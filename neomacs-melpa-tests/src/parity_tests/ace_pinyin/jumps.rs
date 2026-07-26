use super::assert_ace_pinyin_parity;
use expect_test::expect;

#[test]
fn ace_pinyin_jump_impl_avy_backend_builds_and_jumps_with_prefix() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy t)
             (ace-pinyin-enable-punctuation-translation nil)
             (ace-pinyin-simplified-chinese-only-p nil))
         (setq ace-pinyin--test-events nil)
         (cl-letf
             (((symbol-function
                'pinyinlib-build-regexp-char)
               (lambda (query no-punctuation traditional prefix)
                 (push
                  (list 'build
                        query
                        no-punctuation
                        traditional
                        prefix)
                  ace-pinyin--test-events)
                 "fixture-regexp"))
              ((symbol-function 'avy-jump)
               (lambda (&rest arguments)
                 (push (cons 'jump arguments)
                       ace-pinyin--test-events)
                 'jump-result)))
           (list
            (ace-pinyin--jump-impl ?z 'chinese-only)
            (nreverse ace-pinyin--test-events))))"##;
    let expect = expect![[
        r#"OK (jump-result ((build 122 t t chinese-only) (jump "fixture-regexp" :window-flip nil)))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_impl_ace_backend_finishes_active_jump_then_sets_state_and_runs() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy nil))
         (setq ace-pinyin--test-events nil)
         (cl-progv
             '(ace-jump-current-mode
               ace-jump-query-char)
             '(active-mode old-query)
           (cl-letf
               (((symbol-function
                  'ace-pinyin--build-regexp)
                 (lambda (query prefix)
                   (push (list 'build query prefix)
                         ace-pinyin--test-events)
                   "fixture-regexp"))
                ((symbol-function 'ace-jump-done)
                 (lambda ()
                   (push
                    (list 'done
                          ace-jump-current-mode)
                    ace-pinyin--test-events)
                   (setq ace-jump-current-mode nil)))
                ((symbol-function
                  'ace-jump-char-category)
                 (lambda (query)
                   (push (list 'category query)
                         ace-pinyin--test-events)
                   'alpha))
                ((symbol-function 'ace-jump-do)
                 (lambda (regexp)
                   (push
                    (list 'jump
                          regexp
                          ace-jump-query-char
                          ace-jump-current-mode)
                    ace-pinyin--test-events)
                   'jump-result)))
             (list
              (ace-pinyin--jump-impl ?n 'prefix)
              ace-jump-query-char
              ace-jump-current-mode
              (nreverse ace-pinyin--test-events)))))"##;
    let expect = expect![[
        r#"OK (jump-result 110 ace-jump-char-mode ((build 110 prefix) (done active-mode) (category 110) (jump "fixture-regexp" 110 ace-jump-char-mode)))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_impl_ace_backend_rejects_other_category_after_finishing_active_jump() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy nil))
         (setq ace-pinyin--test-events nil)
         (cl-progv
             '(ace-jump-current-mode)
             '(active-mode)
           (cl-letf
               (((symbol-function
                  'ace-pinyin--build-regexp)
                 (lambda (_query _prefix)
                   "fixture-regexp"))
                ((symbol-function 'ace-jump-done)
                 (lambda ()
                   (push 'done ace-pinyin--test-events)
                   (setq ace-jump-current-mode nil)))
                ((symbol-function
                  'ace-jump-char-category)
                (lambda (query)
                   (push (list 'category query)
                         ace-pinyin--test-events)
                   'other)))
             (list
              (condition-case error
                  (list
                   'ok
                   (ace-pinyin--jump-impl
                    0
                    nil))
                (error
                 (list 'error error)))
              (nreverse
               ace-pinyin--test-events)
              ace-jump-current-mode))))"##;
    let expect = expect![[
        r#"OK ((error (error "[AceJump] Non-printable character")) (done (category 0)) nil)"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_char_dispatches_mode_avy_and_ace_paths() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (let ((ace-pinyin-mode (nth 0 fixture))
                 (ace-pinyin-use-avy (nth 1 fixture))
                 (ace-pinyin--original-avy
                  'ace-pinyin-fixture-avy)
                 (ace-pinyin--original-ace
                  'ace-pinyin-fixture-ace))
             (setq ace-pinyin--test-events nil)
             (cl-letf
                 (((symbol-function
                    'ace-pinyin--jump-impl)
                   (lambda (query &optional prefix)
                     (push (list 'impl query prefix)
                           ace-pinyin--test-events)
                     'impl-result))
                  ((symbol-function
                    'ace-pinyin-fixture-avy)
                   (lambda (query)
                     (push (list 'avy query)
                           ace-pinyin--test-events)
                     'avy-result))
                  ((symbol-function
                    'ace-pinyin-fixture-ace)
                   (lambda (query)
                     (push (list 'ace query)
                           ace-pinyin--test-events)
                     'ace-result)))
               (list
                fixture
                (ace-pinyin-jump-char ?z)
                (nreverse
                 ace-pinyin--test-events)))))
         '((t t)
           (t nil)
           (nil t)
           (nil nil)))"##;
    let expect = expect![
        "OK (((t t) impl-result ((impl 122 nil))) ((t nil) impl-result ((impl 122 nil))) ((nil t) avy-result ((avy 122))) ((nil nil) ace-result ((ace 122))))"
    ];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_char_interactive_prompt_depends_on_backend() {
    let elisp_form = r##"(mapcar
         (lambda (use-avy)
           (let ((ace-pinyin-use-avy use-avy)
                 (ace-pinyin-mode t))
             (setq ace-pinyin--test-events nil)
             (cl-letf
                 (((symbol-function 'read-char)
                   (lambda (prompt &rest arguments)
                     (push
                      (list 'read prompt arguments)
                      ace-pinyin--test-events)
                     ?q))
                  ((symbol-function
                    'ace-pinyin--jump-impl)
                   (lambda (query &optional prefix)
                     (push (list 'impl query prefix)
                           ace-pinyin--test-events)
                     'impl-result)))
               (list
                use-avy
                (call-interactively
                 'ace-pinyin-jump-char)
                (nreverse
                 ace-pinyin--test-events)))))
         '(t nil))"##;
    let expect = expect![[
        r#"OK ((t impl-result ((read "char: " nil) (impl 113 nil))) (nil impl-result ((read "Query Char:" nil) (impl 113 nil))))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_char_2_builds_two_character_regexp_and_forwards_window_flip() {
    let elisp_form = r##"(let ((ace-pinyin-enable-punctuation-translation nil)
             (ace-pinyin-simplified-chinese-only-p nil))
         (setq ace-pinyin--test-events nil)
         (cl-letf
             (((symbol-function
                'pinyinlib-build-regexp-string)
               (lambda (text no-punctuation traditional)
                 (push
                  (list 'build
                        text
                        no-punctuation
                        traditional)
                  ace-pinyin--test-events)
                 "fixture-regexp"))
              ((symbol-function 'avy-jump)
               (lambda (&rest arguments)
                 (push (cons 'jump arguments)
                       ace-pinyin--test-events)
                 'jump-result)))
           (list
            (ace-pinyin-jump-char-2 ?n ?h 'flip)
            (nreverse ace-pinyin--test-events))))"##;
    let expect = expect![[
        r#"OK (jump-result ((build "nh" t t) (jump "fixture-regexp" :window-flip flip)))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_char_in_line_limits_jump_to_current_line_and_flips_global_scope() {
    let elisp_form = r##"(with-temp-buffer
         (insert "first\nsecond line\nthird")
         (goto-char 11)
         (let ((avy-all-windows 'all-windows))
           (setq ace-pinyin--test-events nil)
           (cl-letf
               (((symbol-function
                  'ace-pinyin--build-regexp)
                 (lambda (query prefix)
                   (push (list 'build query prefix)
                         ace-pinyin--test-events)
                   "fixture-regexp"))
                ((symbol-function 'avy-jump)
                 (lambda (&rest arguments)
                   (push (cons 'jump arguments)
                         ace-pinyin--test-events)
                   'jump-result)))
             (list
              (ace-pinyin-jump-char-in-line ?s)
              (line-beginning-position)
              (line-end-position)
              (nreverse
               ace-pinyin--test-events)))))"##;
    let expect = expect![[
        r#"OK (jump-result 7 18 ((build 115 nil) (jump "fixture-regexp" :window-flip all-windows :beg 7 :end 18)))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_dwim_reads_backend_specific_prompt_and_forwards_prefix() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (let ((ace-pinyin-use-avy (car fixture))
                 (current-prefix-arg (cdr fixture)))
             (setq ace-pinyin--test-events nil)
             (cl-letf
                 (((symbol-function 'read-char)
                   (lambda (prompt)
                     (push (list 'read prompt)
                           ace-pinyin--test-events)
                     ?z))
                  ((symbol-function
                    'ace-pinyin--jump-impl)
                   (lambda (query prefix)
                     (push (list 'impl query prefix)
                           ace-pinyin--test-events)
                     'impl-result)))
               (list
                fixture
                (call-interactively 'ace-pinyin-dwim)
                (nreverse
                 ace-pinyin--test-events)))))
         '((t . nil)
           (nil . (4))))"##;
    let expect = expect![[
        r#"OK (((t) impl-result ((read "char: ") (impl 122 nil))) ((nil . #1=(4)) impl-result ((read "Query Char:") (impl 122 #1#))))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}
