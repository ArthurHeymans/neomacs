use expect_test::expect;

use super::assert_adafruit_wisdom_parity;

#[test]
fn adafruit_wisdom_without_prefix_messages_quote_preserves_buffer_and_returns_true() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "before")
         (set-buffer-modified-p
          nil)
         (let (messages)
           (cl-letf
               (((symbol-function
                  'adafruit-wisdom-select)
                 (lambda ()
                   "a quote"))
                ((symbol-function
                  'message)
                 (lambda (format-string &rest arguments)
                   (push
                    (cons
                     format-string
                     arguments)
                    messages)
                   "message-result")))
             (list
              (adafruit-wisdom)
              (buffer-string)
              (nreverse
               messages)
              (point)
              (buffer-modified-p)))))"##;
    let expect = expect![[r#"OK (t "before" (("a quote")) 7 nil)"#]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_every_non_nil_prefix_shape_inserts_quote_and_returns_true() {
    let elisp_form = r##"(mapcar
         (lambda (prefix)
           (with-temp-buffer
             (insert
              "before:")
             (let (messages)
               (cl-letf
                   (((symbol-function
                      'adafruit-wisdom-select)
                     (lambda ()
                       "quote"))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        messages))))
                 (list
                  prefix
                  (adafruit-wisdom
                   prefix)
                  (buffer-string)
                  (point)
                  messages)))))
         '(t
           0
           4
           (4)
           -1))"##;
    let expect = expect![[
        r#"OK ((t t "before:quote" 13 nil) (0 t "before:quote" 13 nil) (4 t "before:quote" 13 nil) ((4) t "before:quote" 13 nil) (-1 t "before:quote" 13 nil))"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_nil_quote_signals_exact_error_without_message_or_insertion() {
    let elisp_form = r##"(mapcar
         (lambda (prefix)
           (with-temp-buffer
             (insert
              "unchanged")
             (let (messages)
               (cl-letf
                   (((symbol-function
                      'adafruit-wisdom-select)
                     (lambda ()
                       nil))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        messages))))
                 (condition-case error-data
                     (list
                      'ok
                      (adafruit-wisdom
                       prefix))
                   (error
                    (list
                     prefix
                     (car
                      error-data)
                     (error-message-string
                      error-data)
                     (buffer-string)
                     messages
                     (point))))))))
         '(nil t))"##;
    let expect = expect![[
        r#"OK ((nil error "Couldn’t retrieve a quote from adafruit" "unchanged" nil 10) (t error "Couldn’t retrieve a quote from adafruit" "unchanged" nil 10))"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_empty_string_quote_is_valid_for_message_and_insert_paths() {
    let elisp_form = r##"(let (messages)
         (cl-letf
             (((symbol-function
                'adafruit-wisdom-select)
               (lambda ()
                 ""))
              ((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (push
                  (cons
                   format-string
                   arguments)
                  messages))))
           (list
            (with-temp-buffer
              (list
               (adafruit-wisdom)
               (buffer-string)))
            (with-temp-buffer
              (list
               (adafruit-wisdom
                t)
               (buffer-string)))
            (nreverse
             messages))))"##;
    let expect = expect![[r#"OK ((t "") (t "") (("")))"#]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_call_interactively_routes_raw_prefix_to_message_or_insert() {
    let elisp_form = r##"(mapcar
         (lambda (prefix)
           (with-temp-buffer
             (let ((current-prefix-arg
                    prefix)
                   messages)
               (cl-letf
                   (((symbol-function
                      'adafruit-wisdom-select)
                     (lambda ()
                       "interactive"))
                    ((symbol-function
                      'message)
                     (lambda (format-string &rest arguments)
                       (push
                        (cons
                         format-string
                         arguments)
                        messages))))
                 (list
                  prefix
                  (call-interactively
                   #'adafruit-wisdom)
                  (buffer-string)
                  (nreverse
                   messages))))))
         '(nil
           (4)
           -
           0))"##;
    let expect = expect![[
        r#"OK ((nil t "" (("interactive"))) ((4) t "interactive" nil) (- t "interactive" nil) (0 t "interactive" nil))"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}
