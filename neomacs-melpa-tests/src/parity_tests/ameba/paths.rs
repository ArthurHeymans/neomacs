use expect_test::expect;

use super::assert_ameba_parity;

#[test]
fn local_and_tramp_file_names_map_to_the_paths_consumed_by_local_ameba() {
    let elisp_form = r##"(mapcar
                      (lambda (path)
                        (list
                         path
                         (tramp-tramp-file-p path)
                         (ameba-local-file-name path)))
                      '("/workspace/app/src/main.cr"
                        "/ssh:alice@example.test:/srv/app/src/main.cr"
                        "/sudo:root@localhost:/etc/crystal/check.cr"
                        "relative/source.cr"
                        ""))"##;
    let expect = expect![[
        r#"OK (("/workspace/app/src/main.cr" nil "/workspace/app/src/main.cr") ("/ssh:alice@example.test:/srv/app/src/main.cr" t "/srv/app/src/main.cr") ("/sudo:root@localhost:/etc/crystal/check.cr" t "/etc/crystal/check.cr") ("relative/source.cr" nil "relative/source.cr") ("" nil ""))"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn command_and_buffer_builders_preserve_real_paths_flags_and_shell_metacharacters_exactly() {
    let elisp_form = r##"(list
                      (ameba-buffer-name
                       "/workspace/customer app/src/main.cr")
                      (ameba-buffer-name
                       "/workspace/customer app/ !/workspace/customer app/lib")
                      (ameba-build-command
                       "ameba --format flycheck --config .ameba.yml"
                       "/workspace/customer app/src/main.cr")
                      (ameba-build-command
                       "bundle exec ameba"
                       "/workspace/app/ !/workspace/app/lib")
                      (ameba-build-command "" "")
                      (ameba-buffer-name ""))"##;
    let expect = expect![[
        r#"OK ("*Ameba /workspace/customer app/src/main.cr*" "*Ameba /workspace/customer app/ !/workspace/customer app/lib*" "ameba --format flycheck --config .ameba.yml /workspace/customer app/src/main.cr" "bundle exec ameba /workspace/app/ !/workspace/app/lib" " " "*Ameba *")"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn project_library_argument_uses_the_discovered_root_and_ameba_exclusion_syntax() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (project
                           (file-name-as-directory
                            (expand-file-name "crystal-app" sandbox)))
                          (nested
                           (file-name-as-directory
                            (expand-file-name "src/services" project)))
                          (default-directory nested)
                          (ameba-project-root-files
                           '("shard.yml")))
                      (make-directory nested t)
                      (with-temp-file
                          (expand-file-name "shard.yml" project)
                        (insert "name: crystal_app\n"))
                      (list
                       (file-relative-name
                        (ameba-project-root) sandbox)
                       (file-relative-name
                        (substring (ameba-project-lib) 1)
                        sandbox)
                       (substring (ameba-project-lib) 0 1)
                       (ameba-build-command
                        ameba-check-command
                        (concat
                         (ameba-project-root)
                         " "
                         (ameba-project-lib)))))"##;
    let expect = expect![[
        r#"OK ("crystal-app/" "crystal-app/lib" "!" "ameba --format flycheck [ORACLE-SANDBOX]/crystal-app/ ![ORACLE-SANDBOX]/crystal-app/lib")"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}
