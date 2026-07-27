use expect_test::expect;

use super::assert_archive_phar_parity;

#[test]
fn archive_phar_find_type_accepts_exact_lowercase_extension() {
    let elisp_form = r##"(mapcar
         (lambda (name)
           (with-temp-buffer
             (setq buffer-file-name name)
             (list name (archive-phar-find-type))))
         '("/work/app.phar"
           "/work/release.tar.phar"
           "/work/with space.phar"
           "/work/日本語.phar"))"##;
    let expect = expect![[
        r#"OK (("/work/app.phar" phar) ("/work/release.tar.phar" phar) ("/work/with space.phar" phar) ("/work/日本語.phar" phar))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_find_type_rejects_near_miss_names_case_sensitively() {
    let elisp_form = r##"(mapcar
         (lambda (name)
           (with-temp-buffer
             (setq buffer-file-name name)
             (list name (archive-phar-find-type))))
         '(nil
           "/work/app.PHAR"
           "/work/app.Phar"
           "/work/app.phar.bak"
           "/work/phar"
           "/work/.phar/"
           "/work/app.phar.gz"))"##;
    let expect = expect![[
        r#"OK ((nil nil) ("/work/app.PHAR" nil) ("/work/app.Phar" nil) ("/work/app.phar.bak" nil) ("/work/phar" nil) ("/work/.phar/" nil) ("/work/app.phar.gz" nil))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_find_type_advice_recognizes_phar_without_reading_bytes() {
    let elisp_form = r##"(mapcar
         (lambda (contents)
           (with-temp-buffer
             (setq buffer-file-name "/work/application.phar")
             (insert contents)
             (list contents
                   (point)
                   (archive-find-type)
                   (point))))
         '("" "not an archive" "PK\003\004payload" "Rar!"))"##;
    let expect = expect![[
        r#"OK (("" 1 phar 1) ("not an archive" 15 phar 15) ("PK\3\4payload" 12 phar 12) ("Rar!" 5 phar 5))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_find_type_advice_defers_to_builtin_formats_for_non_phar() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (setq buffer-file-name (car case))
             (insert (cadr case))
             (list
              (car case)
              (archive-find-type))))
         '(("/work/data.zip" "PK\003\004payload")
           ("/work/data.rar" "Rar!payload")
           ("/work/data.a" "!<arch>\nrest")
           ("/work/data.7z" "7z\274\257\047\034rest")))"##;
    let expect = expect![[
        r#"OK (("/work/data.zip" zip) ("/work/data.rar" rar) ("/work/data.a" ar) ("/work/data.7z" 7z))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_find_type_non_phar_unknown_bytes_preserve_builtin_error() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/application.bin")
         (insert "not an archive")
         (condition-case error
             (list :ok (archive-find-type))
           (error
            (list :error
                  (car error)
                  (error-message-string error)
                  (point)))))"##;
    let expect = expect![[r#"OK (:error error "Buffer format not recognized" 1)"#]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_mode_rule_resolves_realistic_file_names() {
    let elisp_form = r##"(mapcar
         (lambda (name)
           (list
            name
            (assoc-default
             name auto-mode-alist #'string-match)))
         '("/work/app.phar"
           "/work/app.PHAR"
           "/work/app.phar.bak"
           "/work/nested/archive.phar"
           "/work/日本語.phar"))"##;
    let expect = expect![[
        r#"OK (("/work/app.phar" archive-mode) ("/work/app.PHAR" archive-mode) ("/work/app.phar.bak" (nil t)) ("/work/nested/archive.phar" archive-mode) ("/work/日本語.phar" archive-mode))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_detection_does_not_mutate_match_or_case_state() {
    let elisp_form = r##"(let ((case-fold-search t))
         (string-match "\\(seed\\)" "seed")
         (let ((before (match-data)))
           (with-temp-buffer
             (setq buffer-file-name "/work/app.phar")
             (list
              (archive-phar-find-type)
              case-fold-search
              before
              (match-data)))))"##;
    let expect = expect!["OK (phar t (0 4 0 4) (9 14))"];
    assert_archive_phar_parity(elisp_form, expect);
}
