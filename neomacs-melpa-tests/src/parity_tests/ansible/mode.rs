use expect_test::expect;

use super::assert_ansible_parity;

#[test]
fn ansible_regexes_classify_sections_tasks_options_names_and_jinja_constructs() {
    let elisp_form = r##"(let ((samples
                '(("  tasks:" section)
                  ("  - name: Deploy API" name)
                  ("    ansible.builtin.copy:" task)
                  ("    copy:" task)
                  ("    become_user: deploy" keyword)
                  ("    when: release_ready" keyword)
                  ("    value: {{ release_version }}" variable)
                  ("    value: {% if enabled %}" statement)
                  ("    value: {# internal note #}" comment))))
         (mapcar
          (lambda (sample)
            (let* ((line (car sample))
                   (kind (cadr sample))
                   (regexp
                    (pcase kind
                      ('section
                       ansible-section-keywords-regex)
                      ('task
                       ansible-task-keywords-regex)
                      ('keyword
                       ansible-keywords-regex)
                      ('name
                       (nth
                        3
                        ansible-playbook-font-lock))
                      ('variable
                       (nth
                        4
                        ansible-playbook-font-lock))
                      ('statement
                       (nth
                        5
                        ansible-playbook-font-lock))
                      ('comment
                       (nth
                        6
                        ansible-playbook-font-lock)))))
              (when (listp regexp)
                (setq regexp (car regexp)))
              (list
               kind
               (and
                (string-match regexp line)
                (match-string 1 line))
               (and
                (eq kind 'name)
                (match-string 2 line)))))
          samples))"##;
    let expect = expect![[
        r#"OK ((section "tasks" nil) (name "name" " Deploy API") (task nil nil) (task "copy" nil) (keyword "become_user" nil) (keyword "when" nil) (variable "{{" nil) (statement "{%" nil) (comment "{#" nil))"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_mode_fontifies_a_practical_playbook_with_expected_semantic_faces() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (setq buffer-file-name
               (expand-file-name
                "deploy.yml"
                default-directory))
         (insert
          "- hosts: production\n"
          "  tasks:\n"
          "    - name: Deploy API\n"
          "      copy:\n"
          "        src: \"{{ artifact_path }}\"\n"
          "        when: release_ready\n"
          "        note: {# audited deployment #}\n")
         (ansible-mode 1)
         (font-lock-ensure)
         (list
          (ansible-test-face-at "tasks")
          (ansible-test-face-at "name")
          (ansible-test-face-at "Deploy API")
          (ansible-test-face-at "copy")
          (ansible-test-face-at "when")
          (ansible-test-face-at "{{")
          (ansible-test-face-at "artifact_path")
          (ansible-test-face-at "{#")
          (ansible-test-face-at "audited deployment")))"##;
    let expect = expect![
        "OK (ansible-section-face font-lock-builtin-face ansible-task-label-face font-lock-keyword-face font-lock-builtin-face font-lock-builtin-face font-lock-function-name-face font-lock-comment-delimiter-face font-lock-comment-face)"
    ];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_mode_activation_sets_lighter_map_hooks_dictionary_and_lint_command() {
    let elisp_form = r##"(progn
         (defvar ac-user-dictionary-files nil)
         (let* ((hook-events nil)
               (ac-user-dictionary-files nil)
               (ansible-hook
                (append
                 ansible-hook
                 (list
                  (lambda ()
                    (setq hook-events
                          (append
                           hook-events
                           (list
                            (list
                             ansible-mode
                             (local-variable-p
                              'compile-command))))))))))
         (with-temp-buffer
           (text-mode)
           (setq buffer-file-name
                 (expand-file-name
                  "site.yml"
                  default-directory))
           (let ((before-map-count
                  (length
                   (cl-remove-if-not
                    (lambda (entry)
                      (eq (car-safe entry) 'ansible))
                    minor-mode-map-alist))))
             (ansible-mode 1)
             (let ((enabled
                    (list
                     ansible-mode
                     (let ((entry
                            (assq
                             'ansible-mode
                             minor-mode-alist)))
                       (list
                        (and entry t)
                        (equal
                         (cadr entry)
                         " Ansible")))
                     (eq
                      (cdr
                       (assq
                        'ansible
                        minor-mode-map-alist))
                      ansible-key-map)
                     (- (length
                         (cl-remove-if-not
                          (lambda (entry)
                            (eq
                             (car-safe entry)
                             'ansible))
                          minor-mode-map-alist))
                        before-map-count)
                     (and
                      (member
                       (expand-file-name
                        "dict/ansible"
                        ansible-dir)
                       ac-user-dictionary-files)
                      t)
                     (and
                      (memq
                       #'ansible-maybe-unload-snippets
                       kill-buffer-hook)
                      t)
                     compile-command
                     hook-events)))
               (ansible-mode -1)
               (list
                enabled
                ansible-mode
                (and
                 (memq
                  #'ansible-maybe-unload-snippets
                  kill-buffer-hook)
                 t)))))))"##;
    let expect = expect![[
        r#"OK ((t (t t) t 1 t t "LANG=C.UTF-8 ansible-lint [ORACLE-SANDBOX]/site.yml" ((t t))) nil t)"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_add_and_remove_font_lock_call_runtime_with_exact_keyword_contract() {
    let elisp_form = r##"(let (events)
         (cl-letf
             (((symbol-function
                'font-lock-add-keywords)
               (lambda
                   (major keywords how)
                 (setq events
                       (append
                        events
                        (list
                         (list
                          :add
                          major
                          (eq
                           keywords
                           ansible-playbook-font-lock)
                          how))))))
              ((symbol-function
                'font-lock-remove-keywords)
               (lambda
                   (major keywords)
                 (setq events
                       (append
                        events
                        (list
                         (list
                          :remove
                          major
                          (eq
                           keywords
                           ansible-playbook-font-lock)))))))
              ((symbol-function 'font-lock-flush)
               (lambda ()
                 (setq events
                       (append
                        events
                        (list :flush))))))
           (ansible-add-font-lock)
           (ansible-remove-font-lock)
           events))"##;
    let expect = expect!["OK ((:add nil t append) :flush (:remove nil t) :flush)"];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_mode_disable_removes_ansible_fontification_from_refontified_buffer() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (setq buffer-file-name
               (expand-file-name
                "tasks.yml"
                default-directory))
         (insert
          "  tasks:\n"
          "    - name: Restart service\n"
          "      service:\n")
         (ansible-mode 1)
         (font-lock-ensure)
         (let ((before
                (list
                 (ansible-test-face-at "tasks")
                 (ansible-test-face-at "service"))))
           (ansible-mode -1)
           (font-lock-flush)
           (font-lock-ensure)
           (list
            before
            (list
             (ansible-test-face-at "tasks")
             (ansible-test-face-at "service")))))"##;
    let expect = expect!["OK ((ansible-section-face ansible-task-label-face) (nil nil))"];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_mode_runs_each_configured_hook_in_order_on_every_enable() {
    let elisp_form = r##"(let* ((events nil)
               (ansible-hook
                (list
                 (lambda ()
                   (setq events
                         (append
                          events
                          '(first))))
                 (lambda ()
                   (setq events
                         (append
                          events
                          (list
                           (if ansible-mode
                               'enabled
                             'disabled))))))))
         (with-temp-buffer
           (text-mode)
           (ansible-mode 1)
           (ansible-mode -1)
           (ansible-mode 1)
           (list
            events
            ansible-mode
            (length
             (cl-remove-if-not
              (lambda (entry)
                (eq
                 (car-safe entry)
                 'ansible))
              minor-mode-map-alist)))))"##;
    let expect = expect!["OK ((first enabled first enabled) t 2)"];

    assert_ansible_parity(elisp_form, expect);
}
