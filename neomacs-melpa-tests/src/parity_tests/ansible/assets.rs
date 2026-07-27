use expect_test::expect;

use super::assert_ansible_parity;

#[test]
fn ansible_dictionary_has_exact_size_boundaries_uniqueness_and_practical_options() {
    let elisp_form = r##"(let* ((path
                  (expand-file-name
                   "dict/ansible"
                   ansible-dir))
                 (words
                  (split-string
                   (ansible-test-read-file path)
                   "\n"
                   t)))
         (list
          (length words)
          (car words)
          (car (last words))
          (= (length words)
             (length
              (delete-dups
               (copy-sequence words))))
          (mapcar
           (lambda (word)
             (list
              word
              (and
               (member word words)
               t)))
           '("name"
             "state"
             "validate_certs"
             "vault_identity"
             "ui_repoid_vars"))))"##;
    let expect = expect![[
        r#"OK (432 "name" "ui_repoid_vars" t (("name" t) ("state" t) ("validate_certs" t) ("vault_identity" nil) ("ui_repoid_vars" t)))"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_snippet_inventory_and_selected_real_module_templates_match() {
    let elisp_form = r##"(let* ((directory
                  (expand-file-name
                   "snippets/text-mode/ansible/modules"
                   ansible-dir))
                 (names
                  (sort
                   (directory-files
                    directory
                    nil
                    "\\`[^.]")
                   #'string<)))
         (list
          (length names)
          (car names)
          (car (last names))
          (equal
           (ansible-test-read-file
            (expand-file-name
             "systemd"
             directory))
           (ansible-test-read-file
            (expand-file-name
             "systemd_service"
             directory)))
          (mapcar
           (lambda (name)
             (list
              name
              (ansible-test-read-file
               (expand-file-name
                name
                directory))))
           '("apt"
             "copy"
             "git"
             "package"
             "replace"
             "unarchive"))))"##;
    let expect = expect![[
        r##"OK (71 "add_host" "yum_repository" t (("apt" "# name : Manages apt-packages\n# key : apt\n# condition: ansible\n# --\n- name: ${1:Manages apt-packages}\n  apt: $0\n") ("copy" "# name : Copy files to remote locations\n# key : copy\n# condition: ansible\n# --\n- name: ${1:Copy files to remote locations}\n  copy: dest=$2 $0\n") ("git" "# name : Deploy software (or files) from git checkouts\n# key : git\n# condition: ansible\n# --\n- name: ${1:Deploy software (or files) from git checkouts}\n  git: repo=$2 dest=$3 $0\n") ("package" "# name : Generic OS package manager\n# key : package\n# condition: ansible\n# --\n- name: ${1:Generic OS package manager}\n  package: name=$2 state=$3 $0\n") ("replace" "# name : Replace all instances of a particular string in a file using a back-referenced regular expression\n# key : replace\n# condition: ansible\n# --\n- name: ${1:Replace all instances of a particular string in a file using a back-referenced regular expression}\n  replace: path=$2 regexp=$3 $0\n") ("unarchive" "# name : Unpacks an archive after (optionally) copying it from the local machine\n# key : unarchive\n# condition: ansible\n# --\n- name: ${1:Unpacks an archive after (optionally) copying it from the local machine}\n  unarchive: src=$2 dest=$3 $0\n")))"##
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_runtime_asset_paths_stay_inside_installed_package() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr (assq 'ansible package-alist)))
                 (directory
                  (file-name-as-directory
                   (package-desc-dir descriptor))))
         (list
          (file-equal-p ansible-dir directory)
          (file-equal-p
           ansible-snip-dir
           (expand-file-name
            "snippets"
            directory))
          (file-directory-p ansible-snip-dir)
          (file-regular-p
           (expand-file-name
            "dict/ansible"
            directory))
          (ansible-test-read-file
           (expand-file-name
            "snippets/yaml-mode/.yas-parents"
            directory))))"##;
    let expect = expect![[r#"OK (t t t t "text-mode")"#]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_dictionary_initialization_handles_unbound_bound_and_duplicate_states() {
    let elisp_form = r##"(let ((dictionary
                (expand-file-name
                 "dict/ansible"
                 ansible-dir)))
         (when (boundp
                'ac-user-dictionary-files)
           (makunbound
            'ac-user-dictionary-files))
         (let ((unbound-result
                (ansible-dict-initialize)))
           (defvar ac-user-dictionary-files nil)
           (setq ac-user-dictionary-files
                 '("/existing/dictionary"))
           (let ((first
                  (ansible-dict-initialize))
                 (after-first nil)
                 second)
             (setq after-first
                   (copy-sequence
                    ac-user-dictionary-files))
             (setq second
                   (ansible-dict-initialize))
             (list
              unbound-result
              (mapcar
               (lambda (entry)
                 (if
                     (equal entry dictionary)
                     :ansible-dictionary
                   entry))
               first)
              (mapcar
               (lambda (entry)
                 (if
                     (equal entry dictionary)
                     :ansible-dictionary
                   entry))
               after-first)
              (mapcar
               (lambda (entry)
                 (if
                     (equal entry dictionary)
                     :ansible-dictionary
                   entry))
               second)
              (mapcar
               (lambda (entry)
                 (if
                     (equal entry dictionary)
                     :ansible-dictionary
                   entry))
               ac-user-dictionary-files)
              (cl-count
               dictionary
               ac-user-dictionary-files
               :test #'equal)))))"##;
    let expect = expect![[
        r#"OK (nil ("/existing/dictionary" :ansible-dictionary) ("/existing/dictionary" :ansible-dictionary) ("/existing/dictionary" :ansible-dictionary) ("/existing/dictionary" :ansible-dictionary) 1)"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_optional_yasnippet_lifecycle_loads_then_unloads_packaged_snippets() {
    let elisp_form = r##"(progn
         (defvar yas-snippet-dirs nil)
         (provide 'yasnippet)
         (let ((yas-snippet-dirs
                '("/existing/snippets"))
               events)
           (cl-letf
               (((symbol-function
                  'yas-load-directory)
                 (lambda (directory)
                   (setq events
                         (append
                          events
                          (list
                           (list
                            :load
                            (file-equal-p
                             directory
                             ansible-snip-dir)))))))
                ((symbol-function
                  'yas-reload-all)
                 (lambda ()
                   (setq events
                         (append
                          events
                          (list :reload))))))
             (with-temp-buffer
               (text-mode)
               (ansible-mode 1)
               (let ((enabled
                      (list
                       (featurep 'yasnippet)
                       (and
                        (member
                         ansible-snip-dir
                         yas-snippet-dirs)
                        t)
                       (copy-tree events))))
                 (ansible-mode -1)
                 (list
                  enabled
                  (and
                   (member
                    ansible-snip-dir
                    yas-snippet-dirs)
                   t)
                  events))))))"##;
    let expect = expect!["OK ((t t ((:load t))) nil ((:load t) :reload))"];

    assert_ansible_parity(elisp_form, expect);
}
