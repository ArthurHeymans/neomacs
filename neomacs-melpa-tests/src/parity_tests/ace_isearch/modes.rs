use expect_test::expect;

use super::assert_ace_isearch_parity;

#[test]
fn ace_isearch_minor_mode_variable_hook_lighter_and_registry_metadata_match() {
    let elisp_form = r##"(list
               ace-isearch-mode
               (default-boundp 'ace-isearch-mode)
               (local-variable-if-set-p 'ace-isearch-mode)
               (get 'ace-isearch-mode 'variable-documentation)
               (boundp 'ace-isearch-mode-hook)
               (get 'ace-isearch-mode-hook 'variable-documentation)
               (assq 'ace-isearch-mode minor-mode-alist)
               (assq 'ace-isearch-mode minor-mode-map-alist))"##;
    let expect = expect![[
        r#"OK (nil t t "Non-nil if Ace-Isearch mode is enabled.\nUse the command `ace-isearch-mode' to change this variable." t "Hook run after entering or leaving `ace-isearch-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" (ace-isearch-mode ace-isearch-lighter) nil)"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_mode_one_character_enable_and_disable_manage_local_hook_and_backend() {
    let elisp_form = r##"(with-temp-buffer
               (let ((ace-isearch-jump-based-on-one-char t)
                     calls)
                 (cl-letf (((symbol-function
                            'ace-isearch--make-ace-jump-or-avy)
                            (lambda () (push 'one calls)))
                           ((symbol-function
                            'ace-isearch-2--make-ace-jump-or-avy)
                            (lambda () (push 'two calls))))
                   (let ((enable-result (ace-isearch-mode +1))
                         (enabled-hook
                          (and (local-variable-p 'isearch-update-post-hook)
                               isearch-update-post-hook))
                         (disable-result (ace-isearch-mode -1)))
                     (list enable-result
                           enabled-hook
                           disable-result
                           ace-isearch-mode
                           (and (local-variable-p 'isearch-update-post-hook)
                                isearch-update-post-hook)
                           (nreverse calls))))))"##;
    let expect = expect!["OK (t (ace-isearch--jumper-function t) nil nil nil (one))"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_mode_two_character_enable_selects_two_character_backend() {
    let elisp_form = r##"(with-temp-buffer
               (let ((ace-isearch-jump-based-on-one-char nil)
                     calls)
                 (cl-letf (((symbol-function
                            'ace-isearch--make-ace-jump-or-avy)
                            (lambda () (push 'one calls)))
                           ((symbol-function
                            'ace-isearch-2--make-ace-jump-or-avy)
                            (lambda () (push 'two calls))))
                   (list
                    (ace-isearch-mode +1)
                    ace-isearch-mode
                    isearch-update-post-hook
                    (nreverse calls)))))"##;
    let expect = expect!["OK (t t (ace-isearch--jumper-function t) (two))"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_mode_repeated_enable_keeps_one_hook_but_reclassifies_each_time() {
    let elisp_form = r##"(with-temp-buffer
               (let ((ace-isearch-jump-based-on-one-char t)
                     (classifications 0))
                 (cl-letf (((symbol-function
                            'ace-isearch--make-ace-jump-or-avy)
                            (lambda ()
                              (setq classifications
                                    (1+ classifications)))))
                   (ace-isearch-mode +1)
                   (ace-isearch-mode +1)
                   (list
                    ace-isearch-mode
                    (cl-count 'ace-isearch--jumper-function
                              isearch-update-post-hook)
                    classifications))))"##;
    let expect = expect!["OK (t 1 2)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_mode_state_and_update_hook_are_buffer_local() {
    let elisp_form = r##"(let ((first (generate-new-buffer "ace-first"))
                   (second (generate-new-buffer "ace-second")))
               (unwind-protect
                   (cl-letf (((symbol-function
                              'ace-isearch--make-ace-jump-or-avy)
                              (lambda () nil)))
                     (with-current-buffer first
                       (ace-isearch-mode +1))
                     (list
                      (with-current-buffer first
                        (list ace-isearch-mode
                              (memq 'ace-isearch--jumper-function
                                    isearch-update-post-hook)))
                      (with-current-buffer second
                        (list ace-isearch-mode
                              (memq 'ace-isearch--jumper-function
                                    isearch-update-post-hook)))))
                 (kill-buffer first)
                 (kill-buffer second)))"##;
    let expect = expect!["OK ((t (ace-isearch--jumper-function t)) (nil nil))"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_turn_on_enables_non_minibuffers_and_skips_minibuffers() {
    let elisp_form = r##"(let (calls)
               (cl-letf (((symbol-function 'ace-isearch-mode)
                          (lambda (&optional argument)
                            (push argument calls)
                            'mode-result))
                         ((symbol-function 'minibufferp)
                          (lambda (&optional _buffer) nil)))
                 (let ((normal-result (ace-isearch--turn-on)))
                   (cl-letf (((symbol-function 'minibufferp)
                              (lambda (&optional _buffer) t)))
                     (list normal-result
                           (ace-isearch--turn-on)
                           (nreverse calls))))))"##;
    let expect = expect!["OK (mode-result nil (1))"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_global_mode_variable_and_generated_helper_metadata_match() {
    let elisp_form = r##"(list
               global-ace-isearch-mode
               (get 'global-ace-isearch-mode 'globalized-minor-mode)
               (get 'global-ace-isearch-mode 'variable-documentation)
               (get 'global-ace-isearch-mode 'custom-type)
               (assq 'global-ace-isearch-mode
                     (get 'ace-isearch 'custom-group))
               (fboundp
                'global-ace-isearch-mode-enable-in-buffer)
               (help-function-arglist
                'global-ace-isearch-mode-enable-in-buffer
                t)
               (documentation
                'global-ace-isearch-mode-enable-in-buffer
                t)
               (file-name-nondirectory
                (symbol-file
                 'global-ace-isearch-mode-enable-in-buffer
                 'defun))
               (memq
                'global-ace-isearch-mode-enable-in-buffer
                after-change-major-mode-hook))"##;
    let expect = expect![[
        r#"OK (nil t "Non-nil if Global Ace-Isearch mode is enabled.\nSee the `global-ace-isearch-mode' command\nfor a description of this minor mode.\nSetting this variable directly does not take effect;\neither customize it (see the info node `Easy Customization')\nor call the function `global-ace-isearch-mode'." boolean (global-ace-isearch-mode custom-variable) t nil nil "ace-isearch.el" nil)"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_global_mode_enable_and_disable_manage_actual_hook_registration() {
    let elisp_form = r##"(let ((before
                    (copy-sequence
                     after-change-major-mode-hook)))
               (let ((enable-result
                      (global-ace-isearch-mode +1)))
                 (let ((enabled-state
                        global-ace-isearch-mode)
                       (while-enabled
                        (copy-sequence
                         after-change-major-mode-hook)))
                   (let ((disable-result
                          (global-ace-isearch-mode -1)))
                     (list
                      enable-result
                      enabled-state
                      (cl-set-difference
                       while-enabled
                       before
                       :test #'eq)
                      disable-result
                      (cl-set-difference
                       after-change-major-mode-hook
                       before
                       :test #'eq)
                      (cl-set-difference
                       before
                       after-change-major-mode-hook
                       :test #'eq))))))"##;
    let expect = expect!["OK (t t (global-ace-isearch-mode-enable-in-buffer) nil nil nil)"];
    assert_ace_isearch_parity(elisp_form, expect);
}
