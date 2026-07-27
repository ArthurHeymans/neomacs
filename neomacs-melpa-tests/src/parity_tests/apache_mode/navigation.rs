use expect_test::expect;

use super::assert_apache_mode_parity;

#[test]
fn apache_mode_forward_sexp_traverses_nested_apache_sections_as_balanced_forms() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<VirtualHost *:443>\n"
          "  <Directory \"/srv/www\">\n"
          "    Require all granted\n"
          "  </Directory>\n"
          "</VirtualHost>\n")
         (goto-char
          (point-min))
         (let (steps)
           (dotimes (_ 3)
             (forward-sexp 1)
             (setq steps
                   (append
                    steps
                    (list
                     (list
                      (point)
                      (line-number-at-pos)
                      (current-column)
                      (buffer-substring-no-properties
                       (line-beginning-position)
                       (line-end-position)))))))
           steps))"##;
    let expect = expect![[
        r#"OK ((20 1 19 "<VirtualHost *:443>") (45 2 24 "  <Directory \"/srv/www\">") (57 3 11 "    Require all granted"))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_scan_sexps_and_backward_sexp_agree_on_nested_section_boundaries() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<IfModule mod_ssl.c>\n"
          "  <VirtualHost *:443>\n"
          "    SSLEngine on\n"
          "  </VirtualHost>\n"
          "</IfModule>\n")
         (let* ((start
                 (point-min))
                (outer-end
                 (scan-sexps start 1))
                (inner-start
                 (progn
                   (goto-char start)
                   (search-forward
                    "<VirtualHost")
                   (- (point)
                      (length
                       "<VirtualHost"))))
                (inner-end
                 (scan-sexps
                  inner-start
                  1)))
           (goto-char outer-end)
           (backward-sexp 1)
           (list
            start
            outer-end
            inner-start
            inner-end
            (point)
            (buffer-substring-no-properties
             inner-start
             inner-end)
            (buffer-substring-no-properties
             start
             outer-end))))"##;
    let expect = expect![[r#"OK (1 21 24 43 1 "<VirtualHost *:443>" "<IfModule mod_ssl.c>")"#]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_symbol_navigation_keeps_hyphenated_and_underscored_tokens_together() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "Header set X-Request_ID deployment-blue\n")
         (goto-char
          (point-min))
         (let (forward)
           (while
               (re-search-forward
                "\\_<\\(?:\\sw\\|\\s_\\)+\\_>"
                nil
                t)
             (setq forward
                   (append
                    forward
                    (list
                     (list
                      (match-string-no-properties 0)
                      (match-beginning 0)
                      (match-end 0))))))
           (goto-char
            (point-max))
           (backward-word 1)
           (let ((last-start
                  (point)))
             (forward-word 1)
             (list
              forward
              last-start
              (point)
              (buffer-substring-no-properties
               last-start
               (point))))))"##;
    let expect = expect![[
        r#"OK ((("Header" 1 7) ("set" 8 11) ("X-Request_ID" 12 24) ("deployment-blue" 25 40)) 36 40 "blue")"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_forward_comment_skips_full_line_and_trailing_comments_but_not_hashes_in_strings() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "# first comment\n"
          "# second comment\n"
          "Header set X-Note \"blue#green\"\n"
          "ServerName example.test # trailing\n"
          "DocumentRoot /srv/www\n")
         (goto-char
          (point-min))
         (let ((first
                (progn
                  (forward-comment
                   (buffer-size))
                  (list
                   (line-number-at-pos)
                   (current-column)))))
           (search-forward
            "\"blue#green\"")
           (let ((string-end
                  (list
                   (line-number-at-pos)
                   (current-column))))
             (search-forward
              "# trailing")
             (backward-char
              (length
               "# trailing"))
             (forward-comment 1)
             (list
              first
              string-end
              (line-number-at-pos)
              (current-column)
              (buffer-substring-no-properties
               (line-beginning-position)
               (line-end-position))))))"##;
    let expect = expect![[r#"OK ((3 0) (3 30) 5 0 "DocumentRoot /srv/www")"#]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_unbalanced_section_navigation_returns_the_exact_scan_error_shape() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<VirtualHost *:443>\n"
          "  <Directory \"/srv/www\">\n"
          "    Require all granted\n"
          "</VirtualHost>\n")
         (goto-char
          (point-min))
         (condition-case error
             (list
              :ok
              (scan-sexps
               (point)
               1))
           (scan-error
            (list
             :scan-error
             (car error)
             (cadr error)
             (nth 2 error)
             (nth 3 error)))))"##;
    let expect = expect!["OK (:ok 20)"];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_up_list_and_down_list_navigate_inside_real_section_tags() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<VirtualHost *:443>\n"
          "  <Directory \"/srv/www\">\n"
          "    <FilesMatch \"\\.php$\">\n"
          "      Require all granted\n"
          "    </FilesMatch>\n"
          "  </Directory>\n"
          "</VirtualHost>\n")
         (goto-char
          (point-min))
         (down-list 1)
         (let ((inside-opener
                (list
                 (line-number-at-pos)
                 (current-column)
                 (char-after)
                 (buffer-substring-no-properties
                  (point)
                  (line-end-position)))))
           (up-list 1)
           (let ((after-opener
                  (list
                   (line-number-at-pos)
                   (current-column)
                   (char-before))))
             (search-forward
              "<FilesMatch")
             (backward-char
              (length
               "<FilesMatch"))
             (down-list 1)
             (list
              inside-opener
              after-opener
              (line-number-at-pos)
              (current-column)
              (buffer-substring-no-properties
               (point)
               (line-end-position))))))"##;
    let expect =
        expect![[r#"OK ((1 1 86 "VirtualHost *:443>") (1 19 62) 3 5 "FilesMatch \"\\.php$\">")"#]];

    assert_apache_mode_parity(elisp_form, expect);
}
