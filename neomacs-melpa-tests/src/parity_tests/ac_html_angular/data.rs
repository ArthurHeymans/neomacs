use expect_test::expect;

use super::assert_ac_html_angular_parity;

#[test]
fn ac_html_angular_tag_data_preserves_exact_order_boundaries_and_no_final_newline() {
    let elisp_form = r##"(let ((file
                    (expand-file-name
                     "html-tag-list"
                     ac-html-angular-source-dir)))
               (with-temp-buffer
                 (insert-file-contents
                  file)
                 (let ((contents
                        (buffer-string)))
                   (list
                    (split-string
                     contents "\n" t)
                    (length
                     (split-string
                      contents "\n" t))
                    (string-suffix-p
                     "\n"
                     contents)
                    (file-relative-name
                     file
                     ac-html-angular-source-dir)))))"##;
    let expect = expect![[
        r#"OK (("a" "form" "input" "ng-form" "ng-include" "ng-message" "ng-message-exp" "ng-messages" "ng-messages-include" "ng-pluralize" "ng-switch" "ng-transclude" "ng-view" "script" "select" "textarea") 16 nil "html-tag-list")"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_attribute_lists_preserve_every_file_shape_and_digest() {
    let elisp_form = r##"(let* ((directory
                     (expand-file-name
                      "html-attributes-list"
                      ac-html-angular-source-dir))
                    (files
                     (sort
                      (directory-files
                       directory t
                       "^[^.]" t)
                      #'string<)))
               (mapcar
                (lambda (file)
                  (with-temp-buffer
                    (insert-file-contents
                     file)
                    (let ((contents
                           (buffer-string)))
                      (list
                       (file-name-nondirectory
                        file)
                       (length
                        (split-string
                         contents "\n" t))
                       (car
                        (split-string
                         contents "\n" t))
                       (car
                        (last
                         (split-string
                          contents "\n" t)))
                       (string-suffix-p
                        "\n"
                        contents)
                       (secure-hash
                        'sha256
                        contents)))))
                files))"##;
    let expect = expect![[
        r#"OK (("a" 6 "ng-blur" "ng-paste" nil "9e1d7e8db59bcae1fa971dcdba8a0faf05df48ce6cc1708b15ca5e5af45240b9") ("details" 1 "ng-open" "ng-open" nil "ad0a7afe623e8c5075645cb208c9f38b0f673276588dcf79494c762da3ec5a9a") ("form" 2 "name" "ng-submit" nil "1594a49456cd7a011f61ce080f2e438e307d30131414fcd3f58c3a21bc8b9f23") ("global" 43 "ng-animate-swap" "ng-view" nil "6eb3ec0e281476648dd81f8de7b9682305c4112f64f596f7332d5e7438b9a80a") ("html" 1 "ng-csp" "ng-csp" nil "ae746dd227ff3ab305ab3ac6850f17d90c93bd3737d2012155cce21b5f446787") ("img" 2 "ng-src" "ng-srcset" nil "6b4a814b1a0545fd271b70feb1dc0a37fdb46c66518b14e5969e53fc0b2b13cb") ("input" 26 "max" "value" nil "907984028acf1fbcb877e164c055e10885430e624a231398fab5e6e6b1495008") ("ng-include" 2 "autoscroll" "onload" nil "fdc6d7936790f57c6ea1a72ecee9dc69da22146c1b7523a09b61c5bc0c009a06") ("ng-messages" 1 "ng-messages" "ng-messages" nil "00b69bf79b6eb372fea58eb81030e1dae1e3bc86d50c2c6396524ffd73b643f6") ("ng-pluralize" 3 "count" "when" nil "2cdcd3d4596ce3162b0ac5815ff51748b1f8bb250f8c2285c8e3daae43b1a039") ("ng-view" 2 "autoscroll" "onload" nil "fdc6d7936790f57c6ea1a72ecee9dc69da22146c1b7523a09b61c5bc0c009a06") ("option" 1 "ng-selected" "ng-selected" nil "a2dc2962050427b07fc749d66939370eb71eb0ccdb1c2b35f4302091184782d9") ("script" 2 "id" "type" nil "a07abcd7cc2a057387057c18590c1424c6267749264ae4f2dd18d2bea10d3dc9") ("select" 12 "multiple" "required" nil "14d1d5d7f0fee8e84de27df1894dd2bfec2fe0c794d59a3eff28c023e4701c9b") ("textarea" 13 "name" "required" nil "78f75302323b5637fa7a9b501fb5e3d02303a18eb05517e760eb3bb429f94071") ("window" 5 "ng-blur" "ng-paste" nil "bb84326e4a2d92ffcb08a6a237029118da3c551298cc9c9369911b4057928772"))"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_representative_documentation_preserves_markup_and_newlines() {
    let elisp_form = r##"(mapcar
               (lambda (relative)
                 (let ((file
                        (expand-file-name
                         relative
                         ac-html-angular-source-dir)))
                   (with-temp-buffer
                     (insert-file-contents
                      file)
                     (list
                      relative
                      (buffer-size)
                      (buffer-substring-no-properties
                       (point-min)
                       (min
                        (+ (point-min) 90)
                        (point-max)))
                      (string-suffix-p
                       "\n"
                       (buffer-string))
                      (secure-hash
                       'sha256
                       (buffer-string))))))
               '("html-tag-short-docs/ng-view"
                 "html-tag-short-docs/ng-messages"
                 "html-attributes-short-docs/global-ng-app"
                 "html-attributes-short-docs/input-ng-model"
                 "html-attributes-short-docs/a-ng-href"))"##;
    let expect = expect![[
        r##"OK (("html-tag-short-docs/ng-view" 343 "# Overview\n`ngView` is a directive that complements the $route service by\nincluding the re" nil "18f7127eef745d23c0bf91d77c8b4d0b6faf1482467811eea6921b9955b0cefb") ("html-tag-short-docs/ng-messages" 921 "`ngMessages` is a directive that is designed to show and hide messages based on the state\n" nil "6d47aeb93b091c526283314d7fe2ca8fc4cef16f421ef19f4d49158eb29ca06c") ("html-attributes-short-docs/global-ng-app" 3889 "Use this directive to **auto-bootstrap** an AngularJS application. The `ngApp` directive\nd" nil "d9c98a73b08a2a5a4ecbd0cc5b22adf902f61bda58eb4ab60f5d4372ece95961") ("html-attributes-short-docs/input-ng-model" 46 "Assignable angular expression to data-bind to." nil "cbe88b91ad5f4a65b736d3e1c0c1817916664e1c7c3896e6107a0ea4d7368636") ("html-attributes-short-docs/a-ng-href" 541 "Using Angular markup like `{{hash}}` in an href attribute will\nmake the link go to the wro" nil "52eb32677a1ecaa7d274e6bc9b234ac66fc984988e9205d76696806fa2fb12d9"))"##
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_documentation_directories_have_exact_names_counts_and_tree_digest() {
    let elisp_form = r##"(let* ((directories
                     '("html-tag-short-docs"
                       "html-attributes-short-docs"))
                    (files
                     (apply
                      #'append
                      (mapcar
                       (lambda (relative)
                         (directory-files
                          (expand-file-name
                           relative
                           ac-html-angular-source-dir)
                          t "^[^.]" t))
                       directories)))
                    (sorted
                     (sort files #'string<))
                    (payload
                     (mapconcat
                      (lambda (file)
                        (with-temp-buffer
                          (insert
                           (file-relative-name
                            file
                            ac-html-angular-source-dir)
                           "\0")
                          (insert-file-contents
                           file)
                          (buffer-string)))
                      sorted
                      "\0")))
               (list
                (mapcar
                 (lambda (relative)
                   (let ((names
                          (directory-files
                           (expand-file-name
                            relative
                            ac-html-angular-source-dir)
                           nil "^[^.]")))
                     (list
                      relative
                      (length names)
                      (car names)
                      (car
                       (last names)))))
                 directories)
                (length sorted)
                (secure-hash
                 'sha256
                 payload)))"##;
    let expect = expect![[
        r#"OK ((("html-tag-short-docs" 16 "a" "textarea") ("html-attributes-short-docs" 122 "a-ng-blur" "window-ng-paste")) 138 "914107ea61207609a254562c6d4fa055fd17ffe03b4c864165f6ab7048358a39")"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_data_tree_contains_only_the_documented_completion_shapes() {
    let elisp_form = r##"(let ((entries
                    (directory-files
                     ac-html-angular-source-dir
                     nil "^[^.]")))
               (list
                entries
                (mapcar
                 (lambda (entry)
                   (let ((path
                          (expand-file-name
                           entry
                           ac-html-angular-source-dir)))
                     (list
                      entry
                      (file-directory-p path)
                      (and
                       (file-regular-p path)
                       (file-attribute-size
                        (file-attributes
                         path))))))
                 entries)
                (file-exists-p
                 (expand-file-name
                  "html-attrv-list"
                  ac-html-angular-source-dir))
                (file-exists-p
                 (expand-file-name
                  "missing"
                  ac-html-angular-source-dir))))"##;
    let expect = expect![[
        r#"OK (("html-attributes-list" "html-attributes-short-docs" "html-tag-list" "html-tag-short-docs") (("html-attributes-list" t nil) ("html-attributes-short-docs" t nil) ("html-tag-list" nil 157) ("html-tag-short-docs" t nil)) nil nil)"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}
