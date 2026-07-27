use expect_test::expect;

use super::assert_ansible_doc_parity;

#[test]
fn ansible_doc_modules_parses_realistic_list_output_and_caches_identity() {
    let elisp_form = r##"(let ((ansible-doc--modules nil)
               calls)
         (cl-letf (((symbol-function 'call-process)
                    (lambda (&rest args)
                      (push args calls)
                      (insert "apt              Manages apt packages\n"
                              "ansible.builtin.copy Copies files\n"
                              "malformed\n"
                              "user             Manages users\n")
                      0)))
           (let* ((first (ansible-doc-modules))
                  (second (ansible-doc-modules)))
             (list first
                   second
                   (eq first second)
                   (nreverse calls)
                   ansible-doc--modules))))"##;
    let expect = expect![[
        r#"OK (#1=("apt" "ansible.builtin.copy" "user") #1# t (("ansible-doc" nil t nil "--list")) #1#)"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_modules_handles_command_failure_without_populating_cache() {
    let elisp_form = r##"(let ((ansible-doc--modules nil)
               calls messages)
         (cl-letf (((symbol-function 'call-process)
                    (lambda (&rest args)
                      (push args calls)
                      (insert "ansible-doc failed loudly")
                      7))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args) messages))))
           (list (ansible-doc-modules)
                 ansible-doc--modules
                 (nreverse calls)
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (nil nil (("ansible-doc" nil t nil "--list")) ("Finding Ansible modules..." "Error while finding Ansible modules: (error \"Command ansible-doc --list failed with code 7, returned ansible-doc failed loudly\")"))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_modules_accepts_tabs_spaces_namespaces_and_empty_descriptions() {
    let elisp_form = r##"(let ((ansible-doc--modules nil))
         (cl-letf (((symbol-function 'call-process)
                    (lambda (&rest _)
                      (insert "short x\n"
                              "namespace.collection.module\tlong description\n"
                              "_private    hidden module\n"
                              "no-description \n"
                              " two-leading-spaces invalid\n"
                              "\n")
                      0)))
           (ansible-doc-modules)))"##;
    let expect = expect![[r#"OK ("short" "namespace.collection.module" "_private")"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_read_module_uses_known_symbol_as_required_default() {
    let elisp_form = r##"(let ((ansible-doc--modules '("apt" "copy" "user"))
               captured)
         (cl-letf (((symbol-function 'thing-at-point)
                    (lambda (&rest _) "copy"))
                   ((symbol-function 'completing-read)
                    (lambda (&rest args)
                      (setq captured args)
                      "")))
           (list (ansible-doc-read-module "Lookup")
                 captured)))"##;
    let expect = expect![[
        r#"OK ("copy" ("Lookup (default copy): " ("apt" "copy" "user") nil t nil nil "copy"))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_read_module_rejects_unknown_symbol_as_default() {
    let elisp_form = r##"(let ((ansible-doc--modules '("apt" "copy"))
               captured)
         (cl-letf (((symbol-function 'thing-at-point)
                    (lambda (&rest _) "unknown"))
                   ((symbol-function 'completing-read)
                    (lambda (&rest args)
                      (setq captured args)
                      "apt")))
           (list (ansible-doc-read-module "Module")
                 captured)))"##;
    let expect = expect![[r#"OK ("apt" ("Module: " ("apt" "copy") nil t nil nil nil))"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_read_module_without_discovery_uses_symbol_as_only_candidate() {
    let elisp_form = r##"(let ((ansible-doc--modules nil)
               captured)
         (cl-letf (((symbol-function 'ansible-doc-modules)
                    (lambda () nil))
                   ((symbol-function 'thing-at-point)
                    (lambda (&rest _) "playbook_module"))
                   ((symbol-function 'completing-read)
                    (lambda (&rest args)
                      (setq captured args)
                      "")))
           (list (ansible-doc-read-module "Documentation")
                 captured)))"##;
    let expect = expect![[
        r#"OK ("playbook_module" ("Documentation (default playbook_module): " ("playbook_module") nil nil nil nil "playbook_module"))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_read_module_returns_typed_reply_unchanged() {
    let elisp_form = r##"(let ((ansible-doc--modules nil)
               captured)
         (cl-letf (((symbol-function 'ansible-doc-modules)
                    (lambda () nil))
                   ((symbol-function 'thing-at-point)
                    (lambda (&rest _) nil))
                   ((symbol-function 'completing-read)
                    (lambda (&rest args)
                      (setq captured args)
                      "community.general.ufw")))
           (list (ansible-doc-read-module "Open")
                 captured)))"##;
    let expect = expect![[r#"OK ("community.general.ufw" ("Open: " (nil) nil nil nil nil nil))"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}
