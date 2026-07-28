use expect_test::expect;

use super::assert_auto_minor_mode_parity;

#[test]
fn auto_minor_mode_plain_filename_removes_backup_versions_and_remote_prefixes() {
    let elisp_form = r##"(mapcar
                           (lambda (name)
                             (list
                              name
                              (auto-minor-mode--plain-filename
                               name)))
                           '("/workspace/report.txt"
                             "/workspace/report.txt~"
                             "/workspace/report.txt.~12~"
                             "/ssh:user@example.test:/srv/app/main.rb"
                             "/ssh:user@example.test:/srv/app/main.rb~"
                             "/sudo:root@localhost:/etc/service.conf.~3~"
                             "relative/file.el~"))"##;
    let expect = expect![[
        r#"OK (("/workspace/report.txt" "/workspace/report.txt") ("/workspace/report.txt~" "/workspace/report.txt") ("/workspace/report.txt.~12~" "/workspace/report.txt") ("/ssh:user@example.test:/srv/app/main.rb" "/srv/app/main.rb") ("/ssh:user@example.test:/srv/app/main.rb~" "/srv/app/main.rb") ("/sudo:root@localhost:/etc/service.conf.~3~" "/etc/service.conf") ("relative/file.el~" "relative/file.el"))"#
    ]];

    assert_auto_minor_mode_parity(elisp_form, expect);
}

#[test]
fn auto_minor_mode_filename_rules_enable_every_matching_mode_in_order() {
    let elisp_form = r##"(with-temp-buffer
                           (auto-minor-mode-test-reset)
                           (setq
                            buffer-file-name
                            "/workspace/service-theme.el"
                            auto-minor-mode-alist
                            '(("-theme\\.el\\'"
                               . auto-minor-mode-test-alpha-mode)
                              ("service-"
                               . auto-minor-mode-test-beta-mode)
                              ("\\.el\\'"
                               . auto-minor-mode-test-gamma-mode)))
                           (auto-minor-mode-set)
                           (list
                            auto-minor-mode-test-alpha-mode
                            auto-minor-mode-test-beta-mode
                            auto-minor-mode-test-gamma-mode
                            (nreverse
                             auto-minor-mode-test-events)))"##;
    let expect = expect![
        "OK (t t t ((:alpha 1 t 1 fundamental-mode) (:beta 1 t 1 fundamental-mode) (:gamma 1 t 1 fundamental-mode)))"
    ];

    assert_auto_minor_mode_parity(elisp_form, expect);
}

#[test]
fn auto_minor_mode_filename_matching_is_always_case_folded() {
    let elisp_form = r##"(mapcar
                           (lambda (name)
                             (with-temp-buffer
                               (auto-minor-mode-test-reset)
                               (setq
                                buffer-file-name name
                                auto-minor-mode-alist
                                '(("\\.ammtest\\'"
                                   . auto-minor-mode-test-alpha-mode)))
                               (auto-minor-mode-set)
                               (list
                                name
                                auto-minor-mode-test-alpha-mode
                                (nreverse
                                 auto-minor-mode-test-events))))
                           '("/project/file.ammtest"
                             "/project/file.aMmTeSt"
                             "/project/FILE.AMMTEST"
                             "/project/file.ammtestx"))"##;
    let expect = expect![[
        r#"OK (("/project/file.ammtest" t ((:alpha 1 t 1 fundamental-mode))) ("/project/file.aMmTeSt" t ((:alpha 1 t 1 fundamental-mode))) ("/project/FILE.AMMTEST" t ((:alpha 1 t 1 fundamental-mode))) ("/project/file.ammtestx" nil nil))"#
    ]];

    assert_auto_minor_mode_parity(elisp_form, expect);
}

#[test]
fn auto_minor_mode_filename_regex_boundaries_reject_near_matches() {
    let elisp_form = r##"(mapcar
                           (lambda (name)
                             (with-temp-buffer
                               (auto-minor-mode-test-reset)
                               (setq
                                buffer-file-name name
                                auto-minor-mode-alist
                                '(("/\\.env\\(?:\\.[^/]+\\)?\\'"
                                   . auto-minor-mode-test-alpha-mode)))
                               (auto-minor-mode-set)
                               (list
                                name
                                auto-minor-mode-test-alpha-mode)))
                           '("/project/.env"
                             "/project/.env.local"
                             "/project/x.env"
                             "/project/.environment"
                             "/project/.env/local"))"##;
    let expect = expect![[
        r#"OK (("/project/.env" t) ("/project/.env.local" t) ("/project/x.env" nil) ("/project/.environment" nil) ("/project/.env/local" nil))"#
    ]];

    assert_auto_minor_mode_parity(elisp_form, expect);
}

