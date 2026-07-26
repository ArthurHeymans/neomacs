use super::{assert_ace_jump_mode_parity, assert_ace_jump_mode_signal_parity};
use expect_test::expect;

#[test]
fn ace_jump_mode_char_command_sets_state_and_quotes_each_query_character() {
    let elisp_form = r##"(mapcar
         (lambda (character)
           (let ((ace-jump-current-mode nil)
                 (ace-jump-query-char nil)
                 observed)
             (cl-letf (((symbol-function 'ace-jump-do)
                        (lambda (regexp)
                          (setq observed regexp))))
               (ace-jump-char-mode character))
             (list
              character
              ace-jump-query-char
              ace-jump-current-mode
              observed)))
         '(?a ?7 ?. ?[ ?\\ ?\t))"##;
    let expect = expect![[
        r#"OK ((97 97 ace-jump-char-mode "a") (55 55 ace-jump-char-mode "7") (46 46 ace-jump-char-mode "\\.") (91 91 ace-jump-char-mode "\\[") (92 92 ace-jump-char-mode "\\\\") (9 9 ace-jump-char-mode "\11"))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_char_command_cleans_previous_session_before_restart() {
    let elisp_form = r##"(let ((ace-jump-current-mode 'old)
             (ace-jump-query-char ?o)
             events)
         (cl-letf (((symbol-function 'ace-jump-done)
                    (lambda ()
                      (setq events
                            (cons 'done events))
                      (setq ace-jump-current-mode nil)
                      (setq ace-jump-query-char nil)))
                   ((symbol-function 'ace-jump-do)
                    (lambda (regexp)
                      (setq events
                            (cons
                             (list
                              'do
                              regexp
                              ace-jump-current-mode
                              ace-jump-query-char)
                             events)))))
           (ace-jump-char-mode ?x))
         (nreverse events))"##;
    let expect = expect![[r#"OK (done (do "x" ace-jump-char-mode 120))"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_char_command_rejects_nonprintable_character() {
    let elisp_form = r##"(ace-jump-char-mode 10)"##;
    let expect = expect![[r#"ERR (error "[AceJump] Non-printable character")"#]];
    assert_ace_jump_mode_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_word_command_without_query_uses_word_start_regexp() {
    let elisp_form = r##"(let ((ace-jump-current-mode nil)
             observed)
         (cl-letf (((symbol-function 'ace-jump-do)
                    (lambda (regexp)
                      (setq observed regexp))))
           (ace-jump-word-mode nil))
         (list
          observed
          ace-jump-query-char
          ace-jump-current-mode))"##;
    let expect = expect![[r#"OK ("\\<\\sw" nil nil)"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_word_command_alphanumeric_queries_set_word_state() {
    let elisp_form = r##"(mapcar
         (lambda (character)
           (let ((ace-jump-current-mode nil)
                 (ace-jump-query-char nil)
                 observed)
             (cl-letf (((symbol-function 'ace-jump-do)
                        (lambda (regexp)
                          (setq observed regexp))))
               (ace-jump-word-mode character))
             (list
              character
              observed
              ace-jump-query-char
              ace-jump-current-mode)))
         '(?a ?Z ?0 ?9))"##;
    let expect = expect![[
        r#"OK ((97 "\\<a" 97 ace-jump-word-mode) (90 "\\<Z" 90 ace-jump-word-mode) (48 "\\<0" 48 ace-jump-word-mode) (57 "\\<9" 57 ace-jump-word-mode))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_word_command_cleans_previous_session_before_restart() {
    let elisp_form = r##"(let ((ace-jump-current-mode 'old)
             (ace-jump-query-char ?o)
             events)
         (cl-letf (((symbol-function 'ace-jump-done)
                    (lambda ()
                      (setq events
                            (cons 'done events))
                      (setq ace-jump-current-mode nil)
                      (setq ace-jump-query-char nil)))
                   ((symbol-function 'ace-jump-do)
                    (lambda (regexp)
                      (setq events
                            (cons
                             (list
                              'do
                              regexp
                              ace-jump-current-mode
                              ace-jump-query-char)
                             events)))))
           (ace-jump-word-mode ?x))
         (nreverse events))"##;
    let expect = expect![[r#"OK (done (do "\\<x" ace-jump-word-mode 120))"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_word_command_punctuation_falls_back_to_char_state() {
    let elisp_form = r##"(mapcar
         (lambda (character)
           (let ((ace-jump-current-mode nil)
                 (ace-jump-query-char nil)
                 (ace-jump-mode-detect-punc t)
                 observed)
             (cl-letf (((symbol-function 'ace-jump-do)
                        (lambda (regexp)
                          (setq observed regexp))))
               (ace-jump-word-mode character))
             (list
              character
              observed
              ace-jump-query-char
              ace-jump-current-mode)))
         '(?! ?[ ?\t))"##;
    let expect = expect![[
        r#"OK ((33 "!" 33 ace-jump-char-mode) (91 "\\[" 91 ace-jump-char-mode) (9 "\11" 9 ace-jump-char-mode))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_word_command_rejects_punctuation_when_detection_disabled() {
    let elisp_form = r##"(let ((ace-jump-mode-detect-punc nil))
         (ace-jump-word-mode ?!))"##;
    let expect = expect![[r#"ERR (error "[AceJump] Not a valid word constituent")"#]];
    assert_ace_jump_mode_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_word_command_rejects_nonprintable_character() {
    let elisp_form = r##"(ace-jump-word-mode 10)"##;
    let expect = expect![[r#"ERR (error "[AceJump] Non-printable character")"#]];
    assert_ace_jump_mode_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_line_command_cleans_previous_state_then_uses_anchor() {
    let elisp_form = r##"(let ((ace-jump-current-mode 'old)
             events)
         (cl-letf (((symbol-function 'ace-jump-done)
                    (lambda ()
                      (setq events
                            (cons 'done events))
                      (setq ace-jump-current-mode nil)))
                   ((symbol-function 'ace-jump-do)
                    (lambda (regexp)
                      (setq events
                            (cons
                             (list
                              'do
                              regexp
                              ace-jump-current-mode)
                             events)))))
           (ace-jump-line-mode))
         (nreverse events))"##;
    let expect = expect![[r#"OK (done (do "^" ace-jump-line-mode))"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_prefix_dispatch_selects_and_caps_submode_index() {
    let elisp_form = r##"(let ((ace-jump-mode-submode-list
              '(first second third))
             calls)
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (function &optional record keys)
                      (setq calls
                            (cons
                             (list function record keys)
                             calls)))))
           (mapc
            #'ace-jump-mode
            '(0 1 3 4 7 8 15 16 64)))
         (nreverse calls))"##;
    let expect = expect![
        "OK ((first nil nil) (first nil nil) (first nil nil) (second nil nil) (second nil nil) (third nil nil) (third nil nil) (third nil nil) (third nil nil))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_prefix_dispatch_rejects_negative_index() {
    let elisp_form = r##"(ace-jump-mode -4)"##;
    let expect = expect![[r#"ERR (error "[AceJump] Invalid prefix command")"#]];
    assert_ace_jump_mode_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_quick_exchange_switches_char_to_word_with_saved_query() {
    let elisp_form = r##"(let ((ace-jump-current-mode
              'ace-jump-char-mode)
             (ace-jump-query-char ?x)
             events)
         (cl-letf (((symbol-function 'ace-jump-done)
                    (lambda ()
                      (setq events
                            (cons 'done events))
                      (setq ace-jump-query-char nil)
                      (setq ace-jump-current-mode nil)))
                   ((symbol-function 'ace-jump-word-mode)
                    (lambda (query)
                      (setq events
                            (cons
                             (list 'word query)
                             events)))))
           (ace-jump-quick-exchange))
         (nreverse events))"##;
    let expect = expect!["OK (done (word 120))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_quick_exchange_switches_word_to_char_with_saved_query() {
    let elisp_form = r##"(let ((ace-jump-current-mode
              'ace-jump-word-mode)
             (ace-jump-query-char ?!)
             events)
         (cl-letf (((symbol-function 'ace-jump-done)
                    (lambda ()
                      (setq events
                            (cons 'done events))
                      (setq ace-jump-query-char nil)
                      (setq ace-jump-current-mode nil)))
                   ((symbol-function 'ace-jump-char-mode)
                    (lambda (query)
                      (setq events
                            (cons
                             (list 'char query)
                             events)))))
           (ace-jump-quick-exchange))
         (nreverse events))"##;
    let expect = expect!["OK (done (char 33))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_quick_exchange_is_noop_without_query_or_for_other_modes() {
    let elisp_form = r##"(mapcar
         (lambda (state)
           (let ((ace-jump-current-mode (car state))
                 (ace-jump-query-char (cdr state))
                 events)
             (cl-letf (((symbol-function 'ace-jump-done)
                        (lambda ()
                          (setq events
                                (cons 'done events))))
                       ((symbol-function 'ace-jump-word-mode)
                        (lambda (&rest arguments)
                          (setq events
                                (cons
                                 (cons 'word arguments)
                                 events))))
                       ((symbol-function 'ace-jump-char-mode)
                        (lambda (&rest arguments)
                          (setq events
                                (cons
                                 (cons 'char arguments)
                                 events)))))
               (ace-jump-quick-exchange))
             (list state (nreverse events))))
         '((ace-jump-char-mode)
           (ace-jump-word-mode)
           (ace-jump-line-mode . 120)
           (other . 120)
           (nil . 120)))"##;
    let expect = expect![
        "OK (((ace-jump-char-mode) nil) ((ace-jump-word-mode) nil) ((ace-jump-line-mode . 120) nil) ((other . 120) nil) ((nil . 120) nil))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_move_missing_candidate_reports_and_finishes() {
    let elisp_form = r##"(let ((ace-jump-mode-move-keys '(?a ?b))
             (ace-jump-search-tree
              '(branch (leaf . one)))
             events)
         (cl-letf (((symbol-function 'this-command-keys)
                    (lambda () "b"))
                   ((symbol-function 'message)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'message arguments)
                             events))))
                   ((symbol-function 'ace-jump-done)
                    (lambda ()
                      (setq events
                            (cons 'done events)))))
           (ace-jump-move))
         (nreverse events))"##;
    let expect = expect![[r#"OK ((message "No such position candidate.") done)"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_move_branch_promotes_subtree_before_deleting_old_tree() {
    let elisp_form = r##"(let* ((chosen
               '(branch
                 (leaf . one)
                 (leaf . two)))
              (other '(leaf . three))
              (old-tree
               (list 'branch chosen other))
              (ace-jump-search-tree old-tree)
              (ace-jump-mode-move-keys '(?a ?b))
              events)
         (cl-letf (((symbol-function 'this-command-keys)
                    (lambda () "a"))
                   ((symbol-function
                     'ace-jump-update-overlay-in-search-tree)
                    (lambda (tree keys)
                      (setq events
                            (cons
                             (list
                              'update
                              (copy-tree tree)
                              keys)
                             events))))
                   ((symbol-function
                     'ace-jump-delete-overlay-in-search-tree)
                    (lambda (tree)
                      (setq events
                            (cons
                             (list
                              'delete
                              (copy-tree tree))
                             events)))))
           (ace-jump-move))
         (list
          (nreverse events)
          ace-jump-search-tree
          old-tree))"##;
    let expect = expect![
        "OK (((update (branch (leaf . one) (leaf . two)) (97 98)) (delete (branch (branch) (leaf . three)))) (branch (leaf . one) (leaf . two)) (branch (branch) (leaf . three)))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_move_leaf_runs_mark_hook_jump_cleanup_and_end_hook_in_order() {
    let elisp_form = r##"(let ((ace-jump-search-tree
              '(branch (leaf . overlay)))
             (ace-jump-mode-move-keys '(?a ?b))
             (ace-jump-mode-before-jump-hook
              '(before-hook))
             (ace-jump-mode-end-hook
              '(end-hook))
             events)
         (cl-letf (((symbol-function 'this-command-keys)
                    (lambda () "a"))
                   ((symbol-function 'overlay-get)
                    (lambda (overlay property)
                      (list overlay property)))
                   ((symbol-function 'ace-jump-push-mark)
                    (lambda ()
                      (setq events
                            (cons 'push events))))
                   ((symbol-function 'run-hooks)
                    (lambda (&rest hooks)
                      (setq events
                            (cons
                             (cons 'hooks hooks)
                             events))))
                   ((symbol-function 'ace-jump-jump-to)
                    (lambda (position)
                      (setq events
                            (cons
                             (list 'jump position)
                             events))))
                   ((symbol-function 'ace-jump-done)
                    (lambda ()
                      (setq events
                            (cons 'done events)))))
           (ace-jump-move))
         (nreverse events))"##;
    let expect = expect![
        "OK (push (hooks ace-jump-mode-before-jump-hook) (jump (overlay aj-data)) done (hooks ace-jump-mode-end-hook))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_move_invalid_tree_node_cleans_then_signals() {
    let elisp_form = r##"(let ((ace-jump-search-tree
              '(branch (mystery . value)))
             (ace-jump-mode-move-keys '(?a ?b))
             events)
         (cl-letf (((symbol-function 'this-command-keys)
                    (lambda () "a"))
                   ((symbol-function 'ace-jump-done)
                    (lambda ()
                      (setq events
                            (cons 'done events)))))
           (list
            (condition-case error-data
                (ace-jump-move)
              (error error-data))
            (nreverse events))))"##;
    let expect =
        expect![[r#"OK ((error "[AceJump] Internal error: tree node type is invalid") (done))"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
