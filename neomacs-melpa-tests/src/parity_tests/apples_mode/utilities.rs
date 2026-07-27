use expect_test::expect;

use super::assert_apples_mode_parity;

#[test]
fn regexp_builders_turn_human_readable_keyword_patterns_into_syntax_space_regexps() {
    let elisp_form = r##"(mapcar
                (lambda (input)
                  (list input
                        (apples-replace-re-comma->spaces input)
                        (apples-replace-re-space->spaces input)))
                '("tell,application"
                  "repeat with item in values"
                  "set,[[:word:]_]+,to"
                  "a  reference   to"
                  "already\\s-+encoded"))"##;
    let expect = expect![[
        r#"OK (("tell,application" "tell\\s-+application" "tell,application") ("repeat with item in values" "repeat with item in values" "repeat\\s-+with\\s-+item\\s-+in\\s-+values") ("set,[[:word:]_]+,to" "set\\s-+[[:word:]_]+\\s-+to" "set,[[:word:]_]+,to") ("a  reference   to" "a  reference   to" "a\\s-+\\s-+reference\\s-+\\s-+\\s-+to") ("already\\s-+encoded" "already\\s-+encoded" "already\\s-+encoded"))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn package_property_list_supports_real_updates_reads_and_missing_keys() {
    let elisp_form = r##"(let ((apples-plist nil))
                (list
                 apples-plist
                 (apples-plist-put :version "2.1")
                 (apples-plist-put :run-info '(:buffer t :actual-beg 17))
                 (apples-plist-get :version)
                 (apples-plist-get :run-info)
                 (apples-plist-get :missing)
                 apples-plist))"##;
    let expect = expect![[
        r#"OK (nil "2.1" #1=(:buffer t :actual-beg 17) "2.1" #1# nil (:version "2.1" :run-info #1#))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn temporary_file_setup_creates_stable_scratch_and_send_files_without_random_names() {
    let elisp_form = r##"(let* ((apples-tmp-dir
                          (expand-file-name
                           "apples-mode-tmp"
                           temporary-file-directory))
                         (apples-tmp-scratch
                          (expand-file-name "scratch.scpt" apples-tmp-dir))
                         (apples-tmp-send
                          (expand-file-name "send.scpt" apples-tmp-dir)))
                    (apples-tmp-files-setup)
                    (with-temp-file apples-tmp-scratch
                      (insert "persisted scratch\n"))
                    (apples-tmp-files-setup)
                    (list
                     (file-directory-p apples-tmp-dir)
                     (file-exists-p apples-tmp-scratch)
                     (file-exists-p apples-tmp-send)
                     (with-temp-buffer
                       (insert-file-contents apples-tmp-scratch)
                       (buffer-string))
                     (with-temp-buffer
                       (insert-file-contents apples-tmp-send)
                       (buffer-string))))"##;
    let expect = expect![[r#"OK (t t t "persisted scratch\n" "")"#]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn buffer_string_consumes_process_output_and_intentionally_drops_its_final_newline() {
    let elisp_form = r##"(let ((buffer (get-buffer-create " *apples-output-contract*")))
                (unwind-protect
                    (with-current-buffer buffer
                      (erase-buffer)
                      (insert "first\nsecond\n")
                      (let ((first (apples-buffer-string buffer))
                            (after-first (buffer-string)))
                        (insert "unterminated")
                        (list first
                              after-first
                              (apples-buffer-string buffer)
                              (buffer-string)
                              (apples-buffer-string buffer))))
                  (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK ("first\nsecond" "" "unterminate" "" "")"#]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn quoting_and_encoding_preserve_a_real_multiline_applescript_payload() {
    let elisp_form = r##"(let ((script
                         "set p to \"Macintosh HD:Users:me\"\nset q to \"c:\\\\tmp\"\nreturn p & q"))
                (list
                 (apples-quoted-string script)
                 (let ((apples-prefer-coding-system nil))
                   (equal script (apples-encode-string script)))
                 (let ((apples-prefer-coding-system 'utf-8))
                   (decode-coding-string
                    (apples-encode-string (concat script "\n日本語"))
                    'utf-8))))"##;
    let expect = expect![[
        r#"OK ("set p to \\\"Macintosh HD:Users:me\\\"\nset q to \\\"c:\\\\\\\\tmp\\\"\nreturn p & q" t "set p to \"Macintosh HD:Users:me\"\nset q to \"c:\\\\tmp\"\nreturn p & q\n日本語")"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn key_code_table_round_trips_letters_digits_navigation_and_function_keys() {
    let elisp_form = r##"(let ((keys '(?a ?z ?0 ?9 f1 f12 escape tab ?\s return
                               backspace left right down up)))
                (list
                 (mapcar
                  (lambda (key)
                    (cons key (cdr (assq key apples-key-codes))))
                  keys)
                 (mapcar
                  (lambda (key)
                    (let ((code (cdr (assq key apples-key-codes))))
                      (cons code (car (rassq code apples-key-codes)))))
                  keys)
                 (length apples-key-codes)
                 (= (length apples-key-codes)
                    (length (delete-dups
                             (mapcar #'cdr apples-key-codes))))))"##;
    let expect = expect![
        "OK (((97 . 0) (122 . 6) (48 . 29) (57 . 25) (f1 . 122) (f12 . 111) (escape . 53) (tab . 48) (32 . 49) (return . 36) (backspace . 51) (left . 123) (right . 124) (down . 125) (up . 126)) ((0 . 97) (6 . 122) (29 . 48) (25 . 57) (122 . f1) (111 . f12) (53 . escape) (48 . tab) (49 . 32) (36 . return) (51 . backspace) (123 . left) (124 . right) (125 . down) (126 . up)) 57 t)"
    ];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn continuation_insertion_normalizes_trailing_space_and_preserves_following_text() {
    let elisp_form = r##"(with-temp-buffer
                (insert "set total to first    \nnext")
                (goto-char (point-min))
                (end-of-line)
                (apples-insert-continuation-char)
                (let ((first (buffer-string)))
                  (goto-char (point-max))
                  (insert "   ")
                  (apples-insert-continuation-char)
                  (list first (buffer-string)
                        (char-to-string apples-continuation-char))))"##;
    let expect = expect![[
        r#"OK ("set total to first    ¬\nnext" "set total to first    ¬\nnext   ¬" "¬")"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}
