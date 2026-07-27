use expect_test::expect;

use super::{assert_ansible_autoload_parity, assert_ansible_parity};

#[test]
fn ansible_exact_pin_metadata_dependencies_and_origin_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr (assq 'ansible package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-reqs descriptor)
          (package-desc-summary descriptor)
          (copy-tree (package-desc-extras descriptor))
          (featurep 'ansible)
          (mapcar
           #'featurep
           '(s f))))"##;
    let expect = expect![[
        r#"OK (ansible "20260607.1852" ((s (1 9 0)) (f (0 16 2)) (emacs (25 1))) "Ansible minor mode." ((:maintainers (nil . "k1lowxb[at]gmail[dot]com") (nil . "k1low[at]101000lab[dot]org")) (:authors (nil . "k1lowxb[at]gmail[dot]com") (nil . "k1low[at]101000lab[dot]org")) (:revdesc . "0d7bc93ad963") (:commit . "0d7bc93ad963677880d99c846a30ea6e6ed9eec5") (:url . "https://gitlab.com/emacs-ansible/emacs-ansible")) t (t t))"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_installed_payload_has_exact_inventory_size_and_content_digest() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr (assq 'ansible package-alist)))
                 (directory
                  (package-desc-dir descriptor))
                 (all-files
                  (sort
                   (directory-files-recursively
                    directory
                    ".*"
                    nil)
                   #'string<))
                 (source-files
                  (cl-remove-if
                   (lambda (path)
                     (member
                      (file-name-nondirectory path)
                      '("README-elpa"
                        "ansible-autoloads.el"
                        "ansible.elc")))
                   all-files))
                 (relative-files
                  (mapcar
                   (lambda (path)
                     (file-relative-name
                      path
                      directory))
                   source-files))
                 (total-bytes
                  (apply
                   #'+
                   (mapcar
                    (lambda (path)
                      (nth
                       7
                       (file-attributes path)))
                    source-files)))
                 digest)
         (with-temp-buffer
           (set-buffer-multibyte nil)
           (cl-mapc
            (lambda (relative path)
              (insert relative "\0")
              (insert-file-contents-literally path)
              (insert "\0"))
            relative-files
            source-files)
           (setq digest
                 (secure-hash
                  'sha256
                  (current-buffer))))
         (list
          (length all-files)
          (length source-files)
          total-bytes
          digest
          (car relative-files)
          (car (last relative-files))
          (cl-count-if
           (lambda (relative)
             (string-prefix-p
              "snippets/text-mode/ansible/modules/"
              relative))
           relative-files)
          (mapcar
           (lambda (name)
             (let ((path
                    (expand-file-name
                     name
                     directory)))
               (list
                name
                (file-regular-p path)
                (and
                 (file-regular-p path)
                 (> (nth
                     7
                     (file-attributes path))
                    0)))))
           '("ansible.elc"
             "ansible-autoloads.el"
             "dict/ansible"
             "snippets/yaml-mode/.yas-parents"))))"##;
    let expect = expect![[
        r#"OK (78 75 41881 "d18fd7a40c6cd2f9181d74b7a9db5fecfc87a89d2e18ca3e69af7b2d5583c5c2" "ansible-pkg.el" "snippets/yaml-mode/.yas-parents" 71 (("ansible.elc" t t) ("ansible-autoloads.el" t t) ("dict/ansible" t t) ("snippets/yaml-mode/.yas-parents" t t)))"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_customization_faces_aliases_and_mode_contract_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (let ((standard
                   (get symbol 'standard-value)))
              (list
               symbol
               (default-value symbol)
               (and
                standard
                (eval (car standard)))
               (get symbol 'custom-type)
               (get symbol 'custom-group)
               (get symbol 'risky-local-variable))))
          '(ansible-dir-search-limit
            ansible-vault-password-file
            ansible-vault-password-environment-variable
            ansible-vault-password))
         (mapcar
          (lambda (face)
            (list
             face
             (and
              (facep face)
              t)
             (face-documentation face)))
          '(ansible-section-face
            ansible-task-label-face))
         (eq
          (symbol-function 'ansible)
          (symbol-function 'ansible-mode))
         (eq
          (indirect-variable 'ansible)
          'ansible-mode)
         (get 'ansible 'byte-obsolete-info)
         (get 'ansible-mode 'custom-type)
         (get 'ansible-mode 'variable-documentation)
         (keymapp ansible-key-map))"##;
    let expect = expect![[
        r#"OK (((ansible-dir-search-limit 5 5 integer nil nil) (ansible-vault-password-file "~/.vault_pass.txt" "~/.vault_pass.txt" file nil t) (ansible-vault-password-environment-variable "VAULT_PASSWORD" "VAULT_PASSWORD" string nil nil) (ansible-vault-password file file (choice (const :tag "Use the contents of the file `ansible-vault-password-file`" file) (const :tag "Prompt for a password" :value ansible-vault-prompt-for-password) (const :tag "Use the contents of the environment variable `ansible-vault-password-environment-variable`" :value ansible-vault-password-from-environment) (function :tag "Function")) nil nil)) ((ansible-section-face t "Face for ansible first level section names in playbooks.\nUsed for vars, tasks, handlers, etc.") (ansible-task-label-face t "Face for ansible task names in playbooks.")) nil t (ansible-mode nil "2024-11-28") nil "Non-nil if Ansible mode is enabled.\nUse the command `ansible-mode' to change this variable." t)"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_function_surface_has_exact_command_and_documentation_shape() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (fboundp symbol)
            (commandp symbol)
            (documentation symbol)))
         '(ansible-mode
           ansible-add-font-lock
           ansible-remove-font-lock
           ansible-update-root-path
           ansible-find-root-path
           ansible-list-playbooks
           ansible-vault-get-password
           ansible-vault
           ansible-vault-string
           ansible-decrypt-buffer
           ansible-encrypt-buffer
           ansible-vault-buffer
           ansible-vault-region
           ansible-decrypt-region
           ansible-encrypt-region
           ansible-auto-decrypt-encrypt
           ansible-dict-initialize
           ansible-lint-errors))"##;
    let expect = expect![[
        r#"OK ((ansible-mode t t "Ansible minor mode.\n\nThis is a minor mode.  If called interactively, toggle the ‘Ansible\nmode’ mode.  If the prefix argument is positive, enable the mode, and if\nit is zero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is ‘toggle’.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable ‘ansible-mode’.\n\nThe mode’s hook is called both when the mode is enabled and when it is\ndisabled.") (ansible-add-font-lock t t "Extend YAML with syntax highlight for ansible playbooks.") (ansible-remove-font-lock t t "Add syntax highlight to ansible playbooks.") (ansible-update-root-path t nil "Update ‘ansible-root-path’.") (ansible-find-root-path t nil "Find ansible directory.") (ansible-list-playbooks t nil "Find .yml files in ‘ansible-root-path‘.") (ansible-vault-get-password t nil "Retrieve the password based on the value of ‘ansible-vault-password’.") (ansible-vault t nil "Execute ‘ansible-vault‘ MODE on STR with the given PARAMS.\n\nMODE is ‘encrypt’ or ‘decrypt’.\n\nSTR is the string to be handled.\n\nPARAMS is produced by ‘ansible-vault-get-password’ and is meant to be an\nlist of args that can be passed to ansible-vault.\n\nIf the first line of STR is indented with whitespace, only those lines\nin STR that match that whitespace will be handled by ‘ansible-vault MODE’.\nThe rest will be untouched.\n\nThe string that results will be returned.\n\nSee the man page ‘ansible-vault(1)’ for more details.") (ansible-vault-string t nil "Do ‘ansible-vault’ MODE on STR and return result.\nMODE should be one of ‘decrypt’ or ‘encrypt’.") (ansible-decrypt-buffer t t "Decrypt current buffer.") (ansible-encrypt-buffer t t "Encrypt current buffer.") (ansible-vault-buffer t nil "Execute ‘ansible-vault’ MODE and update current buffer.") (ansible-vault-region t nil "Execute ‘ansible-vault’ MODE from START to END and update the region.") (ansible-decrypt-region t t "Decrypt from START to END (current region).") (ansible-encrypt-region t t "Encrypt from START to END (current region).") (ansible-auto-decrypt-encrypt t nil "Decrypt current buffer if it is a vault encrypted file.\nAlso, automatically encrypts the file before saving the buffer.") (ansible-dict-initialize t nil "Initialize Ansible auto-complete.") (ansible-lint-errors t nil "Replace make -k with ansible-lint, with an UTF-8 locale to avoid crashes."))"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_autoloads_expose_only_mode_keymap_and_dictionary_entry_points() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (boundp symbol)
            (and
             (fboundp symbol)
             t)
            (and
             (fboundp symbol)
             (autoloadp
              (symbol-function symbol)))
            (commandp symbol)))
         '(ansible-key-map
           ansible-mode
           ansible-dict-initialize
           ansible-list-playbooks
           ansible-vault-string))"##;
    let expect = expect![
        "OK ((ansible-key-map t nil nil nil) (ansible-mode nil t t t) (ansible-dict-initialize nil t t nil) (ansible-list-playbooks nil nil nil nil) (ansible-vault-string nil nil nil nil))"
    ];

    assert_ansible_autoload_parity(elisp_form, expect);
}
