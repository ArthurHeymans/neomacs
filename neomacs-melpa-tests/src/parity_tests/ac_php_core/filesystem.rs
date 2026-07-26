use expect_test::expect;

use super::assert_ac_php_core_parity;

#[test]
fn ac_php_core_find_php_files_recurses_filters_hidden_vendor_tests_and_extensions() {
    let elisp_form = r##"(let ((root
                    (make-temp-file
                     (expand-file-name
                      "ac-php-core-files-"
                      (getenv
                       "TMPDIR"))
                     t)))
               (unwind-protect
                   (progn
                     (dolist
                         (directory
                          '("src"
                            "src/nested"
                            ".hidden"
                            "vendor/pkg/tests"
                            "vendor/pkg/src"))
                       (make-directory
                        (expand-file-name
                         directory
                         root)
                        t))
                     (dolist
                         (file
                          '("root.php"
                            "root.txt"
                            "src/one.php"
                            "src/nested/two.inc"
                            "src/nested/three.php"
                            ".hidden/hidden.php"
                            "vendor/pkg/tests/skipped.php"
                            "vendor/pkg/src/included.php"))
                       (with-temp-file
                           (expand-file-name
                            file
                            root)
                         (insert
                          file)))
                     (mapcar
                      (lambda (fixture)
                        (let ((found
                               (ac-php-find-php-files
                                root
                                (car fixture)
                                (cdr fixture))))
                          (sort
                           (mapcar
                            (lambda (item)
                              (list
                               (file-relative-name
                                (car item)
                                root)
                               (integerp
                                (cadr item))))
                            found)
                           (lambda (left right)
                             (string-lessp
                              (car left)
                              (car right))))))
                      '(("\\.php$" . nil)
                        ("\\.php$" . t)
                        ("\\.\\(?:php\\|inc\\)$" . t))))
                 (delete-directory
                  root t)))"##;
    let expect = expect![[
        r#"OK ((("root.php" t)) (("root.php" t) ("src/nested/three.php" t) ("src/one.php" t) ("vendor/pkg/src/included.php" t) ("vendor/pkg/tests/skipped.php" t)) (("root.php" t) ("src/nested/three.php" t) ("src/nested/two.inc" t) ("src/one.php" t) ("vendor/pkg/src/included.php" t) ("vendor/pkg/tests/skipped.php" t)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_json_config_creation_readback_cache_and_pretty_print_restoration_match() {
    let elisp_form = r##"(let ((root
                    (file-name-as-directory
                     (make-temp-file
                      (expand-file-name
                       "ac-php-core-config-"
                       (getenv
                        "TMPDIR"))
                      t)))
                   (original-pretty
                    json-encoding-pretty-print))
               (unwind-protect
                   (let* ((config-path
                           (expand-file-name
                            ac-php-config-file
                            root))
                          (created
                           (ac-php--get-config
                            root))
                          (created-text
                           (with-temp-buffer
                             (insert-file-contents
                              config-path)
                             (buffer-string)))
                          (cache-path
                           (expand-file-name
                            "cache.json"
                            root))
                          (cache-return
                           (ac-php--cache-files-save
                            cache-path
                            '(("one.php" 1)
                              ("two.php" 2))))
                          (cache-data
                           (json-read-file
                            cache-path)))
                     (with-temp-file
                         config-path
                       (insert
                        "{\"use-cscope\":true,\"tag-dir\":\"./custom-tags\",\"filter\":{\"php-file-ext-list\":[\"php\",\"inc\"]}}"))
                     (let ((custom
                            (ac-php--get-config
                             root)))
                       (list
                        created
                        (string-match-p
                         "\n"
                         created-text)
                        cache-return
                        cache-data
                        custom
                        (eq
                         json-encoding-pretty-print
                         original-pretty))))
                 (setq
                  json-encoding-pretty-print
                  original-pretty)
                 (delete-directory
                  root t)))"##;
    let expect = expect![[
        r##"OK (((use-cscope) (tag-dir) (filter (php-file-ext-list . ["php"]) (php-path-list . ["."]) (ignore-ruleset . ["# like .gitignore file " "/vendor/**/[tT]ests/**/*.php" "/vendor/**/[Ee]xamples/**/*.php" "/vendor/composer/*.php" "/vendor/*.php" "# not need php_codesniffer" "/vendor/squizlabs/php_codesniffer/**/*.php" "#  -- end -- "]))) 1 nil ((cache1-files (one.php . [1]) (two.php . [2]))) ((use-cscope . t) (tag-dir . "./custom-tags") (filter (php-file-ext-list . ["php" "inc"]))) t)"##
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_ctags_options_config_path_notices_and_cscope_setting_match() {
    let elisp_form = r##"(let ((ac-php-config-file
                    ".fixture.json")
                   (ac-php-tags-path
                    "/cache/tags")
                   (project
                    "/workspace/project/"))
               (list
                (let ((ac-php-project-root-dir-use-truename
                       t))
                  (list
                   (ac-php--ctags-opts
                    project nil)
                   (ac-php--ctags-opts
                    project t)))
                (let ((ac-php-project-root-dir-use-truename
                       nil))
                  (ac-php--ctags-opts
                   project nil))
                (mapcar
                 (lambda (path)
                   (ac-php--get-config-path-noti-str
                    project
                    path))
                 '("/workspace/project/src"
                   "/workspace/project/src/*.php"))
                (cl-letf
                    (((symbol-function
                       'ac-php--get-config)
                      (lambda (_root)
                        '(("use-cscope"
                           . t)
                          ("tag-dir"
                           . nil)))))
                  (ac-php--get-use-cscope-from-config-file
                   project))))"##;
    let expect = expect![[
        r#"OK ((("--config-file=/workspace/project/.fixture.json" "--tags_dir=/cache/tags" "--rebuild=no" "--realpath_flag=yes") ("--config-file=/workspace/project/.fixture.json" "--tags_dir=/cache/tags" "--rebuild=yes" "--realpath_flag=yes")) ("--config-file=/workspace/project/.fixture.json" "--tags_dir=/cache/tags" "--rebuild=no" "--realpath_flag=no") ("php-path-list->src" "php-path-list-without-subdir->src") t)"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_tags_save_directory_honors_custom_relative_and_generated_paths() {
    let elisp_form = r##"(let ((root
                    (file-name-as-directory
                     (make-temp-file
                      (expand-file-name
                       "ac-php-core-tags-dir-"
                       (getenv
                        "TMPDIR"))
                      t))))
               (unwind-protect
                   (let* ((project
                           (expand-file-name
                            "project/"
                            root))
                          (cache
                           (expand-file-name
                            "cache"
                            root))
                          custom
                          generated)
                     (make-directory
                      project t)
                     (make-directory
                      (expand-file-name
                       "relative-tags"
                       project)
                      t)
                     (let ((ac-php-tags-path
                            cache))
                       (cl-letf
                           (((symbol-function
                              'ac-php--get-config)
                             (lambda (_root)
                               '(("tag-dir"
                                  . "./relative-tags")))))
                         (setq
                          custom
                          (ac-php--get-tags-save-dir
                           project)))
                       (cl-letf
                           (((symbol-function
                              'ac-php--get-config)
                             (lambda (_root)
                               '(("tag-dir"
                                  . nil)))))
                         (setq
                          generated
                          (ac-php--get-tags-save-dir
                           project))))
                     (list
                      (file-relative-name
                       custom
                       root)
                     (file-directory-p
                       custom)
                      (let* ((project-key
                              (replace-regexp-in-string
                               (regexp-quote
                                "/")
                               "-"
                               (replace-regexp-in-string
                                "[/\\]*$"
                                ""
                                project)))
                             (normalized
                              (replace-regexp-in-string
                               (regexp-quote
                                (directory-file-name
                                 root))
                               "[ROOT]"
                               generated t t)))
                        (replace-regexp-in-string
                         (regexp-quote
                          project-key)
                         "[PROJECT-KEY]"
                         normalized t t))
                      (file-directory-p
                       generated)))
                 (delete-directory
                  root t)))"##;
    let expect =
        expect![[r#"OK ("project/relative-tags/" t "[ROOT]/cache/tags[PROJECT-KEY]/" t)"#]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_project_root_resolution_handles_config_projectile_vendor_and_missing_markers() {
    let elisp_form = r##"(let ((root
                    (file-name-as-directory
                     (make-temp-file
                      (expand-file-name
                       "ac-php-core-roots-"
                       (getenv
                        "TMPDIR"))
                      t))))
               (unwind-protect
                   (let (results calls)
                     (dolist
                         (fixture
                          '((config
                             ".ac-php-conf.json")
                            (projectile
                             ".projectile")
                            (vendor
                             "vendor/autoload.php")))
                       (let* ((name
                               (symbol-name
                                (car fixture)))
                              (project
                               (expand-file-name
                                (concat
                                 name
                                 "/")
                                root))
                              (nested
                               (expand-file-name
                                "src/deep/"
                                project))
                              (marker
                               (expand-file-name
                                (cadr fixture)
                                project)))
                         (make-directory
                          nested t)
                         (make-directory
                          (file-name-directory
                           marker)
                          t)
                         (with-temp-file
                             marker)
                         (let ((buffer-file-name
                                (expand-file-name
                                 "file.php"
                                 nested))
                               (ac-php-project-root-dir-use-truename
                                t))
                           (push
                            (list
                             (car fixture)
                             (file-relative-name
                              (ac-php--get-project-root-dir)
                              root))
                            results))))
                     (let* ((project
                             (expand-file-name
                              "logical/"
                              root))
                            (nested
                             (expand-file-name
                              "src/"
                              project)))
                       (make-directory
                        nested t)
                       (with-temp-file
                           (expand-file-name
                            ".projectile"
                            project))
                       (let ((buffer-file-name
                              nil)
                             (default-directory
                              nested)
                             (ac-php-project-root-dir-use-truename
                              nil))
                         (push
                          (list
                           'default-directory
                           (file-relative-name
                            (ac-php--get-project-root-dir)
                            root))
                          results)))
                     (cl-letf
                         (((symbol-function
                            'message)
                           (lambda (&rest arguments)
                             (push arguments calls)
                             'messaged)))
                       (let ((buffer-file-name
                              (expand-file-name
                               "unmarked/file.php"
                               root))
                             (ac-php-project-root-dir-use-truename
                              t))
                         (make-directory
                          (file-name-directory
                           buffer-file-name)
                          t)
                         (push
                          (list
                           'missing
                           (ac-php--get-project-root-dir))
                          results)))
                     (list
                      (nreverse results)
                      (nreverse calls)))
                 (delete-directory
                  root t)))"##;
    let expect = expect![[
        r#"OK (((config "config/") (projectile "projectile/") (vendor "vendor/") (default-directory "logical/") (missing nil)) (("ac-php: Unable to resolve project root")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_save_and_load_tag_data_build_case_fold_maps_cache_and_file_vectors() {
    let elisp_form = r##"(let ((root
                    (file-name-as-directory
                     (make-temp-file
                      (expand-file-name
                       "ac-php-core-load-"
                       (getenv
                        "TMPDIR"))
                      t))))
               (unwind-protect
                   (let* ((tags-file
                           (expand-file-name
                            "tags.el"
                            root))
                          (saved-file
                           (expand-file-name
                            "saved.el"
                            root))
                          (ac-php-tag-last-data-list
                           nil)
                          first
                          second)
                     (with-temp-file
                         tags-file
                       (insert
                        "(setq g-ac-php-tmp-tags\n      [[(\"\\\\App\\\\Child\" . [[\"p\" \"value\" \"docs\" \"0:1\" \"string\" \"\\\\App\\\\Child\" \"public\" \"0\"]])]\n       [[\"f\" \"\\\\App\\\\run(\" \"$arg\" \"0:2\" \"Result\"]]\n       [(\"\\\\App\\\\Child\" . [\"\\\\App\\\\Base\"])]\n       [\"/project/one.php\"]])\n"))
                     (setq
                      first
                      (ac-php-load-data
                       tags-file nil
                       "/project/"))
                     (setq
                      second
                      (ac-php-load-data
                       tags-file nil
                       "/project/"))
                     (let* ((circular
                             (list
                              'root))
                            save-return
                            saved-text)
                       (setcdr
                        circular
                        circular)
                       (setq
                        save-return
                        (ac-php-save-data
                         saved-file
                         circular))
                       (setq
                        saved-text
                        (with-temp-buffer
                          (insert-file-contents
                           saved-file)
                          (buffer-string)))
                       (list
                        (eq first second)
                        (hash-table-test
                         (ac-php-g--class-map
                          first))
                        (gethash
                         "\\app\\CHILD"
                         (ac-php-g--class-map
                          first))
                        (gethash
                         "\\APP\\RUN("
                         (ac-php-g--function-map
                          first))
                        (gethash
                         "\\app\\child"
                         (ac-php-g--inherit-map
                          first))
                        (ac-php-g--file-list
                         first)
                        (ac-php-g--project-root-dir
                         first)
                        (length
                         ac-php-tag-last-data-list)
                        save-return
                        saved-text)))
                 (delete-directory
                  root t)))"##;
    let expect = expect![[
        r##"OK (t case-fold [["p" "value" "docs" "0:1" "string" "\\App\\Child" "public" "0"]] ["f" "\\App\\run(" "$arg" "0:2" "Result"] ["\\App\\Base"] ["/project/one.php"] "/project/" 1 #1=(root . #1#) "#1=(root . #1#)")"##
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_get_tags_file_checks_age_and_returns_project_and_cache_paths() {
    let elisp_form = r##"(let ((ac-php-auto-update-intval
                    100)
                   (project
                    "/project/")
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php--get-project-root-dir)
                     (lambda ()
                       project))
                    ((symbol-function
                      'ac-php--get-tags-save-dir)
                     (lambda (root)
                       (push
                        (list
                         'save-dir
                         root)
                        calls)
                       "/cache/"))
                    ((symbol-function
                      'file-attributes)
                     (lambda (_file)
                       '(nil nil nil nil nil
                             (0 10))))
                    ((symbol-function
                      'current-time)
                     (lambda ()
                       '(0 200)))
                    ((symbol-function
                      'ac-php--remake-tags)
                     (lambda
                         (root force)
                       (push
                        (list
                         'remake
                         root force)
                        calls)
                       'remade)))
                 (let ((stale
                        (ac-php-get-tags-file)))
                   (setq
                    ac-php-auto-update-intval
                    1000)
                   (let ((fresh
                          (ac-php-get-tags-file)))
                     (setq
                      project
                      nil)
                     (let ((missing
                            (ac-php-get-tags-file)))
                       (list
                        stale
                        fresh
                        missing
                        (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (("/project/" "/cache/tags.el" "/cache/tags-vendor.el") ("/project/" "/cache/tags.el" "/cache/tags-vendor.el") nil ((save-dir "/project/") (remake "/project/" nil) (save-dir "/project/")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_get_tags_data_selects_project_common_generation_load_and_remake_paths() {
    let elisp_form = r##"(let ((definition
                    '("/project/"
                      "/cache/tags.el"
                      "/cache/vendor.el"))
                   (existing
                    '("/cache/tags.el"
                      "/cache/vendor.el"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php-get-tags-file)
                     (lambda ()
                       definition))
                    ((symbol-function
                      'ac-php--get-common-json-file)
                     (lambda ()
                       "/cache/common.el"))
                    ((symbol-function
                      'f-exists?)
                     (lambda (path)
                       (member path
                               existing)))
                    ((symbol-function
                      'file-exists-p)
                     (lambda (path)
                       (member path
                               existing)))
                    ((symbol-function
                      'shell-command-to-string)
                     (lambda (command)
                       (push
                        (list
                         'shell command)
                        calls)
                       "generated"))
                    ((symbol-function
                      'ac-php-load-data)
                     (lambda
                         (tags vendor root)
                       (push
                        (list
                         'load tags vendor root)
                        calls)
                       'loaded))
                    ((symbol-function
                      'ac-php-remake-tags)
                     (lambda ()
                       (push
                        '(remake)
                        calls)
                       'remade)))
                 (let ((project-result
                        (ac-php-get-tags-data)))
                   (setq
                    definition
                    nil
                    existing
                    '("/cache/common.el"))
                   (let ((common-existing
                          (ac-php-get-tags-data)))
                     (setq
                      existing
                      nil)
                     (let ((common-generated
                            (ac-php-get-tags-data)))
                       (list
                        project-result
                        common-existing
                        common-generated
                        (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (loaded loaded remade ((load "/cache/tags.el" "/cache/vendor.el" "/project/") (load "/cache/common.el" nil nil) (shell " [ORACLE-WORKSPACE]/tmp/melpa/package-cache/ac-php-core/20260210.846/home/.emacs.d/elpa/ac-php-core-20260210.846/phpctags --save-common-el=/cache/common.el") (remake)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_remote_config_paths_are_read_without_local_creation_or_size_probes() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'f-exists?)
                     (lambda (path)
                       (push
                        (list
                         'exists path)
                        calls)
                       nil))
                    ((symbol-function
                      'f-size)
                     (lambda (path)
                       (push
                        (list
                         'size path)
                        calls)
                       0))
                    ((symbol-function
                      'ac-php--json-save-data)
                     (lambda
                         (path data)
                       (push
                        (list
                         'save path data)
                        calls)
                       'saved))
                    ((symbol-function
                      'json-read-file)
                     (lambda (path)
                       (push
                        (list
                         'read path)
                        calls)
                       (list
                        (cons
                         'path path)))))
                 (list
                  (ac-php--get-config
                   "/ssh:host:/project/")
                  (ac-php--get-config
                   "/server:host:/project/")
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (((path . "/ssh:host:/project/.ac-php-conf.json")) ((path . "/server:host:/project/.ac-php-conf.json")) ((read "/ssh:host:/project/.ac-php-conf.json") (read "/server:host:/project/.ac-php-conf.json")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_object_tag_directory_uses_user_creates_tree_and_lists_only_elisp_recursively() {
    let elisp_form = r##"(let ((root
                    (file-name-as-directory
                     (make-temp-file
                      (expand-file-name
                       "ac-php-core-object-tags-"
                       (getenv
                        "TMPDIR"))
                      t)))
                   (process-environment
                    (copy-sequence
                     process-environment)))
               (unwind-protect
                   (progn
                     (setenv
                      "USER"
                      "oracle-user")
                     (let* ((object-dir
                             (ac-php--get-obj-tags-dir
                              root))
                            (before
                             (file-directory-p
                              object-dir))
                            (empty
                             (ac-php--get-obj-tags-file-list
                              root)))
                       (make-directory
                        (expand-file-name
                         "nested"
                         object-dir)
                        t)
                       (dolist
                           (fixture
                            '(("one.el"
                               . "one")
                              ("nested/two.el"
                               . "two")
                              ("nested/no.elc"
                               . "compiled")
                              ("nested/no.php"
                               . "php")))
                         (with-temp-file
                             (expand-file-name
                              (car fixture)
                              object-dir)
                           (insert
                            (cdr fixture))))
                       (list
                        (file-relative-name
                         object-dir
                         root)
                        before
                        (file-directory-p
                         object-dir)
                        empty
                        (sort
                         (mapcar
                          (lambda (item)
                            (file-relative-name
                             (car item)
                             object-dir))
                          (ac-php--get-obj-tags-file-list
                           root))
                         #'string<))))
                 (delete-directory
                  root t)))"##;
    let expect = expect![[r#"OK ("tags_dir_oracle-user/" nil t nil ("nested/two.el" "one.el"))"#]];

    assert_ac_php_core_parity(elisp_form, expect);
}
