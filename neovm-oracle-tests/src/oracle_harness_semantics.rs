#[test]
fn external_load_root_does_not_replace_the_project_root_domain() {
    let load_root = tempfile::Builder::new()
        .prefix("neovm-external-load-root-")
        .tempdir()
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
