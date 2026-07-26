use super::assert_ace_jump_zap_parity;
use expect_test::expect;

#[test]
fn ace_jump_zap_up_to_command_sets_saved_point_window_scope_and_active_state() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdef")
         (goto-char 4)
         (let ((ajz/zapping nil)
               (ajz/saved-point nil)
               (ajz/forward-only nil)
               (ace-jump-mode-scope 'outer)
               (ace-jump-search-filter 'outer-filter)
               (overriding-local-map nil)
               calls)
           (cl-letf (((symbol-function 'call-interactively)
                      (lambda (function &optional record keys)
                        (setq calls
                              (cons
                               (list
                                function
                                record
                                keys
                                ajz/zapping
                                ajz/saved-point
                                ace-jump-mode-scope
                                ace-jump-search-filter)
                               calls))
                        'called)))
             (list
              (ace-jump-zap-up-to-char)
              (nreverse calls)
              ajz/zapping
              ajz/saved-point
              ace-jump-mode-scope
              ace-jump-search-filter))))"##;
    let expect =
        expect!["OK (nil ((ace-jump-char-mode nil nil t 4 window nil)) t 4 outer outer-filter)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_up_to_command_uses_forward_filter_when_configured() {
    let elisp_form = r##"(let ((ajz/forward-only t)
             (overriding-local-map nil)
             observed)
         (cl-letf (((symbol-function 'point)
                    (lambda () 9))
                   ((symbol-function 'call-interactively)
                    (lambda (_function &rest _arguments)
                      (setq observed
                            (list
                             ace-jump-mode-scope
                             ace-jump-search-filter
                             ajz/zapping
                             ajz/saved-point)))))
           (ace-jump-zap-up-to-char))
         observed)"##;
    let expect = expect!["OK (window ajz/forward-query t 9)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_up_to_command_rebinds_catchall_key_after_ace_jump_map_exists() {
    let elisp_form = r##"(let ((ajz/forward-only nil)
             (overriding-local-map nil))
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (_function &rest _arguments)
                      (setq overriding-local-map
                            (let ((map
                                   (make-keymap)))
                              (define-key
                               map
                               [t]
                               'ace-jump-done)
                              map)))))
           (ace-jump-zap-up-to-char))
         (list
          (lookup-key
           overriding-local-map
           [t])
          (lookup-key
           overriding-local-map
           "x")))"##;
    let expect = expect!["OK (ajz/keyboard-reset nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_up_to_command_leaves_active_state_when_ace_jump_errors() {
    let elisp_form = r##"(let ((ajz/zapping nil)
             (ajz/saved-point nil)
             (ajz/to-char nil)
             (overriding-local-map nil))
         (cl-letf (((symbol-function 'point)
                    (lambda () 8))
                   ((symbol-function 'call-interactively)
                    (lambda (&rest _arguments)
                      (error "ace jump failed"))))
           (list
            (condition-case error-data
                (ace-jump-zap-up-to-char)
              (error error-data))
            ajz/zapping
            ajz/saved-point
            ajz/to-char)))"##;
    let expect = expect![[r#"OK ((error "ace jump failed") t 8 nil)"#]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_to_command_sets_to_char_before_delegating() {
    let elisp_form = r##"(let ((ajz/to-char nil)
             calls)
         (cl-letf (((symbol-function
                     'ace-jump-zap-up-to-char)
                    (lambda ()
                      (setq calls
                            (cons
                             (list
                              'up-to
                              ajz/to-char)
                             calls))
                      'delegated)))
           (list
            (ace-jump-zap-to-char)
            (nreverse calls)
            ajz/to-char)))"##;
    let expect = expect!["OK (delegated ((up-to t)) t)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_to_dwim_without_prefix_calls_builtin_interactively() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (function &optional record keys)
                      (setq calls
                            (cons
                             (list
                              function
                              record
                              keys)
                             calls))
                      'builtin))
                   ((symbol-function 'ace-jump-zap-to-char)
                    (lambda ()
                      (setq calls
                            (cons 'ace calls)))))
           (list
            (ace-jump-zap-to-char-dwim nil)
            (nreverse calls))))"##;
    let expect = expect!["OK (builtin ((zap-to-char nil nil)))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_to_dwim_with_each_non_nil_prefix_calls_ace_variant() {
    let elisp_form = r##"(mapcar
         (lambda (prefix)
           (let (calls)
             (cl-letf (((symbol-function
                         'call-interactively)
                        (lambda (&rest arguments)
                          (setq calls
                                (cons
                                 (cons
                                  'interactive
                                  arguments)
                                 calls))))
                       ((symbol-function
                         'ace-jump-zap-to-char)
                        (lambda ()
                          (setq calls
                                (cons 'ace calls))
                          'ace-result)))
               (list
                prefix
                (ace-jump-zap-to-char-dwim
                 prefix)
                (nreverse calls)))))
         '(t 0 1 -1 (4)))"##;
    let expect = expect![
        "OK ((t ace-result (ace)) (0 ace-result (ace)) (1 ace-result (ace)) (-1 ace-result (ace)) ((4) ace-result (ace)))"
    ];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_up_to_dwim_without_prefix_calls_builtin_interactively() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (function &optional record keys)
                      (setq calls
                            (cons
                             (list
                              function
                              record
                              keys)
                             calls))
                      'builtin))
                   ((symbol-function
                     'ace-jump-zap-up-to-char)
                    (lambda ()
                      (setq calls
                            (cons 'ace calls)))))
           (list
            (ace-jump-zap-up-to-char-dwim nil)
            (nreverse calls))))"##;
    let expect = expect!["OK (builtin ((zap-up-to-char nil nil)))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_up_to_dwim_with_each_non_nil_prefix_calls_ace_variant() {
    let elisp_form = r##"(mapcar
         (lambda (prefix)
           (let (calls)
             (cl-letf (((symbol-function
                         'call-interactively)
                        (lambda (&rest arguments)
                          (setq calls
                                (cons
                                 (cons
                                  'interactive
                                  arguments)
                                 calls))))
                       ((symbol-function
                         'ace-jump-zap-up-to-char)
                        (lambda ()
                          (setq calls
                                (cons 'ace calls))
                          'ace-result)))
               (list
                prefix
                (ace-jump-zap-up-to-char-dwim
                 prefix)
                (nreverse calls)))))
         '(t 0 1 -1 (4)))"##;
    let expect = expect![
        "OK ((t ace-result (ace)) (0 ace-result (ace)) (1 ace-result (ace)) (-1 ace-result (ace)) ((4) ace-result (ace)))"
    ];
    assert_ace_jump_zap_parity(elisp_form, expect);
}
