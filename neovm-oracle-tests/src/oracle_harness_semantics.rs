#[test]
fn load_root_normalization_precedes_project_root_normalization() {
    let load_root = crate::common::oracle_sandbox::OracleSandbox::create_fixture_tempdir()
        .expect("external oracle load root");
    let expect =
        expect_test::expect![[r#""OK (\"[ORACLE-LOAD-ROOT]\" \"[ORACLE-PROJECT-ROOT]\")""#]];

    crate::common::assert_oracle_parity_with_load_root_expect(
        r#"(list (getenv "NEOVM_ORACLE_LOAD_ROOT")
                  (getenv "NEOVM_ORACLE_PROJECT_ROOT"))"#,
        &[],
        load_root.path(),
        expect,
    );
}

#[test]
fn oracle_sandbox_keeps_case_files_under_workspace_tmp() {
    let expect = expect_test::expect![[r#""OK (t t)""#]];

    crate::common::assert_oracle_parity_with_shared_tempdir_expect(
        r#"(let ((scratch (file-name-as-directory
                          (getenv "NEOVM_ORACLE_SCRATCH_ROOT"))))
            (list (file-in-directory-p
                   (getenv "NEOVM_ORACLE_FORM_FILE") scratch)
                  (file-in-directory-p
                   (getenv "NEOVM_ORACLE_TEST_TMPDIR") scratch)))"#,
        expect,
    );
}

#[test]
fn oracle_sandbox_owns_child_tmpdir() {
    let expect = expect_test::expect![[r#""OK (t t)""#]];

    crate::common::assert_oracle_parity_with_env_expect(
        r#"(let ((scratch (file-name-as-directory
                          (getenv "NEOVM_ORACLE_SCRATCH_ROOT"))))
            (list (equal (file-name-as-directory (getenv "TMPDIR")) scratch)
                  (file-in-directory-p
                   (getenv "NEOVM_ORACLE_FORM_FILE") scratch)))"#,
        &[("TMPDIR", "/should-not-win")],
        expect,
    );
}

#[test]
fn oracle_sandbox_pins_snapshot_locale() {
    let expect = expect_test::expect![[r#""OK (\"en_US.UTF-8\" \"en_US.UTF-8\")""#]];

    crate::common::assert_oracle_parity_expect(
        r#"(list (getenv "LANG") (getenv "LC_ALL"))"#,
        expect,
    );
}
