use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};
#[test]
fn strict_org_entities_help_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn(require'org-entities)(list :help-fbound(fboundp'org-entities-help):total(length org-entities)))"##,
    );
}
#[test]
fn strict_org_protocol_uri() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn(condition-case nil(require'org-protocol)(error nil))(list:loaded(featurep'org-protocol):capture-fbound(fboundp'org-protocol-capture)))"##,
    );
}
#[test]
fn strict_org_mobile_files_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn(condition-case nil(require'org-mobile)(error nil))(list:files-bound(boundp'org-mobile-files):push-fbound(fboundp'org-mobile-push)))"##,
    );
}
#[test]
fn strict_org_babel_ob_ref_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn(require'ob-ref)(list:resolve-fbound(fboundp'org-babel-ref-resolve):parse-fbound(fboundp'org-babel-ref-parse)))"##,
    );
}
#[test]
fn strict_org_plot_script_generation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn(require'org-plot)(list:script-fbound(fboundp'org-plot/gnuplot-script):script-to-data-fbound(fboundp'org-plot/gnuplot-to-data)))"##,
    );
}
#[test]
fn strict_org_table_tab_first_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn(require'org)(list:tab-first-hook-bound(boundp'org-tab-first-hook):cycle-fbound(fboundp'org-cycle)))"##,
    );
}
#[test]
fn strict_org_list_search_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer(org-mode)(insert"- apple\n- banana\n- cherry\n")(goto-char(point-min))(search-forward"banana")(beginning-of-line)(list:at-item(org-at-item-p):item-bullet(org-list-bullet-string 1))))"##,
    );
}
#[test]
fn strict_org_export_title_date_author() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer(org-mode)(require'ox)(insert"#+TITLE: Test\n#+AUTHOR: X\n#+DATE: 2024-01\n")(let((info(org-export-get-environment)))(list:title(plist-get info:title):author(stringp(plist-get info:author)):date(stringp(plist-get info:date)))))"##,
    );
}
#[test]
fn strict_org_babel_execute_results_wrap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer(org-mode)(require'ob-emacs-lisp)(let((org-confirm-babel-evaluate nil))(insert"#+begin_src emacs-lisp :results value wrap\n'(a b c)\n#+end_src\n")(goto-char(point-min))(search-forward"#+begin_src")(org-babel-execute-src-block)(list:result-count(length(org-element-map(org-element-parse-buffer)'result'identity)))))"##,
    );
}
#[test]
fn strict_org_block_indent_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn(require'org)(list:indent-bound(boundp'org-adapt-indentation):edit-src-bound(boundp'org-src-preserve-indentation):content-bound(boundp'org-edit-src-content-indentation)))"##,
    );
}
