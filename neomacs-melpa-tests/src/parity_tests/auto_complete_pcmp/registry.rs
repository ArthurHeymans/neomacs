use expect_test::expect;

use super::{assert_auto_complete_pcmp_autoload_parity, assert_auto_complete_pcmp_parity};

#[test]
fn auto_complete_pcmp_registers_feature_api_commands_and_documentation() {
    let elisp_form = r##"(list
         (featurep 'auto-complete-pcmp)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (commandp symbol)
             (documentation symbol)))
          '(ac-pcmp/get-ac-candidates
            ac-pcmp/do-ac-action
            ac-pcmp/self-insert-command-with-ac-start)))"##;
    let expect = expect![[
        r#"OK (t ((ac-pcmp/get-ac-candidates t nil "Return the result of ‘pcomplete’.") (ac-pcmp/do-ac-action t nil "Do the same action that ‘pcomplete’ does after completion.") (ac-pcmp/self-insert-command-with-ac-start t t "Do ‘self-insert-command’ and ‘auto-complete’.")))"#
    ]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_internal_state_variables_have_exact_initial_contract() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (boundp symbol)
            (symbol-value symbol)
            (documentation-property
             symbol 'variable-documentation)))
         '(ac-pcmp--active-p
           ac-pcmp--candidates
           ac-pcmp--status
           ac-pcmp--point))"##;
    let expect = expect![
        "OK ((ac-pcmp--active-p t nil nil) (ac-pcmp--candidates t nil nil) (ac-pcmp--status t nil nil) (ac-pcmp--point t nil nil))"
    ];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_advice_registry_covers_all_capture_phases() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (let ((function (nth 0 case))
                 (class (nth 1 case))
                 (name (nth 2 case)))
             (list
              function
              class
              name
              (not
               (null
                (ad-find-advice function class name)))
              (ad-is-active function))))
         '((pcomplete-completions after ac-pcmp)
           (pcomplete-show-completions around ac-pcmp)
           (pcomplete-stub before ac-pcmp)
           (pcomplete-stub after ac-pcmp)))"##;
    let expect = expect![
        "OK ((pcomplete-completions after ac-pcmp t t) (pcomplete-show-completions around ac-pcmp t t) (pcomplete-stub before ac-pcmp t t) (pcomplete-stub after ac-pcmp t t))"
    ];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_logger_generates_expected_levels_and_control_functions() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list symbol (fboundp symbol)))
          '(ac-pcmp--fatal
            ac-pcmp--error
            ac-pcmp--warn
            ac-pcmp--info
            ac-pcmp--debug
            ac-pcmp--trace
            ac-pcmp--log-set-level
            ac-pcmp--log-enable-logging
            ac-pcmp--log-disable-logging))
         (ac-pcmp--log-set-level 'warn)
         (ac-pcmp--log-enable-logging)
         (ac-pcmp--log-disable-logging))"##;
    let expect = expect![
        "OK (((ac-pcmp--fatal t) (ac-pcmp--error t) (ac-pcmp--warn t) (ac-pcmp--info t) (ac-pcmp--debug t) (ac-pcmp--trace t) (ac-pcmp--log-set-level t) (ac-pcmp--log-enable-logging t) (ac-pcmp--log-disable-logging t)) nil t nil)"
    ];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_show_message_formats_prefix_arguments_and_return() {
    let elisp_form = r##"(list
         (ac-pcmp--show-message "plain")
         (current-message)
         (ac-pcmp--show-message
          "command=%s count=%d" "build" 3)
         (current-message))"##;
    let expect = expect!["OK (nil nil nil nil)"];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_load_history_records_api_and_provide_entries() {
    let elisp_form = r##"(let* ((entry
                                 (cl-find-if
                                  (lambda (item)
                                    (memq
                                     '(provide . auto-complete-pcmp)
                                     (cdr item)))
                                  load-history))
              (definitions (cdr entry)))
         (list
          (not (null entry))
          (mapcar
           (lambda (definition)
             (member definition definitions))
           '((defun . ac-pcmp/get-ac-candidates)
             (defun . ac-pcmp/do-ac-action)
             (defun . ac-pcmp/self-insert-command-with-ac-start)
             (provide . auto-complete-pcmp)))))"##;
    let expect = expect!["OK (nil (nil nil nil nil))"];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_generated_autoload_file_has_no_eager_runtime_side_effects() {
    let elisp_form = r##"(list
         (featurep 'auto-complete-pcmp)
         (boundp 'ac-pcmp--active-p)
         (fboundp 'ac-pcmp/get-ac-candidates)
         (fboundp 'ac-pcmp/do-ac-action)
         (cl-some
          (lambda (entry)
            (memq
             '(provide . auto-complete-pcmp)
             (cdr entry)))
          load-history))"##;
    let expect = expect!["OK (nil nil nil nil nil)"];
    assert_auto_complete_pcmp_autoload_parity(elisp_form, expect);
}
