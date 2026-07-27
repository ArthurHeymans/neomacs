use expect_test::expect;

use super::assert_apdl_mode_parity;

#[test]
fn mode_activation_establishes_the_complete_editing_environment() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil))
    (apdl-mode)
    (list
     major-mode mode-name
     apdl-previous-major-mode
     (eq (current-local-map) apdl-mode-map)
     (eq (syntax-table) apdl-mode-syntax-table)
     (eq local-abbrev-table apdl-mode-abbrev-table)
     completion-ignore-case
     indent-line-function
     comment-start comment-padding comment-add comment-column
     comment-start-skip
     parens-require-spaces
     outline-regexp
     font-lock-defaults
     font-lock-mode
     (local-variable-p 'apdl-user-variable-regexp)
     (local-variable-p 'apdl-hide-region-overlays)
     (overlayp apdl-help-overlay))))"##;
    let expect = expect![[
        r#"OK (apdl-mode "APDL" fundamental-mode t t t t apdl-indent-line-function "!" " " 1 15 "\\S<+\\S-*" nil "![@]+" ((apdl-font-lock-keywords apdl-font-lock-keywords-1 apdl-font-lock-keywords-2) nil 'case-ignore) nil t t t)"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn default_mode_hook_enables_outline_and_mode_toggle_restores_the_previous_mode() {
    let elisp_form = r##"(with-temp-buffer
  (emacs-lisp-mode)
  (let ((before major-mode)
        (apdl-dynamic-highlighting-flag nil))
    (apdl-mode)
    (let ((during
           (list major-mode outline-minor-mode
                 (local-variable-p 'outline-minor-mode)
                 apdl-previous-major-mode)))
      (apdl-toggle-mode)
      (list before during major-mode mode-name
            (eq major-mode before)))))"##;
    let expect = expect![[
        r#"OK (emacs-lisp-mode (apdl-mode t t emacs-lisp-mode) emacs-lisp-mode ("Elisp" (lexical-binding (:propertize "/l" help-echo "Using lexical-binding mode") (:propertize "/d" help-echo "Using old dynamic scoping mode\nmouse-1: Enable lexical-binding mode" face warning mouse-face mode-line-highlight local-map (keymap (mode-line keymap (mouse-1 . elisp-enable-lexical-binding)))))) t)"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn extension_dispatch_activates_apdl_mode_for_every_supported_file_kind() {
    let elisp_form = r##"(mapcar
 (lambda (name)
   (with-temp-buffer
     (setq buffer-file-name (concat "/workspace/model/" name))
     (let ((apdl-mode-hook nil)
           (apdl-dynamic-highlighting-flag nil))
       (cl-letf (((symbol-function 'file-attributes)
                  (lambda (&rest _arguments)
                    '(nil nil nil nil nil nil nil 0))))
         (set-auto-mode)
         (list name major-mode mode-name)))))
 '("analysis.mac" "solver.ans" "mesh.dat" "input.inp"))"##;
    let expect = expect![[
        r#"OK (("analysis.mac" apdl-mode "APDL") ("solver.ans" apdl-mode "APDL") ("mesh.dat" apdl-mode "APDL") ("input.inp" apdl-mode "APDL"))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn mode_keymap_exposes_real_editing_navigation_solver_and_template_workflows() {
    let elisp_form = r##"(mapcar
 (lambda (key)
   (list key (lookup-key apdl-mode-map (kbd key))))
 '("TAB" "RET" "SPC" "M-a" "M-e" "M-<up>" "M-<down>"
   "C-M-f" "C-M-b" "C-M-d" "C-M-u" "C-M-h"
   "C-c C-a" "C-c C-b" "C-c C-c" "C-c C-d"
   "C-c C-h" "C-c C-j" "C-c C-m" "C-c C-s"
   "C-c C-u" "C-c C-v" "C-c C-w" "C-c C-x"))"##;
    let expect = expect![[
        r#"OK (("TAB" nil) ("RET" nil) ("SPC" apdl-electric-space) ("M-a" apdl-command-start) ("M-e" apdl-command-end) ("M-<up>" nil) ("M-<down>" nil) ("C-M-f" apdl-next-block-end) ("C-M-b" apdl-previous-block-start-and-conditional) ("C-M-d" apdl-down-block) ("C-M-u" apdl-up-block) ("C-M-h" apdl-mark-block) ("C-c C-a" apdl-align) ("C-c C-b" apdl-browse-apdl-help) ("C-c C-c" apdl-send-to-ansys) ("C-c C-d" apdl-do) ("C-c C-h" apdl-mode-help) ("C-c C-j" apdl-send-to-apdl-and-proceed) ("C-c C-m" apdl-start-ansys) ("C-c C-s" apdl-display-skeleton) ("C-c C-u" apdl-copy-or-send-above) ("C-c C-v" apdl-display-variables) ("C-c C-w" apdl-display-wb-skeleton) ("C-c C-x" apdl-start-classics))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn public_version_command_reports_the_exact_mode_and_ansys_language_baseline() {
    let elisp_form = r##"(let (messages)
  (cl-letf (((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (let ((text (apply #'format format-string arguments)))
                 (push text messages)
                 text))))
    (list apdl-mode-version apdl-mode-update apdl-ansys-version
          (apdl-mode-version)
          (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ("20.7.0" "2021-10-23" "v201" "APDL-Mode version: 20.7.0 (2021-10-23) based on Ansys v201" ("APDL-Mode version: 20.7.0 (2021-10-23) based on Ansys v201"))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}
