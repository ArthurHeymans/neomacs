use expect_test::expect;

use super::assert_age_parity;

#[test]
fn age_data_struct_constructors_preserve_file_and_string_variants() {
    let elisp_form = r##"(let ((file-data
                (age-make-data-from-file "/vault/input.age"))
               (string-data
                (age-make-data-from-string
                 (concat "binary" (string 0 255)))))
         (list
          (age-data-file file-data)
          (age-data-string file-data)
          (age-data-file string-data)
          (age-data-string string-data)
          (copy-tree file-data)
          (copy-tree string-data)))"##;
    let expect = expect![[
        r#"OK ("/vault/input.age" nil nil "binary\0ÿ" #s(age-data "/vault/input.age" nil) #s(age-data nil "binary\0ÿ"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_context_constructor_initializes_every_field_from_configuration_and_options() {
    let elisp_form = r##"(let ((age-pinentry-mode 'ask))
         (cl-letf (((symbol-function 'age-find-configuration)
                    (lambda (protocol)
                      `((program . ,(format "%s-client" protocol))
                        (version . "1.2.3")))))
           (let ((context (age-make-context 'Age t)))
             (list
              (age-context-protocol context)
              (age-context-program context)
              (age-context-armor context)
              (age-context-passphrase context)
              (mapcar #'functionp
                      (age-context-passphrase-callback context))
              (age-context-edit-callback context)
              (age-context-process context)
              (age-context-output-file context)
              (age-context-result context)
              (age-context-operation context)
              (age-context-pinentry-mode context)
              (age-context-error-output context)
              (age-context-error-buffer context)))))"##;
    let expect = expect![[r#"OK (Age "Age-client" t nil (t) nil nil nil nil nil ask "" nil)"#]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_context_constructor_signals_age_error_without_usable_configuration() {
    let elisp_form = r##"(cl-letf (((symbol-function 'age-find-configuration)
                    (lambda (_protocol) nil)))
         (condition-case error-data
             (age-make-context)
           (error
            (list
             (car error-data)
             (cdr error-data)
             (get (car error-data) 'error-conditions)
             (get (car error-data) 'error-message)))))"##;
    let expect = expect![[
        r#"OK (age-error ("no usable configuration" Age) (age-error error) "Age error")"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_context_callback_and_named_results_support_replacement_and_insertion() {
    let elisp_form = r##"(let ((context
                (cl-letf (((symbol-function
                            'age-find-configuration)
                           (lambda (_protocol)
                             '((program . "age")))))
                  (age-make-context)))
               (callback
                (lambda (&rest arguments) arguments)))
         (age-context-set-passphrase-callback context callback)
         (age-context-set-result-for context 'error 'first)
         (age-context-set-result-for context 'recipient 'alice)
         (age-context-set-result-for context 'error 'replaced)
         (let ((function-form
                (age-context-passphrase-callback context)))
           (age-context-set-passphrase-callback
            context
            (cons callback '(handback data)))
           (list
            (length function-form)
            (functionp (car function-form))
            (cdr (age-context-passphrase-callback context))
            (age-context-result-for context 'error)
            (age-context-result-for context 'recipient)
            (age-context-result-for context 'missing)
            (age-context-result context))))"##;
    let expect = expect![
        "OK (1 t (handback data) replaced alice nil ((recipient . alice) (error . replaced)))"
    ];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_error_formatters_join_known_errors_and_report_unknown_shapes() {
    let elisp_form = r##"(let (messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (let ((rendered
                             (apply #'format
                                    format-string
                                    arguments)))
                        (push rendered messages)
                        rendered))))
           (list
            (age-error-to-string
             '(age-error "bad recipient"))
            (age-errors-to-string
             '((age-error "bad recipient")
               (age-error "bad identity")
               (age-error "bad armor")))
            (age-error-to-string
             '(file-error "missing"))
            (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ("bad recipient" "bad recipient; bad identity; bad armor" "XXX: Translate this error: (file-error missing)" ("XXX: Translate this error: (file-error missing)"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_with_dev_shm_scopes_temporary_directory_without_leaking_binding() {
    let elisp_form = r##"(let ((temporary-file-directory
                (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
         (list
          temporary-file-directory
          (age-with-dev-shm
           (list temporary-file-directory
                 (file-directory-p "/dev/shm/")))
          temporary-file-directory))"##;
    let expect = expect![[r#"OK ("[ORACLE-SANDBOX]" ("/dev/shm/" t) "[ORACLE-SANDBOX]")"#]];
    assert_age_parity(elisp_form, expect);
}
