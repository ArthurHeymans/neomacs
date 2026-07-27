use expect_test::expect;

use super::{assert_amd_mode_autoload_parity, assert_amd_mode_parity};

#[test]
fn exact_release_descriptor_dependency_versions_and_features_are_stable() {
    let elisp_form = r##"
(let* ((descriptor (cadr (assq 'amd-mode package-alist)))
       (extras (package-desc-extras descriptor)))
  (list
   (package-desc-name descriptor)
   (package-version-join (package-desc-version descriptor))
   (package-desc-reqs descriptor)
   (alist-get :url extras)
   (alist-get :commit extras)
   (alist-get :revdesc extras)
   (mapcar
    (lambda (package)
      (let ((description (cadr (assq package package-alist))))
        (list package
              (and description
                   (package-version-join
                    (package-desc-version description))))))
    '(projectile s f makey js2-mode js2-refactor dash))
   (mapcar #'featurep
           '(amd-mode projectile s f makey js2-mode
             js2-refactor dash xref subr-x))))
"##;
    let expect = expect![[
        r#"OK (amd-mode "20180111.1402" ((emacs (25)) (projectile (20161008 47)) (s (1 9 0)) (f (0 16 2)) (seq (2 16)) (makey (0 3)) (js2-mode (20140114)) (js2-refactor (0 6 1))) "https://github.com/NicolasPetton/amd-mode.el" "01fd19e0d635ccaf8e812364d8720733f2e84126" "01fd19e0d635" ((projectile "20260727.800") (s "20220902.1511") (f "20241003.1131") (makey "20131231.1430") (js2-mode "20260627.1342") (js2-refactor "20250210.1811") (dash "20260221.1346")) (t t t t t t t t t t))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn all_custom_options_and_buffer_local_rewrite_rules_keep_exact_contracts() {
    let elisp_form = r##"
(list
 (mapcar
  (lambda (variable)
    (list variable
          (default-value variable)
          (custom-variable-p variable)
          (get variable 'standard-value)
          (get variable 'custom-type)
          (get variable 'custom-group)))
  '(amd-use-relative-file-name
    amd-always-use-relative-file-name
    amd-write-file-function
    amd-ag-arguments
    amd-ag-ignored-dirs
    amd-ag-ignored-files))
 (default-value 'amd-rewrite-rules-alist)
 (local-variable-if-set-p 'amd-rewrite-rules-alist)
 (with-temp-buffer
   (setq amd-rewrite-rules-alist '(("^src/" . "")))
   (list (local-variable-p 'amd-rewrite-rules-alist)
         amd-rewrite-rules-alist
         (default-value 'amd-rewrite-rules-alist))))
"##;
    let expect = expect![[
        r#"OK (((amd-use-relative-file-name nil #1=(nil) #1# boolean nil) (amd-always-use-relative-file-name nil #2=(nil) #2# boolean nil) (amd-write-file-function write-file #3=('write-file) #3# symbol nil) (amd-ag-arguments #4=("--js" "--noheading") #5=('#4#) #5# list nil) (amd-ag-ignored-dirs #6=("bower_components" "node_modules" "build" "lib") #7=('#6#) #7# list nil) (amd-ag-ignored-files #8=("*.min.js") #9=('#8#) #9# list nil)) nil t (nil #10=(("^src/" . "")) #10#))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn complete_function_inventory_arglists_commands_and_docs_are_stable() {
    let elisp_form = r##"
(mapcar
 (lambda (function)
   (let ((documentation (documentation function)))
     (list function
           (help-function-arglist function t)
           (commandp function)
           (interactive-form function)
           (and documentation
                (secure-hash 'sha256 documentation)))))
 '(amd-mode amd-kill-buffer-module amd-search-references
   amd--xref-search-references amd--find-references amd--make-xref
   amd--xref-candidate amd--xref-false-positive
   amd-find-module-at-point amd-auto-insert amd-import-file
   amd-rename-file amd--replace-all-file-references
   amd--replace-references-in-file amd-import-module amd-kill-line
   amd-kill-module amd--remove-module-from-params amd-move-line-up
   amd-move-line-down amd--guard amd--move-module-up
   amd--move-module-down amd--move-module amd--delete-function-params
   amd--set-function-params amd--define-function-node amd--import
   amd--imported-modules amd--insert-dependency amd--insert-module-name
   amd--module-name amd--goto-define amd--goto-define-function-params
   amd--goto-define-function amd--number-of-named-modules
   amd--goto-imports amd--buffer-file-name amd--buffer-module
   amd--node-content amd--function-node-params amd--find-file-matching
   amd--current-files-matching amd--file-name amd--relative-file-name
   amd--project-file-name amd--module amd--rewrite-path
   amd--buffer-directory amd--use-relative-file-name-p
   amd--inside-imports-p amd--imports-node-p amd--define-node-p
   amd--enclosing-scopes amd--symbol-defined-in-scope-chain-p
   amd--file-search-regexp amd--file-replace-regexp
   amd-initialize-makey-group))
"##;
    let expect = expect![[
        r#"OK ((amd-mode (&optional arg) t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "2e72f870d104caa6174e293a92206d93544172435aab46f5dd5afc8777008792") (amd-kill-buffer-module nil t (interactive nil) "67dd409c81afd0914d4ff11b187a806dc4d5c993db6a846aab79d10800967c57") (amd-search-references nil t (interactive nil) "0c5e31e72c4b0ae98e6343115b61e772cbf286866e4c603539a1a61f5c192414") (amd--xref-search-references (file) nil nil "f26af08b478975bc894beaddde4955d72a4e84799bf45082d2b23c3d889c7142") (amd--find-references (file) nil nil "d5b21af2981a3cb7fdc6dbdf0cf3e786abf1830e74770df0b9a921ba1c456bf2") (amd--make-xref (candidate) nil nil "7f12c4c170bb8dbbe4f8b1b8dae5e8a96f2735e9045a53a1a0615f67b6cc3197") (amd--xref-candidate (symbol match) nil nil "aeca13ead621bd5f856f4b087ec768101ed6597a1d1c249253b7c7c5f38e86b6") (amd--xref-false-positive (match name) nil nil "a14428cfa8db071b8859633c6d6ad5319ba7de2201fcc042eaf59afab0522f03") (amd-find-module-at-point nil t (interactive nil) "8a811d7193e37457c1b51eccad2ba11427931eac96724bc12c9862e02d08d099") (amd-auto-insert nil t (interactive nil) "a074eb5308074cfaedf9a4f33bee447624c858fb772b2a0250e813dd1703748d") (amd-import-file nil t (interactive nil) "a488c91a68baf27f42e9c7559f5f91a268fd8af4d8325cdca66ce819ae4985bb") (amd-rename-file nil t (interactive nil) "6eb45bd13d0ad3a3b72595e6ba527885688c436dda25c0157c71cbeefa9afaf3") (amd--replace-all-file-references (from buffer files) nil nil "0e2335f9135cb0fae95b48894bc6b718eed92932a260831159094b87cb050b2b") (amd--replace-references-in-file (from to file) nil nil nil) (amd-import-module (module) t (interactive (list (read-string "Import module name: " (word-at-point)))) "16ed61eedb5440c415e87cfc92b7cb7ddb128b803ca40fa9fa2d068fe1d3c62b") (amd-kill-line nil t (interactive nil) "d33c815c3f2bc84a66cfbb18d0ef5b9d4e0b8fa43ad29266890d05472c38a2ab") (amd-kill-module nil nil nil nil) (amd--remove-module-from-params nil nil nil nil) (amd-move-line-up nil t (interactive nil) "35f0ec7a701df3a7288d1b86fa90668421474ea6432a85e360b2b1a703edb036") (amd-move-line-down nil t (interactive nil) "ed0ac97ce8dcf8e9b07436e286b3568cb0135e42aa2209b20ae6a16facd87519") (amd--guard nil nil nil "738d6203edb0ccee2a230351926b614505767d320f21dc8483ed29f2d789c41e") (amd--move-module-up nil nil nil nil) (amd--move-module-down nil nil nil nil) (amd--move-module (offset) nil nil nil) (amd--delete-function-params (node) nil nil nil) (amd--set-function-params (node params) nil nil nil) (amd--define-function-node nil nil nil nil) (amd--import (file-or-name) nil nil "c9c98ea59056f96b87cbebca7446fce4f3fa59d721c12534f354b50fe48082fa") (amd--imported-modules nil nil nil "7df00ea00c34a804ba6192294e53dc71467e39631749c5bc4cfaec963574771c") (amd--insert-dependency (file-or-name) nil nil "025640595409d5e8617f61fb7b4b2c608cc1d0d476be875d9545477ef9917a64") (amd--insert-module-name (name) nil nil nil) (amd--module-name (file) nil nil nil) (amd--goto-define nil nil nil nil) (amd--goto-define-function-params nil nil nil nil) (amd--goto-define-function nil nil nil nil) (amd--number-of-named-modules nil nil nil "8be27317efdf58e64ee09adc95dad0e11e99a95794e9965619954c8265956e1d") (amd--goto-imports nil nil nil "4336c1362b22f9a678fd86f6c59e9a43758991e5f34c21d8340e27ece257311d") (amd--buffer-file-name (&optional buffer) nil nil "ec5832dcbf1b72f04eb197e1967dbcdbc6a870cca45a82d503d56821816acf7a") (amd--buffer-module nil nil nil nil) (amd--node-content (node) nil nil nil) (amd--function-node-params (node) nil nil nil) (amd--find-file-matching (name) nil nil "5cfce3937346f5a48017c77df0b324300caa64c8f34e4f659c9c6878e58aedc8") (amd--current-files-matching (name) nil nil nil) (amd--file-name (file) nil nil "a97fa06e951130d7fd558f594d4e6204b9cf3aea62026497c68e3a9f0dadc178") (amd--relative-file-name (file) nil nil "5d01777e8fba8a37fe7fd664cbe844e13664f192214e37fd88cce49340bb7357") (amd--project-file-name (file) nil nil "b82cdbab49690fa382e3d513d223c011d360ba95075e6c297246ebc3be113504") (amd--module (file-or-name) nil nil "4791577e7ceb7e041b5a12448ab537ad3a2f3951f29b25d009b0c61d918dc0ce") (amd--rewrite-path (path) nil nil "fa443a4241ecc5b425a69b0d76481cdd84e54321b12e8003d6c4701509cfe51d") (amd--buffer-directory (&optional buffer) nil nil "1c8cf94c690485037fd450c864af03e7a0f22f5f5b2d4cf0283e3848d0d12740") (amd--use-relative-file-name-p (file) nil nil "31002bcac3dbf00b377801ad748175e0e924d1df6d706cdc113f9529edeaebb9") (amd--inside-imports-p nil nil nil nil) (amd--imports-node-p (node) nil nil nil) (amd--define-node-p (node) nil nil nil) (amd--enclosing-scopes (node) nil nil "caac2c90700b5ba17343d69b54564d3c63ce21afc5c05f7ef11dbe22d8c92d0a") (amd--symbol-defined-in-scope-chain-p (symbol node) nil nil "54e4b78815fb23cde8753f7789c2abe45b3889d1d7aecd278f84da2bc80c6ea8") (amd--file-search-regexp (name) nil nil "c5970b38730bb525b5f688537ceae4575dbccb976eb2033b615946cfc008afff") (amd--file-replace-regexp nil nil nil nil) (amd-initialize-makey-group nil t (interactive nil) nil))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn minor_mode_lifecycle_lighter_keymap_and_bindings_are_stable() {
    let elisp_form = r##"
(with-temp-buffer
  (let ((before (list amd-mode
                      (assq 'amd-mode minor-mode-alist))))
    (amd-mode 1)
    (let ((enabled
           (list amd-mode
                 (assq 'amd-mode minor-mode-alist)
                 (current-minor-mode-maps)
                 (mapcar
                  (lambda (key)
                    (lookup-key amd-mode-map (kbd key)))
                  '("C-c C-a" "C-k"
                    "<C-S-up>" "<C-S-down>")))))
      (amd-mode -1)
      (list before enabled amd-mode))))
"##;
    let expect = expect![[
        r#"OK ((nil #1=(amd-mode " AMD")) (t #1# ((keymap (C-S-down . amd-move-line-down) (C-S-up . amd-move-line-up) (11 . amd-kill-line) (3 keymap (1 . amd-initialize-makey-group)))) (amd-initialize-makey-group amd-kill-line amd-move-line-up amd-move-line-down)) nil)"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_loads_without_exposing_any_amd_function() {
    let elisp_form = r##"
(list
 (featurep 'amd-mode)
 (featurep 'amd-mode-autoloads)
 (mapcar
  (lambda (symbol)
    (list symbol
          (fboundp symbol)
          (and (fboundp symbol)
               (autoloadp
                (symbol-function symbol)))))
  '(amd-mode amd-auto-insert amd-import-file
    amd-import-module amd-find-module-at-point
    amd-search-references amd-kill-buffer-module)))
"##;
    let expect = expect![
        "OK (nil t ((amd-mode nil nil) (amd-auto-insert nil nil) (amd-import-file nil nil) (amd-import-module nil nil) (amd-find-module-at-point nil nil) (amd-search-references nil nil) (amd-kill-buffer-module nil nil)))"
    ];
    assert_amd_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn guard_distinguishes_real_projectile_project_from_outside_directory() {
    let elisp_form = r##"
(let ((root (amd-test-project "guard-project"))
      (outside
       (expand-file-name
        "outside/"
        (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
  (make-directory outside t)
  (list
   (let ((default-directory outside))
     (condition-case error-data
         (amd--guard)
       (error
        (cons (car error-data)
              (cdr error-data)))))
   (let ((default-directory root))
     (list
      (projectile-project-p)
      (amd--guard)
      (projectile-project-root)))))
"##;
    let expect = expect![[
        r#"OK (nil ("[ORACLE-SANDBOX]/guard-project/" nil "[ORACLE-SANDBOX]/guard-project/"))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}
