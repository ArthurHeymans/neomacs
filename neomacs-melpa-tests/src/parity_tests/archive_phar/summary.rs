use expect_test::expect;

use super::assert_archive_phar_parity;

#[test]
fn archive_phar_summarize_transforms_realistic_json_entries() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/releases/application.phar")
         (let (runtime-call summarized)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (code input)
                   (setq runtime-call
                         (list
                          (equal code
                                 archive-phar--code-summarize-file)
                          (with-current-buffer input
                            (buffer-string))
                          php-runtime-php-executable))
                   "[{\"pathname\":\"bin/tool\",\"mtime\":0,\"size\":42,\"perms\":493,\"type\":\"file\"},{\"pathname\":\"src/日本語 name.php\",\"mtime\":1700000000,\"size\":8192,\"perms\":420,\"type\":\"file\"}]"))
                ((symbol-function 'archive--summarize-descs)
                 (lambda (descs)
                   (setq summarized
                         (mapcar
                          (lambda (desc)
                            (list
                             (archive--file-desc-ext-file-name desc)
                             (archive--file-desc-int-file-name desc)
                             (archive--file-desc-mode desc)
                             (archive--file-desc-size desc)
                             (archive--file-desc-time desc)
                             (archive--file-desc-pos desc)
                             (archive--file-desc-ratio desc)
                             (archive--file-desc-uid desc)
                             (archive--file-desc-gid desc)))
                          descs))
                   :summarized)))
             (list
              (archive-phar-summarize)
              runtime-call
              summarized))))"##;
    let expect = expect![[
        r#"OK (:summarized (t "/work/releases/application.phar" "/usr/bin/php") (("bin/tool" "bin/tool" nil 42 " 1-Jan-1970 00:00:00" nil nil nil nil) ("src/日本語 name.php" "src/日本語 name.php" nil 8192 "14-Nov-2023 22:13:20" nil nil nil nil)))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_summarize_empty_archive_forwards_empty_descriptor_list() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/empty.phar")
         (let (received)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code _input) "[]"))
                ((symbol-function 'archive--summarize-descs)
                 (lambda (descs)
                   (setq received descs)
                   'empty-summary)))
             (list
              (archive-phar-summarize)
              received))))"##;
    let expect = expect!["OK (empty-summary nil)"];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_summarize_binds_configured_php_executable_only_during_runtime() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/app.phar")
         (let ((archive-phar-php-executable
                "/opt/php/bin/php-custom")
               (php-runtime-php-executable
                "/outer/php")
               seen)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code _input)
                   (setq seen php-runtime-php-executable)
                   "[]"))
                ((symbol-function 'archive--summarize-descs)
                 (lambda (_descs) :done)))
             (list
              (archive-phar-summarize)
              seen
              php-runtime-php-executable))))"##;
    let expect = expect![[r#"OK (:done "/opt/php/bin/php-custom" "/outer/php")"#]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_summarize_date_pipeline_receives_timestamp_halves() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/dates.phar")
         (let (calls received)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code _input)
                   "[{\"pathname\":\"a.php\",\"mtime\":1234567890,\"size\":7}]"))
                ((symbol-function
                  'datetime-format--int-to-timestamp)
                 (lambda (mtime)
                   (push (list :timestamp mtime) calls)
                   '(111 222)))
                ((symbol-function 'archive-unixdate)
                 (lambda (low high)
                   (push (list :date low high) calls)
                   "DATE"))
                ((symbol-function 'archive-unixtime)
                 (lambda (low high)
                   (push (list :time low high) calls)
                   "TIME"))
                ((symbol-function 'archive--summarize-descs)
                 (lambda (descs)
                   (setq received
                         (archive--file-desc-time
                          (car descs)))
                   :done)))
             (list
              (archive-phar-summarize)
              (nreverse calls)
              received))))"##;
    let expect = expect![[
        r#"OK (:done ((:timestamp 1234567890) (:date 222 111) (:time 222 111)) "DATE TIME")"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_summarize_preserves_json_order_duplicates_and_zero_sizes() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/order.phar")
         (let (names)
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code _input)
                   "[{\"pathname\":\"z.php\",\"mtime\":1,\"size\":0},{\"pathname\":\"a.php\",\"mtime\":1,\"size\":9},{\"pathname\":\"z.php\",\"mtime\":1,\"size\":3}]"))
                ((symbol-function 'archive--summarize-descs)
                 (lambda (descs)
                   (setq names
                         (mapcar
                          (lambda (desc)
                            (list
                             (archive--file-desc-ext-file-name desc)
                             (archive--file-desc-size desc)))
                          descs))
                   :done)))
             (list
              (archive-phar-summarize)
              names))))"##;
    let expect = expect![[r#"OK (:done (("z.php" 0) ("a.php" 9) ("z.php" 3)))"#]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_summarize_uses_real_archive_listing_renderer() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/render.phar"
               archive-hidden-columns nil)
         (cl-letf
             (((symbol-function 'php-runtime-eval)
               (lambda (_code _input)
                 "[{\"pathname\":\"bin/run\",\"mtime\":0,\"size\":42},{\"pathname\":\"src/main.php\",\"mtime\":86400,\"size\":1024}]")))
           (let ((descs (archive-phar-summarize)))
             (list
              (buffer-string)
              archive-file-name-indent
              (marker-position archive-file-list-start)
              (marker-position archive-file-list-end)
              (mapcar
               (lambda (desc)
                 (list
                  (archive--file-desc-int-file-name desc)
                  (archive--file-desc-size desc)
                  (archive--file-desc-time desc)))
               (append descs nil))))))"##;
    let expect = expect![[
        r#"OK (#("M Size       Date&time         Filename\n- ----  --------------------  ----------------\n    42   1-Jan-1970 00:00:00  bin/run\n  1024   2-Jan-1970 00:00:00  src/main.php\n- ----  --------------------  ----------------\n  1066                         2 files\n" 117 124 (mouse-face highlight help-echo #1="mouse-2: extract this file into a buffer") 155 167 (mouse-face highlight help-echo #1#)) 30 88 169 (("bin/run" 42 " 1-Jan-1970 00:00:00") ("src/main.php" 1024 " 2-Jan-1970 00:00:00")))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_summarize_invalid_json_surfaces_parser_error_without_rendering() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/broken.phar")
         (let ((rendered nil))
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code _input) "{broken-json"))
                ((symbol-function 'archive--summarize-descs)
                 (lambda (_descs)
                   (setq rendered t))))
             (condition-case error
                 (list :ok (archive-phar-summarize))
               (error
                (list
                 :error
                 (car error)
                 (error-message-string error)
                 rendered
                 (buffer-string)))))))"##;
    let expect = expect![[
        r#"OK (:error json-parse-error "could not parse JSON stream: 1, nil, 2" nil "")"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_summarize_runtime_failure_propagates_before_json_or_rendering() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/failure.phar")
         (let ((rendered nil))
           (cl-letf
               (((symbol-function 'php-runtime-eval)
                 (lambda (_code _input)
                   (error "PHP runtime failed")))
                ((symbol-function 'archive--summarize-descs)
                 (lambda (_descs)
                   (setq rendered t))))
             (condition-case error
                 (list :ok (archive-phar-summarize))
               (error
                (list
                 :error
                 (car error)
                 (error-message-string error)
                 rendered))))))"##;
    let expect = expect![[r#"OK (:error error "PHP runtime failed" nil)"#]];
    assert_archive_phar_parity(elisp_form, expect);
}
