use expect_test::expect;

use super::assert_apache_mode_parity;

#[test]
fn apache_mode_indent_region_formats_a_real_nested_virtual_host() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<VirtualHost *:443>\n"
          "ServerName api.example.test\n"
          "<Directory \"/srv/www/api\">\n"
          "Options FollowSymLinks\n"
          "<FilesMatch \"\\.php$\">\n"
          "Require all granted\n"
          "</FilesMatch>\n"
          "</Directory>\n"
          "</VirtualHost>\n")
         (indent-region
          (point-min)
          (point-max))
         (list
          (buffer-string)
          (apache-mode-test-line-indents)))"##;
    let expect = expect![[
        r#"OK ("<VirtualHost *:443>\n    ServerName api.example.test\n    <Directory \"/srv/www/api\">\n\11Options FollowSymLinks\n\11<FilesMatch \"\\.php$\">\n\11    Require all granted\n\11</FilesMatch>\n    </Directory>\n</VirtualHost>\n" (("<VirtualHost *:443>" 0) ("    ServerName api.example.test" 4) ("    <Directory \"/srv/www/api\">" 4) ("\11Options FollowSymLinks" 8) ("\11<FilesMatch \"\\.php$\">" 8) ("\11    Require all granted" 12) ("\11</FilesMatch>" 8) ("    </Directory>" 4) ("</VirtualHost>" 0)))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_calculates_indentation_from_the_previous_significant_line() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<VirtualHost *:80>\n"
          "    ServerName example.test\n"
          "\n"
          "      # comment deliberately over-indented\n"
          "DocumentRoot /srv/www\n"
         "</VirtualHost>\n")
         (cl-labels
             ((probe
               (line)
               (save-excursion
                 (goto-char
                  (point-min))
                 (forward-line
                  (1- line))
                 (list
                  (line-number-at-pos)
                  (current-indentation)
                  (apache-calculate-indentation)
                  (line-number-at-pos)
                  (current-column)))))
           (list
            (probe 1)
            (probe 2)
            (probe 3)
            (probe 4)
            (probe 5)
            (probe 6))))"##;
    let expect =
        expect!["OK ((1 0 0 1 0) (2 4 4 2 0) (3 0 4 3 0) (4 6 4 4 0) (5 0 4 5 0) (6 0 -4 6 0))"];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_custom_indent_levels_scale_nested_sections_exactly() {
    let elisp_form = r##"(mapcar
         (lambda (level)
           (with-temp-buffer
             (apache-mode)
             (setq-local
              apache-indent-level
              level)
             (insert
              "<VirtualHost *:443>\n"
              "<Directory \"/srv/www\">\n"
              "Require all granted\n"
              "</Directory>\n"
              "</VirtualHost>\n")
             (indent-region
              (point-min)
              (point-max))
             (list
              level
              (apache-mode-test-line-indents)
              (buffer-string))))
         '(0 2 6))"##;
    let expect = expect![[
        r#"OK ((0 (("<VirtualHost *:443>" 0) ("<Directory \"/srv/www\">" 0) ("Require all granted" 0) ("</Directory>" 0) ("</VirtualHost>" 0)) "<VirtualHost *:443>\n<Directory \"/srv/www\">\nRequire all granted\n</Directory>\n</VirtualHost>\n") (2 (("<VirtualHost *:443>" 0) ("  <Directory \"/srv/www\">" 2) ("    Require all granted" 4) ("  </Directory>" 2) ("</VirtualHost>" 0)) "<VirtualHost *:443>\n  <Directory \"/srv/www\">\n    Require all granted\n  </Directory>\n</VirtualHost>\n") (6 (("<VirtualHost *:443>" 0) ("      <Directory \"/srv/www\">" 6) ("\11    Require all granted" 12) ("      </Directory>" 6) ("</VirtualHost>" 0)) "<VirtualHost *:443>\n      <Directory \"/srv/www\">\n\11    Require all granted\n      </Directory>\n</VirtualHost>\n"))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_indent_line_clamps_negative_closer_indentation_to_column_zero() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "        </Directory>\n")
         (goto-char
          (point-min))
         (let ((calculated
                (apache-calculate-indentation)))
           (apache-indent-line)
           (list
            calculated
            (current-indentation)
            (current-column)
            (buffer-string))))"##;
    let expect = expect![[r#"OK (0 0 0 "</Directory>\n")"#]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_indent_line_preserves_point_offset_in_content_and_moves_indentation_points() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<Directory \"/srv/www\">\n"
          " ServerName example.test\n"
          "</Directory>\n")
         (goto-char
          (point-min))
         (forward-line 1)
         (search-forward
          "Name")
         (let ((before-content
                (list
                 (point)
                 (current-column))))
           (apache-indent-line)
           (let ((after-content
                  (list
                   (point)
                   (current-column)
                   (current-indentation)
                   (buffer-string))))
             (beginning-of-line)
             (forward-char 2)
             (apache-indent-line)
             (list
              before-content
              after-content
              (point)
              (current-column)
              (current-indentation)
              (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ((35 11) (38 14 4 "<Directory \"/srv/www\">\n    ServerName example.test\n</Directory>\n") 28 4 4 "<Directory \"/srv/www\">\n    ServerName example.test\n</Directory>\n")"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_previous_indentation_skips_blank_and_comment_lines_and_moves_point() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "  ServerName example.test\n"
          "\n"
          "          # maintenance note\n"
          "Directive value\n")
         (goto-char
          (point-max))
         (forward-line -1)
         (end-of-line)
         (let ((before
                (list
                 (line-number-at-pos)
                 (current-column))))
           (list
            before
            (apache-previous-indentation)
            (line-number-at-pos)
            (current-indentation)
            (current-column))))"##;
    let expect = expect!["OK ((4 15) 2 1 2 0)"];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_indentation_exposes_its_literal_angle_line_heuristic() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<VirtualHost *:80> trailing text\n"
          "Directive one\n"
          "<NotARealSection\n"
          "Directive two\n"
          "</NotARealSection> trailing text\n"
          "Directive three\n")
         (indent-region
          (point-min)
          (point-max))
         (list
          (buffer-string)
          (apache-mode-test-line-indents)))"##;
    let expect = expect![[
        r#"OK ("<VirtualHost *:80> trailing text\n    Directive one\n    <NotARealSection\n\11Directive two\n    </NotARealSection> trailing text\n    Directive three\n" (("<VirtualHost *:80> trailing text" 0) ("    Directive one" 4) ("    <NotARealSection" 4) ("\11Directive two" 8) ("    </NotARealSection> trailing text" 4) ("    Directive three" 4)))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_indentation_normalizes_tabs_using_the_buffer_indentation_policy() {
    let elisp_form = r##"(mapcar
         (lambda (tabs)
           (with-temp-buffer
             (apache-mode)
             (setq-local
              indent-tabs-mode
              tabs)
             (setq-local
              tab-width
              4)
             (setq-local
              apache-indent-level
              8)
             (insert
              "<Directory \"/srv/www\">\n"
              "Require all granted\n"
              "</Directory>\n")
             (indent-region
              (point-min)
              (point-max))
             (list
              tabs
              (buffer-string)
              (apache-mode-test-line-indents))))
         '(nil t))"##;
    let expect = expect![[
        r#"OK ((nil "<Directory \"/srv/www\">\n        Require all granted\n</Directory>\n" (("<Directory \"/srv/www\">" 0) ("        Require all granted" 8) ("</Directory>" 0))) (t "<Directory \"/srv/www\">\n\11\11Require all granted\n</Directory>\n" (("<Directory \"/srv/www\">" 0) ("\11\11Require all granted" 8) ("</Directory>" 0))))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}
