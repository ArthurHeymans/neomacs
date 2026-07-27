use expect_test::expect;

use super::{assert_annoying_arrows_mode_autoload_parity, assert_annoying_arrows_mode_parity};

#[test]
fn annoying_arrows_registers_exact_modes_functions_macro_and_defaults() {
    let elisp_form = r##"(list
         (featurep 'annoying-arrows-mode)
         annoying-arrows-too-far-count
         annoying-arrows--current-count
         annoying-arrows--commands
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (commandp symbol)
                  (macrop symbol)
                  (help-function-arglist symbol t)))
          '(annoying-arrows-mode global-annoying-arrows-mode
            annoying-arrows--commands-with-shortcuts
            annoying-arrows--maybe-complain
            add-annoying-arrows-advice aa-add-suggestion
            aa-add-suggestions)))"##;
    let expect = expect![
        "OK (t 10 0 (backward-delete-char backward-delete-char-untabify backward-char forward-char left-char right-char next-line previous-line) ((annoying-arrows-mode t t nil (&optional arg)) (global-annoying-arrows-mode t t nil (&optional arg)) (annoying-arrows--commands-with-shortcuts t nil nil (cmds)) (annoying-arrows--maybe-complain t nil nil (cmd)) (add-annoying-arrows-advice t nil t (cmd alternatives)) (aa-add-suggestion t nil nil (cmd alternative)) (aa-add-suggestions t nil nil (cmd alternatives))))"
    ];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_registers_exact_advised_commands_and_alternatives() {
    let elisp_form = r##"(mapcar
         (lambda (command)
           (list command
                 (memq command annoying-arrows--commands)
                 (get command 'annoying-arrows--alts)
                 (ad-find-advice command 'before 'annoying-arrows)))
         '(previous-line next-line right-char left-char
           forward-char backward-char
           backward-delete-char-untabify backward-delete-char))"##;
    let expect = expect![
        "OK ((previous-line #1=(previous-line) (ace-jump-mode backward-paragraph isearch-backward ido-imenu smart-up) (annoying-arrows nil t (advice lambda nil (when annoying-arrows-mode (annoying-arrows--maybe-complain 'previous-line))))) (next-line #2=(next-line . #1#) (ace-jump-mode forward-paragraph isearch-forward ido-imenu smart-down) (annoying-arrows nil t (advice lambda nil (when annoying-arrows-mode (annoying-arrows--maybe-complain 'next-line))))) (right-char #3=(right-char . #2#) (jump-char-forward iy-go-to-char right-word smart-forward) (annoying-arrows nil t (advice lambda nil (when annoying-arrows-mode (annoying-arrows--maybe-complain 'right-char))))) (left-char #4=(left-char . #3#) (jump-char-backward iy-go-to-char-backward left-word smart-backward) (annoying-arrows nil t (advice lambda nil (when annoying-arrows-mode (annoying-arrows--maybe-complain 'left-char))))) (forward-char #5=(forward-char . #4#) (jump-char-forward iy-go-to-char right-word smart-forward) (annoying-arrows nil t (advice lambda nil (when annoying-arrows-mode (annoying-arrows--maybe-complain 'forward-char))))) (backward-char #6=(backward-char . #5#) (jump-char-backward iy-go-to-char-backward left-word smart-backward) (annoying-arrows nil t (advice lambda nil (when annoying-arrows-mode (annoying-arrows--maybe-complain 'backward-char))))) (backward-delete-char-untabify #7=(backward-delete-char-untabify . #6#) (backward-kill-word kill-region-or-backward-word subword-backward-kill) (annoying-arrows nil t (advice lambda nil (when annoying-arrows-mode (annoying-arrows--maybe-complain 'backward-delete-char-untabify))))) (backward-delete-char (backward-delete-char . #7#) (backward-kill-word kill-region-or-backward-word subword-backward-kill) (annoying-arrows nil t (advice lambda nil (when annoying-arrows-mode (annoying-arrows--maybe-complain 'backward-delete-char))))))"
    ];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_mode_and_global_mode_metadata_are_exact() {
    let elisp_form = r##"(list
         (get 'annoying-arrows-mode 'custom-group)
         (get 'global-annoying-arrows-mode 'custom-group)
         (get 'global-annoying-arrows-mode 'globalized-minor-mode)
         (get 'global-annoying-arrows-mode 'custom-autoload)
         (get 'annoying-arrows-mode 'custom-autoload)
         (assq 'annoying-arrows-mode minor-mode-alist)
         (assq 'annoying-arrows-mode minor-mode-map-alist))"##;
    let expect = expect![[r#"OK (nil nil t t nil (annoying-arrows-mode "") nil)"#]];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_descriptor_records_exact_pin_requirement_and_payload() {
    let elisp_form = r##"(let* ((description
                          (cadr (assq 'annoying-arrows-mode package-alist)))
               (directory (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar
            (lambda (file)
              (let ((relative (file-relative-name file directory)))
                (list relative
                      (file-attribute-size (file-attributes file))
                      (secure-hash 'sha256 file))))
            (directory-files-recursively directory "." nil))
           (lambda (a b) (string< (car a) (car b))))))"##;
    let expect = expect![[
        r#"OK (annoying-arrows-mode "20161024.646" nil "Ring the bell if using arrows too much." ((cl-lib (0 5))) (("README-elpa" 182 "6073ecb2f8a610dfd431e2f24c39273c7b8fb605f1f209179c0e76889ea4b289") ("annoying-arrows-mode-autoloads.el" 2523 "bc8309fe1cf63f3a007663a874471b5b81eaa66005ef9dc82dfd94471c57b35d") ("annoying-arrows-mode-pkg.el" 420 "59e5c821b745fdf444294d0545e3c27fe4d3805356f8c7a52178b201202d80c5") ("annoying-arrows-mode.el" 4459 "36c570c0cb392d6427b74813c72f04aecb6f557716b4a79729ec605a426c5577") ("annoying-arrows-mode.elc" 9725 "bfa18079d2917682624a41a950ca2b0807efdbfeacf02d03c7e7af9ccc6c58b4")))"#
    ]];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_autoloads_expose_both_modes_without_loading_feature() {
    let elisp_form = r##"(list
         (featurep 'annoying-arrows-mode)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (commandp symbol)
                  (autoloadp (symbol-function symbol))
                  (symbol-function symbol)))
          '(annoying-arrows-mode global-annoying-arrows-mode)))"##;
    let expect = expect![[
        r#"OK (nil ((annoying-arrows-mode t t (autoload "annoying-arrows-mode" "Annoying-Arrows emacs minor mode.\n\nThis is a minor mode.  If called interactively, toggle the\n`Annoying-Arrows mode' mode.  If the prefix argument is positive, enable\nthe mode, and if it is zero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `annoying-arrows-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled.\n\n(fn &optional ARG)" t nil)) (global-annoying-arrows-mode t t (autoload "annoying-arrows-mode" "Toggle Annoying-Arrows mode in many buffers.\nSpecifically, Annoying-Arrows mode is enabled in all buffers where\n`annoying-arrows-mode' would do it.\n\nWith prefix ARG, enable Global Annoying-Arrows mode if ARG is\npositive; otherwise, disable it.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.\nEnable the mode if ARG is nil, omitted, or is a positive number.\nDisable the mode if ARG is a negative number.\n\nSee `annoying-arrows-mode' for more information on Annoying-Arrows\nmode.\n\n(fn &optional ARG)" t nil))))"#
    ]];
    assert_annoying_arrows_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_reload_keeps_command_registry_deduplicated() {
    let elisp_form = r##"(let ((source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (load source nil t t)
         (list (length annoying-arrows--commands)
               (length (delete-dups
                        (copy-sequence annoying-arrows--commands)))
               (length
                (cl-remove-if-not
                 (lambda (feature) (eq feature 'annoying-arrows-mode))
                 features))
               (featurep 'annoying-arrows-mode)))"##;
    let expect = expect!["OK (8 8 1 t)"];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}
