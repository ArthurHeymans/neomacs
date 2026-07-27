use expect_test::expect;

use super::assert_archive_phar_parity;

#[test]
fn archive_phar_extract_sends_exact_archive_and_member_protocol() {
    let elisp_form = r##"(with-temp-buffer
         (let (runtime-call)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (code input)
                   (setq runtime-call
                         (list
                          (equal code
                                 archive-phar--code-extract-file)
                          (with-current-buffer input
                            (buffer-string))))
                   "<?php echo 'hello';\n")))
             (let ((result
                    (archive-phar-extract
                     "/work/releases/app.phar"
                     "src/Hello.php")))
               (list
                (eq result (current-buffer))
                (buffer-string)
                (point)
                runtime-call)))))"##;
    let expect = expect![[
        r#"OK (t "<?php echo 'hello';\n" 21 (t "/work/releases/app.phar\11src/Hello.php"))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_extract_inserts_at_point_without_replacing_existing_text() {
    let elisp_form = r##"(with-temp-buffer
         (insert "prefix<>suffix")
         (goto-char 8)
         (cl-letf
             (((symbol-function 'php-runtime-eval)
               (lambda (_code _input) "PAYLOAD")))
           (let ((before (point)))
             (archive-phar-extract "app.phar" "data.txt")
             (list
              before
              (point)
              (buffer-string)))))"##;
    let expect = expect![[r#"OK (8 15 "prefix<PAYLOAD>suffix")"#]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_extract_preserves_unicode_archive_member_and_payload() {
    let elisp_form = r##"(with-temp-buffer
         (let (stdin)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code input)
                   (setq stdin
                         (with-current-buffer input
                           (buffer-string)))
                   "こんにちは café λ\n")))
             (archive-phar-extract
              "/work/配布 アプリ.phar"
              "資料/日本語 file.txt")
             (list
              stdin
              (buffer-string)
              (multibyte-string-p (buffer-string))
              (string-bytes (buffer-string))
              (length (buffer-string))))))"##;
    let expect = expect![[
        r#"OK ("/work/配布 アプリ.phar\11資料/日本語 file.txt" "こんにちは café λ\n" t 25 13)"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_extract_empty_member_is_successful_noop() {
    let elisp_form = r##"(with-temp-buffer
         (insert "before")
         (let (stdin)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code input)
                   (setq stdin
                         (with-current-buffer input
                           (buffer-string)))
                   "")))
             (let ((before-point (point)))
               (archive-phar-extract
                "/work/empty.phar" "empty.txt")
               (list
                stdin
                before-point
                (point)
                (buffer-string))))))"##;
    let expect = expect![[r#"OK ("/work/empty.phar\11empty.txt" 7 7 "before")"#]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_extract_runtime_error_leaves_destination_unchanged() {
    let elisp_form = r##"(with-temp-buffer
         (insert "stable")
         (goto-char 4)
         (let ((before-point (point)))
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code _input)
                   (error "member not found"))))
             (condition-case error
                 (list
                  :ok
                  (archive-phar-extract
                   "/work/app.phar" "missing.php"))
               (error
                (list
                 :error
                 (car error)
                 (error-message-string error)
                 (buffer-string)
                 before-point
                 (point)))))))"##;
    let expect = expect![[r#"OK (:error error "member not found" "stable" 4 4)"#]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_extract_protocol_exposes_embedded_tab_edge_case_exactly() {
    let elisp_form = r##"(with-temp-buffer
         (let (stdin)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code input)
                   (setq stdin
                         (with-current-buffer input
                           (buffer-string)))
                   "result")))
             (archive-phar-extract
              "/work/tab\tarchive.phar"
              "path/tab\tmember.txt")
             (list
              stdin
              (split-string stdin "\t" nil)
              (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ("/work/tab\11archive.phar\11path/tab\11member.txt" ("/work/tab" "archive.phar" "path/tab" "member.txt") "result")"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_extract_does_not_override_global_php_runtime_executable() {
    let elisp_form = r##"(with-temp-buffer
         (let ((archive-phar-php-executable
                "/configured/php")
               (php-runtime-php-executable
                "/runtime/php")
               seen)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code _input)
                   (setq seen php-runtime-php-executable)
                   "payload")))
             (archive-phar-extract
              "/work/app.phar" "member")
             (list
              seen
              php-runtime-php-executable
              archive-phar-php-executable
              (buffer-string)))))"##;
    let expect = expect![[r#"OK ("/runtime/php" "/runtime/php" "/configured/php" "payload")"#]];
    assert_archive_phar_parity(elisp_form, expect);
}
