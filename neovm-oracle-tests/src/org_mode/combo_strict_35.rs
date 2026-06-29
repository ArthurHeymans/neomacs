use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};
#[test]
fn strict_org_archive_location() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn(require'org-archive)(list:location-bound(boundp'org-archive-location):save-bound(boundp'org-archive-save-context-info)))"##,
        expect_test::expect![[r#""ERR (void-function list:location-bound)""#]],
    );
}
#[test]
fn strict_org_image_file_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn(require'org)(list:image-bound(boundp'org-image-actual-width):redisplay-fbound(fboundp'org-display-inline-images)))"##,
        expect_test::expect![[r#""ERR (void-function list:image-bound)""#]],
    );
}
#[test]
fn strict_org_lint_invalid_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer(org-mode)(require'org-lint)(insert"#+BEGIN_OLD_BLOCK\nold\n#+END_OLD_BLOCK\n")(let((r()))(condition-case nil(let((reports(org-lint)))(push(length reports)r))(error(push:err r)))(nreverse r)))"##,
        expect_test::expect![[r#""OK (0)""#]],
    );
}
#[test]
fn strict_org_occur_by_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer(org-mode)(insert"* [#A] Top\n* [#B] Mid\n* [#A] Hi\n")(goto-char(point-min))(condition-case nil(org-occur"\\[#A\\]")(error nil))(list:matched(length(org-element-map(org-element-parse-buffer nil t)'headline'identity)))(org-remove-occur-highlights))"##,
        expect_test::expect![[r#""ERR (void-function list:matched)""#]],
    );
}
#[test]
fn strict_org_babel_noweb_evaluation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn(require'ob-core)(list:eval-fbound(fboundp'org-babel-expand-noweb-references):strip-fbound(fboundp'org-babel-strip-noweb)))"##,
        expect_test::expect![[r#""ERR (void-function list:eval-fbound)""#]],
    );
}
#[test]
fn strict_org_export_template_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer(org-mode)(require'ox)(insert"#+SETUPFILE: \n")(goto-char(point-min))(list:setupfile(condition-case nil(org-export-get-environment)(error:err))))"##,
        expect_test::expect![[r#""ERR (void-function list:setupfile)""#]],
    );
}
#[test]
fn strict_org_checkbox_hierarchical() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer(org-mode)(insert"* Tasks[/]\n- [-] Parent[/]\n  - [X] C1\n  - [ ] C2\n")(goto-char(point-min))(org-update-statistics-cookies t)(let*((t(org-element-parse-buffer))(its(org-element-map t'item'identity)))(list:checkboxes(mapcar(lambda(i)(org-element-property:checkbox i))its):count(length its))))"##,
        expect_test::expect![[r#""ERR (setting-constant t)""#]],
    );
}
#[test]
fn strict_org_timestamp_reformat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn(require'org)(with-temp-buffer(org-mode)(insert"<2024-06-15 Sat 10:30>\n")(goto-char(point-min))(condition-case nil(org-timestamp-change 1'day)(error nil))(list:after(buffer-string))))"##,
        expect_test::expect![[r#""ERR (void-function list:after)""#]],
    );
}
#[test]
fn strict_org_table_create_from_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer(org-mode)(insert"a b c\n1 2 3\n")(goto-char(point-min))(org-table-create-or-convert-from-region(point-min)(point-max))(list:tablep(org-at-table-p):to-lisp(org-table-to-lisp)))"##,
        expect_test::expect![[r#""ERR (wrong-number-of-arguments (1 . 1) 2)""#]],
    );
}
#[test]
fn strict_org_insert_heading_with_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer(org-mode)(insert"* H\nBody.")(goto-char(point-max))(org-insert-heading nil t)(insert"New")(list:heads(mapcar(lambda(h)(substring-no-properties(org-element-property:raw-value h)))(org-element-map(org-element-parse-buffer)'headline'identity))))"##,
        expect_test::expect![[r#""ERR (void-function list:heads)""#]],
    );
}
