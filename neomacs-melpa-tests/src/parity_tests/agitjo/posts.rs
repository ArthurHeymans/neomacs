use expect_test::expect;

use super::assert_agitjo_parity;

#[test]
fn agitjo_description_sanitization_round_trips_ascii_newlines_unicode_and_nul() {
    let elisp_form = r##"(mapcar
         (lambda (entry)
           (let* ((text (car entry))
                  (coding (cadr entry))
                  (encoded (agitjo--sanitize-description text coding))
                  (payload (substring encoded (length "{base64}"))))
             (list
              encoded
              (decode-coding-string
               (base64-decode-string payload)
               coding))))
         '(("Title\n\nBody with spaces" utf-8)
           ("λ 日本語 café" utf-8)
           ("a\0b" no-conversion)
           ("" utf-8)))"##;
    let expect = expect![[
        r#"OK (("{base64}VGl0bGUKCkJvZHkgd2l0aCBzcGFjZXM=" "Title\n\nBody with spaces") ("{base64}zrsg5pel5pys6KqeIGNhZsOp" "λ 日本語 café") ("{base64}YQBi" "a\0b") ("{base64}" ""))"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_front_matter_parser_returns_exact_end_and_preserves_non_front_matter() {
    let elisp_form = r##"(mapcar
         (lambda (text)
           (with-temp-buffer
             (insert text)
             (goto-char (point-min))
             (let ((position (agitjo-post--point-after-front-matter)))
               (list position
                     (and position
                          (buffer-substring-no-properties
                           position (point-max)))))))
         '("---\ntitle: Pull request\nlabels: docs\n---\nDescribe changes\n"
           "---\ntitle: Missing end\nDescribe changes\n"
           "# Ordinary markdown\n---\nBody\n"
           ""))"##;
    let expect = expect![[
        r#"OK ((41 "\nDescribe changes\n") (4 "\ntitle: Missing end\nDescribe changes\n") (nil nil) (nil nil))"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_template_object_search_follows_directory_file_and_remote_precedence() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'magit-primary-remote)
                    (lambda () "origin"))
                   ((symbol-function 'magit-main-branch)
                    (lambda () "main"))
                   ((symbol-function 'magit-git-items)
                    (lambda (&rest args)
                      (push args calls)
                      '(".gitea/pull_request_template.md"
                        ".github/PULL_REQUEST_TEMPLATE.md"))))
           (list
            (agitjo-post--find-pullreq-template-object)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("origin/main:.gitea/pull_request_template.md" (("ls-tree" "-z" "-r" "--full-tree" "--name-only" "origin/main" "--" (".forgejo/PULL_REQUEST_TEMPLATE.md" ".forgejo/pull_request_template.md" ".gitea/PULL_REQUEST_TEMPLATE.md" ".gitea/pull_request_template.md" ".github/PULL_REQUEST_TEMPLATE.md" ".github/pull_request_template.md"))))"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_template_object_search_falls_back_to_head_and_nil_when_absent() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'magit-primary-remote)
                    (lambda () nil))
                   ((symbol-function 'magit-main-branch)
                    (lambda () nil))
                   ((symbol-function 'magit-git-items)
                    (lambda (&rest args)
                      (push args calls)
                      nil)))
           (list
            (agitjo-post--find-pullreq-template-object)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (nil (("ls-tree" "-z" "-r" "--full-tree" "--name-only" "HEAD" "--" (".forgejo/PULL_REQUEST_TEMPLATE.md" ".forgejo/pull_request_template.md" ".gitea/PULL_REQUEST_TEMPLATE.md" ".gitea/pull_request_template.md" ".github/PULL_REQUEST_TEMPLATE.md" ".github/pull_request_template.md"))))"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_replace_template_strips_yaml_front_matter_and_keeps_markdown_body() {
    let elisp_form = r##"(with-temp-buffer
         (cl-letf (((symbol-function 'agitjo-post--find-pullreq-template-object)
                    (lambda () "origin/main:.forgejo/PULL_REQUEST_TEMPLATE.md"))
                   ((symbol-function 'agitjo-post--insert-git-object-contents)
                    (lambda (object)
                      (insert "---\ntitle: ignored\n---\n# Summary\n\nChecklist\n")
                      object)))
           (list
            (agitjo-post--replace-buffer-with-pullreq-template)
            (buffer-string)
            (point))))"##;
    let expect = expect![[r#"OK (t "\n# Summary\n\nChecklist\n" 1)"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_new_description_combines_commit_body_and_template_with_separator() {
    let elisp_form = r##"(with-temp-buffer
         (let* ((default-directory
                 (file-name-as-directory
                  (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
                (config
                (agitjo--pullreq-configuration
                 :type "for"
                 :source "@"
                 :target "origin/main"
                 :args nil)))
           (cl-letf (((symbol-function 'agitjo-post--replace-buffer-with-pullreq-template)
                      (lambda ()
                        (erase-buffer)
                        (insert "# Template\n\n- [ ] Tests")
                        t))
                     ((symbol-function 'agitjo-post--insert-source-head-commit-body)
                      (lambda (_config)
                        (insert "Explain the implementation")
                        t)))
             (agitjo-post--replace-buffer-with-new-description config)
             (list (buffer-string) (point)))))"##;
    let expect =
        expect![[r#"OK ("Explain the implementation\n\n-----\n\n# Template\n\n- [ ] Tests" 36)"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_source_commit_body_insertion_distinguishes_blank_and_multiline_messages() {
    let elisp_form = r##"(let* ((default-directory
                  (file-name-as-directory
                   (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
                 (config
                (agitjo--pullreq-configuration
                 :type "for"
                 :source "@"
                 :target "origin/main"
                 :args nil)))
         (list
          (with-temp-buffer
            (cl-letf (((symbol-function 'magit-rev-insert-format)
                       (lambda (format revision)
                         (insert "\n\n")
                         (list format revision))))
              (list
               (agitjo-post--insert-source-head-commit-body config)
               (buffer-string))))
          (with-temp-buffer
            (cl-letf (((symbol-function 'magit-rev-insert-format)
                       (lambda (format revision)
                         (insert "First paragraph\n\nDetails\n")
                         (list format revision))))
              (list
               (agitjo-post--insert-source-head-commit-body config)
               (buffer-string))))))"##;
    let expect = expect![[r#"OK ((nil "") (t "First paragraph\n\nDetails"))"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_post_buffer_uses_real_deterministic_gitdir_path_and_reuses_visiting_buffer() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agitjo-post-buffer"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (gitdir (expand-file-name ".git/" root))
                first second)
         (unwind-protect
             (progn
               (make-directory gitdir t)
               (cl-letf (((symbol-function 'magit-gitdir)
                          (lambda () gitdir)))
                 (setq first (agitjo-post--buffer)
                       second (agitjo-post--buffer))
                 (with-current-buffer first
                   (insert "draft")
                   (save-buffer))
                 (list
                  (eq first second)
                  (buffer-live-p first)
                  (with-current-buffer first
                    (list
                     (buffer-file-name)
                     (buffer-string)))
                  (file-exists-p
                   (expand-file-name
                    agitjo-post--draft-file-name gitdir)))))
           (when (buffer-live-p first) (kill-buffer first))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (t t ("[ORACLE-SANDBOX]/agitjo-post-buffer/.git/agitjo/pullreq-draft" "draft") t)"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}
