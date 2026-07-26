use super::assert_achievements_autoload_parity;
use expect_test::expect;

#[test]
fn achievements_autoload_file_registers_public_commands_without_loading_runtime() {
    let elisp_form = r##"(list
         (featurep
          'achievements-autoloads)
         (featurep
          'achievements-functions)
         (featurep
          'achievements)
         (mapcar
          (lambda (symbol)
            (let ((definition
                   (symbol-function
                    symbol)))
              (list
               symbol
               (autoloadp
                definition)
               (nth 1 definition)
               (nth 2 definition)
               (nth 3 definition)
               (nth 4 definition)
               (commandp symbol))))
          '(achievements-init
            achievements-list-achievements
            achievements-mode))
         (copy-sequence
          (gethash
           "achievements-functions"
           definition-prefixes))
         (copy-sequence
          (gethash
           "basic-achievements"
           definition-prefixes)))"##;
    let expect = expect![[
        r#"OK (t nil nil ((achievements-init t "achievements" nil t nil t) (achievements-list-achievements t "achievements" nil t nil t) (achievements-mode t "achievements" nil t nil t)) nil nil)"#
    ]];
    assert_achievements_autoload_parity(elisp_form, expect);
}

#[test]
fn achievements_init_autoload_loads_main_runtime_and_basic_catalog() {
    let elisp_form = r##"(list
         (achievements-init)
         (featurep
          'achievements-functions)
         (featurep
          'basic-achievements)
         (featurep
          'achievements)
         (length
          achievements-list)
         (memq
          #'achievements-save-achievements
          kill-emacs-hook)
         (autoloadp
          (symbol-function
           'achievements-init)))"##;
    let expect = expect!["OK (basic-achievements t t t 101 (achievements-save-achievements) nil)"];
    assert_achievements_autoload_parity(elisp_form, expect);
}

#[test]
fn achievements_list_command_autoload_loads_before_downstream_dispatch() {
    let elisp_form = r##"(progn
         (autoload-do-load
          (symbol-function
           'achievements-list-achievements)
          'achievements-list-achievements)
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function
                'pop-to-buffer)
               (lambda (buffer)
                 (push
                  (list 'pop buffer)
                  achievements--test-events)))
              ((symbol-function
                'achievements-list-mode)
               (lambda ()
                 (push '(mode)
                       achievements--test-events)))
              ((symbol-function
                'achievements-update-score)
               (lambda ()
                 (push '(update)
                       achievements--test-events)))
              ((symbol-function
                'tabulated-list-print)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'print
                   arguments)
                  achievements--test-events)
                 'printed)))
           (list
            (achievements-list-achievements)
            (featurep 'achievements)
            (nreverse
             achievements--test-events))))"##;
    let expect = expect![[r#"OK (printed t ((pop "*Achievements*") (mode) (update) (print t)))"#]];
    assert_achievements_autoload_parity(elisp_form, expect);
}

#[test]
fn achievements_mode_autoload_loads_and_enables_timer_and_hook() {
    let elisp_form = r##"(progn
         (autoload-do-load
          (symbol-function
           'achievements-mode)
          'achievements-mode)
         (setq
          achievements-mode
          nil
          achievements-timer
          nil
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function
                'run-with-idle-timer)
               (lambda
                   (seconds repeat
                    function
                    &rest arguments)
                 (push
                  (list
                   'timer
                   seconds
                   repeat
                   function
                   arguments)
                  achievements--test-events)
                 'fixture-timer))
              ((symbol-function
                'achievements-setup-post-command-hook)
               (lambda ()
                 (push '(setup)
                       achievements--test-events)))
              ((symbol-function 'add-hook)
               (lambda
                   (hook function
                    &optional append local)
                 (push
                  (list
                   'add-hook
                   hook
                   function
                   append
                   local)
                  achievements--test-events))))
           (list
            (achievements-mode 1)
            achievements-mode
            achievements-timer
            (featurep
             'achievements)
            (nreverse
             achievements--test-events))))"##;
    let expect = expect![
        "OK (t t fixture-timer t ((timer 10 t achievements-update-score nil) (setup) (add-hook post-command-hook achievements-post-command-function nil nil)))"
    ];
    assert_achievements_autoload_parity(elisp_form, expect);
}
