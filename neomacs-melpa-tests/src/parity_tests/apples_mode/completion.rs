use expect_test::expect;

use super::assert_apples_mode_parity;

#[test]
fn end_completion_closes_nested_if_repeat_tell_and_handler_blocks_in_order() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (let ((apples-end-completion-hl nil))
                  (insert
                   "on collectNames(theItems)\n"
                   "repeat with itemValue in theItems\n"
                   "tell application \"Finder\"\n"
                   "if exists itemValue then\n"
                   "set end of results to name of itemValue\n")
                  (apples-end-completion)
                  (insert "\n")
                  (apples-end-completion)
                  (insert "\n")
                  (apples-end-completion)
                  (insert "\n")
                  (apples-end-completion)
                  (buffer-string)))"##;
    let expect = expect![[
        r#"OK "on collectNames(theItems)\nrepeat with itemValue in theItems\ntell application \"Finder\"\nif exists itemValue then\nset end of results to name of itemValue\nend if\nend tell\nend repeat\nend collectNames""#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn end_completion_supports_multiword_timeout_transaction_and_terms_blocks() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (let ((apples-end-completion-hl nil))
                  (insert
                   "using terms from application \"Finder\"\n"
                   "with timeout of 5 seconds\n"
                   "with transaction\n"
                   "set answer to 42\n")
                  (apples-end-completion)
                  (insert "\n")
                  (apples-end-completion)
                  (insert "\n")
                  (apples-end-completion)
                  (buffer-string)))"##;
    let expect = expect![[
        r#"OK "using terms from application \"Finder\"\nwith timeout of 5 seconds\nwith transaction\nset answer to 42\nend transaction\nend timeout\nend using terms from""#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn parse_statement_ignores_inline_blocks_comments_strings_and_balances_existing_ends() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "tell application \"Finder\" to activate\n"
                 "-- repeat with x in xs\n"
                 "set quoted to \"if ready then\"\n"
                 "try\n"
                 "repeat with x in {1, 2}\n"
                 "if x is 1 then\n"
                 "set answer to x\n"
                 "end if\n"
                 "set answer to answer + x\n")
                (goto-char (point-max))
                (multiple-value-list (apples-parse-statement)))"##;
    let expect = expect![[r#"OK (96 "repeat" "repeat")"#]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn completion_is_a_noop_inside_string_comment_and_at_top_level() {
    let elisp_form = r##"(let ((run
                         (lambda (text position)
                           (with-temp-buffer
                             (setq apples-plist
                                   (list :AS-version "2.1" :tmp-files nil))
                             (apples-mode)
                             (let ((apples-end-completion-hl nil))
                               (insert text)
                               (goto-char position)
                               (let ((before (buffer-string)))
                                 (apples-end-completion)
                                 (list before (buffer-string) (point))))))))
                (list
                 (funcall run "\"tell application Finder\"" 8)
                 (funcall run "-- repeat with item in values" 12)
                 (funcall run "set answer to 42\n" 18)))"##;
    let expect = expect![[
        r#"OK (("\"tell application Finder\"" "\"tell application Finder\"" 8) ("-- repeat with item in values" "-- repeat with item in values" 12) ("set answer to 42\n" "set answer to 42\n" 18))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn word_highlighting_moves_both_overlays_to_the_opening_and_inserted_end_words() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert "repeat with itemValue in {1, 2}\n    set total to itemValue\n")
                (let ((apples-end-completion-hl 'words)
                      timer-arguments)
                  (cl-letf (((symbol-function 'run-at-time)
                             (lambda (&rest args)
                               (setq timer-arguments args)
                               'fake-timer)))
                    (apples-end-completion)
                    (let* ((overlays (apples-plist-get :end-ovs))
                           (begin (car overlays))
                           (end (cdr overlays)))
                      (list
                       (buffer-string)
                       (list (overlay-start begin) (overlay-end begin)
                             (buffer-substring-no-properties
                              (overlay-start begin) (overlay-end begin)))
                       (list (overlay-start end) (overlay-end end)
                             (buffer-substring-no-properties
                              (overlay-start end) (overlay-end end)))
                       timer-arguments)))))"##;
    let expect = expect![[
        r#"OK ("repeat with itemValue in {1, 2}\n    set total to itemValue\nend repeat" (1 7 "repeat") (60 70 "end repeat") (0.3 nil apples-delete-overlay (#<overlay in no buffer> #<overlay in no buffer>)))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn keyword_inventory_preserves_duplicates_order_and_standard_folder_metadata() {
    let elisp_form = r##"(let* ((all (apples-keywords))
                     (folders (apples-keywords 'standard-folders))
                     (duplicate-count
                      (let ((count 0))
                        (dolist (word (apples-keywords 'reserved-words) count)
                          (when (string= word "contains")
                            (setq count (1+ count)))))))
                (list
                 (length all)
                 (car all)
                 (car (last all))
                 duplicate-count
                 (mapcar
                  (lambda (word)
                    (list
                     word
                     (get-text-property 0 'path word)
                     (get-text-property 0 'posix word)))
                  folders)
                 (apples-keywords 'missing)))"##;
    let expect = expect![[
        r#"OK (303 "about" #("voices" 0 6 (path #1="Macintosh HD:System:Library:Speech:Voices:" posix #2="/System/Library/Speech/Voices/")) 2 ((#("voices" 0 6 (path #1# posix #2#)) "Macintosh HD:System:Library:Speech:Voices:" "/System/Library/Speech/Voices/")) nil)"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn font_lock_classifies_a_practical_script_across_commands_operators_labels_and_records() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "on greet(personName)\n"
                 "set dialogResult to display dialog \"Hello\" default answer personName\n"
                 "set personRecord to {name:personName, active:true}\n"
                 "if text returned of dialogResult is not \"\" then return personRecord\n"
                 "end greet\n")
                (font-lock-ensure)
                (let ((tokens
                       '("greet" "set" "dialogResult" "display dialog"
                         "name:" "true" "if" "is not" "return" "end")))
                  (mapcar
                   (lambda (token)
                     (goto-char (point-min))
                     (search-forward token)
                     (list token
                           (get-text-property
                            (- (point) (length token))
                            'face)))
                   tokens)))"##;
    let expect = expect![[
        r#"OK (("greet" font-lock-function-name-face) ("set" apples-commands) ("dialogResult" font-lock-variable-name-face) ("display dialog" apples-commands) ("name:" apples-records) ("true" apples-reserved-words) ("if" apples-statements) ("is not" apples-operators) ("return" nil) ("end" apples-statements))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}
