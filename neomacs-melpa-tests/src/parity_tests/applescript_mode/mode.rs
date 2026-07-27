use expect_test::expect;

use super::assert_applescript_mode_parity;

#[test]
fn applescript_mode_initializes_all_editing_locals_map_syntax_and_outline_contracts() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (list
          major-mode
          mode-name
          (eq
           (current-local-map)
           as-mode-map)
          (eq
           (syntax-table)
           as-mode-syntax-table)
          (eq
           local-abbrev-table
           applescript-mode-abbrev-table)
          font-lock-defaults
          paragraph-separate
          paragraph-start
          require-final-newline
          comment-start
          comment-end
          comment-start-skip
          comment-column
          outline-regexp
          outline-level
          (mapcar
           (lambda (symbol)
             (list
              symbol
              (local-variable-p
               symbol)
              (and
               (boundp symbol)
               (symbol-value symbol))))
           '(comment-indent-function
             indent-region-function
             indent-line-function
             add-log-current-defun-function
             fill-paragraph-function))))"##;
    let expect = expect![[
        r#"OK (applescript-mode "AppleScript" t t t (applescript-font-lock-keywords) "[ \11\n\f]*$" "[ \11\n\f]*$" t "-- " "" "---*[ \11]*" 40 "\\([ \11]*\\(on\\|to\\|if\\|repeat\\|tell\\|end\\)\\|--\\)" as-outline-level ((comment-indent-function t comment-indent-default) (indent-region-function t indent-region-line-by-line) (indent-line-function t indent-relative) (add-log-current-defun-function t nil) (fill-paragraph-function t nil)))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_clears_preexisting_locals_and_runs_its_hook_once_in_the_new_mode() {
    let elisp_form = r##"(with-temp-buffer
         (setq-local
          applescript-test-local
          :stale)
         (let ((applescript-mode-hook
                '((lambda ()
                    (setq applescript-test-events
                          (append
                           applescript-test-events
                           (list
                            (list
                             major-mode
                             mode-name
                             (local-variable-p
                              'applescript-test-local)))))))))
           (applescript-mode)
           (list
            (local-variable-p
             'applescript-test-local)
            (boundp
             'applescript-test-local)
            applescript-test-events)))"##;
    let expect = expect![[r#"OK (nil nil ((applescript-mode "AppleScript" nil)))"#]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_real_file_and_shebang_detection_activate_the_mode() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (with-temp-buffer
             (setq-local
              buffer-file-name
              (car spec))
             (when (cadr spec)
               (insert
                (cadr spec)))
             (set-auto-mode)
             (list
              spec
              major-mode
              mode-name)))
         '(("/workspace/example.applescript" nil)
           ("/workspace/example.scpt" nil)
           ("/workspace/example.txt"
            "#!/usr/bin/osascript\nreturn 42\n")
           ("/workspace/example.txt"
            "#!/usr/bin/env osascript\nreturn 42\n")
           ("/workspace/example.txt"
            "return 42\n")))"##;
    let expect = expect![[
        r##"OK ((("/workspace/example.applescript" nil) applescript-mode "AppleScript") (("/workspace/example.scpt" nil) applescript-mode "AppleScript") (("/workspace/example.txt" "#!/usr/bin/osascript\nreturn 42\n") applescript-mode "AppleScript") (("/workspace/example.txt" "#!/usr/bin/env osascript\nreturn 42\n") applescript-mode "AppleScript") (("/workspace/example.txt" "return 42\n") text-mode "Text"))"##
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_comment_and_uncomment_round_trip_real_selected_lines() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (insert
          "set greeting to \"hello\"\n"
          "display dialog greeting\n"
          "\n"
          "return greeting\n")
         (let ((original
                (buffer-string)))
           (goto-char
            (point-min))
           (forward-line 1)
           (let ((start
                  (line-beginning-position)))
             (forward-line 2)
             (comment-region
              start
              (point))
             (let ((commented
                    (buffer-string))
                   (comment-syntax
                    (progn
                      (goto-char start)
                      (search-forward
                       "display")
                      (nth 4
                           (syntax-ppss
                            (match-beginning 0))))))
               (uncomment-region
                start
                (point))
               (list
                original
                commented
                (buffer-string)
                comment-syntax
                (= (point-min) 1))))))"##;
    let expect = expect![[
        r#"OK ("set greeting to \"hello\"\ndisplay dialog greeting\n\nreturn greeting\n" "set greeting to \"hello\"\n-- display dialog greeting\n\nreturn greeting\n" "set greeting to \"hello\"\ndisplay dialog greeting\n\nreturn greeting\n" t t)"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_syntax_parser_distinguishes_line_block_comments_strings_and_code() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (insert
          "set value to \"-- not comment (* text *)\"\n"
          "-- line comment with \"quote\"\n"
          "(* block comment\n"
          "   nested-looking -- marker\n"
          "*)\n"
          "display dialog value\n")
         (syntax-propertize
          (point-max))
         (list
          (applescript-test-syntax-at
           "not comment")
          (applescript-test-syntax-at
           "line comment")
          (applescript-test-syntax-at
           "block comment")
          (applescript-test-syntax-at
           "nested-looking")
          (applescript-test-syntax-at
           "display")))"##;
    let expect = expect!["OK ((34 nil 14) (nil t 42) (nil t 71) (nil t 71) (nil nil nil))"];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_outline_levels_follow_indentation_for_handlers_blocks_and_comments() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (insert
          "on greet(name)\n"
          "    tell application \"Finder\"\n"
          "        if name is not \"\" then\n"
          "            display dialog name\n"
          "        end if\n"
          "    end tell\n"
          "end greet\n"
          "-- section\n")
         (goto-char
          (point-min))
         (let (rows)
           (while
               (not
                (eobp))
             (setq rows
                   (append
                    rows
                    (list
                     (list
                      (buffer-substring-no-properties
                       (line-beginning-position)
                       (line-end-position))
                      (and
                       (looking-at
                        outline-regexp)
                       t)
                      (as-outline-level)))))
             (forward-line 1))
           rows))"##;
    let expect = expect![[
        r#"OK (("on greet(name)" t 0) ("    tell application \"Finder\"" t 4) ("        if name is not \"\" then" t 8) ("            display dialog name" nil 12) ("        end if" t 8) ("    end tell" t 4) ("end greet" t 0) ("-- section" t 0))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_abbrev_comment_and_final_newline_settings_drive_real_editing_commands() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (define-abbrev
           local-abbrev-table
           "dlg"
           "display dialog")
         (abbrev-mode 1)
         (insert
          "dlg ")
         (expand-abbrev)
         (let ((expanded
                (buffer-string)))
           (insert
            "message")
           (comment-dwim nil)
           (let ((commented
                  (buffer-string)))
             (goto-char
              (point-max))
             (insert
              "\nreturn 1")
             (let* ((before
                     (buffer-string))
                    (path
                     (expand-file-name
                      "applescript-final-newline.scpt"
                      (getenv
                       "NEOMACS_TEST_SANDBOX_ROOT")))
                    (make-backup-files nil)
                    (backup-inhibited t)
                    (auto-save-default nil))
               (setq-local
                buffer-file-name
                path)
               (save-buffer)
               (prog1
                   (list
                    expanded
                    commented
                    before
                    (buffer-string)
                    (with-temp-buffer
                      (insert-file-contents-literally
                       path)
                      (buffer-string))
                    (bolp)
                    (buffer-modified-p))
                 (delete-file
                  path))))))"##;
    let expect = expect![[
        r#"OK ("display dialog " "display dialog message\11\11\11-- " "display dialog message\11\11\11-- \nreturn 1" "display dialog message\11\11\11-- \nreturn 1\n" "display dialog message\11\11\11-- \nreturn 1\n" nil nil)"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}
