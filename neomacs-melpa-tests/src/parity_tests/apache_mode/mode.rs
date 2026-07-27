use expect_test::expect;

use super::assert_apache_mode_parity;

#[test]
fn apache_mode_activation_sets_the_complete_major_mode_local_contract() {
    let elisp_form = r##"(with-temp-buffer
         (setq-local
          indent-tabs-mode
          t)
         (apache-mode)
         (list
          major-mode
          mode-name
          (derived-mode-p
           'fundamental-mode)
          comment-start
          comment-start-skip
          comment-column
          (eq
           indent-line-function
           #'apache-indent-line)
          indent-tabs-mode
          font-lock-defaults
          (eq
           (syntax-table)
           apache-mode-syntax-table)
          (current-local-map)))"##;
    let expect = expect![[
        r##"OK (apache-mode "Apache" nil "# " "#\\W*" 48 t t (apache-font-lock-keywords nil t ((95 . "w") (45 . "w")) beginning-of-line) t (keymap))"##
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_syntax_table_classifies_symbols_delimiters_strings_and_comments() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (mapcar
          (lambda (character)
            (let ((start (point)))
              (insert character)
              (list
               character
               (char-syntax character)
               (string
                (char-syntax character))
               (syntax-after start))))
          '(?_ ?- ?\( ?\) ?< ?> ?\" ?, ?# ?\n ?/ ?*)))"##;
    let expect = expect![[
        r#"OK ((95 95 "_" #1=(3)) (45 95 "_" #1#) (40 40 "(" (4 . 41)) (41 41 ")" (5 . 40)) (60 40 "(" (4 . 62)) (62 41 ")" (5 . 60)) (34 34 "\"" (7)) (44 46 "." (1)) (35 60 "<" (11)) (10 62 ">" (12 . 35)) (47 95 "_" #1#) (42 95 "_" #1#))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_parse_state_distinguishes_real_comments_strings_and_nested_sections() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<VirtualHost *:443>\n"
          "  ServerName \"api#blue.example\"\n"
          "  # disabled: ProxyPass /old http://old\n"
          "  <Directory \"/srv/www\">\n"
          "    Require all granted\n"
          "  </Directory>\n"
          "</VirtualHost>\n")
         (mapcar
          (lambda (sample)
            (let* ((needle (car sample))
                   (offset (cdr sample))
                   (position
                    (apache-mode-test-point-at
                     needle
                     offset))
                   (state
                    (syntax-ppss position)))
              (list
               needle
               (nth 0 state)
               (and
                (nth 3 state)
                t)
               (and
                (nth 4 state)
                t)
               (nth 8 state))))
          '(("VirtualHost" . 0)
            ("api#blue" . 3)
            ("# disabled" . 2)
            ("Directory" . 0)
            ("Require" . 0)
            ("</Directory>" . 2)
            ("</VirtualHost>" . 2))))"##;
    let expect = expect![[
        r##"OK (("VirtualHost" 1 nil nil nil) ("api#blue" 0 t nil 34) ("# disabled" 0 nil t 55) ("Directory" 1 nil nil nil) ("Require" 0 nil nil nil) ("</Directory>" 1 nil nil nil) ("</VirtualHost>" 1 nil nil nil))"##
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_comment_region_and_uncomment_region_round_trip_a_practical_block() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "ProxyPass /api http://127.0.0.1:3000\n"
          "ProxyPassReverse /api http://127.0.0.1:3000\n")
         (comment-region
          (point-min)
          (point-max))
         (let ((commented
                (buffer-string))
               (comment-indents
                (apache-mode-test-line-indents)))
           (uncomment-region
            (point-min)
            (point-max))
           (list
            commented
            comment-indents
            (buffer-string))))"##;
    let expect = expect![[
        r##"OK ("# ProxyPass /api http://127.0.0.1:3000\n# ProxyPassReverse /api http://127.0.0.1:3000\n" (("# ProxyPass /api http://127.0.0.1:3000" 0) ("# ProxyPassReverse /api http://127.0.0.1:3000" 0)) "ProxyPass /api http://127.0.0.1:3000\nProxyPassReverse /api http://127.0.0.1:3000\n")"##
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_auto_mode_selection_accepts_real_apache_paths_and_rejects_near_misses() {
    let elisp_form = r##"(mapcar
         (lambda (path)
           (with-temp-buffer
             (setq buffer-file-name path)
             (set-auto-mode)
             (list
              path
              major-mode)))
         '("/srv/www/.htaccess"
           "/etc/httpd/conf/httpd.conf"
           "/etc/httpd/conf/extra/vhosts.conf"
           "/etc/apache2/apache2.conf"
           "/etc/apache2/mods-enabled/ssl.conf"
           "/etc/apache2/sites-available/api.example"
           "/etc/apache2/sites-enabled/000-default.conf"
           "/work/httpd.conf.backup"
           "/work/.htaccess.sample"
           "/work/apache.conf"))"##;
    let expect = expect![[
        r#"OK (("/srv/www/.htaccess" apache-mode) ("/etc/httpd/conf/httpd.conf" apache-mode) ("/etc/httpd/conf/extra/vhosts.conf" apache-mode) ("/etc/apache2/apache2.conf" apache-mode) ("/etc/apache2/mods-enabled/ssl.conf" apache-mode) ("/etc/apache2/sites-available/api.example" apache-mode) ("/etc/apache2/sites-enabled/000-default.conf" apache-mode) ("/work/httpd.conf.backup" conf-unix-mode) ("/work/.htaccess.sample" fundamental-mode) ("/work/apache.conf" conf-unix-mode))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_repeated_activation_is_stable_and_runs_mode_hooks_once_per_call() {
    let elisp_form = r##"(let ((apache-mode-hook
                (list
                 (lambda ()
                   (setq apache-mode-test-events
                         (append
                          apache-mode-test-events
                          (list
                           (list
                            major-mode
                            (current-indentation)))))))))
         (setq apache-mode-test-events nil)
         (with-temp-buffer
           (insert
            "    ServerName example.test\n")
           (apache-mode)
           (apache-mode)
           (list
            apache-mode-test-events
            major-mode
            (current-indentation)
            (eq
             (syntax-table)
             apache-mode-syntax-table)
            (eq
             indent-line-function
             #'apache-indent-line))))"##;
    let expect = expect!["OK (((apache-mode 0) (apache-mode 0)) apache-mode 0 t t)"];

    assert_apache_mode_parity(elisp_form, expect);
}
