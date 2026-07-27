use expect_test::expect;

use super::assert_ansible_parity;

#[test]
fn ansible_finds_project_root_from_deep_playbook_directory_within_limit() {
    let elisp_form = r##"(let ((root
                (ansible-test-make-project)))
         (unwind-protect
             (let* ((nested
                     (expand-file-name
                      "playbooks/services/api/tasks"
                      root))
                    (default-directory
                     (file-name-as-directory nested))
                    (ansible-dir-search-limit 5)
                    found)
               (make-directory nested t)
               (setq found
                     (ansible-find-root-path))
               (list
                (file-equal-p found root)
                (file-directory-p
                 (expand-file-name
                  "roles"
                  found))
                (file-name-absolute-p found)
                (string-suffix-p "/" found)))
           (delete-directory root t)))"##;
    let expect = expect!["OK (t t t nil)"];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_root_search_honors_boundary_and_missing_roles_cases() {
    let elisp_form = r##"(let ((root
                (make-temp-file
                 "ansible-root-limit-"
                 t)))
         (unwind-protect
             (let* ((nested
                     (expand-file-name
                      "one/two/three"
                      root))
                    (default-directory
                     (file-name-as-directory nested)))
               (make-directory nested t)
               (make-directory
                (expand-file-name "roles" root)
                t)
               (list
                (let ((ansible-dir-search-limit 2))
                  (ansible-find-root-path))
                (let ((ansible-dir-search-limit 3))
                  (file-equal-p
                   (ansible-find-root-path)
                   root))
                (progn
                  (delete-directory
                   (expand-file-name "roles" root))
                  (let ((ansible-dir-search-limit 8))
                    (ansible-find-root-path)))))
           (delete-directory root t)))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_lists_recursive_playbook_candidates_relative_to_project_root() {
    let elisp_form = r##"(let ((root
                (ansible-test-make-project)))
         (unwind-protect
             (let ((default-directory
                    (file-name-as-directory
                     (expand-file-name
                      "playbooks"
                      root)))
                   (ansible-root-path nil)
                   (ansible-dir-search-limit 4))
               (list
                (sort
                 (ansible-list-playbooks)
                 #'string<)
                (file-equal-p
                 ansible-root-path
                 root)))
           (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("notes/notayml.txt" "playbooks/deploy.yml" "playbooks/rollback.yml.backup" "site.yml") t)"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_update_root_path_replaces_on_success_and_retains_cached_root_on_failure() {
    let elisp_form = r##"(let ((root
                (ansible-test-make-project))
               (outside
                (make-temp-file
                 "ansible-outside-project-"
                 t)))
         (unwind-protect
             (let ((ansible-root-path nil)
                   successful-root
                   failed-root)
               (let ((default-directory
                      (file-name-as-directory
                       (expand-file-name
                        "playbooks"
                        root))))
                 (setq successful-root
                       (list
                        (ansible-update-root-path)
                        (file-equal-p
                         ansible-root-path
                         root))))
               (let ((default-directory
                      (file-name-as-directory outside))
                     (ansible-dir-search-limit 1))
                 (setq failed-root
                       (list
                        (ansible-update-root-path)
                        (file-equal-p
                         ansible-root-path
                         root))))
               (list
                successful-root
                failed-root))
           (delete-directory root t)
           (delete-directory outside t)))"##;
    let expect = expect!["OK ((t t) (t t))"];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_list_playbooks_returns_nil_without_current_or_cached_project() {
    let elisp_form = r##"(let ((outside
                (make-temp-file
                 "ansible-no-project-"
                 t)))
         (unwind-protect
             (let ((default-directory
                    (file-name-as-directory outside))
                   (ansible-root-path nil)
                   (ansible-dir-search-limit 2))
               (list
                (ansible-list-playbooks)
                ansible-root-path))
           (delete-directory outside t)))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_lint_command_is_buffer_local_and_preserves_file_names_with_spaces() {
    let elisp_form = r##"(let ((root
                (make-temp-file
                 "ansible-lint-project-"
                 t))
               first
               second)
         (unwind-protect
             (let ((playbook
                    (expand-file-name
                     "playbooks/release candidate.yml"
                     root)))
               (ansible-test-write-file
                root
                "playbooks/release candidate.yml"
                "---\n")
               (with-temp-buffer
                 (setq buffer-file-name playbook)
                 (ansible-lint-errors)
                 (setq first
                       (list
                        (local-variable-p
                         'compile-command)
                        (string-prefix-p
                         "LANG=C.UTF-8 ansible-lint "
                         compile-command)
                        (string-suffix-p
                         "playbooks/release candidate.yml"
                         compile-command))))
               (with-temp-buffer
                 (setq second
                       (list
                        (local-variable-p
                         'compile-command)
                        compile-command)))
               (list first second))
           (delete-directory root t)))"##;
    let expect = expect![[r#"OK ((t t t) (nil "make -k -j22 "))"#]];

    assert_ansible_parity(elisp_form, expect);
}
