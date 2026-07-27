use expect_test::expect;

use super::assert_apples_mode_parity;

#[test]
fn entering_mode_sets_the_complete_editing_contract_and_runs_the_user_hook_once() {
    let elisp_form = r##"(with-temp-buffer
                (let ((apples-mode-test-events nil)
                      (apples-mode-hook nil))
                  (setq apples-mode-hook
                        (list
                         (lambda ()
                           (setq apples-mode-test-events
                                 (cons
                                  (list major-mode mode-name)
                                  apples-mode-test-events)))))
                  (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                  (insert "set answer to 42\n")
                  (set-buffer-modified-p nil)
                  (apples-mode)
                  (list
                   major-mode mode-name
                   (eq (current-local-map) apples-mode-map)
                   (eq (syntax-table) apples-mode-syntax-table)
                   font-lock-defaults
                   comment-start comment-end comment-start-skip comment-column
                   indent-line-function
                   imenu-generic-expression
                   (buffer-string)
                   (buffer-modified-p)
                   apples-mode-test-events)))"##;
    let expect = expect![[
        r#"OK (apples-mode "AppleScript" t t (apples-font-lock-keywords) "-- " "" "\\(?:#\\|---*\\|(\\*\\)+[ \11]*" 40 apples-indent-line (("Variables" "^\\s-*set\\s-+\\(.+\\)\\s-+to" . #1=(1)) ("Tells" "^\\s-*tell\\s-+\\(.+\\)$" . #1#) ("Handlers" "^\\s-*\\(?:on\\|to\\)\\s-+\\(.+\\)$" . #1#)) "set answer to 42\n" nil ((apples-mode "AppleScript")))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn configured_keymap_binds_every_command_and_setup_is_idempotent() {
    let elisp_form = r##"(let ((apples-plist nil)
                     (apples-keymap
                      '(("C-c a r" . apples-run-region/buffer)
                        ("C-c a e" . apples-end-completion)
                        ("C-c a c" . apples-compile)
                        ("<f8>" . apples-open-scratch))))
                (setq apples-mode-map (make-sparse-keymap))
                (apples-keymap-setup)
                (let ((first
                       (mapcar
                        (lambda (entry)
                          (cons (car entry)
                                (lookup-key apples-mode-map
                                            (read-kbd-macro (car entry)))))
                        apples-keymap)))
                  (setq apples-keymap
                        '(("C-c a r" . apples-run-file)))
                  (apples-keymap-setup)
                  (list first
                        (lookup-key apples-mode-map (kbd "C-c a r"))
                        (apples-plist-get :keybinded?))))"##;
    let expect = expect![[
        r#"OK ((("C-c a r" . apples-run-region/buffer) ("C-c a e" . apples-end-completion) ("C-c a c" . apples-compile) ("<f8>" . apples-open-scratch)) apples-run-region/buffer t)"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn syntax_table_distinguishes_strings_line_comments_block_comments_records_and_lists() {
    let elisp_form = r##"(with-temp-buffer
                (set-syntax-table apples-mode-syntax-table)
                (insert
                 "set textValue to \"-- not comment\"\n"
                 "-- line comment\n"
                 "# shell-style comment\n"
                 "(* block\ncomment *)\n"
                 "set recordValue to {name:\"Ada\", active:true}\n")
                (let ((positions nil))
                  (dolist (needle '("not comment" "line comment" "shell-style"
                                    "block" "comment *)" "name:" "active:true"))
                    (goto-char (point-min))
                    (search-forward needle)
                    (let ((state (syntax-ppss (match-beginning 0))))
                      (push (list needle (nth 3 state) (nth 4 state)
                                  (char-syntax
                                   (char-after (match-beginning 0))))
                            positions)))
                  (nreverse positions)))"##;
    let expect = expect![[
        r#"OK (("not comment" 34 nil 119) ("line comment" nil t 119) ("shell-style" nil t 119) ("block" nil t 119) ("comment *)" nil t 119) ("name:" nil nil 119) ("active:true" nil nil 119))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn comment_commands_round_trip_a_selected_real_script_region() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "set firstValue to 1\n"
                 "set secondValue to 2\n"
                 "return firstValue + secondValue\n")
                (let ((original (buffer-string)))
                  (apples-comment-or-uncomment-region
                   (point-min) (point-max))
                  (let ((commented (buffer-string)))
                    (apples-comment-or-uncomment-region
                     (point-min) (point-max))
                    (list original commented (buffer-string)
                          (equal original (buffer-string))))))"##;
    let expect = expect![[
        r#"OK ("set firstValue to 1\nset secondValue to 2\nreturn firstValue + secondValue\n" "-- set firstValue to 1\n-- set secondValue to 2\n-- return firstValue + secondValue\n" "set firstValue to 1\nset secondValue to 2\nreturn firstValue + secondValue\n" t)"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn imenu_extracts_handlers_tells_and_assignments_from_a_practical_script() {
    let elisp_form = r##"(progn
                (require 'imenu)
                (with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "property projectName : \"Neomacs\"\n"
                 "on buildGreeting(personName)\n"
                 "set greetingText to \"Hello, \" & personName\n"
                 "tell application \"Finder\"\n"
                 "set desktopItems to every item of desktop\n"
                 "end tell\n"
                 "return greetingText\n"
                 "end buildGreeting\n")
                (let ((index
                       (imenu--generic-function
                        apples-imenu-generic-expression)))
                  (mapcar
                   (lambda (group)
                     (cons
                      (car group)
                      (mapcar #'car (cdr group))))
                   index))))"##;
    let expect = expect![[
        r#"OK (("Handlers" "buildGreeting(personName)") ("Tells" "application \"Finder\"") ("Variables" "greetingText" "desktopItems"))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_version_uses_external_probe_once_then_serves_the_cached_value() {
    let elisp_form = r##"(let ((apples-plist nil)
                     (calls nil))
                (cl-letf (((symbol-function 'call-process)
                           (lambda (program _in destination _display &rest args)
                             (push (cons program args) calls)
                             (with-current-buffer destination
                               (insert "2.10\n"))
                             0)))
                  (list
                   (apples-applescript-version)
                   (apples-applescript-version)
                   (nreverse calls)
                   (apples-plist-get :AS-version))))"##;
    let expect =
        expect![[r#"OK ("2.10" nil (("osascript" "-e" "AppleScript's version")) "2.10")"#]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn scratch_workflow_restores_persists_and_reopens_content_in_a_stable_workspace_file() {
    let elisp_form = r##"(let* ((apples-tmp-dir
                          (expand-file-name
                           "apples-scratch-contract"
                           temporary-file-directory))
                         (apples-plist
                          (list :AS-version "2.1"
                                :tmp-files '(apples-tmp-scratch)))
                         (scratch (get-buffer "*apples-scratch*")))
                    (when scratch (kill-buffer scratch))
                    (unwind-protect
                        (progn
                          (apples-tmp-files-setup)
                          (with-temp-file apples-tmp-scratch
                            (insert "set restoredValue to 7\n"))
                          (cl-letf (((symbol-function 'pop-to-buffer)
                                     (lambda (buffer)
                                       (set-buffer buffer)
                                       buffer)))
                            (apples-open-scratch))
                          (let ((restored (buffer-string))
                                (mode major-mode))
                            (goto-char (point-max))
                            (insert "return restoredValue\n")
                            (apples-save-scratch)
                            (list
                             restored mode
                             (with-temp-buffer
                               (insert-file-contents apples-tmp-scratch)
                               (buffer-string))
                             (memq 'apples-save-scratch
                                   (buffer-local-value
                                    'kill-buffer-hook
                                    (current-buffer))))))
                      (when (get-buffer "*apples-scratch*")
                        (kill-buffer "*apples-scratch*"))))"##;
    let expect = expect![[
        r#"OK ("set restoredValue to 7\n" apples-mode "set restoredValue to 7\nreturn restoredValue\n" (apples-save-scratch t))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}
