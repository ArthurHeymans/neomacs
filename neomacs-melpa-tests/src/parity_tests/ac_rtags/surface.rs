use expect_test::expect;

use super::{assert_ac_rtags_autoload_parity, assert_ac_rtags_parity};

#[test]
fn ac_rtags_constant_hook_and_source_metadata_match() {
    let elisp_form = r##"(list
               (list
                rtags-location-regx
                (get
                 'rtags-location-regx
                 'variable-documentation)
                (get
                 'rtags-location-regx
                 'standard-value))
               (copy-tree
                ac-source-rtags)
               (cl-count
                'ac-rtags-completions-hook
                rtags-completions-hook))"##;
    let expect = expect![[
        r#"OK (("\\([^:]*\\):\\([0-9]*\\):\\([0-9]*\\)" nil nil) ((init . ac-rtags-init) (prefix . ac-rtags-prefix) (candidates . ac-rtags-candidates) (action . ac-rtags-action) (document . ac-rtags-document) (requires . 0) (symbol . "r")) 1)"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_packaged_source_descriptor_and_autoload_assets_have_exact_hashes() {
    let elisp_form = r##"(let ((root
                    (file-name-directory
                     (symbol-file
                      'ac-rtags-action
                      'defun))))
               (mapcar
                (lambda (file)
                  (with-temp-buffer
                    (insert-file-contents-literally
                     (expand-file-name
                      file
                      root))
                    (list
                     file
                     (buffer-size)
                     (secure-hash
                      'sha256
                      (current-buffer)))))
                '("ac-rtags.el"
                  "ac-rtags-pkg.el"
                  "ac-rtags-autoloads.el")))"##;
    let expect = expect![[
        r#"OK (("ac-rtags.el" 5701 "07c2b58f4887c4bbd8587222f66a71058ba3bfc4d514ded81100c9c8201a8160") ("ac-rtags-pkg.el" 547 "6e34e26ce830ae05452c58194cab0db16b7aa98cd903dea1b0d046bfc214d0df") ("ac-rtags-autoloads.el" 705 "13b910a787bb68d8b902297fde06b5d93fb7cc2dae18a106547bd2e3879b0217"))"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_registers_prefixes_without_loading_runtime_state() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (boundp
                'ac-source-rtags)
               (boundp
                'rtags-location-regx)
               (boundp
                'ac-rtags-expand-functions)
               (get
                'ac-rtags
                'custom-loads)
               (list
                (gethash
                 "ac-rtags-"
                 definition-prefixes)
                (gethash
                 "rtags-location-regx"
                 definition-prefixes)))"##;
    let expect = expect![[
        r#"OK (nil t nil nil nil nil (("ac-rtags" "ac-rtags") ("ac-rtags" "ac-rtags")))"#
    ]];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_does_not_define_candidates() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (fboundp
                'ac-rtags-candidates))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_does_not_define_action() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (fboundp
                'ac-rtags-action))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_does_not_define_prefix() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (fboundp
                'ac-rtags-prefix))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_does_not_define_completions_hook() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (fboundp
                'ac-rtags-completions-hook))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_does_not_define_trim_whitespace() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (fboundp
                'ac-rtags-trim-leading-trailing-whitespace))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_does_not_define_document() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (fboundp
                'ac-rtags-document))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_does_not_define_action_function() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (fboundp
                'ac-rtags-action-function))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_does_not_define_action_namespace() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (fboundp
                'ac-rtags-action-namespace))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_fresh_autoload_does_not_define_init() {
    let elisp_form = r##"(list
               (featurep
                'ac-rtags)
               (featurep
                'ac-rtags-autoloads)
               (fboundp
                'ac-rtags-init))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_rtags_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_source_and_completion_hook_honor_runtime_function_rebinding() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-rtags-init)
                     (lambda ()
                       (push
                        'init
                        calls)
                       'initialized))
                    ((symbol-function
                      'ac-rtags-prefix)
                     (lambda ()
                       (push
                        'prefix
                        calls)
                       8))
                    ((symbol-function
                      'ac-start)
                     (lambda ()
                       (push
                        'start
                        calls)
                       'started)))
                 (list
                  (funcall
                   (cdr
                    (assq
                     'init
                     ac-source-rtags)))
                  (funcall
                   (cdr
                    (assq
                     'prefix
                     ac-source-rtags)))
                  (ac-rtags-completions-hook)
                  (nreverse calls))))"##;
    let expect = expect!["OK (initialized 8 started (init prefix start))"];

    assert_ac_rtags_parity(elisp_form, expect);
}
