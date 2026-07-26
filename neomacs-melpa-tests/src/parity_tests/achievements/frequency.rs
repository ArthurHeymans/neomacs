use super::assert_achievements_functions_parity;
use expect_test::expect;

#[test]
fn achievements_variable_was_set_covers_pairs_hooks_custom_defaults_and_plain_variables() {
    let elisp_form = r##"(progn
         (defvar achievements--test-pair
           'alpha)
         (defvar achievements--test-hook
           nil)
         (defvar achievements--test-function
           'configured)
         (defcustom achievements--test-custom
           10
           "Fixture."
           :type 'integer)
         (defvar achievements--test-plain
           t)
         (mapcar
          (lambda (fixture)
            (pcase
                (car fixture)
              ('hook
               (setq
                achievements--test-hook
                (cadr fixture)))
              ('custom
               (setq
                achievements--test-custom
                (cadr fixture))))
            (list
             fixture
             (achievements-variable-was-set
              (nth 2 fixture))))
          '((pair-match nil
             (achievements--test-pair
              alpha))
            (pair-miss nil
             (achievements--test-pair
              beta))
            (hook nil
             achievements--test-hook)
            (hook
             (fixture-function)
             achievements--test-hook)
            (function configured
             achievements--test-function)
            (custom 10
             achievements--test-custom)
            (custom 11
             achievements--test-custom)
            (plain t
             achievements--test-plain))))"##;
    let expect = expect![
        "OK (((pair-match nil (achievements--test-pair alpha)) t) ((pair-miss nil (achievements--test-pair beta)) nil) ((hook nil achievements--test-hook) nil) ((hook (fixture-function) achievements--test-hook) 18) ((function configured achievements--test-function) 18) ((custom 10 achievements--test-custom) nil) ((custom 11 achievements--test-custom) t) ((plain t achievements--test-plain) nil))"
    ];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_command_count_prefers_keyfreq_and_merges_saved_table_values() {
    let elisp_form = r##"(progn
         (setq
          keyfreq-table
          (let ((table
                 (make-hash-table
                  :test 'equal)))
            (puthash
             '(global . alpha)
             2
             table)
            (puthash
             '(mode . beta)
             3
             table)
            (puthash
             '(global . other)
             100
             table)
            table)
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function 'require)
               (lambda
                   (feature
                    &optional _filename
                    _noerror)
                 (push
                  (list 'require feature)
                  achievements--test-events)
                 (eq feature
                     'keyfreq)))
              ((symbol-function
                'keyfreq-table-load)
               (lambda (table)
                 (push '(load-table)
                       achievements--test-events)
                 (puthash
                  '(saved . alpha)
                  5
                  table))))
           (list
            (achievements-num-times-commands-were-run
             '(alpha beta))
            (nreverse
             achievements--test-events)
            (gethash
             '(saved . alpha)
             keyfreq-table))))"##;
    let expect = expect!["OK (10 ((require keyfreq) (load-table)) nil)"];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_command_count_falls_back_to_command_frequency() {
    let elisp_form = r##"(progn
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function 'require)
               (lambda
                   (feature
                    &optional _filename
                    _noerror)
                 (push
                  (list 'require feature)
                  achievements--test-events)
                 (eq feature
                     'command-frequency)))
              ((symbol-function
                'command-frequency-list)
               (lambda ()
                 (push
                  '(frequency-list)
                  achievements--test-events)
                 '(ignored-heading
                   (alpha . 4)
                   (other . 40)
                   (beta . 6)))))
           (list
            (achievements-num-times-commands-were-run
             '(alpha beta))
            (nreverse
             achievements--test-events))))"##;
    let expect =
        expect!["OK (10 ((require keyfreq) (require command-frequency) (frequency-list)))"];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_command_count_falls_back_to_command_history_occurrences() {
    let elisp_form = r##"(let ((command-history
              '((alpha 1)
                (other)
                (alpha 2)
                (beta))))
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function 'require)
               (lambda
                   (feature
                    &optional _filename
                    _noerror)
                 (push
                  (list 'require feature)
                  achievements--test-events)
                 nil)))
           (list
            (achievements-num-times-commands-were-run
             '(alpha beta))
            (nreverse
             achievements--test-events))))"##;
    let expect = expect!["OK (3 ((require keyfreq) (require command-frequency)))"];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_command_was_run_covers_symbol_counts_all_and_any_shapes() {
    let elisp_form = r##"(progn
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function
                'achievements-num-times-commands-were-run)
               (lambda (commands)
                 (push commands
                       achievements--test-events)
                 (cond
                  ((equal commands
                          '(alpha))
                   2)
                  ((equal commands
                          '(beta))
                   0)
                  ((equal commands
                          '(alpha beta))
                   4)
                  (t 0)))))
           (mapcar
            (lambda (command)
              (setq
               achievements--test-events
               nil)
              (list
               command
               (condition-case error
                   (list
                    'ok
                    (achievements-command-was-run
                     command))
                 (error
                  (list 'error error)))
               (nreverse
                achievements--test-events)))
            '(alpha
              beta
              (alpha . 2)
              (alpha . 3)
              ((alpha beta) . 4)
              (alpha beta)
              ((alpha beta))
              nil))))"##;
    let expect = expect![
        "OK ((alpha (ok t) ((alpha))) (beta (ok nil) ((beta))) ((alpha . 2) (ok t) ((alpha))) ((alpha . 3) (ok nil) ((alpha))) ((#1=(alpha beta) . 4) (ok t) (#1#)) ((alpha beta) (ok nil) ((alpha) (beta))) ((#2=(alpha beta)) (ok t) (#2#)) (nil (ok nil) ((nil))))"
    ];
    assert_achievements_functions_parity(elisp_form, expect);
}
