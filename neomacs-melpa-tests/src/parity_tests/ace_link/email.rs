use super::assert_ace_link_parity;
use expect_test::expect;

#[test]
fn ace_link_email_plain_collect_finds_http_and_https_starts_in_visible_order() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "prefix http://one.test middle https://two.test suffix")
         (goto-char 9)
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (list
            (ace-link--email-view-plain-collect)
            (point))))"##;
    let expect = expect!["OK ((8 31) 9)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_email_next_link_orders_shr_mu4e_url_and_attachment_positions() {
    let elisp_form = r##"(with-temp-buffer
         (insert "01234567890123456789")
         (add-text-properties
          12 15
          '(shr-url shr))
         (add-text-properties
          5 8
          '(mu4e-url mu4e))
         (add-text-properties
          9 11
          '(mu4e-attnum 3))
         (list
          (ace-link--email-view-next-link
           1 nil)
          (ace-link--email-view-next-link
           1 t)
          (ace-link--email-view-next-link
           8 t)
          (ace-link--email-view-next-link
           15 t)
          (ace-link--email-view-next-link
           (point-max)
           t)))"##;
    let expect = expect!["OK ((shr-url 12) (mu4e-url 5) (mu4e-attnum 9) nil nil)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_email_end_of_link_finds_property_boundary_or_point_max() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (add-text-properties
          3 7
          '(shr-url first))
         (add-text-properties
          8 11
          '(mu4e-url second))
         (list
          (ace-link--email-view-end-of-link
           '(shr-url 3))
          (ace-link--email-view-end-of-link
           '(mu4e-url 8))
          (ace-link--email-view-end-of-link
           '(missing 5))))"##;
    let expect = expect!["OK (7 11 5)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_email_html_collect_includes_shr_only_or_all_mu4e_link_types() {
    let elisp_form = r##"(with-temp-buffer
         (insert "aaSHR bbURL ccATT dd")
         (add-text-properties
          3 6
          '(shr-url shr))
         (add-text-properties
          9 12
          '(mu4e-url url))
         (add-text-properties
          15 18
          '(mu4e-attnum 7))
         (goto-char 20)
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (list
            (ace-link--email-view-html-collect)
            (ace-link--email-view-html-collect t)
            (point))))"##;
    let expect = expect![[r#"OK ((("SHR" . 3)) (("SHR" . 3) ("URL" . 9) ("ATT" . 15)) 20)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_email_html_collect_does_not_infer_mu4e_from_major_mode_alone() {
    let elisp_form = r##"(with-temp-buffer
         (insert "aaSHR bbURL")
         (add-text-properties
          3 6
          '(shr-url shr))
         (add-text-properties
          9 12
          '(mu4e-url url))
         (let ((major-mode
                'mu4e-view-mode))
           (cl-letf (((symbol-function 'window-start)
                      (lambda (&rest _)
                        (point-min)))
                     ((symbol-function 'window-end)
                      (lambda (&rest _)
                        (point-max))))
             (ace-link--email-view-html-collect))))"##;
    let expect = expect![[r#"OK (("SHR" . 3))"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_notmuch_collect_combines_plain_and_html_actions_without_reordering() {
    let elisp_form = r##"(cl-letf
         (((symbol-function 'ace-link--email-view-plain-collect)
           (lambda ()
             '(3 9)))
          ((symbol-function 'ace-link--email-view-html-collect)
           (lambda (&optional _mu4e)
             '(("first" . 5)
               ("second" . 12)))))
         (mapcar
          (lambda (candidate)
            (list
             (car candidate)
             (cdr candidate)))
          (ace-link--notmuch-collect)))"##;
    let expect = expect![
        "OK ((3 ace-link--notmuch-plain-action) (9 ace-link--notmuch-plain-action) (5 ace-link--notmuch-html-action) (12 ace-link--notmuch-html-action))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}
