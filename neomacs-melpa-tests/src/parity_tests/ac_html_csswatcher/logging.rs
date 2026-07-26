use expect_test::expect;

use super::assert_ac_html_csswatcher_parity;

#[test]
fn ac_html_csswatcher_log_buffer_is_created_lazily_and_reused_by_name() {
    let elisp_form = r##"(let ((ac-html-csswatcher-log-buf-name
                    "*ac-html-csswatcher parity log*"))
               (unwind-protect
                   (let ((first
                          (ac-html-csswatcher-log-buf))
                         (second
                          (ac-html-csswatcher-log-buf)))
                     (list
                      (buffer-name first)
                      (eq first second)
                      (buffer-live-p first)))
                 (when
                     (get-buffer
                      ac-html-csswatcher-log-buf-name)
                   (kill-buffer
                    ac-html-csswatcher-log-buf-name))))"##;
    let expect = expect![[r#"OK ("*ac-html-csswatcher parity log*" t t)"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_logging_disabled_has_no_buffer_or_format_side_effects() {
    let elisp_form = r##"(let ((ac-html-csswatcher-debug nil)
                   (ac-html-csswatcher-log-buf-name
                    "*ac-html-csswatcher disabled log*"))
               (unwind-protect
                   (list
                    (AC-HTML-CSSWATCHER-LOG
                     "hidden %s" "message")
                    (get-buffer
                     ac-html-csswatcher-log-buf-name))
                 (when
                     (get-buffer
                      ac-html-csswatcher-log-buf-name)
                   (kill-buffer
                    ac-html-csswatcher-log-buf-name))))"##;
    let expect = expect![[r#"OK (nil nil)"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_logging_formats_unicode_and_appends_every_message() {
    let elisp_form = r##"(let ((ac-html-csswatcher-debug t)
                   (ac-html-csswatcher-log-buf-name
                    "*ac-html-csswatcher formatted log*"))
               (unwind-protect
                   (let ((first
                          (AC-HTML-CSSWATCHER-LOG
                           "first %s %d"
                           "λ雪" 7))
                         (second
                          (AC-HTML-CSSWATCHER-LOG
                           "second")))
                     (list
                      first
                      second
                      (with-current-buffer
                          ac-html-csswatcher-log-buf-name
                        (buffer-string))))
                 (when
                     (get-buffer
                      ac-html-csswatcher-log-buf-name)
                   (kill-buffer
                    ac-html-csswatcher-log-buf-name))))"##;
    let expect = expect![[r#"OK ("first λ雪 7" "second" "first λ雪 7\nsecond\n")"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_logging_pretty_prints_non_format_values_verbatim() {
    let elisp_form = r##"(let ((ac-html-csswatcher-debug t)
                   (ac-html-csswatcher-log-buf-name
                    "*ac-html-csswatcher pretty log*"))
               (unwind-protect
                   (let ((result
                          (AC-HTML-CSSWATCHER-LOG
                           '(:alpha 1
                             :beta (2 3)))))
                     (list
                      result
                      (with-current-buffer
                          ac-html-csswatcher-log-buf-name
                        (buffer-string))))
                 (when
                     (get-buffer
                      ac-html-csswatcher-log-buf-name)
                   (kill-buffer
                    ac-html-csswatcher-log-buf-name))))"##;
    let expect = expect![[r#"OK ("(:alpha 1 :beta (2 3))\n" "(:alpha 1 :beta (2 3))\n\n")"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_logging_swallows_log_buffer_failures() {
    let elisp_form = r##"(let ((ac-html-csswatcher-debug t)
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-html-csswatcher-log-buf)
                     (lambda ()
                       (setq calls
                             (1+ (or calls 0)))
                       (error
                        "fixture log failure"))))
                 (list
                  (AC-HTML-CSSWATCHER-LOG
                   "message %d" 9)
                  calls)))"##;
    let expect = expect![[r#"OK (nil 1)"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}
