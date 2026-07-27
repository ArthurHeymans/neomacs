use expect_test::expect;

use super::{assert_anki_mode_autoload_parity, assert_anki_mode_parity};

#[test]
fn anki_mode_registers_exact_public_surface_and_defaults() {
    let elisp_form = r##"(list
         (featurep 'anki-mode)
         (get 'anki-mode 'derived-mode-parent)
         (get 'anki-mode-menu-mode 'derived-mode-parent)
         (get 'anki 'custom-group)
         (get 'anki 'group-documentation)
         anki-mode--required-anki-connect-version
         anki-mode--field-start-regex
         anki-mode-markdown-command
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (commandp symbol)
                  (help-function-arglist symbol t)))
          '(anki-mode anki-mode-new-card anki-mode-menu
            anki-mode-send-new-card anki-mode-cloze-region)))"##;
    let expect = expect![[
        r#"OK (t gfm-mode special-mode ((anki-mode-markdown-command custom-variable)) "Customisation options for interacting with Anki, a spaced repetition flashcard program." 6 "^\\s-*@" "pandoc --from gfm --to html" ((anki-mode t t nil) (anki-mode-new-card t t nil) (anki-mode-menu t t nil) (anki-mode-send-new-card t t nil) (anki-mode-cloze-region t t (start end))))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_keymaps_preserve_exact_user_workflow_bindings() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (key)
            (list key (lookup-key anki-mode-map (kbd key))))
          '("C-c C-c" "$" "<tab>" "TAB"))
         (mapcar
          (lambda (key)
            (list key
                  (lookup-key anki-mode-menu-mode-map (kbd key))
                  (commandp
                   (lookup-key anki-mode-menu-mode-map (kbd key)))))
          '("n" "r" "a")))"##;
    let expect = expect![[
        r#"OK ((("C-c C-c" anki-mode-send-new-card) ("$" anki-mode-insert-latex-math) ("<tab>" anki-mode-next-field) ("TAB" nil)) (("n" anki-mode-new-card t) ("r" #[nil ((anki-mode-refresh) (anki-mode-menu-render)) #1=(anki-mode-menu-mode-abbrev-table anki-mode-menu-mode-syntax-table anki-mode-abbrev-table anki-mode-syntax-table t) nil nil nil] t) ("a" #[nil ((if (not (and anki-mode--previous-deck anki-mode--previous-card-type)) (progn (error "Can't reuse the previous options because no previous deck/card type is set"))) (anki-mode-new-card-noninteractive anki-mode--previous-deck anki-mode--previous-card-type)) #1# nil nil nil] t)))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_descriptor_records_exact_pin_requirements_and_payload() {
    let elisp_form = r##"(let* ((description (cadr (assq 'anki-mode package-alist)))
               (directory (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar
            (lambda (file) (file-relative-name file directory))
            (directory-files-recursively directory "." nil))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (anki-mode "20201223.719" nil "A major mode for creating anki cards." ((emacs (24 4)) (dash (2 12 0)) (markdown-mode (2 2)) (s (1 11 0)) (request (0 3 0))) ("README-elpa" "anki-mode-autoloads.el" "anki-mode-pkg.el" "anki-mode.el" "anki-mode.elc"))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_autoloads_expose_commands_without_loading_package() {
    let elisp_form = r##"(list
         (featurep 'anki-mode)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (commandp symbol)
                  (autoloadp (symbol-function symbol))
                  (symbol-function symbol)))
          '(anki-mode-new-card anki-mode-menu anki-mode-send-new-card)))"##;
    let expect = expect![[
        r#"OK (nil ((anki-mode-new-card t t (autoload "anki-mode" "Create a buffer for a new Anki card." t nil)) (anki-mode-menu t t (autoload "anki-mode" "Open an Anki menu buffer." t nil)) (anki-mode-send-new-card t t (autoload "anki-mode" "Send the current buffer as a card to anki-connect." t nil))))"#
    ]];
    assert_anki_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn anki_mode_reload_preserves_state_and_does_not_duplicate_features() {
    let elisp_form = r##"(let ((anki-mode--decks '("Work" "Personal"))
               (anki-mode--card-types '(("Basic" "Front" "Back")))
               (source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (load source nil t t)
         (list anki-mode--decks
               anki-mode--card-types
               (length (delq nil
                             (mapcar
                              (lambda (feature)
                                (and (eq feature 'anki-mode) feature))
                              features)))
               (featurep 'anki-mode)))"##;
    let expect = expect![[r#"OK (("Work" "Personal") (("Basic" "Front" "Back")) 1 t)"#]];
    assert_anki_mode_parity(elisp_form, expect);
}
