use super::assert_assess_parity;
use expect_test::{Expect, expect};

#[test]
fn preserved_buffer_list_removes_indirectly_created_buffers_after_values_and_signals() {
    let elisp_form = r##"
(let ((before
       (mapcar #'buffer-name (buffer-list)))
      normal-result
      signaled-result)
  (setq normal-result
        (assess-with-preserved-buffer-list
          (generate-new-buffer " *assess-direct*")
          (with-current-buffer
              (generate-new-buffer " *assess-indirect*")
            (list
             (buffer-name)
             (length (buffer-list))))))
  (setq signaled-result
        (condition-case condition
            (assess-with-preserved-buffer-list
              (generate-new-buffer " *assess-before-signal*")
              (signal 'assess-deliberate-error '(fixture)))
          (assess-deliberate-error
           (list
            (car condition)
            (cdr condition)
            (mapcar #'buffer-name (buffer-list))))))
  (list
   normal-result
   (equal before (mapcar #'buffer-name (buffer-list)))
   (car signaled-result)
   (cadr signaled-result)
   (equal before (caddr signaled-result))))
"##;
    let expect: Expect =
        expect![[r#"OK ((" *assess-indirect*" 5) t assess-deliberate-error (fixture) t)"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn temp_buffer_binding_expansion_and_runtime_initializers_preserve_order_and_cleanup() {
    let elisp_form = r##"
(let ((before (length (buffer-list)))
      expansion
      observed
      escaped)
  (setq expansion
        (assess--temp-buffer-let-form
         '(alpha
           (insert "A")
           (insert "B"))))
  (setq observed
        (assess-with-temp-buffers
            ((alpha
              (insert "A")
              (insert "B"))
             beta
             (gamma
              (insert
               (with-current-buffer alpha
                 (buffer-string)))
              (insert "C")))
          (list
           (mapcar #'buffer-live-p
                   (list alpha beta gamma))
           (mapcar
            (lambda (buffer)
              (with-current-buffer buffer
                (buffer-string)))
            (list alpha beta gamma))
           (length (buffer-list))
           (setq escaped (list alpha beta gamma)))))
  (list
   expansion
   observed
   (mapcar #'buffer-live-p escaped)
   (= before (length (buffer-list)))))
"##;
    let expect: Expect = expect![[
        r#"OK ((alpha (with-current-buffer (generate-new-buffer " *assess-with-temp-buffers*") (insert "A") (insert "B") (current-buffer))) ((t t t) ("AB" "" "ABC") 6 ((:buffer nil) (:buffer nil) (:buffer nil))) (nil nil nil) t)"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn temp_buffer_macro_cleans_user_created_buffers_not_only_bound_buffers() {
    let elisp_form = r##"
(let ((before (mapcar #'buffer-name (buffer-list)))
      bound
      extra)
  (condition-case condition
      (assess-with-temp-buffers
          ((alpha (insert "payload")))
        (setq bound alpha
              extra
              (generate-new-buffer
               " *assess-user-created*"))
        (signal 'assess-deliberate-error '(cleanup)))
    (assess-deliberate-error
     (list
      (car condition)
      (cdr condition)
      (buffer-live-p bound)
      (buffer-live-p extra)
      (equal before
             (mapcar #'buffer-name
                     (buffer-list)))))))
"##;
    let expect: Expect = expect!["OK (assess-deliberate-error (cleanup) nil nil t)"];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn temporary_buffer_conversion_handles_strings_and_buffers_without_properties() {
    let elisp_form = r##"
(let ((source (generate-new-buffer " *assess-source*")))
  (unwind-protect
      (progn
        (with-current-buffer source
          (insert
           (propertize
            "alpha\nbeta"
            'fixture-property
            17)))
        (list
         (assess-as-temp-buffer
             source
           (list
            (buffer-string)
            (text-properties-at (point-min))
            (current-buffer)))
         (assess-as-temp-buffer
             "literal"
           (list
            (buffer-string)
            (= (point) (point-max))))
         (assess-ensure-string source)))
    (kill-buffer source)))
"##;
    let expect: Expect = expect![[
        r#"OK ((#("alpha\nbeta" 0 10 (fixture-property 17)) (fixture-property 17) (:buffer nil)) ("literal" t) #("alpha\nbeta" 0 10 (fixture-property 17)))"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn ensure_string_dispatches_strings_and_buffers_and_reports_unsupported_types() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "buffer payload")
  (list
   (assess-ensure-string "literal")
   (assess-ensure-string (current-buffer))
   (condition-case condition
       (assess-ensure-string
        '(:unsupported t))
     (error
      (list
       (car condition)
       (cadr condition))))))
"##;
    let expect: Expect =
        expect![[r#"OK ("literal" "buffer payload" (error "Type not recognised"))"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn buffer_alias_creates_and_reuses_named_buffers_with_get_buffer_create_semantics() {
    let elisp_form = r##"
(let* ((name " *assess-buffer-alias*")
       (first (assess-buffer name))
       (second (assess-buffer name)))
  (unwind-protect
      (progn
        (with-current-buffer first
          (insert "state"))
        (list
         (eq first second)
         (buffer-name second)
         (with-current-buffer second
           (buffer-string))
         (eq
          (symbol-function 'assess-buffer)
          (symbol-function 'get-buffer-create))))
    (kill-buffer first)))
"##;
    let expect: Expect = expect![[r#"OK (t " *assess-buffer-alias*" "state" nil)"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn file_converter_round_trips_binary_safe_workspace_content() {
    let elisp_form = r##"
(let ((path
       (assess-test-path
        "converter/nested/input.txt")))
  (make-directory (file-name-directory path) t)
  (with-temp-file path
    (insert "first\nλ second\n"))
  (let ((converted (assess-file path)))
    (list
     converted
     (string= converted
              (assess-test-read-file path))
     (multibyte-string-p converted)
     (file-exists-p path))))
"##;
    let expect: Expect = expect![[r#"OK ("first\nλ second\n" nil t t)"#]];
    assert_assess_parity(elisp_form, expect);
}
