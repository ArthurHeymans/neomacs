use expect_test::expect;

use super::assert_apache_mode_parity;

#[test]
fn apache_mode_fontifies_a_real_tls_virtual_host_by_semantic_role() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<VirtualHost *:443>\n"
          "  ServerName api.example.test\n"
          "  DocumentRoot \"/srv/www/api\"\n"
          "  SSLEngine on\n"
          "  SSLProtocol all -SSLv3 -TLSv1\n"
          "  CustomLog /var/log/apache2/api-access.log combined\n"
          "</VirtualHost>\n")
         (font-lock-ensure)
         (mapcar
          (lambda (needle)
            (list
             needle
             (apache-mode-test-face-at needle)))
          '("<VirtualHost"
            "VirtualHost"
            "ServerName"
            "api.example.test"
            "DocumentRoot"
            "\"/srv/www/api\""
            "SSLEngine"
            "on"
            "SSLProtocol"
            "all"
            "-SSLv3"
            "CustomLog"
            "combined"
            "</VirtualHost>")))"##;
    let expect = expect![[
        r#"OK (("<VirtualHost" nil) ("VirtualHost" font-lock-function-name-face) ("ServerName" font-lock-keyword-face) ("api.example.test" nil) ("DocumentRoot" font-lock-keyword-face) ("\"/srv/www/api\"" font-lock-string-face) ("SSLEngine" font-lock-keyword-face) ("on" font-lock-type-face) ("SSLProtocol" font-lock-keyword-face) ("all" font-lock-type-face) ("-SSLv3" nil) ("CustomLog" font-lock-keyword-face) ("combined" nil) ("</VirtualHost>" nil))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_fontifies_auth_rewrite_proxy_and_handler_workflows() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<LocationMatch \"^/private/\">\n"
          "  AuthType Basic\n"
          "  AuthName \"Restricted\"\n"
          "  Require valid-user\n"
          "  RewriteEngine On\n"
          "  RewriteCond %{HTTPS} !=on\n"
          "  RewriteRule ^ https://%{HTTP_HOST}%{REQUEST_URI} [R=301,L]\n"
          "  ProxyPass http://127.0.0.1:9000 retry=0\n"
          "  SetHandler proxy:fcgi://127.0.0.1:9000\n"
          "</LocationMatch>\n")
         (font-lock-ensure)
         (mapcar
          (lambda (needle)
            (list
             needle
             (apache-mode-test-face-at needle)))
          '("LocationMatch"
            "AuthType"
            "Basic"
            "AuthName"
            "Require"
            "valid-user"
            "RewriteEngine"
            "On"
            "RewriteCond"
            "RewriteRule"
            "ProxyPass"
            "retry"
            "SetHandler"
            "proxy:fcgi"
            "</LocationMatch>")))"##;
    let expect = expect![[
        r#"OK (("LocationMatch" font-lock-function-name-face) ("AuthType" font-lock-keyword-face) ("Basic" font-lock-type-face) ("AuthName" font-lock-keyword-face) ("Require" font-lock-keyword-face) ("valid-user" font-lock-type-face) ("RewriteEngine" font-lock-keyword-face) ("On" font-lock-type-face) ("RewriteCond" font-lock-keyword-face) ("RewriteRule" font-lock-keyword-face) ("ProxyPass" font-lock-keyword-face) ("retry" nil) ("SetHandler" font-lock-keyword-face) ("proxy:fcgi" nil) ("</LocationMatch>" nil))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_fontification_is_case_insensitive_but_respects_token_boundaries() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "servername lower.example\n"
          "SERVERNAME upper.example\n"
          "XServerName prefixed.example\n"
          "ServerNameSuffix suffixed.example\n"
          "SSLEngine on\n"
          "ssleNGINE OFF\n"
          "noton oncall offsite\n")
         (font-lock-ensure)
         (mapcar
          (lambda (sample)
            (list
             (car sample)
             (cdr sample)
             (apache-mode-test-face-at
              (car sample)
              (cdr sample))))
          '(("servername" . 1)
            ("SERVERNAME" . 1)
            ("ServerName" . 1)
            ("ServerNameSuffix" . 1)
            ("SSLEngine" . 1)
            ("ssleNGINE" . 1)
            ("on" . 1)
            ("OFF" . 1)
            ("noton" . 1)
            ("oncall" . 1)
            ("offsite" . 1))))"##;
    let expect = expect![[
        r#"OK (("servername" 1 font-lock-keyword-face) ("SERVERNAME" 1 font-lock-keyword-face) ("ServerName" 1 nil) ("ServerNameSuffix" 1 nil) ("SSLEngine" 1 font-lock-keyword-face) ("ssleNGINE" 1 font-lock-keyword-face) ("on" 1 font-lock-type-face) ("OFF" 1 font-lock-type-face) ("noton" 1 nil) ("oncall" 1 nil) ("offsite" 1 nil))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_fontification_obeys_comment_and_string_syntax() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "# ServerName commented.example SSLEngine on\n"
          "Header set X-Note \"ServerName # still-a-string\"\n"
          "ServerName live.example # SSLEngine off\n")
         (font-lock-ensure)
         (mapcar
          (lambda (sample)
            (let ((needle
                   (car sample))
                  (occurrence
                   (cdr sample)))
              (list
               needle
               occurrence
               (apache-mode-test-face-at
                needle
                occurrence))))
          '(("#" . 1)
            ("ServerName" . 1)
            ("SSLEngine" . 1)
            ("Header" . 1)
            ("ServerName" . 2)
            ("# still-a-string" . 1)
            ("ServerName" . 3)
            ("# SSLEngine" . 1)
            ("SSLEngine" . 2))))"##;
    let expect = expect![[
        r##"OK (("#" 1 font-lock-comment-delimiter-face) ("ServerName" 1 font-lock-comment-face) ("SSLEngine" 1 font-lock-comment-face) ("Header" 1 font-lock-keyword-face) ("ServerName" 2 font-lock-string-face) ("# still-a-string" 1 font-lock-string-face) ("ServerName" 3 font-lock-keyword-face) ("# SSLEngine" 1 font-lock-comment-delimiter-face) ("SSLEngine" 2 font-lock-comment-face))"##
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_nested_section_openers_and_closers_share_the_section_face() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "<IfModule mod_ssl.c>\n"
          "  <VirtualHost *:443>\n"
          "    <Directory \"/srv/www\">\n"
          "      <FilesMatch \"\\.php$\">\n"
          "      </FilesMatch>\n"
          "    </Directory>\n"
          "  </VirtualHost>\n"
          "</IfModule>\n")
         (font-lock-ensure)
         (mapcar
          (lambda (sample)
            (list
             (car sample)
             (cdr sample)
             (apache-mode-test-face-at
              (car sample)
              (cdr sample))))
          '(("IfModule" . 1)
            ("VirtualHost" . 1)
            ("Directory" . 1)
            ("FilesMatch" . 1)
            ("FilesMatch" . 2)
            ("Directory" . 2)
            ("VirtualHost" . 2)
            ("IfModule" . 2))))"##;
    let expect = expect![[
        r#"OK (("IfModule" 1 font-lock-function-name-face) ("VirtualHost" 1 font-lock-function-name-face) ("Directory" 1 font-lock-function-name-face) ("FilesMatch" 1 font-lock-function-name-face) ("FilesMatch" 2 font-lock-function-name-face) ("Directory" 2 font-lock-function-name-face) ("VirtualHost" 2 font-lock-function-name-face) ("IfModule" 2 font-lock-function-name-face))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_font_lock_flush_refontifies_after_live_configuration_edits() {
    let elisp_form = r##"(with-temp-buffer
         (apache-mode)
         (insert
          "UnknownDirective off\n")
         (font-lock-ensure)
         (let ((before
                (list
                 (apache-mode-test-face-at
                  "UnknownDirective")
                 (apache-mode-test-face-at
                  "off"))))
           (goto-char
            (point-min))
           (delete-region
            (line-beginning-position)
            (line-end-position))
           (insert
            "ServerTokens Prod")
           (font-lock-flush)
           (font-lock-ensure)
           (list
            before
            (apache-mode-test-face-at
             "ServerTokens")
            (apache-mode-test-face-at
             "Prod")
            (buffer-string))))"##;
    let expect = expect![[
        r#"OK ((nil font-lock-type-face) font-lock-keyword-face nil #("ServerTokens Prod\n" 0 12 (face font-lock-keyword-face)))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}
