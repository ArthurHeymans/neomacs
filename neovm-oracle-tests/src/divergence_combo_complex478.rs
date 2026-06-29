/// Batch 478: desktop, recentf, savehist, bookmark, winner, follow, hl-line.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx478_desktop_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'desktop)
  (list (boundp 'desktop-dirname) (fboundp 'desktop-save)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx478_recentf_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'recentf)
  (list (boundp 'recentf-list) (fboundp 'recentf-save-list)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx478_savehist_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'savehist)
  (list (boundp 'savehist-mode) (fboundp 'savehist-save)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx478_bookmark_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'bookmark)
  (list (boundp 'bookmark-alist) (fboundp 'bookmark-set)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx478_winner_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'winner)
  (list (boundp 'winner-mode-map) (fboundp 'winner-undo)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx478_follow_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'follow)
  (list (fboundp 'follow-mode) (boundp 'follow-mode-map)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx478_hl_line_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'hl-line)
  (with-temp-buffer
    (hl-line-mode 1)
    hl-line-mode))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx478_whitespace_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'whitespace)
  (with-temp-buffer
    (whitespace-mode 1)
    whitespace-mode))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx478_tabs_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'tabs)
  (list (fboundp 'tab-bar-new-tab) (boundp 'tab-bar-mode)))
"##,
        expect_test::expect![[
            r#""ERR (file-missing \"Cannot open load file\" \"No such file or directory\" \"tabs\")""#
        ]],
    );
}

#[test]
fn div_cx478_server_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'server)
  (list (boundp 'server-process) (fboundp 'server-start)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx478_emacsclient_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'emacsclient)
  (list (fboundp 'emacsclient-mail-command)))
"##,
        expect_test::expect![[
            r#""ERR (file-missing \"Cannot open load file\" \"No such file or directory\" \"emacsclient\")""#
        ]],
    );
}

#[test]
fn div_cx478_tramp_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'tramp)
  (list (boundp 'tramp-methods) (fboundp 'tramp-cleanup-all-connections)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx478_dbus_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e (require 'dbus) (error (car e)))
"##,
        expect_test::expect![[r#""OK dbus""#]],
    );
}

#[test]
fn div_cx478_makefile_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'make-mode)
  (list (fboundp 'makefile-mode) (boundp 'makefile-mode-map)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx478_imenu_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'imenu)
  (list (boundp 'imenu-auto-rescan) (fboundp 'imenu-add-to-menubar)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}
