use expect_test::expect;

use super::assert_act_mode_parity;

#[test]
fn act_mode_derives_from_prog_mode_and_installs_exact_buffer_local_font_lock_defaults() {
    let elisp_form = r##"(with-temp-buffer
         (let ((before-map
                (current-local-map))
               (before-syntax
                (syntax-table)))
           (act-mode)
           (list
            major-mode
            mode-name
            (derived-mode-p
             'prog-mode)
            (eq
             (current-local-map)
             before-map)
            (eq
             (syntax-table)
             before-syntax)
            (eq
             (current-local-map)
             act-mode-map)
            (eq
             (syntax-table)
             act-mode-syntax-table)
            (eq
             local-abbrev-table
             act-mode-abbrev-table)
            font-lock-defaults
            (local-variable-p
             'font-lock-defaults)
            (local-variable-p
             'font-lock-keywords)
            (buffer-modified-p))))"##;
    let expect =
        expect![[r#"OK (act-mode "act" prog-mode nil nil t t t ((act-fontlock)) t nil nil)"#]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_generated_map_syntax_abbrev_and_hook_symbols_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((value
                  (default-value
                   symbol)))
             (list
              symbol
              (boundp symbol)
              (default-boundp symbol)
              (local-variable-if-set-p
               symbol)
              (documentation-property
               symbol
               'variable-documentation
               t)
              (let ((file
                     (symbol-file
                      symbol
                      'defvar)))
                (and file
                     (file-name-nondirectory
                      file)))
              (cond
               ((eq symbol
                    'act-mode-map)
                (list
                 (keymapp value)
                 (eq
                  (keymap-parent value)
                  prog-mode-map)
                 (let (bindings)
                   (map-keymap
                    (lambda
                      (event binding)
                      (push
                       (cons event binding)
                       bindings))
                    value)
                   (nreverse bindings))))
               ((eq symbol
                    'act-mode-syntax-table)
                (list
                 (char-table-p value)
                 (eq
                  (char-table-parent value)
                  prog-mode-syntax-table)
                 (mapcar
                  (lambda (character)
                    (list
                     character
                     (char-table-range
                      value
                      character)
                     (char-table-range
                      prog-mode-syntax-table
                      character)))
                  '(?/ ?\n ?_ ?\"))))
               ((eq symbol
                    'act-mode-abbrev-table)
                (list
                 (abbrev-table-p value)
                 (let (entries)
                   (mapatoms
                    (lambda (entry)
                      (push
                       (symbol-name entry)
                       entries))
                    value)
                   (sort entries
                         #'string<))
                 (abbrev-table-get
                  value
                  :parents)))
               (t value)))))
         '(act-mode-map
           act-mode-syntax-table
           act-mode-abbrev-table
           act-mode-hook))"##;
    let expect = expect![[
        r#"OK ((act-mode-map t t nil "Keymap for `act-mode'." "act-mode.el" (t nil nil)) (act-mode-syntax-table t t nil "Syntax table for `act-mode'." "act-mode.el" (t nil ((47 #1=(3) #1#) (10 #2=(0) #2#) (95 #1# #1#) (34 #3=(7) #3#)))) (act-mode-abbrev-table t t nil "Abbrev table for `act-mode'." "act-mode.el" (t ("") nil)) (act-mode-hook t t nil "Hook run after entering `act-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "act-mode.el" nil))"#
    ]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_auto_selection_matches_case_folded_act_suffix_and_rejects_other_names() {
    let elisp_form = r##"(mapcar
         (lambda (name)
           (with-temp-buffer
             (setq buffer-file-name
                   name)
             (set-auto-mode)
             (list
              name
              major-mode
              mode-name)))
         '("/fixture/main.act"
           "/fixture/UPPER.ACT"
           "/fixture/main.act~"
           "/fixture/act"
           "/fixture/main.act.txt"))"##;
    let expect = expect![[
        r#"OK (("/fixture/main.act" act-mode "act") ("/fixture/UPPER.ACT" act-mode "act") ("/fixture/main.act~" act-mode "act") ("/fixture/act" fundamental-mode "Fundamental") ("/fixture/main.act.txt" text-mode "Text"))"#
    ]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_hook_runs_on_each_activation_and_font_lock_state_is_buffer_local() {
    let elisp_form = r##"(let (calls
               first
               second)
         (let ((act-mode-hook
                (list
                 (lambda ()
                   (push
                    (list
                     major-mode
                     (buffer-name))
                    calls)))))
           (setq first
                 (generate-new-buffer
                  " *act-first*")
                 second
                 (generate-new-buffer
                  " *act-second*"))
           (unwind-protect
               (progn
                 (with-current-buffer first
                   (act-mode)
                   (setq-local font-lock-defaults
                               '(changed))
                   (act-mode))
                 (with-current-buffer second
                   (act-mode))
                 (list
                  (with-current-buffer first
                    font-lock-defaults)
                  (with-current-buffer second
                    font-lock-defaults)
                  (nreverse calls)))
             (when
                 (buffer-live-p first)
               (kill-buffer first))
             (when
                 (buffer-live-p second)
               (kill-buffer second)))))"##;
    let expect = expect![[
        r#"OK (#1=((act-fontlock)) #1# ((act-mode " *act-first*") (act-mode " *act-first*") (act-mode " *act-second*")))"#
    ]];
    assert_act_mode_parity(elisp_form, expect);
}
