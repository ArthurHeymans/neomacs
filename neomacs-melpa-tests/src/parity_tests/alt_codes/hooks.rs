use expect_test::expect;

use super::assert_alt_codes_parity;

#[test]
fn alt_codes_pre_command_accumulates_keypad_digits_then_inserts_on_terminator() {
    let elisp_form = r##"(with-temp-buffer
         (let (messages states)
           (dolist (event
                    '(M-kp-1 M-kp-2 M-kp-8 return))
             (setq last-input-event event)
             (cl-letf
                 (((symbol-function 'message)
                   (lambda (format-string &rest arguments)
                     (push
                      (apply #'format
                             format-string arguments)
                      messages))))
               (alt-codes--pre-command-hook))
             (push
              (list event alt-codes--code
                    (buffer-string))
              states))
           (list
            (nreverse states)
            (nreverse messages)
            (string-to-list
             (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (((M-kp-1 "1" "") (M-kp-2 "12" "") (M-kp-8 "128" "") (return "" "Ç")) ("[Alt Code]: 1" "[Alt Code]: 12" "[Alt Code]: 128") (199))"#
    ]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_pre_command_supports_leading_zero_sequences() {
    let elisp_form = r##"(with-temp-buffer
         (dolist (event
                  '(M-kp-0 M-kp-1 M-kp-5 M-kp-3 left))
           (setq last-input-event event)
           (cl-letf
               (((symbol-function 'message)
                 (lambda (&rest _) nil)))
             (alt-codes--pre-command-hook)))
         (list
          (buffer-string)
          (string-to-list (buffer-string))
          alt-codes--code))"##;
    let expect = expect![[r#"OK ("™" (8482) "")"#]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_pre_command_discards_invalid_completed_sequence_and_resets_state() {
    let elisp_form = r##"(with-temp-buffer
         (setq-local alt-codes--code "999")
         (setq last-input-event 'space)
         (alt-codes--pre-command-hook)
         (list
          (buffer-string)
          alt-codes--code
          (local-variable-p 'alt-codes--code)))"##;
    let expect = expect![[r#"OK ("" "" t)"#]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_pre_command_ignores_non_symbol_events_and_read_only_buffers() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (setq-local alt-codes--code "65")
           (setq last-input-event 49)
           (alt-codes--pre-command-hook)
           (list (buffer-string)
                 alt-codes--code))
         (with-temp-buffer
           (setq-local alt-codes--code "65"
                       buffer-read-only t)
           (setq last-input-event 'return)
           (alt-codes--pre-command-hook)
           (list (buffer-string)
                 alt-codes--code)))"##;
    let expect = expect![[r#"OK (("" "65") ("" "65"))"#]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_pre_command_treats_non_keypad_meta_symbols_as_terminators() {
    let elisp_form = r##"(with-temp-buffer
         (setq-local alt-codes--code "65")
         (setq last-input-event 'M-a)
         (alt-codes--pre-command-hook)
         (list
          (buffer-string)
          alt-codes--code
          last-input-event))"##;
    let expect = expect![[r#"OK ("A" "" M-a)"#]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_pre_command_empty_string_mapping_performs_no_visible_insertion() {
    let elisp_form = r##"(with-temp-buffer
         (insert "before")
         (setq-local alt-codes--code "189")
         (setq last-input-event 'return)
         (alt-codes--pre-command-hook)
         (list
          (buffer-string)
          (point)
          alt-codes--code
          (buffer-modified-p)))"##;
    let expect = expect![[r#"OK ("before" 7 "" t)"#]];
    assert_alt_codes_parity(elisp_form, expect);
}
