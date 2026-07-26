use expect_test::expect;

use super::assert_ac_skk_parity;

#[test]
fn ac_skk_enable_disable_and_toggle_update_global_state_and_emit_exact_messages() {
    let elisp_form = r##"(let ((ac-skk-enable
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        calls)
                       'messaged)))
                 (list
                  (ac-skk-enable)
                  ac-skk-enable
                  (ac-skk-disable)
                  ac-skk-enable
                  (ac-skk-toggle)
                  ac-skk-enable
                  (ac-skk-toggle)
                  ac-skk-enable
                  (nreverse
                   calls))))"##;
    let expect = expect![[
        r#"OK (messaged t messaged nil messaged t messaged nil (("enabled ac-skk.") ("disabled ac-skk.") ("enabled ac-skk.") ("disabled ac-skk.")))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_toggle_honors_runtime_rebinding_of_enable_and_disable_functions() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-skk-enable)
                     (lambda ()
                       (push
                        'enable
                        calls)
                       'enabled))
                    ((symbol-function
                      'ac-skk-disable)
                     (lambda ()
                       (push
                        'disable
                        calls)
                       'disabled)))
                 (list
                  (let ((ac-skk-enable
                         nil))
                    (ac-skk-toggle))
                  (let ((ac-skk-enable
                         t))
                    (ac-skk-toggle))
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK (enabled disabled (enable disable))"];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_setup_is_a_complete_noop_while_disabled() {
    let elisp_form = r##"(let ((ac-skk-enable
                    nil)
                   (ac-sources
                    '(source-a))
                   (ac-trigger-commands
                    '(trigger-a))
                   (skk-dcomp-activate
                    'dcomp)
                   (skk-dcomp-multiple-activate
                    'multiple))
               (with-temp-buffer
                 (list
                  (ac-skk-setup)
                  ac-sources
                  ac-trigger-commands
                  skk-dcomp-activate
                  skk-dcomp-multiple-activate
                  (local-variable-p
                   'ac-skk-ac-sources-orig)
                  (local-variable-p
                   'ac-skk-ac-trigger-commands-orig)
                  (local-variable-p
                   'ac-trigger-commands))))"##;
    let expect = expect!["OK (nil (source-a) (trigger-a) dcomp multiple nil nil nil)"];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_setup_saves_every_buffer_value_disables_dynamic_completion_and_installs_sources_and_triggers()
 {
    let elisp_form = r##"(let ((ac-skk-enable
                    t)
                   (ac-skk-special-sources
                    '(special-a
                      special-b))
                   (ac-sources
                    '(default-source))
                   (ac-trigger-commands
                    '(default-trigger))
                   (skk-dcomp-activate
                    'default-dcomp)
                   (skk-dcomp-multiple-activate
                    'default-multiple))
               (with-temp-buffer
                 (set
                  (make-local-variable
                   'ac-sources)
                  '(buffer-source))
                 (set
                  (make-local-variable
                   'ac-trigger-commands)
                  '(buffer-trigger))
                 (set
                  (make-local-variable
                   'skk-dcomp-activate)
                  'buffer-dcomp)
                 (set
                  (make-local-variable
                   'skk-dcomp-multiple-activate)
                  'buffer-multiple)
                 (list
                  (ac-skk-setup)
                  ac-sources
                  ac-trigger-commands
                  skk-dcomp-activate
                  skk-dcomp-multiple-activate
                  ac-skk-ac-sources-orig
                  ac-skk-ac-trigger-commands-orig
                  ac-skk-skk-dcomp-activate-orig
                  ac-skk-skk-dcomp-multiple-activate-orig
                  (mapcar
                   #'local-variable-p
                   '(ac-sources
                     ac-trigger-commands
                     skk-dcomp-activate
                     skk-dcomp-multiple-activate
                     ac-skk-ac-sources-orig
                     ac-skk-ac-trigger-commands-orig
                     ac-skk-skk-dcomp-activate-orig
                     ac-skk-skk-dcomp-multiple-activate-orig)))))"##;
    let expect = expect![
        "OK (#1=(skk-insert skk-previous-candidate . #2=(buffer-trigger)) (special-a special-b) #1# nil nil (buffer-source) #2# buffer-dcomp buffer-multiple (t t t t t t t t))"
    ];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_cleanup_restores_sources_kills_saved_locals_and_reverts_completion_variables_to_defaults()
{
    let elisp_form = r##"(let ((ac-skk-enable
                    t)
                   (ac-skk-special-sources
                    '(special))
                   (ac-sources
                    '(default-source))
                   (ac-trigger-commands
                    '(default-trigger))
                   (skk-dcomp-activate
                    'default-dcomp)
                   (skk-dcomp-multiple-activate
                    'default-multiple))
               (with-temp-buffer
                 (set
                  (make-local-variable
                   'ac-sources)
                  '(buffer-source))
                 (set
                  (make-local-variable
                   'ac-trigger-commands)
                  '(buffer-trigger))
                 (set
                  (make-local-variable
                   'skk-dcomp-activate)
                  'buffer-dcomp)
                 (set
                  (make-local-variable
                   'skk-dcomp-multiple-activate)
                  'buffer-multiple)
                 (ac-skk-setup)
                 (let ((during
                        (list
                         ac-sources
                         ac-trigger-commands
                         skk-dcomp-activate
                         skk-dcomp-multiple-activate)))
                   (list
                    during
                    (ac-skk-cleanup)
                    ac-sources
                    ac-trigger-commands
                    skk-dcomp-activate
                    skk-dcomp-multiple-activate
                    (mapcar
                     #'local-variable-p
                     '(ac-skk-ac-sources-orig
                       ac-skk-ac-trigger-commands-orig
                       ac-skk-skk-dcomp-activate-orig
                       ac-skk-skk-dcomp-multiple-activate-orig
                       ac-trigger-commands
                       skk-dcomp-activate
                       skk-dcomp-multiple-activate))))))"##;
    let expect = expect![
        "OK (((special) (skk-insert skk-previous-candidate buffer-trigger) nil nil) nil (buffer-source) (default-trigger) default-dcomp default-multiple (nil nil nil nil nil nil nil))"
    ];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_cleanup_is_a_noop_without_a_buffer_local_saved_source() {
    let elisp_form = r##"(let ((ac-sources
                    '(source))
                   (ac-trigger-commands
                    '(trigger)))
               (with-temp-buffer
                 (list
                  (ac-skk-cleanup)
                  ac-sources
                  ac-trigger-commands
                  (local-variable-p
                   'ac-skk-ac-sources-orig))))"##;
    let expect = expect!["OK (nil (source) (trigger) nil)"];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_mode_exit_advice_runs_cleanup_after_the_original_function() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-skk-cleanup)
                     (lambda ()
                       (push
                        'cleanup
                        calls)
                       'cleaned)))
                 (list
                  (ad-Advice-skk-mode-exit
                   (lambda ()
                     (push
                      'original
                      calls)
                     'original-result))
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK (original-result (original cleanup))"];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_j_mode_on_advice_reinstalls_sources_and_unique_trigger_commands_only_when_active() {
    let elisp_form = r##"(let (calls)
               (list
                  (let ((ac-skk-enable
                         t)
                        (ac-skk-ac-sources-orig
                         nil)
                        (ac-skk-special-sources
                         '(special))
                        (ac-sources
                         '(current))
                        (ac-trigger-commands
                         '(other)))
                    (list
                     (ad-Advice-skk-j-mode-on
                      (lambda (&optional katakana)
                        (push
                         (list
                          'original
                          katakana)
                         calls)
                        (setq
                         ac-skk-ac-sources-orig
                         '(saved-after-original)
                         ac-sources
                         '(source-after-original)
                         ac-trigger-commands
                         '(trigger-after-original))
                        'original-result)
                      'fixture)
                     ac-sources
                     ac-trigger-commands
                     ac-skk-ac-sources-orig))
                  (let ((ac-skk-enable
                         nil)
                        (ac-skk-ac-sources-orig
                         '(saved))
                        (ac-sources
                         '(current))
                        (ac-trigger-commands
                         '(other)))
                    (list
                     (ad-Advice-skk-j-mode-on
                      (lambda (&optional katakana)
                        (push
                         (list
                          'original
                          katakana)
                         calls)
                        (setq
                         ac-sources
                         '(source-after-original)
                         ac-trigger-commands
                         '(trigger-after-original))
                        'original-result))
                     ac-sources
                     ac-trigger-commands))
                  (let ((ac-skk-enable
                         t)
                        (ac-skk-ac-sources-orig
                         nil)
                        (ac-skk-special-sources
                         '(special))
                        (ac-sources
                         '(current))
                        (ac-trigger-commands
                         '(other)))
                    (list
                     (ad-Advice-skk-j-mode-on
                      (lambda (&optional katakana)
                        (push
                         (list
                          'original
                          katakana)
                         calls)
                        (setq
                         ac-sources
                         '(source-after-original)
                         ac-trigger-commands
                         '(trigger-after-original))
                        'original-result)
                      'third)
                     ac-sources
                     ac-trigger-commands))
                  (nreverse
                   calls)))"##;
    let expect = expect![
        "OK ((original-result (special) (skk-previous-candidate skk-insert trigger-after-original) (saved-after-original)) (original-result (source-after-original) (trigger-after-original)) (original-result (source-after-original) (trigger-after-original)) ((original fixture) (original nil) (original third)))"
    ];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_latin_mode_advice_restores_saved_sources_and_trigger_commands_only_when_active() {
    let elisp_form = r##"(let (calls)
               (list
                  (let ((ac-skk-enable
                         t)
                        (ac-skk-ac-sources-orig
                         '(old-saved-source))
                        (ac-skk-ac-trigger-commands-orig
                         '(old-saved-trigger))
                        (ac-sources
                         '(current-source))
                        (ac-trigger-commands
                         '(current-trigger)))
                    (list
                     (ad-Advice-skk-latin-mode
                      (lambda (argument)
                        (push
                         (list
                         'original
                          argument)
                         calls)
                        (setq
                         ac-skk-ac-sources-orig
                         '(saved-after-original)
                         ac-skk-ac-trigger-commands-orig
                         '(trigger-after-original)
                         ac-sources
                         '(source-after-original)
                         ac-trigger-commands
                         '(current-after-original))
                        'original-result)
                      'fixture)
                     ac-sources
                     ac-trigger-commands))
                  (let ((ac-skk-enable
                         nil)
                        (ac-skk-ac-sources-orig
                         '(saved-source))
                        (ac-skk-ac-trigger-commands-orig
                         '(saved-trigger))
                        (ac-sources
                         '(current-source))
                        (ac-trigger-commands
                         '(current-trigger)))
                    (list
                     (ad-Advice-skk-latin-mode
                      (lambda (argument)
                        (push
                         (list
                         'original
                          argument)
                         calls)
                        (setq
                         ac-sources
                         '(source-after-original)
                         ac-trigger-commands
                         '(current-after-original))
                        'original-result)
                      'second)
                     ac-sources
                     ac-trigger-commands))
                  (let ((ac-skk-enable
                         t)
                        (ac-skk-ac-sources-orig
                         nil)
                        (ac-skk-ac-trigger-commands-orig
                         '(saved-trigger))
                        (ac-sources
                         '(current-source))
                        (ac-trigger-commands
                         '(current-trigger)))
                    (list
                     (ad-Advice-skk-latin-mode
                      (lambda (argument)
                        (push
                         (list
                          'original
                          argument)
                         calls)
                        (setq
                         ac-sources
                         '(source-after-original)
                         ac-trigger-commands
                         '(current-after-original))
                        'original-result)
                      'third)
                     ac-sources
                     ac-trigger-commands))
                  (nreverse
                   calls)))"##;
    let expect = expect![
        "OK ((original-result (saved-after-original) (trigger-after-original)) (original-result (source-after-original) (current-after-original)) (original-result (source-after-original) (current-after-original)) ((original fixture) (original second) (original third)))"
    ];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_trigger_predicate_advice_suppresses_only_conversion_without_skk_insert_trigger() {
    let elisp_form = r##"(let (calls)
               (list
                (list
                 (let ((skk-henkan-mode
                        'on)
                       (ac-trigger-commands
                        '(other)))
                   (ad-Advice-ac-trigger-command-p
                    (lambda (command)
                      (push
                       command
                       calls)
                      'original-result)
                    'suppressed))
                 (let ((skk-henkan-mode
                        'on)
                       (ac-trigger-commands
                        '(skk-insert
                          other)))
                   (ad-Advice-ac-trigger-command-p
                    (lambda (command)
                      (push
                       command
                       calls)
                      'original-result)
                    'allowed-trigger))
                 (let ((skk-henkan-mode
                        nil)
                       (ac-trigger-commands
                        '(other)))
                   (ad-Advice-ac-trigger-command-p
                    (lambda (command)
                      (push
                       command
                       calls)
                      'original-result)
                    'allowed-mode)))
                (nreverse
                 calls)))"##;
    let expect = expect![
        "OK ((nil original-result original-result) (suppressed allowed-trigger allowed-mode))"
    ];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_expand_string_advice_captures_selected_candidate_after_the_original_function() {
    let elisp_form = r##"(let ((ac-skk-selected-candidate
                    'old)
                   (ac-selected-candidate
                    'before-original)
                   calls)
               (list
                (ad-Advice-ac-expand-string
                 (lambda (&rest arguments)
                   (push
                    (list
                     arguments
                     ac-skk-selected-candidate
                     ac-selected-candidate)
                    calls)
                   (setq
                    ac-selected-candidate
                    'selected-by-original)
                   'original-result)
                 "fixture"
                 t)
                ac-skk-selected-candidate
                ac-selected-candidate
                (nreverse
                 calls)))"##;
    let expect = expect![[
        r#"OK (original-result selected-by-original selected-by-original ((("fixture" t) old before-original)))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}
