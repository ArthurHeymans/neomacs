use expect_test::expect;

use super::{assert_ace_flyspell_parity, assert_ace_flyspell_signal_parity};

#[test]
fn ace_flyspell_avy_word_passes_candidates_and_style_with_local_avy_controls_disabled() {
    let elisp_form = r##"(let ((avy-action
                    'outer-action)
                   (avy-all-windows
                    'outer-windows)
                   (avy-style
                    'chosen-style)
                   calls)
               (cl-letf
                   (((symbol-function
                      'ace-flyspell--collect-candidates)
                     (lambda ()
                       (push
                        (list
                         'collect
                         avy-action
                         avy-all-windows)
                        calls)
                       '(2 8 13)))
                    ((symbol-function
                      'avy--style-fn)
                     (lambda (style)
                       (push
                        (list
                         'style
                         style
                         avy-action
                         avy-all-windows)
                        calls)
                       'style-function))
                    ((symbol-function
                      'avy--process)
                     (lambda (candidates style-function)
                       (push
                        (list
                         'process
                         candidates
                         style-function
                         avy-action
                         avy-all-windows)
                        calls)
                       8)))
                 (list
                  (ace-flyspell--avy-word)
                  avy-action
                  avy-all-windows
                  (nreverse
                   calls))))"##;
    let expect = expect![
        "OK (8 outer-action outer-windows ((collect nil nil) (style chosen-style nil nil) (process (2 8 13) style-function nil nil)))"
    ];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_avy_word_does_not_compute_a_style_when_candidate_collection_signals() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'ace-flyspell--collect-candidates)
                 (lambda ()
                   (error
                    "candidate failure")))
                ((symbol-function
                  'avy--style-fn)
                 (lambda (_style)
                   (error
                    "style should not run"))))
               (ace-flyspell--avy-word))"##;
    let expect = expect![[r#"ERR (error "candidate failure")"#]];

    assert_ace_flyspell_signal_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_correct_word_is_a_noop_for_non_numeric_avy_results() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ace-flyspell--avy-word)
                     (lambda ()
                       (push
                        'avy
                        calls)
                       'cancelled))
                    ((symbol-function
                      'flyspell-get-word)
                     (lambda (&optional _following)
                       (push
                        'word
                        calls)
                       '("unused" 1 2)))
                    ((symbol-function
                      'ace-flyspell-default-handler)
                     (lambda ()
                       (push
                        'handler
                        calls)))
                    ((symbol-function
                      'ace-flyspell--reset)
                     (lambda ()
                       (push
                        'reset
                        calls))))
                 (list
                  (ace-flyspell-correct-word)
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK (nil (avy))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_correct_word_missing_tuple_signals_before_handler_cleanup_and_mark_return() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "wrng")
               (goto-char
                (point-max))
               (push-mark
                2
                t)
               (let ((ace-flyspell--ov
                      (make-overlay
                       1
                       1))
                     (ace-flyspell--current-word
                      'before)
                     (ace-flyspell-handler
                      (lambda ()
                        (push
                         'handler
                         calls)))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'ace-flyspell--avy-word)
                       (lambda ()
                         (push
                          'avy
                          calls)
                         1))
                      ((symbol-function
                        'flyspell-get-word)
                       (lambda (&optional following)
                         (push
                          (list
                           'word
                           following)
                          calls)
                         nil))
                      ((symbol-function
                        'ace-flyspell--reset)
                       (lambda ()
                         (push
                          'reset
                          calls)
                         (delete-overlay
                          ace-flyspell--ov))))
                   (let ((error-value
                          (condition-case error-data
                              (ace-flyspell-correct-word)
                            (error
                             error-data))))
                     (list
                      error-value
                      ace-flyspell--current-word
                      (point)
                      (mark)
                      (overlay-start
                       ace-flyspell--ov)
                      (overlay-end
                       ace-flyspell--ov)
                      (nreverse
                       calls))))))"##;
    let expect =
        expect!["OK ((wrong-type-argument integer-or-marker-p nil) nil 5 2 1 1 (avy (word nil)))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_correct_word_moves_the_overlay_runs_a_custom_handler_and_cleans_up() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "wrng tail")
               (goto-char
                (point-max))
               (push-mark
                6
                t)
               (let ((ace-flyspell--ov
                      (make-overlay
                       1
                       1))
                     (ace-flyspell--current-word
                      nil)
                     calls)
                 (let ((ace-flyspell-handler
                        (lambda ()
                          (push
                           (list
                            'handler
                            ace-flyspell--current-word
                            (overlay-start
                             ace-flyspell--ov)
                            (overlay-end
                             ace-flyspell--ov)
                            (point))
                           calls)
                          'handler-result)))
                   (cl-letf
                       (((symbol-function
                          'ace-flyspell--avy-word)
                         (lambda ()
                           (push
                            'avy
                            calls)
                           1))
                        ((symbol-function
                          'flyspell-get-word)
                         (lambda (&optional following)
                           (push
                            (list
                             'word
                             following
                             (point))
                            calls)
                           '("wrng" 1 5)))
                        ((symbol-function
                          'ace-flyspell--reset)
                         (lambda ()
                           (push
                            (list
                             'reset
                             (overlay-start
                              ace-flyspell--ov)
                             (overlay-end
                              ace-flyspell--ov))
                            calls)
                           (delete-overlay
                            ace-flyspell--ov))))
                     (list
                      (ace-flyspell-correct-word)
                      ace-flyspell--current-word
                      (point)
                      (mark)
                      (overlay-buffer
                       ace-flyspell--ov)
                      (nreverse
                       calls))))))"##;
    let expect = expect![[
        r#"OK (handler-result "wrng" 6 6 nil (avy (word nil 10) (handler "wrng" 1 5 10) (reset 1 5)))"#
    ]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_correct_word_uses_the_default_handler_for_non_functions() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "wrng")
               (push-mark
                3
                t)
               (let ((ace-flyspell--ov
                      (make-overlay
                       1
                       1))
                     (ace-flyspell-handler
                      'not-a-defined-function)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'ace-flyspell--avy-word)
                       (lambda ()
                         1))
                      ((symbol-function
                        'flyspell-get-word)
                       (lambda (&optional _following)
                         '("wrng" 1 5)))
                      ((symbol-function
                        'ace-flyspell-default-handler)
                       (lambda ()
                         (push
                          'default
                          calls)))
                      ((symbol-function
                        'ace-flyspell--reset)
                       (lambda ()
                         (push
                          'reset
                          calls)
                         (delete-overlay
                          ace-flyspell--ov))))
                   (list
                    (ace-flyspell-correct-word)
                    (point)
                    (nreverse
                     calls)))))"##;
    let expect = expect!["OK (#1=(default reset) 3 #1#)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_correct_word_cleans_up_and_returns_to_mark_after_handler_errors() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "wrng tail")
               (goto-char
                (point-max))
               (push-mark
                6
                t)
               (let ((ace-flyspell--ov
                      (make-overlay
                       1
                       1))
                     (ace-flyspell-handler
                      (lambda ()
                        (error
                         "handler failure")))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'ace-flyspell--avy-word)
                       (lambda ()
                         1))
                      ((symbol-function
                        'flyspell-get-word)
                       (lambda (&optional _following)
                         '("wrng" 1 5)))
                      ((symbol-function
                        'ace-flyspell--reset)
                       (lambda ()
                         (push
                          'reset
                          calls)
                         (delete-overlay
                          ace-flyspell--ov))))
                   (let ((error-value
                          (condition-case error-data
                              (ace-flyspell-correct-word)
                            (error
                             error-data))))
                     (list
                      error-value
                      (point)
                      (mark)
                      (overlay-buffer
                       ace-flyspell--ov)
                      (nreverse
                       calls))))))"##;
    let expect = expect![[r#"OK ((error "handler failure") 6 6 nil (reset))"#]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_jump_word_returns_the_avy_workflow_result_unchanged() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ace-flyspell--avy-word)
                     (lambda ()
                       (push
                        'called
                        calls)
                       '(custom result))))
                 (list
                  (ace-flyspell-jump-word)
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK ((custom result) (called))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_dwim_uses_cached_auto_correction_without_rechecking_the_word() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "word")
               (goto-char
                3)
               (let ((flyspell-auto-correct-pos
                      3)
                     (flyspell-auto-correct-region
                      '(1 . 5))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'flyspell-word)
                       (lambda ()
                         (push
                          'word
                          calls)
                         t))
                      ((symbol-function
                        'flyspell-auto-correct-word)
                       (lambda ()
                         (push
                          'auto
                          calls)
                         'auto-result))
                      ((symbol-function
                        'ace-flyspell-correct-word)
                       (lambda ()
                         (push
                          'ace
                          calls)
                         'ace-result)))
                   (list
                    (ace-flyspell-dwim)
                    (nreverse
                     calls)))))"##;
    let expect = expect!["OK (auto-result (auto))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_dwim_auto_corrects_when_flyspell_word_returns_nil() {
    let elisp_form = r##"(let ((flyspell-auto-correct-pos
                    99)
                   (flyspell-auto-correct-region
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'flyspell-word)
                     (lambda ()
                       (push
                        'word
                        calls)
                       nil))
                    ((symbol-function
                      'flyspell-auto-correct-word)
                     (lambda ()
                       (push
                        'auto
                        calls)
                       'auto-result))
                    ((symbol-function
                      'ace-flyspell-correct-word)
                     (lambda ()
                       (push
                        'ace
                        calls)
                       'ace-result)))
                 (list
                  (ace-flyspell-dwim)
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK (auto-result (word auto))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_dwim_uses_ace_correction_for_a_misspelt_word() {
    let elisp_form = r##"(let ((flyspell-auto-correct-pos
                    99)
                   (flyspell-auto-correct-region
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'flyspell-word)
                     (lambda ()
                       (push
                        'word
                        calls)
                       t))
                    ((symbol-function
                      'flyspell-auto-correct-word)
                     (lambda ()
                       (push
                        'auto
                        calls)
                       'auto-result))
                    ((symbol-function
                      'ace-flyspell-correct-word)
                     (lambda ()
                       (push
                        'ace
                        calls)
                       'ace-result)))
                 (list
                  (ace-flyspell-dwim)
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK (ace-result (word ace))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_dwim_requires_a_consp_cached_region_even_at_the_cached_position() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "word")
               (goto-char
                3)
               (let ((flyspell-auto-correct-pos
                      3)
                     (flyspell-auto-correct-region
                      [])
                     calls)
                 (cl-letf
                     (((symbol-function
                        'flyspell-word)
                       (lambda ()
                         (push
                          'word
                          calls)
                         t))
                      ((symbol-function
                        'flyspell-auto-correct-word)
                       (lambda ()
                         (push
                          'auto
                          calls)))
                      ((symbol-function
                        'ace-flyspell-correct-word)
                       (lambda ()
                         (push
                          'ace
                          calls)
                         'ace-result)))
                   (list
                    (ace-flyspell-dwim)
                    (nreverse
                     calls)))))"##;
    let expect = expect!["OK (ace-result (word ace))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_setup_passes_the_exact_global_and_deferred_key_forms() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'global-set-key)
                     (lambda (key command)
                       (push
                        (list
                         'global
                         key
                         command)
                        calls)
                       'global-result))
                    ((symbol-function
                      'eval-after-load)
                     (lambda (file form)
                       (push
                        (list
                         'after-load
                         file
                         form)
                        calls)
                       'after-load-result)))
                 (list
                  (ace-flyspell-setup)
                  (nreverse
                   calls))))"##;
    let expect = expect![[
        r#"OK (after-load-result ((global [67108910] ace-flyspell-dwim) (after-load "flyspell" #[nil ((define-key flyspell-mode-map (kbd "C-.") 'ace-flyspell-dwim)) nil])))"#
    ]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_setup_global_set_key_behavior_under_a_rebound_global_map_matches_gnu() {
    let elisp_form = r##"(let ((global-map
                    (copy-keymap
                     global-map))
                   (flyspell-mode-map
                    (copy-keymap
                     flyspell-mode-map)))
               (list
                (ace-flyspell-setup)
                (lookup-key
                 global-map
                 (kbd
                  "C-."))
                (lookup-key
                 flyspell-mode-map
                 (kbd
                  "C-."))))"##;
    let expect = expect!["OK (ace-flyspell-dwim nil ace-flyspell-dwim)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_setup_installs_both_actual_global_and_flyspell_mode_bindings() {
    let elisp_form = r##"(list
               (ace-flyspell-setup)
               (lookup-key
                (current-global-map)
                (kbd
                 "C-."))
               (lookup-key
                flyspell-mode-map
                (kbd
                 "C-.")))"##;
    let expect = expect!["OK (ace-flyspell-dwim ace-flyspell-dwim ace-flyspell-dwim)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}
