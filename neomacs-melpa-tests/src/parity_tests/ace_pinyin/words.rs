use super::{assert_ace_pinyin_parity, assert_ace_pinyin_signal_parity};
use expect_test::expect;

#[test]
fn ace_pinyin_goto_word_0_binds_chinese_word_regexp_for_original_avy_command() {
    let elisp_form = r##"(let ((ace-pinyin--original-avy-word-0
              'ace-pinyin-fixture-word-0))
         (setq ace-pinyin--test-events nil)
         (cl-letf
             (((symbol-function
                'ace-pinyin-fixture-word-0)
               (lambda (argument)
                 (push
                  (list argument
                        avy-goto-word-0-regexp)
                  ace-pinyin--test-events)
                 'word-result)))
           (list
            (ace-pinyin-goto-word-0 'prefix)
            (nreverse ace-pinyin--test-events))))"##;
    let expect = expect![[r#"OK (word-result ((prefix "\\b\\sw\\|\\cc")))"#]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_goto_word_1_covers_dot_punctuation_and_chinese_regexp_branches() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (let ((char (nth 0 fixture))
                 (avy-word-punc-regexp
                  (nth 1 fixture))
                 (ace-pinyin--test-build-result
                  (nth 2 fixture)))
             (setq ace-pinyin--test-events nil)
             (cl-letf
                 (((symbol-function
                    'ace-pinyin--build-regexp)
                   (lambda (query prefix)
                     (push (list 'build query prefix)
                           ace-pinyin--test-events)
                     ace-pinyin--test-build-result))
                  ((symbol-function 'avy-jump)
                   (lambda (&rest arguments)
                     (push (cons 'jump arguments)
                           ace-pinyin--test-events)
                     'jump-result)))
               (list
                fixture
                (ace-pinyin-goto-word-1
                 char
                 'flip)
                (nreverse
                 ace-pinyin--test-events)))))
         '((46 nil "unused")
           (33 "[!]" "unused")
           (97 nil "中文")
           (97 nil "")))"##;
    let expect = expect![[
        r#"OK (((46 nil "unused") jump-result ((jump "\\." :window-flip flip))) ((33 "[!]" "unused") jump-result ((jump "!" :window-flip flip))) ((97 nil "中文") jump-result ((build 97 t) (jump "\\ba\\|中文" :window-flip flip))) ((97 nil "") jump-result ((build 97 t) (jump "\\ba" :window-flip flip))))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_goto_subword_0_scans_displayed_window_and_processes_candidates() {
    let elisp_form = r##"(save-window-excursion
         (let ((buffer
                (generate-new-buffer
                 " *ace-pinyin-subword*")))
           (unwind-protect
               (progn
                 (set-window-buffer
                  (selected-window)
                  buffer)
                 (with-current-buffer buffer
                   (insert
                    "alphaBeta 中文 tail")
                   (goto-char (point-min)))
                 (set-window-start
                  (selected-window)
                  1)
                 (setq ace-pinyin--test-events nil)
                 (cl-letf
                     (((symbol-function 'avy--process)
                       (lambda (candidates style)
                         (push
                          (list
                           (mapcar #'car candidates)
                           style)
                          ace-pinyin--test-events)
                         'process-result))
                      ((symbol-function 'avy--style-fn)
                       (lambda (style)
                         (list 'style style))))
                   (list
                    (ace-pinyin-goto-subword-0
                     nil
                     nil)
                    (nreverse
                     ace-pinyin--test-events))))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect!["OK (process-result (((1 6 11 12 14) (style at-full))))"];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_goto_subword_0_excludes_invisible_and_predicate_rejected_candidates() {
    let elisp_form = r##"(save-window-excursion
         (let ((buffer
                (generate-new-buffer
                 " *ace-pinyin-filtered-subword*")))
           (unwind-protect
               (progn
                 (set-window-buffer
                  (selected-window)
                  buffer)
                 (with-current-buffer buffer
                   (insert
                    "alphaBeta 中文 tail")
                   (put-text-property
                    1
                    2
                    'invisible
                    t)
                   (put-text-property
                    6
                    7
                    'invisible
                    t)
                   (goto-char (point-min)))
                 (set-window-start
                  (selected-window)
                  1)
                 (setq ace-pinyin--test-events nil
                       ace-pinyin--test-allowed
                       '(1 6 11 14))
                 (cl-letf
                     (((symbol-function 'avy--process)
                       (lambda (candidates style)
                         (push
                          (list
                           (mapcar #'car candidates)
                           style)
                          ace-pinyin--test-events)
                         'process-result))
                      ((symbol-function 'avy--style-fn)
                       (lambda (style)
                         (list 'style style))))
                   (list
                    (ace-pinyin-goto-subword-0
                     nil
                     (lambda ()
                       (memq
                        (point)
                        ace-pinyin--test-allowed)))
                    (nreverse
                     ace-pinyin--test-events))))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect!["OK (process-result (((11 14) (style at-full))))"];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_goto_subword_1_downcases_query_and_combines_ascii_and_chinese_predicates() {
    let elisp_form = r##"(with-temp-buffer
         (insert "A中z")
         (setq ace-pinyin--test-events nil)
         (cl-letf
             (((symbol-function
                'ace-pinyin--build-regexp)
               (lambda (query prefix)
                 (push (list 'build query prefix)
                       ace-pinyin--test-events)
                 "中"))
              ((symbol-function
                'ace-pinyin-goto-subword-0)
               (lambda (argument predicate)
                 (let ((answers nil))
                   (dolist (position '(1 2 3))
                     (goto-char position)
                     (push
                      (list position
                            (funcall predicate))
                      answers))
                   (push
                    (list 'subword
                          argument
                          (nreverse answers))
                    ace-pinyin--test-events))
                 'subword-result)))
           (list
            (ace-pinyin-goto-subword-1
             ?A
             'flip)
            (nreverse ace-pinyin--test-events))))"##;
    let expect = expect!["OK (subword-result ((build 97 t) (subword flip ((1 t) (2 0) (3 nil)))))"];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_word_impl_avy_backend_builds_string_regexp_and_jumps() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy t)
             (ace-pinyin-enable-punctuation-translation nil)
             (ace-pinyin-simplified-chinese-only-p nil))
         (setq ace-pinyin--test-events nil)
         (cl-letf
             (((symbol-function
                'pinyinlib-build-regexp-string)
               (lambda (query no-punctuation traditional)
                 (push
                  (list 'build
                        query
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
            (ace-pinyin--jump-word-1 "nh")
            (nreverse ace-pinyin--test-events))))"##;
    let expect = expect![[
        r#"OK (jump-result ((build "nh" t t) (jump "fixture-regexp" :window-flip nil)))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_word_impl_ace_backend_validates_case_then_uses_ambient_jump_folding() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy nil))
         (setq ace-pinyin--test-events nil)
         (cl-progv
             '(ace-jump-current-mode)
             '(active-mode)
           (cl-letf
               (((symbol-function
                  'pinyinlib-build-regexp-string)
                 (lambda (query no-punctuation traditional)
                   (push
                    (list 'build
                          query
                          no-punctuation
                          traditional)
                    ace-pinyin--test-events)
                   "fixture-regexp"))
                ((symbol-function 'ace-jump-done)
                 (lambda ()
                   (push 'done ace-pinyin--test-events)
                   (setq ace-jump-current-mode nil)))
                ((symbol-function 'ace-jump-do)
                 (lambda (regexp)
                   (push
                    (list 'jump
                          regexp
                          case-fold-search
                          ace-jump-current-mode)
                    ace-pinyin--test-events)
                   'jump-result)))
             (list
              (ace-pinyin--jump-word-1 "nh")
              ace-jump-current-mode
              (nreverse ace-pinyin--test-events)))))"##;
    let expect = expect![[
        r#"OK (jump-result ace-jump-char-mode ((build "nh" nil nil) done (jump "fixture-regexp" t ace-jump-char-mode)))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_word_impl_ace_backend_rejects_non_lowercase_after_finishing() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy nil))
         (setq ace-pinyin--test-events nil)
         (cl-progv
             '(ace-jump-current-mode)
             '(active-mode)
           (cl-letf
               (((symbol-function
                  'pinyinlib-build-regexp-string)
                 (lambda (&rest _arguments)
                   "fixture-regexp"))
                ((symbol-function 'ace-jump-done)
                 (lambda ()
                   (push 'done ace-pinyin--test-events)
                   (setq ace-jump-current-mode nil))))
             (list
              (condition-case error
                  (list
                   'ok
                   (ace-pinyin--jump-word-1
                    "nH"))
                (error
                 (list 'error error)))
              (nreverse
               ace-pinyin--test-events)
              ace-jump-current-mode))))"##;
    let expect =
        expect![[r#"OK ((error (error "[AcePinyin] Non-lower case character")) (done) nil)"#]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_word_prefix_reads_minibuffer_then_forwards_query() {
    let elisp_form = r##"(setq ace-pinyin--test-events nil)
       (cl-letf
           (((symbol-function 'read-string)
             (lambda (prompt)
               (push (list 'read prompt)
                     ace-pinyin--test-events)
               "nh"))
            ((symbol-function
              'ace-pinyin--jump-word-1)
             (lambda (query)
               (push (list 'jump query)
                     ace-pinyin--test-events)
               'jump-result)))
         (list
          (ace-pinyin-jump-word t)
          (nreverse ace-pinyin--test-events)))"##;
    let expect = expect![[r#"OK (jump-result ((read "Query Word: ") (jump "nh")))"#]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_word_timed_input_accumulates_until_return() {
    let elisp_form = r##"(let ((ace-pinyin--test-input
              (list ?n ?h ?\r))
             (ace-pinyin--jump-word-timeout 2))
         (setq ace-pinyin--test-events nil)
         (cl-letf
             (((symbol-function 'read-char)
               (lambda (prompt inherit timeout)
                 (let ((value
                        (pop ace-pinyin--test-input)))
                   (push
                    (list 'read
                          prompt
                          inherit
                          timeout
                          value)
                    ace-pinyin--test-events)
                   value)))
              ((symbol-function 'message)
               (lambda (format &rest arguments)
                 (push
                  (cons 'message
                        (cons format arguments))
                  ace-pinyin--test-events)))
              ((symbol-function
                'ace-pinyin--jump-word-1)
               (lambda (query)
                 (push (list 'jump query)
                       ace-pinyin--test-events)
                 'jump-result)))
           (list
            (ace-pinyin-jump-word nil)
            ace-pinyin--test-input
            (nreverse ace-pinyin--test-events))))"##;
    let expect = expect![[
        r#"OK (jump-result nil ((message "Query word: ") (read nil nil 2 110) (message "Query word: n") (read nil nil 2 104) (message "Query word: nh") (read nil nil 2 13) (jump "nh")))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_word_timed_input_forwards_partial_string_on_timeout() {
    let elisp_form = r##"(let ((ace-pinyin--test-input
              (list ?n nil))
             (ace-pinyin--jump-word-timeout 3))
         (setq ace-pinyin--test-events nil)
         (cl-letf
             (((symbol-function 'read-char)
               (lambda (&rest _arguments)
                 (pop ace-pinyin--test-input)))
              ((symbol-function 'message)
               (lambda (&rest _arguments) nil))
              ((symbol-function
                'ace-pinyin--jump-word-1)
               (lambda (query)
                 (push query
                       ace-pinyin--test-events)
                 'jump-result)))
           (list
            (ace-pinyin-jump-word nil)
            (nreverse ace-pinyin--test-events))))"##;
    let expect = expect![[r#"OK (jump-result ("n"))"#]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_jump_word_timed_input_rejects_initial_timeout() {
    let elisp_form = r##"(cl-letf
           (((symbol-function 'read-char)
             (lambda (&rest _arguments) nil))
            ((symbol-function 'message)
             (lambda (&rest _arguments) nil)))
         (ace-pinyin-jump-word nil))"##;
    let expect = expect![[r#"ERR (error "[AcePinyin] Empty input, timeout")"#]];
    assert_ace_pinyin_signal_parity(elisp_form, expect);
}
