use expect_test::expect;

use super::assert_alt_codes_parity;

#[test]
fn alt_codes_insert_trims_prompt_input_and_inserts_resolved_character() {
    let elisp_form = r##"(with-temp-buffer
         (let (prompts)
           (cl-letf
               (((symbol-function 'read-string)
                 (lambda (prompt &rest arguments)
                   (push (cons prompt arguments)
                         prompts)
                   " 0153 ")))
             (list
              (alt-codes-insert)
              (buffer-string)
              (string-to-list
               (buffer-string))
              (nreverse prompts)))))"##;
    let expect = expect![[r#"OK (nil "™" (8482) (("Insert Alt-Code: ")))"#]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_insert_signals_user_error_for_empty_and_unknown_inputs() {
    let elisp_form = r##"(mapcar
         (lambda (input)
           (condition-case error
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function 'read-string)
                       (lambda (&rest _) input)))
                   (alt-codes-insert)
                   (list 'value (buffer-string))))
             (error
              (list 'signal
                    (car error)
                    (cdr error)))))
         '("" "   " "0" "999" "not-a-code"))"##;
    let expect = expect![[
        r#"OK ((signal user-error ("Invalid Alt Code, please input the valid one")) (signal user-error ("Invalid Alt Code, please input the valid one")) (signal user-error ("Invalid Alt Code, please input the valid one")) (signal user-error ("Invalid Alt Code, please input the valid one")) (signal user-error ("Invalid Alt Code, please input the valid one")))"#
    ]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_insert_empty_mapped_value_succeeds_without_modifying_text() {
    let elisp_form = r##"(with-temp-buffer
         (insert "stable")
         (set-buffer-modified-p nil)
         (cl-letf
             (((symbol-function 'read-string)
               (lambda (&rest _) "189")))
           (list
            (alt-codes-insert)
            (buffer-string)
            (point)
            (buffer-modified-p))))"##;
    let expect = expect![[r#"OK (nil "stable" 7 nil)"#]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_insert_suppresses_insertion_errors_as_documented_by_implementation() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'read-string)
               (lambda (&rest _) "65"))
              ((symbol-function 'insert)
               (lambda (&rest arguments)
                 (push arguments calls)
                 (error "synthetic insertion failure"))))
           (list
            (alt-codes-insert)
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil (("A")))"#]];
    assert_alt_codes_parity(elisp_form, expect);
}
