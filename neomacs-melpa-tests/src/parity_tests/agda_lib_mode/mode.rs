use expect_test::expect;

use super::assert_agda_lib_mode_parity;

#[test]
fn agda_lib_mode_installs_complete_buffer_local_state_without_changing_document_text() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "name: sample\ninclude: src\n")
         (goto-char 8)
         (set-buffer-modified-p nil)
         (agda-lib-mode)
         (list
          major-mode
          mode-name
          (derived-mode-p
           'text-mode)
          (eq
           (current-local-map)
           agda-lib-mode-map)
          (eq
           (syntax-table)
           agda-lib-mode-syntax-table)
          (eq
           local-abbrev-table
           agda-lib-mode-abbrev-table)
          font-lock-defaults
          comment-start
          comment-start-skip
          comment-end
          comment-end-skip
          (mapcar
           #'local-variable-p
           '(font-lock-defaults
             comment-start
             comment-start-skip
             comment-end
             comment-end-skip))
          (point)
          (buffer-string)
          (buffer-modified-p)))"##;
    let expect = expect![[
        r#"OK (agda-lib-mode "agda-lib" text-mode t t t (agda-lib-font-lock-keywords t t nil nil) "-- " "\\(^\\| \\)-- +" "" "[ \11]*\\(\\s>\\|\n\\)" (t t t t t) 8 "name: sample\ninclude: src\n" nil)"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_generated_map_syntax_abbrev_and_hook_artifacts_match_text_mode() {
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
              (file-name-nondirectory
               (symbol-file
                symbol
                'defvar))
              (cond
               ((eq symbol
                    'agda-lib-mode-map)
                (list
                 (keymapp value)
                 (eq
                  (keymap-parent value)
                  text-mode-map)
                 (lookup-key
                  value
                  (kbd "M-q"))))
               ((eq symbol
                    'agda-lib-mode-syntax-table)
                (list
                 (char-table-p value)
                 (eq
                  (char-table-parent value)
                  text-mode-syntax-table)
                 (mapcar
                  (lambda (character)
                    (list
                     character
                     (char-table-range
                      value character)
                     (char-table-range
                      text-mode-syntax-table
                      character)))
                  '(?- ?: ?\n ?_))))
               ((eq symbol
                    'agda-lib-mode-abbrev-table)
                (list
                 (abbrev-table-p value)
                 (abbrev-table-get
                  value
                  :parents)))
               (t value)))))
         '(agda-lib-mode-map
           agda-lib-mode-syntax-table
           agda-lib-mode-abbrev-table
           agda-lib-mode-hook))"##;
    let expect = expect![[
        r#"OK ((agda-lib-mode-map t t nil "Keymap for `agda-lib-mode'." "agda-lib-mode.el" (t nil nil)) (agda-lib-mode-syntax-table t t nil "Syntax table for `agda-lib-mode'." "agda-lib-mode.el" (t nil ((45 #1=(3) #1#) (58 #2=(1) #2#) (10 #3=(0) #3#) (95 #1# #1#)))) (agda-lib-mode-abbrev-table t t nil "Abbrev table for `agda-lib-mode'." "agda-lib-mode.el" (t nil)) (agda-lib-mode-hook t t nil "Hook run after entering `agda-lib-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "agda-lib-mode.el" nil))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_parent_and_mode_hooks_run_in_order_with_final_local_state_visible() {
    let elisp_form = r##"(let (events)
         (let ((text-mode-hook
                (list
                 (lambda ()
                   (push
                    (list
                     'text-hook
                     major-mode
                     font-lock-defaults
                     comment-start)
                    events))))
               (agda-lib-mode-hook
                (list
                 (lambda ()
                   (push
                    (list
                     'agda-hook
                     major-mode
                     font-lock-defaults
                     comment-start)
                    events)))))
           (with-temp-buffer
             (agda-lib-mode)
             (list
              (nreverse events)
              major-mode
              mode-name))))"##;
    let expect = expect![[
        r#"OK (((text-hook agda-lib-mode #1=(agda-lib-font-lock-keywords t t nil nil) "-- ") (agda-hook agda-lib-mode #1# "-- ")) agda-lib-mode "agda-lib")"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_reentry_restores_every_declared_local_and_runs_hooks_each_time() {
    let elisp_form = r##"(let (calls)
         (let ((agda-lib-mode-hook
                (list
                 (lambda ()
                   (push
                    (list
                     font-lock-defaults
                     comment-start
                     comment-end)
                    calls)))))
           (with-temp-buffer
             (agda-lib-mode)
             (setq-local
              font-lock-defaults
              '(corrupt)
              comment-start
              "# "
              comment-start-skip
              "# +"
              comment-end
              "END"
              comment-end-skip
              "SKIP")
             (agda-lib-mode)
             (list
              font-lock-defaults
              comment-start
              comment-start-skip
              comment-end
              comment-end-skip
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (#1=(agda-lib-font-lock-keywords t t nil nil) "-- " "\\(^\\| \\)-- +" "" "[ \11]*\\(\\s>\\|\n\\)" ((#1# "-- " "") (#1# "-- " "")))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_state_is_isolated_across_real_buffers_and_does_not_leak_to_text_mode() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer
                 " *agda-lib-first*"))
               (second
                (generate-new-buffer
                 " *agda-lib-second*")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (agda-lib-mode)
                 (setq-local
                  comment-start
                  "CUSTOM "
                  agda-lib-font-lock-keywords
                  '(("custom" . font-lock-warning-face))))
               (with-current-buffer second
                 (agda-lib-mode))
               (list
                (with-current-buffer first
                  (list
                   major-mode
                   comment-start
                   agda-lib-font-lock-keywords))
                (with-current-buffer second
                  (list
                   major-mode
                   comment-start
                   agda-lib-font-lock-keywords))
                (with-temp-buffer
                  (text-mode)
                  (list
                   major-mode
                   comment-start
                   (local-variable-p
                    'agda-lib-font-lock-keywords)))))
           (when
               (buffer-live-p first)
             (kill-buffer first))
           (when
               (buffer-live-p second)
             (kill-buffer second))))"##;
    let expect = expect![[
        r#"OK ((agda-lib-mode "CUSTOM " (("custom" . font-lock-warning-face))) (agda-lib-mode "-- " (("\\(^\\| \\)-- .*" . font-lock-comment-face) ("^\\([^ ]+:\\)" (1 font-lock-keyword-face)))) (text-mode nil nil))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_auto_selection_handles_case_backup_suffixes_and_nonmatches() {
    let elisp_form = r##"(mapcar
         (lambda (name)
           (with-temp-buffer
             (setq buffer-file-name
                   name)
             (set-auto-mode)
             (list
              name
              major-mode
              mode-name
              comment-start)))
         '("/workspace/project.agda-lib"
           "/workspace/UPPER.AGDA-LIB"
           "/workspace/project.agda-lib~"
           "/workspace/project.agda-lib.txt"
           "/workspace/agda-lib"))"##;
    let expect = expect![[
        r#"OK (("/workspace/project.agda-lib" agda-lib-mode "agda-lib" "-- ") ("/workspace/UPPER.AGDA-LIB" agda-lib-mode "agda-lib" "-- ") ("/workspace/project.agda-lib~" agda-lib-mode "agda-lib" "-- ") ("/workspace/project.agda-lib.txt" text-mode "Text" nil) ("/workspace/agda-lib" fundamental-mode "Fundamental" nil))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_activation_preserves_narrowing_point_mark_and_read_only_state() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "outside\nname: sample\ninclude: src\noutside\n")
         (narrow-to-region 9 35)
         (goto-char 15)
         (set-mark 30)
         (setq buffer-read-only t)
         (agda-lib-mode)
         (list
          (point-min)
          (point-max)
          (point)
          (mark)
          buffer-read-only
          (buffer-substring-no-properties
           (point-min)
           (point-max))
          major-mode))"##;
    let expect = expect![[r#"OK (9 35 15 30 t "name: sample\ninclude: src\n" agda-lib-mode)"#]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}
