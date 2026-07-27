use expect_test::expect;

use super::assert_anaconda_mode_parity;

#[test]
fn anaconda_mode_enable_disable_maintains_buffer_local_transport_and_xref_state() {
    let elisp_form = r##"(with-temp-buffer
  (setq-local xref-backend-functions '(project-backend))
  (setq url-http-attempt-keepalives t)
  (let ((before
         (list
          anaconda-mode
          xref-backend-functions
          url-http-attempt-keepalives
          (local-variable-p 'url-http-attempt-keepalives))))
    (anaconda-mode 1)
    (let ((enabled
           (list
            anaconda-mode
            xref-backend-functions
            url-http-attempt-keepalives
            (local-variable-p 'url-http-attempt-keepalives)
            (key-binding (kbd "M-."))
            (key-binding (kbd "M-?")))))
      (anaconda-mode -1)
      (list
       before
       enabled
       (list
        anaconda-mode
        xref-backend-functions
        url-http-attempt-keepalives
        (local-variable-p 'url-http-attempt-keepalives))))))"##;
    let expect = expect![
        "OK ((nil (project-backend) t nil) (t (anaconda-mode-xref-backend . #1=(project-backend)) nil t anaconda-mode-find-definitions anaconda-mode-show-doc) (nil #1# nil t))"
    ];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn mode_state_and_xref_hooks_are_isolated_across_independent_python_buffers() {
    let elisp_form = r##"(let ((first (generate-new-buffer " *anaconda-first*"))
      (second (generate-new-buffer " *anaconda-second*")))
  (unwind-protect
      (progn
        (with-current-buffer first
          (setq-local xref-backend-functions '(first-backend))
          (anaconda-mode 1))
        (with-current-buffer second
          (setq-local xref-backend-functions '(second-backend))
          (setq url-http-attempt-keepalives 'untouched))
        (list
         (with-current-buffer first
           (list anaconda-mode
                 xref-backend-functions
                 url-http-attempt-keepalives
                 (local-variable-p 'url-http-attempt-keepalives)))
         (with-current-buffer second
           (list anaconda-mode
                 xref-backend-functions
                 url-http-attempt-keepalives
                 (local-variable-p 'url-http-attempt-keepalives)))))
    (when (buffer-live-p first) (kill-buffer first))
    (when (buffer-live-p second) (kill-buffer second))))"##;
    let expect = expect![
        "OK ((t (anaconda-mode-xref-backend first-backend) nil t) (nil (second-backend) untouched nil))"
    ];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn eldoc_minor_mode_registers_a_buffer_local_provider_and_leaves_shared_eldoc_running_on_disable() {
    let elisp_form = r##"(with-temp-buffer
  (setq-local eldoc-documentation-functions '(project-eldoc))
  (eldoc-mode -1)
  (let ((before
         (list
          anaconda-eldoc-mode
          eldoc-mode
          eldoc-documentation-functions)))
    (anaconda-eldoc-mode 1)
    (let ((enabled
           (list
            anaconda-eldoc-mode
            eldoc-mode
            eldoc-documentation-functions
            (local-variable-p 'eldoc-documentation-functions))))
      (anaconda-eldoc-mode -1)
      (list
       before
       enabled
       (list
        anaconda-eldoc-mode
        eldoc-mode
        eldoc-documentation-functions
        (local-variable-p 'eldoc-documentation-functions))))))"##;
    let expect = expect![
        "OK ((nil nil #1=(project-eldoc)) (t t (anaconda-mode-eldoc-function . #1#) t) (nil t #1# t))"
    ];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn every_navigation_command_uses_the_correct_rpc_display_target_and_user_error() {
    let elisp_form = r##"(let (events)
  (cl-letf (((symbol-function 'anaconda-mode-call)
             (lambda (command callback)
               (push (list 'request command) events)
               (funcall callback [["/project/model.py" 7 2 "target"]])))
            ((symbol-function 'anaconda-mode-show-xrefs)
             (lambda (result action error-message)
               (push (list 'display result action error-message) events))))
    (dolist
        (command
         '(anaconda-mode-find-definitions
           anaconda-mode-find-definitions-other-window
           anaconda-mode-find-definitions-other-frame
           anaconda-mode-find-assignments
           anaconda-mode-find-assignments-other-window
           anaconda-mode-find-assignments-other-frame
           anaconda-mode-find-references
           anaconda-mode-find-references-other-window
           anaconda-mode-find-references-other-frame))
      (push (list 'invoke command (funcall command)) events))
    (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((request "infer") . #1=((display #2=[["/project/model.py" 7 2 "target"]] nil "No definitions found") (invoke anaconda-mode-find-definitions #1#) (request "infer") . #3=((display #2# window "No definitions found") (invoke anaconda-mode-find-definitions-other-window #3#) (request "infer") . #4=((display #2# frame "No definitions found") (invoke anaconda-mode-find-definitions-other-frame #4#) (request "goto") . #5=((display #2# nil "No assignments found") (invoke anaconda-mode-find-assignments #5#) (request "goto") . #6=((display #2# window "No assignments found") (invoke anaconda-mode-find-assignments-other-window #6#) (request "goto") . #7=((display #2# frame "No assignments found") (invoke anaconda-mode-find-assignments-other-frame #7#) (request "get_references") . #8=((display #2# nil "No references found") (invoke anaconda-mode-find-references #8#) (request "get_references") . #9=((display #2# window "No references found") (invoke anaconda-mode-find-references-other-window #9#) (request "get_references") . #10=((display #2# frame "No references found") (invoke anaconda-mode-find-references-other-frame #10#)))))))))))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn xref_backend_converts_location_vectors_but_reports_server_strings_and_empty_results_as_nil() {
    let elisp_form = r##"(let ((next-result nil)
      events)
  (cl-letf (((symbol-function 'anaconda-mode-call-sync)
             (lambda (command callback)
               (push (list 'request command) events)
               (funcall callback next-result)))
            ((symbol-function 'anaconda-mode-make-xrefs)
             (lambda (result)
               (push (list 'convert result) events)
               (list 'converted (length result))))
            ((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (let ((text (apply #'format format-string arguments)))
                 (push (list 'message text) events)
                 text))))
    (let (observations)
      (dolist
          (case
           '((definitions nil)
             (definitions "server could not infer")
             (definitions [["/a.py" 3 1 "a"]])
             (references nil)
             (references "server could not search")
             (references [["/b.py" 8 4 "b"] ["/c.py" 9 0 "c"]])))
        (setq next-result (cadr case))
        (push
         (list
          case
          (if (eq (car case) 'definitions)
              (xref-backend-definitions 'anaconda "ignored")
            (xref-backend-references 'anaconda "ignored")))
         observations))
      (list (nreverse observations)
            (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ((((definitions nil) nil) ((definitions "server could not infer") nil) ((definitions #1=[["/a.py" 3 1 "a"]]) (converted 1)) ((references nil) nil) ((references "server could not search") nil) ((references #2=[["/b.py" 8 4 "b"] ["/c.py" 9 0 "c"]]) (converted 2))) ((request "infer") (request "infer") (message "server could not infer") (request "infer") (convert #1#) (request "get_references") (request "get_references") (message "server could not search") (request "get_references") (convert #2#)))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn xref_backend_registration_and_unimplemented_optional_operations_have_stable_contracts() {
    let elisp_form = r##"(list
 (anaconda-mode-xref-backend)
 (xref-backend-apropos 'anaconda "service")
 (xref-backend-identifier-completion-table 'anaconda)
 (xref-backend-identifier-at-point 'anaconda)
 (memq #'anaconda-mode-xref-backend xref-backend-functions))"##;
    let expect = expect!["OK (anaconda nil nil nil nil)"];
    assert_anaconda_mode_parity(elisp_form, expect);
}
