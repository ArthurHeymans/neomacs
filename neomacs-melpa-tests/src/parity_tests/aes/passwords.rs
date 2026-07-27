use expect_test::expect;

use super::{assert_aes_parity, assert_aes_signal_parity};

#[test]
fn aes_password_to_key_handles_unibyte_multibyte_and_all_supported_key_widths() {
    let elisp_form = r##"(let ((hex
                    (lambda (string)
                      (mapconcat
                       (lambda (byte)
                         (format "%02x" byte))
                       string ""))))
               (mapcar
                (lambda (spec)
                  (let ((key
                         (aes-password-to-key
                          (car spec)
                          (cadr spec))))
                    (list
                     spec
                     (length key)
                     (multibyte-string-p key)
                     (funcall hex key))))
                '(("password" 4)
                  ("six-word-key" 6)
                  ("eight-word-key" 8)
                  ("påsswörd" 4))))"##;
    let expect = expect![[
        r#"OK ((("password" 4) 16 nil "dbd7bb45c7b0ea45eabca45360d459c4") (("six-word-key" 6) 24 nil "b8a89821a300d99587f93feb6b038e0517fc85413d32f6a5") (("eight-word-key" 8) 32 nil "62260ba786ca24e616d840d7f3e3f322b80c626e948eaf965ed7a2725a459b99") (("påsswörd" 4) 16 nil "7dd3cf49c6d8877e5772f5424874cb60"))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_password_to_key_rejects_an_empty_password_before_key_expansion() {
    let elisp_form = r##"(aes-password-to-key "" 4)"##;
    let expect = expect![[r#"ERR (args-out-of-range "" 0 16)"#]];

    assert_aes_signal_parity(elisp_form, expect);
}

#[test]
fn aes_clear_password_functions_reset_storage_timer_and_report_idle_cleanup() {
    let elisp_form = r##"(let ((aes--plaintext-passwords
                    '(("one" . "secret")
                      ("two" . "other")))
                   (aes-idle-timer-value
                    'timer-token))
               (let ((clear-result
                      (aes-clear-plaintext-keys))
                     after-clear)
                 (setq after-clear
                       aes--plaintext-passwords)
                 (setq aes--plaintext-passwords
                       '(("again" . "secret")))
                 (let ((idle-result
                        (aes-idle-clear-plaintext-keys)))
                   (list
                    clear-result
                    after-clear
                    idle-result
                    aes--plaintext-passwords
                    aes-idle-timer-value
                    (current-message)))))"##;
    let expect = expect![[r#"OK (nil nil "AES Passwords cleared." nil nil nil)"#]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_password_path_hooks_stop_at_first_success_and_receive_the_original_path() {
    let elisp_form = r##"(let (events)
               (let ((aes-path-passwd-hook
                      (list
                       (lambda (path)
                         (push
                          (list 'first path)
                          events)
                         nil)
                       (lambda (path)
                         (push
                          (list 'second path)
                          events)
                         "  project")
                       (lambda (path)
                         (push
                          (list 'third path)
                          events)
                         "never"))))
                 (let ((group
                        (aes-exec-passws-hooks
                         "/workspace/project/file.txt"))
                       (ran events))
                   (list
                    group
                    (reverse ran)
                    (let ((aes-path-passwd-hook nil))
                      (aes-exec-passws-hooks
                       "/ungrouped"))))))"##;
    let expect = expect![[
        r#"OK ("  project" ((first "/workspace/project/file.txt") (second "/workspace/project/file.txt")) nil)"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_key_from_password_reuses_stored_group_password_without_prompting() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (let ((aes-always-ask-for-passwords nil)
                    (aes-enable-plaintext-password-storage t)
                    (aes--plaintext-passwords
                     '(("  stored-group" . "stored secret")))
                    prompted)
                (cl-letf
                    (((symbol-function 'read-passwd)
                      (lambda (&rest args)
                        (setq prompted args)
                        "unexpected")))
                  (let ((key
                         (aes-key-from-passwd
                          "decryption"
                          "  stored-group"
                          4)))
                    (list
                     prompted
                     aes--plaintext-passwords
                     (length key)
                     (mapconcat
                      (lambda (byte)
                        (format "%02x" byte))
                      key ""))))))"##;
    let expect = expect![[
        r#"OK (nil (("  stored-group" . "stored secret")) 16 "0dba03f0b09a6a7c30cb1bdc74e1cb6c")"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_key_from_password_prompts_stores_groups_and_replaces_idle_timer() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (let ((aes-always-ask-for-passwords nil)
                    (aes-enable-plaintext-password-storage t)
                    (aes-delete-passwords-after-idle 30)
                    (aes--plaintext-passwords nil)
                    (aes-idle-timer-value 'old-timer)
                    (answers
                     '("" "prompt secret"))
                    events)
                (cl-letf
                    (((symbol-function 'read-passwd)
                      (lambda (prompt confirm)
                        (push
                         (list 'prompt prompt confirm)
                         events)
                        (prog1
                            (car answers)
                          (setq answers
                                (cdr answers)))))
                     ((symbol-function 'cancel-timer)
                      (lambda (timer)
                        (push
                         (list 'cancel timer)
                         events)))
                     ((symbol-function 'run-with-idle-timer)
                      (lambda (&rest args)
                        (push
                         (cons 'schedule args)
                         events)
                        'new-timer)))
                  (let ((key
                         (aes-key-from-passwd
                          "encryption"
                          "  new-group"
                          4)))
                    (list
                     (length key)
                     (reverse events)
                     aes--plaintext-passwords
                     aes-idle-timer-value)))))"##;
    let expect = expect![[
        r#"OK (16 ((prompt "encryption Password for   new-group: " t) (prompt "encryption Password for   new-group: " t) (cancel old-timer) (schedule 30 nil aes-idle-clear-plaintext-keys)) (("  new-group" . "prompt secret")) new-timer)"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_key_from_password_does_not_store_string_buffer_or_always_ask_cases() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (let ((aes-enable-plaintext-password-storage t)
                    (aes-delete-passwords-after-idle 0)
                    (aes--plaintext-passwords nil)
                    (answers
                     '("string secret"
                       "buffer secret"
                       "always secret"))
                    events)
                (cl-letf
                    (((symbol-function 'read-passwd)
                      (lambda (prompt confirm)
                        (push
                         (list prompt confirm)
                         events)
                        (prog1
                            (car answers)
                          (setq answers
                                (cdr answers))))))
                  (let ((buffer
                         (generate-new-buffer
                          "aes-password-buffer")))
                    (unwind-protect
                        (let ((aes-always-ask-for-passwords
                               nil))
                          (aes-key-from-passwd
                           "decryption" "string" 4)
                          (aes-key-from-passwd
                           "encryption"
                           (buffer-name buffer) 4))
                      (kill-buffer buffer)))
                  (let ((aes-always-ask-for-passwords
                         t))
                    (aes-key-from-passwd
                     "decryption"
                     "  always-group" 4))
                  (list
                   (reverse events)
                   aes--plaintext-passwords
                   aes-idle-timer-value))))"##;
    let expect = expect![[
        r#"OK ((("decryption Password for string: " nil) ("encryption Password for aes-password-buffer: " t) ("decryption Password for   always-group: " nil)) nil nil)"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_key_from_password_rejects_unknown_usage_before_prompting() {
    let elisp_form = r##"(aes-key-from-passwd
              "rotation" "string" 4)"##;
    let expect =
        expect![[r#"ERR (error "Wrong argument in aes-key-from-passwd: \"\"rotation\"\"")"#]];

    assert_aes_signal_parity(elisp_form, expect);
}

#[test]
fn aes_shuffle_and_noninteractive_entropy_use_random_indices_destructively() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (let ((cursor 0)
                    (string
                     (copy-sequence "abcdef"))
                    (vector
                     (copy-sequence [0 1 2 3 4])))
                (cl-letf
                    (((symbol-function 'random)
                      (lambda (limit)
                        (prog1
                            (% cursor limit)
                          (setq cursor
                                (1+ cursor))))))
                  (let ((string-result
                         (aes--fisher-yates-shuffle-array
                          string))
                        (vector-result
                         (aes--fisher-yates-shuffle-array
                          vector))
                        (aes-user-interaction-entropy
                         nil))
                    (list
                     (eq string string-result)
                     string-result
                     (eq vector vector-result)
                     vector-result
                     (aes-user-entropy 6)
                     (aes-user-entropy 6 7)
                     cursor)))))"##;
    let expect = expect![[r#"OK (t "edfcba" t [3 4 1 2 0] (14 13 12 11 10 9) (6 5 4 3 2 1) 21)"#]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_interactive_entropy_consumes_stubbed_key_events_and_crypto_extracts_values() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (let ((aes-user-interaction-entropy t)
                    (aes-entropy-of-keyinput 128)
                    (command-history nil)
                    (read-count 0)
                    prompts)
                (cl-letf
                    (((symbol-function 'random)
                      (lambda (&optional limit)
                        (if limit 0 0)))
                     ((symbol-function 'recent-keys)
                      (lambda () [97 98]))
                     ((symbol-function 'current-time)
                      (lambda () '(0 0 0 0)))
                     ((symbol-function 'window-body-height)
                      (lambda (&rest _) 1))
                     ((symbol-function 'window-width)
                      (lambda (&rest _) 2))
                     ((symbol-function 'switch-to-buffer)
                      (lambda (buffer &rest _)
                        buffer))
                     ((symbol-function 'selected-window)
                      (lambda () 'window-token))
                     ((symbol-function 'read-event)
                      (lambda (prompt &rest _)
                        (setq read-count
                              (1+ read-count))
                        (push prompt prompts)
                        65)))
                  (let ((values
                         (aes-user-entropy 3 10)))
                    (list
                     values
                     read-count
                     (length prompts)
                     (string-match-p
                      "Move mouse"
                      (car prompts)))))))"##;
    let expect = expect!["OK ((7 8 1) 1 1 0)"];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_interactive_entropy_ignores_unrelated_events_and_accepts_current_window_mouse_motion() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (let ((aes-user-interaction-entropy t)
                    (aes-entropy-of-mousemovement 128)
                    (command-history nil)
                    (events
                     '((mouse-movement
                        (other-window 1 (0 . 0)))
                       ignored-event
                       (mouse-movement
                        (window-token 1 (2 . 3)))))
                    (read-count 0)
                    prompts)
                (cl-letf
                    (((symbol-function 'random)
                      (lambda (&optional limit)
                        (if limit 0 0)))
                     ((symbol-function 'recent-keys)
                      (lambda () []))
                     ((symbol-function 'current-time)
                      (lambda () '(0 0 0 0)))
                     ((symbol-function 'window-body-height)
                      (lambda (&rest _) 1))
                     ((symbol-function 'window-width)
                      (lambda (&rest _) 2))
                     ((symbol-function 'switch-to-buffer)
                      (lambda (buffer &rest _)
                        buffer))
                     ((symbol-function 'selected-window)
                      (lambda () 'window-token))
                     ((symbol-function 'read-event)
                      (lambda (prompt &rest _)
                        (setq read-count
                              (1+ read-count))
                        (push prompt prompts)
                        (prog1
                            (car events)
                          (setq events
                                (cdr events))))))
                  (let ((values
                         (aes-user-entropy 1 1)))
                    (list
                     values
                     read-count
                     events
                     (length prompts))))))"##;
    let expect = expect!["OK ((0) 3 nil 3)"];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_password_generation_selects_default_and_explicit_groups_and_inserts_at_point() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (cl-letf
                  (((symbol-function 'aes-user-entropy)
                    (lambda (length border)
                      (let (result)
                        (dotimes (index length)
                          (push
                           (% index border)
                           result))
                        (nreverse result)))))
                (let ((default
                       (aes-generate-password 12))
                      (lower
                       (aes-generate-password
                        12 "a"))
                      (punctuation
                       (aes-generate-password
                        18 ".+")))
                  (with-temp-buffer
                    (insert "prefix:")
                    (let ((insert-result
                           (aes-insert-password 8)))
                      (list
                       default
                       lower
                       punctuation
                       insert-result
                       (buffer-string)
                       (mapcar
                        (lambda (group)
                          (list
                           (car group)
                           (cadr group)
                           (length
                            (nth 2 group))))
                        aes-password-char-groups)))))))"##;
    let expect = expect![[
        r#"OK ("abcdefghjkmn" "abcdefghjkmn" ",.!?;:_()[]{}<>-+*" nil "prefix:abcdefgh" ((97 t 24) (65 t 24) (53 t 8) (48 t 6) (46 nil 15) (43 nil 5) (37 nil 8)))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_password_generation_rejects_a_type_with_no_character_groups() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (cl-letf
                  (((symbol-function 'aes-user-entropy)
                    (lambda (length border)
                      (make-list length
                                 (if (= border 0)
                                     0
                                   (1- border))))))
                (aes-generate-password
                 1 "x")))"##;
    let expect = expect![[r#"ERR (args-out-of-range "" 0)"#]];

    assert_aes_signal_parity(elisp_form, expect);
}
