use expect_test::expect;

use super::{
    assert_ariadne_autoload_parity, assert_ariadne_parity, assert_ariadne_with_legacy_cl_parity,
};

#[test]
fn descriptor_records_exact_pin_dependency_and_installed_payload() {
    let elisp_form = r##"(let* ((desc (cadr (assq 'ariadne package-alist)))
              (dir (package-desc-dir desc)))
         (list
          (package-version-join (package-desc-version desc))
          (package-desc-reqs desc)
          (package-desc-kind desc)
          (sort
           (mapcar #'file-name-nondirectory
                   (directory-files dir t "^[^.].*"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK ("20131117.1711" ((bert (0 1))) nil ("README-elpa" "ariadne-autoloads.el" "ariadne-pkg.el" "ariadne.el" "ariadne.elc"))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_has_exact_arities_commands_and_docs() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (macrop symbol)
                 (documentation symbol t)))
         '(ariadne-connect
           ariadne-close
           ariadne-filter
           ariadne-sentinel
           ariadne-process-available-output
           ariadne-have-input-p
           ariadne-run-when-idle
           ariadne-read-or-lose
           ariadne-read
           ariadne-encode-length
           ariadne-decode-length
           ariadne-dispatch-event
           ariadne-handle-reply
           ariadne-goto
           ariadne-send
           ariadne-current-line
           ariadne-goto-definition))"##;
    let expect = expect![[
        r#"OK ((ariadne-connect nil nil nil "Connect to the Ariadne server.") (ariadne-close (process) nil nil nil) (ariadne-filter (process string) nil nil "Accept output from the socket and process all complete\nmessages.") (ariadne-sentinel (process message) nil nil nil) (ariadne-process-available-output (process) nil nil "Process all complete messages that have arrived from Ariadne.") (ariadne-have-input-p nil nil nil "Return T if a complete message is available.") (ariadne-run-when-idle (function &rest args) nil nil "Call FUNCTION as soon as Emacs is idle.") (ariadne-read-or-lose (process) nil nil nil) (ariadne-read nil nil nil "Read a message from the Ariadne buffer.") (ariadne-encode-length (length) nil nil nil) (ariadne-decode-length nil nil nil nil) (ariadne-dispatch-event (event process) nil nil nil) (ariadne-handle-reply (reply) nil nil nil) (ariadne-goto (filename line column) nil nil "Go to a given position in a given file.") (ariadne-send (obj process) nil nil "Send OBJ to Ariadne over the socket PROCESS.") (ariadne-current-line nil nil nil "Return the vertical position of point.") (ariadne-goto-definition nil t nil "Go to the definition of a name at point."))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn source_hash_process_state_and_feature_contract_are_exact() {
    let elisp_form = r##"(let ((source (locate-library "ariadne")))
         (list
          (with-temp-buffer
            (set-buffer-multibyte nil)
            (insert-file-contents-literally source)
            (secure-hash 'sha256 (current-buffer)))
          ariadne-process
          (documentation-property
           'ariadne-process 'variable-documentation)
          (featurep 'ariadne)
          (featurep 'bert)
          (featurep 'bindat)))"##;
    let expect = expect![[
        r#"OK ("5dfd2c020052797962fdc8dc1b3ab475beee905899b9cf35ddc37267a6e89f70" nil "Process object representing a network connection to Ariadne." t t t)"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn autoload_file_exposes_only_goto_definition_without_loading_source() {
    let elisp_form = r##"(list
         (featurep 'ariadne)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (autoloadp (symbol-function symbol))
                  (commandp symbol)))
          '(ariadne-goto-definition
            ariadne-connect
            ariadne-send)))"##;
    let expect = expect![
        "OK (nil ((ariadne-goto-definition t t t) (ariadne-connect nil nil nil) (ariadne-send nil nil nil)))"
    ];
    assert_ariadne_autoload_parity(elisp_form, expect);
}

#[test]
fn reload_is_idempotent_for_feature_and_callable_definitions() {
    let elisp_form = r##"(let ((source (locate-library "ariadne"))
               definitions)
         (dotimes (_ 3)
           (load source nil 'nomessage)
           (push
            (mapcar
             (lambda (symbol)
               (list symbol
                     (help-function-arglist symbol t)
                     (commandp symbol)))
             '(ariadne-connect
               ariadne-read
               ariadne-goto-definition))
            definitions))
         (list (cl-count 'ariadne features)
               (and (equal (nth 0 definitions)
                           (nth 1 definitions))
                    (equal (nth 1 definitions)
                           (nth 2 definitions)))
               ariadne-process))"##;
    let expect = expect!["OK (1 t nil)"];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn clean_runtime_pins_undeclared_legacy_cl_dependency_failures() {
    let elisp_form = r##"(list
         (condition-case error
             (list :ok
                   (ariadne-handle-reply
                    (vector 'no_name)))
           (error (list :error error)))
         (condition-case error
             (list :ok
                   (bert-pack
                    (vector 'call 'ariadne 'find
                            '("Main.hs" 1 0))))
           (error (list :error error))))"##;
    let expect = expect!["OK ((:error (void-function case)) (:error (void-function letf)))"];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn package_dependency_descriptor_and_loaded_bert_surface_are_usable() {
    let elisp_form = r##"(let* ((desc (cadr (assq 'bert package-alist)))
              (value (vector 'call 'ariadne 'find
                             '("/workspace/Main.hs" 17 4)))
              (bytes (bert-pack value)))
         (list
          (package-version-join (package-desc-version desc))
          (package-desc-reqs desc)
          (length bytes)
          (equal (bert-unpack bytes) value)))"##;
    let expect = expect![[r#"OK ("20131117.1014" nil 60 t)"#]];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}
