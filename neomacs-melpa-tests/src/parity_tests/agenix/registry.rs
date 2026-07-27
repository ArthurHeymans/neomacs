use expect_test::expect;

use super::{assert_agenix_autoload_parity, assert_agenix_parity};

#[test]
fn agenix_registry_defaults_custom_metadata_and_buffer_local_state_match() {
    let elisp_form = r##"(list
         (featurep 'agenix)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value symbol)
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (local-variable-if-set-p symbol)))
          '(agenix-age-program
            agenix-key-files
            agenix-pre-mode-hook
            agenix--encrypted-fp
            agenix--keys
            agenix--undo-list
            agenix--point))
         (mapcar
          (lambda (symbol)
            (with-temp-buffer
              (set symbol 'local-value)
              (list
               symbol
               (local-variable-p symbol)
               (symbol-value symbol))))
          '(agenix--encrypted-fp
            agenix--keys
            agenix--undo-list
            agenix--point))
         (seq-filter
          (lambda (entry)
            (equal entry
                   '("\\.age\\'"
                     . agenix-mode-if-with-secrets-nix)))
          auto-mode-alist))"##;
    let expect = expect![[
        r#"OK (t ((agenix-age-program "age" string nil nil) (agenix-key-files ("~/.ssh/id_ed25519" "~/.ssh/id_rsa") (repeat (choice (string :tag "Pathname to a key file") (function :tag "Function returning the pathname to a key file"))) nil nil) (agenix-pre-mode-hook nil hook nil nil) (agenix--encrypted-fp nil nil nil t) (agenix--keys nil nil nil t) (agenix--undo-list nil nil nil t) (agenix--point nil nil nil t)) ((agenix--encrypted-fp t local-value) (agenix--keys t local-value) (agenix--undo-list t local-value) (agenix--point t local-value)) (("\\.age\\'" . agenix-mode-if-with-secrets-nix)))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_complete_callable_surface_arglists_and_command_status_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (autoloadp (symbol-function symbol))))
         '(agenix-mode
           agenix--buffer-string*
           agenix--with-temp-buffer
           agenix--identity-protected-p
           agenix--prompt-password
           agenix--create-temp-identity
           agenix--process-exit-code-and-output
           agenix--process-agenix-key-files
           agenix--decrypt-current-buffer-using-cleartext-identities
           agenix-decrypt-buffer
           agenix-save-decrypted
           agenix-secrets-base-dir
           agenix-locate-secrets-nix
           agenix-path-relative-to-secrets-nix
           agenix-mode-if-with-secrets-nix))"##;
    let expect = expect![
        "OK ((agenix-mode nil t nil) (agenix--buffer-string* (buffer) nil nil) (agenix--with-temp-buffer (func) nil nil) (agenix--identity-protected-p (identity-path) nil nil) (agenix--prompt-password (identity-file) nil nil) (agenix--create-temp-identity (identity-path password) nil nil) (agenix--process-exit-code-and-output (program &rest args) nil nil) (agenix--process-agenix-key-files nil nil nil) (agenix--decrypt-current-buffer-using-cleartext-identities (cleartext-key-paths) nil nil) (agenix-decrypt-buffer (&optional encrypted-buffer) t nil) (agenix-save-decrypted (&optional unencrypted-buffer) t nil) (agenix-secrets-base-dir (pathname) nil nil) (agenix-locate-secrets-nix (pathname) nil nil) (agenix-path-relative-to-secrets-nix (pathname) nil nil) (agenix-mode-if-with-secrets-nix nil t nil))"
    ];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_mode_definition_and_real_auto_mode_dispatch_metadata_match() {
    let elisp_form = r##"(list
         (get 'agenix-mode 'derived-mode-parent)
         (get 'agenix-mode 'mode-class)
         (keymapp agenix-mode-map)
         (with-temp-buffer
           (setq buffer-file-name "/work/secret.age")
           (set-auto-mode)
           (list
            major-mode
            (eq major-mode 'agenix-mode)
            (eq
             (cdr
              (assoc "\\.age\\'" auto-mode-alist))
             'agenix-mode-if-with-secrets-nix))))"##;
    let expect = expect!["OK (text-mode nil t (fundamental-mode nil t))"];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_autoloads_register_public_commands_and_age_file_dispatch_without_loading() {
    let elisp_form = r##"(list
         (featurep 'agenix)
         (mapcar
          (lambda (symbol)
            (let ((definition
                   (symbol-function symbol)))
              (list
               symbol
               (autoloadp definition)
               (and
                (autoloadp definition)
                (nth 1 definition))
               (commandp symbol))))
          '(agenix-decrypt-buffer
            agenix-save-decrypted
            agenix-mode-if-with-secrets-nix))
         (seq-filter
          (lambda (entry)
            (equal entry
                   '("\\.age\\'"
                     . agenix-mode-if-with-secrets-nix)))
          auto-mode-alist)
         (file-name-nondirectory
          (getenv "NEOMACS_PACKAGE_SOURCE")))"##;
    let expect = expect![[
        r#"OK (nil ((agenix-decrypt-buffer t "agenix" t) (agenix-save-decrypted t "agenix" t) (agenix-mode-if-with-secrets-nix t "agenix" t)) (("\\.age\\'" . agenix-mode-if-with-secrets-nix)) "agenix-autoloads.el")"#
    ]];
    assert_agenix_autoload_parity(elisp_form, expect);
}
