use expect_test::expect;

use super::assert_ansible_doc_parity;

#[test]
fn ansible_doc_current_module_returns_local_value_and_signals_outside_docs() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (setq ansible-doc-current-module "community.general.ufw")
           (list (ansible-doc-current-module)
                 (local-variable-p 'ansible-doc-current-module)))
         (with-temp-buffer
           (condition-case err
               (ansible-doc-current-module)
             (error (list (car err) (cdr err))))))"##;
    let expect = expect![[
        r#"OK (("community.general.ufw" t) (error ("This buffer does not document an Ansible module")))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_module_mode_configures_real_documentation_buffer_state() {
    let elisp_form = r##"(with-temp-buffer
         (setq ansible-doc-current-module "copy")
         (ansible-doc-module-mode)
         (list major-mode
               mode-name
               buffer-read-only
               buffer-auto-save-file-name
               truncate-lines
               revert-buffer-function
               bookmark-make-record-function
               font-lock-defaults
               imenu-generic-expression
               (lookup-key ansible-doc-module-mode-map (kbd "RET"))
               (local-variable-p 'revert-buffer-function)
               (local-variable-p 'bookmark-make-record-function)
               mode-line-buffer-identification))"##;
    let expect = expect![[
        r#"OK (ansible-doc-module-mode "ADoc Module" t nil t ansible-doc-revert-module-buffer ansible-doc-make-module-bookmark ((ansible-doc-module-font-lock-keywords) t nil) (("Options" "^[=-] \\([^[:space:]]+\\)$" 1)) nil t t ((#("%12b" 0 4 (face mode-line-buffer-id help-echo "Buffer name\nmouse-1: Previous buffer\nmouse-3: Next buffer" mouse-face mode-line-highlight local-map (keymap (header-line keymap (mouse-3 . mode-line-next-buffer) (down-mouse-3 . ignore) (mouse-1 . mode-line-previous-buffer) (down-mouse-1 . ignore)) (mode-line keymap (mouse-3 . mode-line-next-buffer) (mouse-1 . mode-line-previous-buffer)))))) " {" ansible-doc-current-module "}"))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_revert_loads_process_output_cleans_whitespace_and_restores_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert "old documentation")
         (goto-char 5)
         (setq ansible-doc-current-module "copy"
               buffer-read-only t)
         (let (calls fontify-calls messages)
           (cl-letf (((symbol-function 'call-process)
                      (lambda (&rest args)
                        (push args calls)
                        (insert "> COPY  \n"
                                "Options:\t \n"
                                "# example  \n\n")
                        0))
                     ((symbol-function 'ansible-doc-fontify-yaml-examples)
                      (lambda () (push (buffer-string) fontify-calls)))
                     ((symbol-function 'message)
                      (lambda (format-string &rest args)
                        (push (apply #'format format-string args) messages))))
             (list
              (ansible-doc-revert-module-buffer nil t)
              (buffer-string)
              (point)
              buffer-read-only
              (nreverse calls)
              (nreverse fontify-calls)
              (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (5 "> COPY\nOptions:\n# example\n" 5 t (("ansible-doc" nil t t "copy")) ("> COPY\nOptions:\n# example\n") ("Loading documentation for module copy"))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_revert_decline_preserves_buffer_and_skips_external_process() {
    let elisp_form = r##"(with-temp-buffer
         (insert "keep me")
         (goto-char 4)
         (setq ansible-doc-current-module "user")
         (let (prompts calls)
           (cl-letf (((symbol-function 'y-or-n-p)
                      (lambda (prompt)
                        (push prompt prompts)
                        nil))
                     ((symbol-function 'call-process)
                      (lambda (&rest args)
                        (push args calls)
                        0)))
             (list
              (ansible-doc-revert-module-buffer nil nil)
              (buffer-string)
              (point)
              (nreverse prompts)
              calls))))"##;
    let expect = expect![[r#"OK (nil "keep me" 4 ("Reload documentation for user? ") nil)"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_buffer_creates_initializes_populates_and_reuses_one_buffer() {
    let elisp_form = r##"(let ((name (format ansible-doc--buffer-name "copy"))
               calls)
         (when (get-buffer name) (kill-buffer name))
         (unwind-protect
             (cl-letf (((symbol-function 'ansible-doc-revert-module-buffer)
                        (lambda (&rest args)
                          (push (cons (current-buffer) args) calls)
                          (let ((inhibit-read-only t))
                            (insert "fixture docs")))))
               (let* ((first (ansible-doc-buffer "copy"))
                      (second (ansible-doc-buffer "copy")))
                 (list
                  (eq first second)
                  (buffer-name first)
                  (buffer-live-p first)
                  (with-current-buffer first
                    (list major-mode
                          ansible-doc-current-module
                          (buffer-string)
                          buffer-read-only))
                  (mapcar
                   (lambda (call)
                     (list (buffer-name (car call)) (cdr call)))
                   (nreverse calls)))))
           (when (get-buffer name) (kill-buffer name))))"##;
    let expect = expect![[
        r#"OK (t "*ansible-doc copy*" t (ansible-doc-module-mode "copy" #("fixture docs" 0 12 (fontified nil)) t) (("*ansible-doc copy*" (nil noconfirm))))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_command_resolves_buffer_then_displays_it() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ansible-doc-buffer)
                    (lambda (module)
                      (push (list 'buffer module) calls)
                      'fixture-buffer))
                   ((symbol-function 'pop-to-buffer)
                    (lambda (&rest args)
                      (push (cons 'pop args) calls)
                      'fixture-window)))
           (list (ansible-doc "community.general.ufw")
                 (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (fixture-window ((buffer "community.general.ufw") (pop fixture-buffer)))"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_make_module_bookmark_merges_default_record_and_handler() {
    let elisp_form = r##"(with-temp-buffer
         (setq ansible-doc-current-module "apt")
         (let (calls)
           (cl-letf (((symbol-function 'bookmark-make-record-default)
                      (lambda (&rest args)
                        (push args calls)
                        '("fixture" (buffer . "source") (position . 12)))))
             (list (ansible-doc-make-module-bookmark)
                   (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (("Ansible module apt" "fixture" (buffer . "source") (position . 12) (ansible-module . "apt") (handler . ansible-doc-jump-module-bookmark)) ((no-file)))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_jump_module_bookmark_resolves_module_and_delegates_record() {
    let elisp_form = r##"(let ((bookmark
                '("saved" (ansible-module . "copy")
                  (position . 19) (custom . retained)))
               calls)
         (cl-letf (((symbol-function 'bookmark-prop-get)
                    (lambda (&rest args)
                      (push (cons 'prop args) calls)
                      "copy"))
                   ((symbol-function 'ansible-doc-buffer)
                    (lambda (&rest args)
                      (push (cons 'buffer args) calls)
                      'copy-buffer))
                   ((symbol-function 'bookmark-get-bookmark-record)
                    (lambda (&rest args)
                      (push (cons 'record args) calls)
                      '((position . 19) (custom . retained))))
                   ((symbol-function 'bookmark-default-handler)
                    (lambda (&rest args)
                      (push (cons 'handler args) calls)
                      'jumped)))
           (list (ansible-doc-jump-module-bookmark bookmark)
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (jumped ((prop #1=("saved" (ansible-module . "copy") (position . 19) (custom . retained)) ansible-module) (buffer "copy") (record #1#) (handler ("" (buffer . copy-buffer) (position . 19) (custom . retained)))))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_minor_mode_toggles_lighter_and_real_lookup_key() {
    let elisp_form = r##"(with-temp-buffer
         (let ((before (current-local-map)))
           (ansible-doc-mode 1)
           (let ((enabled
                  (list ansible-doc-mode
                        (assq 'ansible-doc-mode minor-mode-alist)
                        (lookup-key
                         (current-minor-mode-maps)
                         (kbd "C-c ?"))
                        (buffer-string))))
             (ansible-doc-mode -1)
             (list enabled
                   ansible-doc-mode
                   (eq before (current-local-map))
                   (buffer-string)))))"##;
    let expect = expect![[r#"OK ((t (ansible-doc-mode " ADoc") ansible-doc "") nil t "")"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_buffer_names_distinguish_modules_and_existing_buffers() {
    let elisp_form = r##"(let ((first-name (format ansible-doc--buffer-name "copy"))
               (second-name
                (format ansible-doc--buffer-name
                        "community.general.copy"))
               calls)
         (dolist (name (list first-name second-name))
           (when (get-buffer name) (kill-buffer name)))
         (unwind-protect
             (cl-letf (((symbol-function 'ansible-doc-revert-module-buffer)
                        (lambda (&rest _)
                          (push ansible-doc-current-module calls))))
               (let ((first (ansible-doc-buffer "copy"))
                     (second
                      (ansible-doc-buffer "community.general.copy")))
                 (list (buffer-name first)
                       (buffer-name second)
                       (eq first second)
                       (nreverse calls))))
           (dolist (name (list first-name second-name))
             (when (get-buffer name) (kill-buffer name)))))"##;
    let expect = expect![[
        r#"OK ("*ansible-doc copy*" "*ansible-doc community.general.copy*" nil ("copy" "community.general.copy"))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}
