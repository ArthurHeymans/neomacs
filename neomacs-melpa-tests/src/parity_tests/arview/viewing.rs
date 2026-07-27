use expect_test::expect;

use super::assert_arview_parity;

#[test]
fn arview_view_extracts_real_tar_tree_into_custom_dired_directory_and_cleans_on_kill() {
    let elisp_form = r##"(save-window-excursion
               (let* ((archive
                       (arview-test-create-tar))
                      (temp-root
                       (file-name-as-directory
                        (arview-test-path
                         "extract-root")))
                      buffer
                      directory
                      observation)
                 (make-directory temp-root t)
                 (unwind-protect
                     (progn
                       (arview-view
                        archive temp-root)
                       (setq buffer
                             (current-buffer)
                             directory
                             default-directory
                             observation
                             (list
                              major-mode
                              arview-buffer-p
                              (string-prefix-p
                               (concat
                                temp-root
                                "arview-fixture.tar.")
                               directory)
                              (file-directory-p
                               directory)
                              (arview-test-tree
                               directory)
                              (file-exists-p
                               archive)))
                       (kill-buffer buffer)
                       (list
                        observation
                        (buffer-live-p buffer)
                        (file-exists-p directory)
                        (file-exists-p archive)))
                   (when
                       (buffer-live-p buffer)
                     (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((dired-mode t t t (("alpha.txt" "alpha\nline two\n") ("nested/bravo λ.txt" "bravo \316\273\n") ("space name.txt" "space payload\n")) t) nil nil t)"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_view_uses_dynamically_configured_default_temporary_directory() {
    let elisp_form = r##"(save-window-excursion
               (let* ((archive
                       (arview-test-create-tar
                        "default-root.tar"))
                      (temporary-file-directory
                       (file-name-as-directory
                        (arview-test-path
                         "default-extract-root")))
                      buffer
                      directory)
                 (make-directory
                  temporary-file-directory t)
                 (unwind-protect
                     (progn
                       (arview-view archive)
                       (setq buffer
                             (current-buffer)
                             directory
                             default-directory)
                       (let ((result
                              (list
                               major-mode
                               arview-buffer-p
                               (string-prefix-p
                                (concat
                                 temporary-file-directory
                                 "arview-default-root.tar.")
                                directory)
                               (arview-test-tree
                                directory))))
                         (kill-buffer buffer)
                         (list
                          result
                          (file-exists-p directory)
                          (file-exists-p archive))))
                   (when
                       (buffer-live-p buffer)
                     (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((dired-mode t t (("alpha.txt" "alpha\nline two\n") ("nested/bravo λ.txt" "bravo \316\273\n") ("space name.txt" "space payload\n"))) nil t)"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_view_same_archive_twice_creates_unique_equivalent_directories() {
    let elisp_form = r##"(save-window-excursion
               (let* ((archive
                       (arview-test-create-tar
                        "repeat.tar"))
                      (temp-root
                       (file-name-as-directory
                        (arview-test-path
                         "repeat-root")))
                      first-buffer
                      second-buffer
                      first-directory
                      second-directory
                      first-tree
                      second-tree)
                 (make-directory temp-root t)
                 (unwind-protect
                     (progn
                       (arview-view
                        archive temp-root)
                       (setq first-buffer
                             (current-buffer)
                             first-directory
                             default-directory
                             first-tree
                             (arview-test-tree
                              default-directory))
                       (arview-view
                        archive temp-root)
                       (setq second-buffer
                             (current-buffer)
                             second-directory
                             default-directory
                             second-tree
                             (arview-test-tree
                              default-directory))
                       (let ((before
                              (list
                               (not
                                (equal
                                 first-directory
                                 second-directory))
                               (not
                                (eq
                                 first-buffer
                                 second-buffer))
                               (equal
                                first-tree
                                second-tree)
                               first-tree
                               second-tree)))
                         (kill-buffer first-buffer)
                         (kill-buffer second-buffer)
                         (list
                          before
                          (file-exists-p
                           first-directory)
                          (file-exists-p
                           second-directory)
                          (file-exists-p archive))))
                   (when
                       (buffer-live-p first-buffer)
                     (kill-buffer first-buffer))
                   (when
                       (buffer-live-p second-buffer)
                     (kill-buffer second-buffer)))))"##;
    let expect = expect![[
        r#"OK ((t t t (("alpha.txt" "alpha\nline two\n") ("nested/bravo λ.txt" "bravo \316\273\n") ("space name.txt" "space payload\n")) (("alpha.txt" "alpha\nline two\n") ("nested/bravo λ.txt" "bravo \316\273\n") ("space name.txt" "space payload\n"))) nil nil t)"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_view_extracts_archive_with_spaces_and_unicode_name() {
    let elisp_form = r##"(save-window-excursion
               (let* ((archive
                       (arview-test-create-tar
                        "資料 archive λ.tar"))
                      (temp-root
                       (file-name-as-directory
                        (arview-test-path
                         "unicode-root")))
                      buffer
                      directory)
                 (make-directory temp-root t)
                 (unwind-protect
                     (progn
                       (arview-view
                        archive temp-root)
                       (setq buffer
                             (current-buffer)
                             directory
                             default-directory)
                       (let ((result
                              (list
                               (file-name-nondirectory
                                archive)
                               (string-prefix-p
                                (concat
                                 temp-root
                                 "arview-資料 archive λ.tar.")
                                directory)
                               (arview-test-tree
                                directory)
                               arview-buffer-p)))
                         (kill-buffer buffer)
                         (list
                          result
                          (file-exists-p directory)
                          (file-exists-p archive))))
                   (when
                       (buffer-live-p buffer)
                     (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK (("資料 archive λ.tar" t (("alpha.txt" "alpha\nline two\n") ("nested/bravo λ.txt" "bravo \316\273\n") ("space name.txt" "space payload\n")) t) nil t)"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_view_clears_stale_log_before_successful_real_extraction() {
    let elisp_form = r##"(save-window-excursion
               (let* ((archive
                       (arview-test-create-tar
                        "clear-log.tar"))
                      (temp-root
                       (file-name-as-directory
                        (arview-test-path
                         "clear-log-root")))
                      (log
                       (get-buffer-create
                        arview-log-buffer-name))
                      buffer
                      directory)
                 (make-directory temp-root t)
                 (with-current-buffer log
                   (erase-buffer)
                   (insert
                    "stale output that must disappear"))
                 (unwind-protect
                     (progn
                       (arview-view
                        archive temp-root)
                       (setq buffer
                             (current-buffer)
                             directory
                             default-directory)
                       (let ((result
                              (list
                               (with-current-buffer log
                                 (buffer-string))
                               (arview-test-tree
                                directory)
                               (buffer-live-p log))))
                         (kill-buffer buffer)
                         result))
                   (when
                       (buffer-live-p buffer)
                     (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ("" (("alpha.txt" "alpha\nline two\n") ("nested/bravo λ.txt" "bravo \316\273\n") ("space name.txt" "space payload\n")) t)"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_view_unknown_type_signals_before_creating_log_or_directory() {
    let elisp_form = r##"(let* ((filename
                    (arview-test-path
                     "unsupported.unknown"))
                   (temp-root
                    (file-name-as-directory
                     (arview-test-path
                      "unknown-root")))
                   (arview-archive-type-functions
                    '(arview-file-extension))
                   (before
                    (directory-files
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT")
                     nil
                     "^[^.]"))
                   (log-before
                    (get-buffer
                     arview-log-buffer-name)))
               (arview-test-write-file
                filename "not an archive")
               (make-directory temp-root t)
               (list
                (condition-case error-data
                    (list
                     :ok
                     (arview-view
                      filename temp-root))
                  (error
                   (list
                    :error
                    (car error-data)
                    (cdr error-data))))
                before
                (directory-files
                 (getenv
                  "NEOMACS_TEST_SANDBOX_ROOT")
                 nil
                 "^[^.]")
                (eq
                 log-before
                 (get-buffer
                  arview-log-buffer-name))
                (directory-files
                 temp-root nil
                 "^[^.]")))"##;
    let expect = expect![[
        r#"OK ((:error error ("Unknown type of archive file: [ORACLE-SANDBOX]/unsupported.unknown")) ("home" "tmp" "xdg") ("home" "tmp" "unknown-root" "unsupported.unknown" "xdg") t nil)"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_view_invalid_real_tar_displays_log_and_keeps_empty_dired_until_killed() {
    let elisp_form = r##"(save-window-excursion
               (let* ((archive
                       (arview-test-path
                        "broken.tar"))
                      (temp-root
                       (file-name-as-directory
                        (arview-test-path
                         "broken-root")))
                      calls
                      buffer
                      directory
                      (original-display-buffer
                       (symbol-function
                        'display-buffer)))
                 (arview-test-write-file
                  archive
                  "this is not a tar archive")
                 (make-directory temp-root t)
                 (unwind-protect
                     (cl-letf
                         (((symbol-function
                           'display-buffer)
                           (lambda (&rest arguments)
                             (push arguments calls)
                             (apply
                              original-display-buffer
                              arguments))))
                       (arview-view
                        archive temp-root)
                       (setq buffer
                             (current-buffer)
                             directory
                             default-directory)
                       (let ((result
                              (list
                               major-mode
                               arview-buffer-p
                               (arview-test-tree
                                directory)
                               (mapcar
                                (lambda (arguments)
                                  (let ((name
                                         (buffer-name
                                          (car arguments))))
                                    (list
                                     (if
                                         (string-prefix-p
                                          "arview-broken.tar."
                                          name)
                                         "arview-broken.tar.<random>"
                                       name)
                                     (cadr arguments))))
                                (nreverse calls))
                               (> (buffer-size
                                   (get-buffer
                                    arview-log-buffer-name))
                                  0))))
                         (kill-buffer buffer)
                         (list
                          result
                          (file-exists-p directory)
                          (file-exists-p archive))))
                   (when
                       (buffer-live-p buffer)
                     (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((dired-mode t nil (("arview-broken.tar.<random>" (display-buffer-same-window (inhibit-same-window))) ("*arview-log*" t)) t) nil t)"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_view_additional_tar_arguments_surface_real_command_order_failure() {
    let elisp_form = r##"(save-window-excursion
               (let* ((archive
                       (arview-test-create-tar
                        "extra-args.tar"))
                      (temp-root
                       (file-name-as-directory
                        (arview-test-path
                         "extra-args-root")))
                      calls
                      buffer
                      directory
                      (original-display-buffer
                       (symbol-function
                        'display-buffer)))
                 (make-directory temp-root t)
                 (unwind-protect
                     (cl-letf
                         (((symbol-function
                           'display-buffer)
                           (lambda (&rest arguments)
                             (push arguments calls)
                             (apply
                              original-display-buffer
                              arguments))))
                       (arview-view
                        archive
                        temp-root
                        " --strip-components=1")
                       (setq buffer
                             (current-buffer)
                             directory
                             default-directory)
                       (let ((result
                              (list
                               (arview-test-tree
                                directory)
                               (length calls)
                               (> (buffer-size
                                   (get-buffer
                                    arview-log-buffer-name))
                                  0)
                               arview-buffer-p)))
                         (kill-buffer buffer)
                         (list
                          result
                          (file-exists-p directory)
                          (file-exists-p archive))))
                   (when
                       (buffer-live-p buffer)
                     (kill-buffer buffer)))))"##;
    let expect = expect!["OK ((nil 2 t t) nil t)"];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_view_custom_type_forwards_copy_process_and_revert_contract_exactly() {
    let elisp_form = r##"(with-temp-buffer
               (let ((arview-types
                      '((fixture
                         "fixture-extractor"
                         "--base")))
                     (temporary
                      (arview-test-path
                       "deterministic-view"))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'arview-archive-type)
                       (lambda (filename)
                         (push
                          (list
                           :type filename)
                          calls)
                         'fixture))
                      ((symbol-function
                        'arview-copy-remote-file)
                       (lambda (filename tempdir)
                         (push
                          (list
                           :copy filename
                           tempdir)
                          calls)
                         "/copied/archive.fixture"))
                      ((symbol-function
                        'make-temp-file)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           :temp arguments)
                          calls)
                         temporary))
                      ((symbol-function
                        'find-file)
                       (lambda (filename)
                         (push
                          (list
                           :find filename)
                          calls)
                         (setq default-directory
                               (file-name-as-directory
                                filename)
                               major-mode
                               'dired-mode)
                         :found))
                      ((symbol-function
                        'arview-process-file)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           :process arguments)
                          calls)
                         0))
                      ((symbol-function
                        'revert-buffer)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           :revert arguments)
                          calls)
                         :reverted)))
                   (let ((result
                          (arview-view
                           "/remote/input.fixture"
                           "/chosen/temp/"
                           " --extra")))
                     (setq arview-buffer-p nil)
                     (list
                      result
                      major-mode
                      default-directory
                      (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (:reverted dired-mode "[ORACLE-SANDBOX]/deterministic-view/" ((:type "/remote/input.fixture") (:copy "/remote/input.fixture" "/chosen/temp/") (:temp "arview-input.fixture." t) (:find "[ORACLE-SANDBOX]/deterministic-view") (:process "fixture-extractor" "--base --extra" "/copied/archive.fixture" (:buffer "*arview-log*")) (:revert)))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}
