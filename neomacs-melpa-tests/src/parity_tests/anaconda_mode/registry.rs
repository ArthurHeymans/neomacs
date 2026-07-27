use expect_test::expect;

use super::{assert_anaconda_mode_autoload_parity, assert_anaconda_mode_parity};

#[test]
fn anaconda_mode_loads_exact_package_dependency_graph_and_feature() {
    let elisp_form = r##"(let* ((description
        (cadr (assq 'anaconda-mode package-alist)))
       (directory
        (file-name-as-directory (package-desc-dir description))))
  (list
   (featurep 'anaconda-mode)
   (package-installed-p 'anaconda-mode)
   (package-version-join (package-desc-version description))
   (mapcar
    (lambda (requirement)
      (list
       (car requirement)
       (package-version-join (cadr requirement))
       (or (package-installed-p (car requirement))
           (package-built-in-p (car requirement)))))
    (package-desc-reqs description))
   (file-readable-p
    (expand-file-name "anaconda-mode.el" directory))
   (file-readable-p
    (expand-file-name "anaconda-mode.py" directory))))"##;
    let expect = expect![[
        r#"OK (t t "20250430.227" ((emacs "25.1" t) (pythonic "0.1.0" t) (dash "2.6.0" t) (s "1.9" t) (f "0.16.2" t)) t t)"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn custom_options_preserve_defaults_types_groups_and_user_facing_contracts() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (copy-tree (symbol-value symbol))
    (get symbol 'custom-type)
    (get symbol 'custom-group)
    (local-variable-if-set-p symbol)))
 '(anaconda-mode-installation-directory
   anaconda-mode-eldoc-as-single-line
   anaconda-mode-lighter
   anaconda-mode-localhost-address
   anaconda-mode-doc-frame-background
   anaconda-mode-doc-frame-foreground
   anaconda-mode-use-posframe-show-doc
   anaconda-mode-tunnel-setup-sleep
   anaconda-mode-sync-request-timeout
   anaconda-mode-disable-rpc))"##;
    let expect = expect![[
        r#"OK ((anaconda-mode-installation-directory "~/.emacs.d/anaconda-mode" directory nil nil) (anaconda-mode-eldoc-as-single-line nil boolean nil nil) (anaconda-mode-lighter " Anaconda" sexp nil nil) (anaconda-mode-localhost-address "127.0.0.1" string nil nil) (anaconda-mode-doc-frame-background "unspecified-bg" string nil nil) (anaconda-mode-doc-frame-foreground "unspecified-fg" string nil nil) (anaconda-mode-use-posframe-show-doc nil boolean nil nil) (anaconda-mode-tunnel-setup-sleep 2 integer nil nil) (anaconda-mode-sync-request-timeout 2 integer nil nil) (anaconda-mode-disable-rpc never (choice (const :tag "Never" never) (const :tag "Always" always) (const :tag "Remote" remote)) nil nil))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn server_process_and_documentation_state_defaults_are_complete_and_unshared() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (copy-tree (symbol-value symbol))
    (get symbol 'variable-documentation)
    (local-variable-if-set-p symbol)))
 '(anaconda-mode-server-version
   anaconda-mode-process-name
   anaconda-mode-process-buffer
   anaconda-mode-process
   anaconda-mode-response-buffer
   anaconda-mode-socat-process-name
   anaconda-mode-socat-process-buffer
   anaconda-mode-socat-process
   anaconda-mode-ssh-process-name
   anaconda-mode-ssh-process-buffer
   anaconda-mode-ssh-process
   anaconda-mode-doc-frame-name
   anaconda-mode-frame-last-point
   anaconda-mode-frame-last-scroll-offset))"##;
    let expect = expect![[
        r#"OK ((anaconda-mode-server-version "0.1.17" "Server version needed to run `anaconda-mode'." nil) (anaconda-mode-process-name "anaconda-mode" "Process name for `anaconda-mode' processes." nil) (anaconda-mode-process-buffer "*anaconda-mode*" "Buffer name for `anaconda-mode' process." nil) (anaconda-mode-process nil "Currently running `anaconda-mode' process." nil) (anaconda-mode-response-buffer "*anaconda-response*" "Buffer name for error report when `anaconda-mode' fail to read server response." nil) (anaconda-mode-socat-process-name "anaconda-socat" "Process name for `anaconda-mode' socat companion process." nil) (anaconda-mode-socat-process-buffer "*anaconda-socat*" "Buffer name for `anaconda-mode' socat companion process." nil) (anaconda-mode-socat-process nil "Currently running `anaconda-mode' socat companion process." nil) (anaconda-mode-ssh-process-name "anaconda-ssh" "Process name for `anaconda-mode' ssh port forward companion process." nil) (anaconda-mode-ssh-process-buffer "*anaconda-ssh*" "Buffer name for `anaconda-mode' ssh port forward companion process." nil) (anaconda-mode-ssh-process nil "Currently running `anaconda-mode' ssh port forward companion process." nil) (anaconda-mode-doc-frame-name "*Anaconda Posframe*" "The posframe to show anaconda documentation." nil) (anaconda-mode-frame-last-point 0 "The last point of anaconda doc view frame, use for hide frame after move point." nil) (anaconda-mode-frame-last-scroll-offset 0 "The last scroll offset when show doc view frame, use for hide frame after window scroll." nil))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn complete_callable_command_and_xref_surface_preserves_arglists() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (commandp symbol)
    (help-function-arglist symbol t)))
 '(anaconda-mode-server-directory
   anaconda-mode-host anaconda-mode-port
   anaconda-mode-start anaconda-mode-stop
   anaconda-mode-running-p
   anaconda-mode-socat-running-p
   anaconda-mode-ssh-running-p
   anaconda-mode-bound-p anaconda-mode-need-restart
   anaconda-mode-get-server-process-cwd
   anaconda-mode-server-command-args
   anaconda-mode-bootstrap anaconda-jump-proxy-string
   anaconda-mode-bootstrap-filter
   anaconda-mode-call anaconda-mode-call-sync
   anaconda-mode-jsonrpc
   anaconda-mode-jsonrpc-request
   anaconda-mode-jsonrpc-request-data
   anaconda-mode-create-response-handler
   anaconda-mode-complete
   anaconda-mode-complete-callback
   anaconda-mode-complete-extract-names
   anaconda-mode-complete-annotation
   anaconda-mode-show-doc
   anaconda-mode-show-doc-callback
   anaconda-mode-documentation-view
   anaconda-mode-documentation-posframe-view
   anaconda-mode-hide-frame
   anaconda-mode-find-definitions
   anaconda-mode-find-definitions-other-window
   anaconda-mode-find-definitions-other-frame
   anaconda-mode-find-assignments
   anaconda-mode-find-assignments-other-window
   anaconda-mode-find-assignments-other-frame
   anaconda-mode-find-references
   anaconda-mode-find-references-other-window
   anaconda-mode-find-references-other-frame
   anaconda-mode-xref-backend
   anaconda-mode-show-xrefs anaconda-mode-make-xrefs
   anaconda-mode-eldoc-function
   anaconda-mode-eldoc-format
   anaconda-mode-eldoc-format-definition
   anaconda-mode anaconda-eldoc-mode
   turn-on-anaconda-eldoc-mode
   turn-off-anaconda-eldoc-mode))"##;
    let expect = expect![
        "OK ((anaconda-mode-server-directory t nil nil) (anaconda-mode-host t nil nil) (anaconda-mode-port t nil nil) (anaconda-mode-start t nil (&optional callback)) (anaconda-mode-stop t nil nil) (anaconda-mode-running-p t nil nil) (anaconda-mode-socat-running-p t nil nil) (anaconda-mode-ssh-running-p t nil nil) (anaconda-mode-bound-p t nil nil) (anaconda-mode-need-restart t nil nil) (anaconda-mode-get-server-process-cwd t nil nil) (anaconda-mode-server-command-args t nil nil) (anaconda-mode-bootstrap t nil (&optional callback)) (anaconda-jump-proxy-string t nil nil) (anaconda-mode-bootstrap-filter t nil (process output &optional callback)) (anaconda-mode-call t nil (command callback)) (anaconda-mode-call-sync t nil (command callback)) (anaconda-mode-jsonrpc t nil (command callback)) (anaconda-mode-jsonrpc-request t nil (command)) (anaconda-mode-jsonrpc-request-data t nil (command)) (anaconda-mode-create-response-handler t nil (callback)) (anaconda-mode-complete t t nil) (anaconda-mode-complete-callback t nil (result)) (anaconda-mode-complete-extract-names t nil (result)) (anaconda-mode-complete-annotation t nil (candidate)) (anaconda-mode-show-doc t t nil) (anaconda-mode-show-doc-callback t nil (result)) (anaconda-mode-documentation-view t nil (result)) (anaconda-mode-documentation-posframe-view t nil (result)) (anaconda-mode-hide-frame t nil nil) (anaconda-mode-find-definitions t t nil) (anaconda-mode-find-definitions-other-window t t nil) (anaconda-mode-find-definitions-other-frame t t nil) (anaconda-mode-find-assignments t t nil) (anaconda-mode-find-assignments-other-window t t nil) (anaconda-mode-find-assignments-other-frame t t nil) (anaconda-mode-find-references t t nil) (anaconda-mode-find-references-other-window t t nil) (anaconda-mode-find-references-other-frame t t nil) (anaconda-mode-xref-backend t nil nil) (anaconda-mode-show-xrefs t nil (result display-action error-message)) (anaconda-mode-make-xrefs t nil (result)) (anaconda-mode-eldoc-function t nil (callback &rest _ignored)) (anaconda-mode-eldoc-format t nil (result)) (anaconda-mode-eldoc-format-definition t nil (name index params)) (anaconda-mode t t (&optional arg)) (anaconda-eldoc-mode t t (&optional arg)) (turn-on-anaconda-eldoc-mode t nil nil) (turn-off-anaconda-eldoc-mode t nil nil))"
    ];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn minor_mode_keymap_preserves_every_documented_navigation_binding() {
    let elisp_form = r##"(mapcar
 (lambda (key)
   (list
    key
    (lookup-key anaconda-mode-map (kbd key))))
 '("C-M-i" "M-." "C-x 4 ." "C-x 5 ."
   "M-=" "C-x 4 =" "C-x 5 ="
   "M-r" "C-x 4 r" "C-x 5 r"
   "M-," "M-?"))"##;
    let expect = expect![[
        r#"OK (("C-M-i" anaconda-mode-complete) ("M-." anaconda-mode-find-definitions) ("C-x 4 ." anaconda-mode-find-definitions-other-window) ("C-x 5 ." anaconda-mode-find-definitions-other-frame) ("M-=" anaconda-mode-find-assignments) ("C-x 4 =" anaconda-mode-find-assignments-other-window) ("C-x 5 =" anaconda-mode-find-assignments-other-frame) ("M-r" anaconda-mode-find-references) ("C-x 4 r" anaconda-mode-find-references-other-window) ("C-x 5 r" anaconda-mode-find-references-other-frame) ("M-," xref-pop-marker-stack) ("M-?" anaconda-mode-show-doc))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_defer_source_and_register_both_minor_modes() {
    let elisp_form = r##"(list
 (featurep 'anaconda-mode)
 (mapcar
  (lambda (symbol)
    (let ((definition (symbol-function symbol)))
      (list
       symbol
       (autoloadp definition)
       (and (autoloadp definition) (nth 1 definition))
       (commandp symbol)
       (help-function-arglist symbol t))))
  '(anaconda-mode anaconda-eldoc-mode))
 (boundp 'anaconda-mode-map)
 (boundp 'anaconda-mode-server-version))"##;
    let expect = expect![[
        r#"OK (nil ((anaconda-mode t "anaconda-mode" t "[Arg list not available until function definition is loaded.]") (anaconda-eldoc-mode t "anaconda-mode" t "[Arg list not available until function definition is loaded.]")) nil nil)"#
    ]];
    assert_anaconda_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn installed_python_server_artifact_matches_exact_windows_capable_pin() {
    let elisp_form = r##"(let* ((library (locate-library "anaconda-mode"))
       (python-file
        (expand-file-name
         "anaconda-mode.py"
         (file-name-directory library))))
  (with-temp-buffer
    (insert-file-contents-literally python-file)
    (let ((contents (buffer-string)))
      (list
       (file-readable-p python-file)
       (secure-hash 'sha256 contents)
       (count-lines (point-min) (point-max))
       (string-match-p
        "binname = 'Scripts' if sys.platform == 'win32' else 'bin'"
        contents)
       (string-match-p
        "jedi_dep = ('jedi', '0.19.2')"
        contents)
       (string-match-p
        "service_factory_dep = ('service_factory', '0.1.6')"
        contents)
       (string-match-p
        "app = \\[complete, company_complete, show_doc, infer, goto, get_references, eldoc\\]"
        contents)))))"##;
    let expect = expect![[
        r#"OK (t "fc3c32bc90a567bc2007c5670e6d07d4f457f01dd75fa830ce66f143a02d4945" 214 2162 565 626 5857)"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}
