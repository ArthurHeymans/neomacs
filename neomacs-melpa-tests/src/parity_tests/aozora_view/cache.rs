use expect_test::expect;

use super::assert_aozora_view_parity;

#[test]
fn cache_path_is_a_literal_directory_file_name_and_extension_composition() {
    let elisp_form = r##"(let ((aozora-view-cache-directory
                         "/sandbox/cache/")
                        (aozora-view-cache-ext
                         ".rendered"))
                     (mapcar
                      #'aozora-view-cache-file
                      '("book.txt"
                        "nested/book.txt"
                        "/library/book.txt"
                        "日本語.txt")))"##;
    let expect = expect![[
        r#"OK ("/sandbox/cache/book.txt.rendered" "/sandbox/cache/nested/book.txt.rendered" "/sandbox/cache//library/book.txt.rendered" "/sandbox/cache/日本語.txt.rendered")"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn save_cache_honors_disabled_declined_and_accepted_policy_without_nondeterminism() {
    let elisp_form = r##"(let* ((aozora-view-cache-directory
                          (expand-file-name
                           "aozora-cache-policy/"
                           temporary-file-directory))
                         (aozora-view-cache-ext
                          ".cache")
                         (cache
                          (aozora-view-cache-file
                           "book.txt"))
                         (prompts nil))
                     (with-temp-buffer
                       (insert "rendered text")
                       (let ((aozora-view-save-cache
                              nil))
                         (list
                          (aozora-view-save-cache
                           "book.txt")
                          (file-exists-p cache)
                          (let ((aozora-view-save-cache
                                 'prompt))
                            (cl-letf
                                (((symbol-function
                                   'y-or-n-p)
                                  (lambda (prompt)
                                    (push prompt prompts)
                                    nil)))
                              (aozora-view-save-cache
                               "book.txt")))
                          (file-exists-p cache)
                          (let ((aozora-view-save-cache
                                 'prompt))
                            (cl-letf
                                (((symbol-function
                                   'y-or-n-p)
                                  (lambda (prompt)
                                    (push prompt prompts)
                                    t)))
                              (aozora-view-save-cache
                               "book.txt")))
                          (file-exists-p cache)
                          (nreverse prompts)))))"##;
    let expect = expect![[
        r#"OK (nil nil nil nil t t ("Do you want to save cache file? " "Do you want to save cache file? "))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn cache_round_trip_preserves_unicode_newlines_and_text_properties() {
    let elisp_form = r##"(let* ((aozora-view-cache-directory
                          (expand-file-name
                           "aozora-cache-roundtrip/"
                           temporary-file-directory))
                         (aozora-view-cache-ext
                          ".cache")
                         (aozora-view-save-cache
                          t)
                         (cache
                          (aozora-view-cache-file
                           "novel.txt")))
                     (with-temp-buffer
                       (insert
                        "第一行\n"
                        (propertize
                         "青空"
                         'face
                         'underline
                         'ruby
                         '(2 . "あおぞら"))
                        "\n終")
                       (let ((original
                              (buffer-string))
                             (saved
                              (aozora-view-save-cache
                               "novel.txt")))
                         (erase-buffer)
                         (let ((loaded
                                (aozora-view-load-cache
                                 "novel.txt")))
                           (list
                            saved
                            loaded
                            (file-exists-p cache)
                            (buffer-string)
                            (equal
                             original
                             (buffer-string))
                            (text-properties-at
                             5))))))"##;
    let expect = expect![[
        r#"OK (t t t #("第一行\n青空\n終" 4 6 (face underline ruby (2 . "あおぞら"))) t (face underline ruby (2 . "あおぞら")))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn default_gzip_cache_round_trip_writes_a_real_compressed_payload() {
    let elisp_form = r##"(let* ((aozora-view-cache-directory
                          (expand-file-name
                           "aozora-cache-gzip/"
                           temporary-file-directory))
                         (aozora-view-save-cache
                          t)
                         (cache
                          (aozora-view-cache-file
                           "compressed-book.txt")))
                     (with-temp-buffer
                       (insert
                        "圧縮された青空文庫\nsecond line")
                       (let ((saved
                              (aozora-view-save-cache
                               "compressed-book.txt"))
                             (magic
                              (with-temp-buffer
                                (set-buffer-multibyte
                                 nil)
                                (insert-file-contents-literally
                                 cache)
                                (list
                                 (char-after 1)
                                 (char-after 2)
                                 (>
                                  (buffer-size)
                                  2)))))
                         (erase-buffer)
                         (let ((loaded
                                (aozora-view-load-cache
                                 "compressed-book.txt")))
                           (list
                            saved
                            loaded
                            magic
                            (buffer-string))))))"##;
    let expect = expect![[r#"OK (t t (31 139 t) "圧縮された青空文庫\nsecond line")"#]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn cache_loader_distinguishes_missing_empty_malformed_and_valid_lisp_payloads() {
    let elisp_form = r##"(let* ((aozora-view-cache-directory
                          (expand-file-name
                           "aozora-cache-invalid/"
                           temporary-file-directory))
                         (aozora-view-cache-ext
                          ".cache")
                         (cache
                          (aozora-view-cache-file
                           "book.txt")))
                     (make-directory
                      aozora-view-cache-directory
                      t)
                     (with-temp-buffer
                       (let ((missing
                              (aozora-view-load-cache
                               "book.txt")))
                         (with-temp-file cache)
                         (let ((empty
                                (condition-case error
                                    (aozora-view-load-cache
                                     "book.txt")
                                  (error
                                   (error-message-string
                                    error)))))
                           (with-temp-file cache
                             (insert
                              "(not-a-string payload)"))
                           (let ((malformed
                                  (condition-case error
                                      (aozora-view-load-cache
                                       "book.txt")
                                    (error
                                     (error-message-string
                                      error)))))
                             (with-temp-file cache
                               (prin1
                                "usable"
                                (current-buffer)))
                             (let ((valid
                                    (aozora-view-load-cache
                                     "book.txt")))
                               (list
                                missing
                                empty
                                malformed
                                valid
                                (buffer-string))))))))"##;
    let expect = expect![[
        r#"OK (nil "End of file during parsing: #<killed buffer>" "Wrong type argument: char-or-string-p, (not-a-string payload)" t "usable")"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}