#[test]
fn auto_minor_mode_filename_rules_require_a_visited_file() {
    let elisp_form = r##"(with-temp-buffer
                           (auto-minor-mode-test-reset)
                           (rename-buffer
                            "notes.ammtest")
                           (setq
                            buffer-file-name nil
                            auto-minor-mode-alist
                            '(("\\.ammtest\\'"
                               . auto-minor-mode-test-alpha-mode)))
                           (auto-minor-mode-set)
                           (list
                            (buffer-name)
                            buffer-file-name
                            auto-minor-mode-test-alpha-mode
                            auto-minor-mode-test-events))"##;
    let expect = expect![[r#"OK ("notes.ammtest" nil nil nil)"#]];

    assert_auto_minor_mode_parity(elisp_form, expect);
}

#[test]
fn auto_minor_mode_remote_backup_filename_drives_practical_rule_without_connection() {
    let elisp_form = r##"(with-temp-buffer
                           (auto-minor-mode-test-reset)
                           (setq
                            buffer-file-name
                            "/ssh:deploy@example.test:/srv/site/config.yaml.~7~"
                            auto-minor-mode-alist
                            '(("/srv/site/.*\\.yaml\\'"
                               . auto-minor-mode-test-alpha-mode)
                              ("^/ssh:"
                               . auto-minor-mode-test-beta-mode)))
                           (auto-minor-mode-set)
                           (list
                            (auto-minor-mode--plain-filename
                             buffer-file-name)
                            auto-minor-mode-test-alpha-mode
                            auto-minor-mode-test-beta-mode
                            (nreverse
                             auto-minor-mode-test-events)))"##;
    let expect =
        expect![[r#"OK ("/srv/site/config.yaml" t nil ((:alpha 1 t 1 fundamental-mode)))"#]];

    assert_auto_minor_mode_parity(elisp_form, expect);
}

#[test]
fn auto_minor_mode_duplicate_filename_rules_reactivate_the_same_mode() {
    let elisp_form = r##"(with-temp-buffer
                           (auto-minor-mode-test-reset)
                           (setq
                            buffer-file-name
                            "/project/service.notes"
                            auto-minor-mode-alist
                            '(("service"
                               . auto-minor-mode-test-alpha-mode)
                              ("\\.notes\\'"
                               . auto-minor-mode-test-alpha-mode)))
                           (auto-minor-mode-set)
                           (let ((without-keep
                                  (nreverse
                                   auto-minor-mode-test-events)))
                             (setq
                              auto-minor-mode-test-events
                              nil)
                             (auto-minor-mode-set t)
                             (list
                              without-keep
                              auto-minor-mode-test-events
                              auto-minor-mode-test-alpha-mode)))"##;
    let expect =
        expect!["OK (((:alpha 1 t 1 fundamental-mode) (:alpha 1 t 1 fundamental-mode)) nil t)"];

    assert_auto_minor_mode_parity(elisp_form, expect);
}

#[test]
fn auto_minor_mode_keep_skips_enabled_modes_but_enables_disabled_matches() {
    let elisp_form = r##"(with-temp-buffer
                           (auto-minor-mode-test-reset)
                           (setq
                            buffer-file-name
                            "/project/service.el"
                            auto-minor-mode-test-alpha-mode
                            t
                            auto-minor-mode-alist
                            '(("service"
                               . auto-minor-mode-test-alpha-mode)
                              ("\\.el\\'"
                               . auto-minor-mode-test-beta-mode)))
                           (auto-minor-mode-set t)
                           (list
                            auto-minor-mode-test-alpha-mode
                            auto-minor-mode-test-beta-mode
                            (nreverse
                             auto-minor-mode-test-events)))"##;
    let expect = expect!["OK (t t ((:beta 1 t 1 fundamental-mode)))"];

    assert_auto_minor_mode_parity(elisp_form, expect);
}

#[test]
fn auto_minor_mode_keep_does_not_skip_unregistered_truthy_mode_variable() {
    let elisp_form = r##"(with-temp-buffer
                           (auto-minor-mode-test-reset)
                           (setq
                            buffer-file-name
                            "/project/service.data"
                            auto-minor-mode-test-unregistered-mode
                            t
                            auto-minor-mode-alist
                            '(("\\.data\\'"
                               . auto-minor-mode-test-unregistered-mode)))
                           (auto-minor-mode-set t)
                           (list
                            (memq
                             'auto-minor-mode-test-unregistered-mode
                             minor-mode-list)
                            auto-minor-mode-test-unregistered-mode
                            (nreverse
                             auto-minor-mode-test-events)))"##;
    let expect = expect!["OK (nil t ((:unregistered 1 t 1 fundamental-mode)))"];

    assert_auto_minor_mode_parity(elisp_form, expect);
}

#[test]
fn auto_minor_mode_direct_filename_runner_uses_ambient_case_fold_and_rejects_function_matcher() {
    let elisp_form = r##"(with-temp-buffer
                           (setq
                            buffer-file-name
                            "/project/FILE.AMM")
                           (list
                            (mapcar
                             (lambda (case-fold)
                               (auto-minor-mode-test-reset)
                               (let ((case-fold-search
                                      case-fold))
                                 (auto-minor-mode--run-auto
                                  '(("\\.amm\\'"
                                     . auto-minor-mode-test-alpha-mode))
                                  nil))
                               (list
                                case-fold
                                auto-minor-mode-test-alpha-mode))
                             '(nil t))
                            (auto-minor-mode-test-error
                             (lambda ()
                               (auto-minor-mode--run-auto
                                (list
                                 (cons
                                  'auto-minor-mode-test-filename-match
                                  'auto-minor-mode-test-beta-mode))
                                nil)))))"##;
    let expect = expect![
        "OK (((nil nil) (t t)) (:signal wrong-type-argument (stringp auto-minor-mode-test-filename-match)))"
    ];

    assert_auto_minor_mode_parity(elisp_form, expect);
}
