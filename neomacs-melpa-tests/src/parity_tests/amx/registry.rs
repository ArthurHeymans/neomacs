use expect_test::expect;

use super::{assert_amx_autoload_parity, assert_amx_parity};

#[test]
fn exact_release_descriptor_dependency_and_installed_source_are_stable() {
    let elisp_form = r##"
(let* ((descriptor (cadr (assq 'amx package-alist)))
       (extras (package-desc-extras descriptor))
       (s-descriptor (cadr (assq 's package-alist))))
  (list
   (package-desc-name descriptor)
   (package-version-join (package-desc-version descriptor))
   (package-desc-reqs descriptor)
   (alist-get :url extras)
   (alist-get :commit extras)
   (alist-get :revdesc extras)
   (package-version-join
    (package-desc-version s-descriptor))
   (package-installed-p 's '(0))
   (featurep 's)
   (file-name-nondirectory (locate-library "s"))
   (and
    (string-prefix-p
     (file-name-as-directory package-user-dir)
     (locate-library "s"))
    t)))
"##;
    let expect = expect![[
        r#"OK (amx "20230413.1210" ((emacs (24 4)) (s (0))) "https://github.com/DarwinAwardWinner/amx/" "1c2428d21e9d2ee8bee944b572a39ca8c91ca13b" "1c2428d21e9d" "20220902.1511" t t "s.el" t)"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn all_custom_options_defaults_types_setters_and_groups_are_stable() {
    let elisp_form = r##"
(mapcar
 (lambda (variable)
   (list
    variable
    (default-value variable)
    (custom-variable-p variable)
    (get variable 'standard-value)
    (get variable 'custom-type)
    (get variable 'custom-set)
    (get variable 'custom-group)))
 '(amx-mode
   amx-debug-mode
   amx-auto-update-interval
   amx-save-file
   amx-history-length
   amx-show-key-bindings
   amx-prompt-string
   amx-ignored-command-matchers
   amx-backend))
"##;
    let expect = expect![[
        r#"OK ((amx-mode nil #1=((funcall #'#[nil (nil) #2=(cl-struct-amx-backend-tags helm-comp-read-map ido-text ido-setup-hook ido-completion-map ido-ubiquitous-mode ivy-text ivy-mode smex-save-file amx-backend amx-history amx-data amx-cache t)])) #1# boolean custom-set-minor-mode nil) (amx-debug-mode nil #3=((funcall #'#[nil (nil) #2#])) #3# boolean custom-set-minor-mode nil) (amx-auto-update-interval nil #4=((funcall #'#[nil (nil) #2#])) #4# (choice (const :tag "Disabled" nil) (number :tag "Minutes")) amx-set-auto-update-interval nil) (amx-save-file "~/.emacs.d/amx-items" #5=((funcall #'#[nil ((locate-user-emacs-file "amx-items" ".amx-items")) #2#])) #5# (choice (string :tag "File name") (const :tag "Don't save" nil)) amx-set-save-file nil) (amx-history-length 7 #6=((funcall #'#[nil (7) #2#])) #6# integer nil nil) (amx-show-key-bindings t #7=((funcall #'#[nil (t) #2#])) #7# boolean nil nil) (amx-prompt-string "M-x " #8=((funcall #'#[nil ("M-x ") #2#])) #8# string nil nil) (amx-ignored-command-matchers ("\\`self-insert-command\\'" "\\`self-insert-and-exit\\'" "\\`ad-Orig-" "\\`menu-bar" "\\`kill-emacs\\'" amx-command-marked-ignored-p amx-command-obsolete-p amx-command-mouse-interactive-p) #9=((funcall #'#[nil ('("\\`self-insert-command\\'" "\\`self-insert-and-exit\\'" "\\`ad-Orig-" "\\`menu-bar" "\\`kill-emacs\\'" amx-command-marked-ignored-p amx-command-obsolete-p amx-command-mouse-interactive-p)) #2#])) #9# (repeat (choice (regexp :tag "Regular expression") (function :tag "Function"))) nil nil) (amx-backend auto #10=((funcall #'#[nil ('auto) (selectrum-should-sort-p selectrum-should-sort . #2#)])) #10# (choice (const :tag "Auto-select" auto) (const :tag "Ido" ido) (const :tag "Ivy" ivy) (const :tag "Helm" helm) (const :tag "Selectrum" selectrum) (const :tag "Standard" standard) (symbol :tag "Custom backend")) amx-set-backend nil))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn backend_struct_and_five_builtin_backend_specs_have_exact_semantics() {
    let elisp_form = r##"
(list
 (mapcar
  (lambda (function)
    (list function
          (fboundp function)
          (copy-tree
           (help-function-arglist function t))))
  '(make-amx-backend
    copy-amx-backend
    amx-backend-p
    amx-backend-name
    amx-backend-required-feature
    amx-backend-comp-fun
    amx-backend-get-text-fun
    amx-backend-exit-fun
    amx-backend-auto-activate))
 (mapcar
  (lambda (name)
    (let ((backend (plist-get amx-known-backends name)))
      (list
       name
       (amx-backend-p backend)
       (amx-backend-name backend)
       (amx-backend-required-feature backend)
       (let ((value
              (amx-backend-comp-fun backend)))
         (if (symbolp value)
             value
           (and (functionp value) 'function)))
       (let ((value
              (amx-backend-get-text-fun backend)))
         (if (symbolp value)
             value
           (and (functionp value) 'function)))
       (let ((value
              (amx-backend-exit-fun backend)))
         (if (symbolp value)
             value
           (and (functionp value) 'function)))
       (amx-backend-auto-activate backend))))
  '(standard ido ivy helm selectrum auto)))
"##;
    let expect = expect![
        "OK (((make-amx-backend t (&rest --cl-rest--)) (copy-amx-backend t (arg)) (amx-backend-p t (x)) (amx-backend-name t (x)) (amx-backend-required-feature t (x)) (amx-backend-comp-fun t (x)) (amx-backend-get-text-fun t (x)) (amx-backend-exit-fun t (x)) (amx-backend-auto-activate t (x))) ((standard t standard nil amx-completing-read-default amx-default-get-text amx-default-exit-minibuffer nil) (ido t ido ido-completing-read+ amx-completing-read-ido amx-ido-get-text amx-default-exit-minibuffer (or (bound-and-true-p ido-mode) (bound-and-true-p ido-ubiquitous-mode))) (ivy t ivy ivy amx-completing-read-ivy amx-ivy-get-text amx-default-exit-minibuffer (bound-and-true-p ivy-mode)) (helm t helm helm amx-completing-read-helm amx-default-get-text helm-confirm-and-exit-minibuffer (bound-and-true-p helm-mode)) (selectrum t selectrum selectrum amx-completing-read-selectrum amx-default-get-text amx-default-exit-minibuffer (bound-and-true-p selectrum-mode)) (auto t auto nil amx-completing-read-auto function function nil)))"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn complete_declared_function_inventory_signatures_commands_and_docs_are_stable() {
    let elisp_form = r##"
(mapcar
 (lambda (function)
   (let ((documentation (documentation function)))
     (list
      function
      (help-function-arglist function t)
      (commandp function)
      (interactive-form function)
      (and documentation
           (secure-hash 'sha256 documentation)))))
 '(amx-mode amx-debug-mode amx--debug-message
   amx-set-auto-update-interval amx-set-save-file
   amx-get-command-name amx-get-command-symbol amx-get-default
   amx-active amx amx-update-and-rerun amx-read-and-run
   amx-major-mode-commands amx-prepare-ido-bindings
   amx-default-exit-minibuffer amx-completing-read
   amx-prompt-with-prefix-arg amx-define-backend amx-get-backend
   amx-completing-read-default amx-default-get-text
   amx-completing-read-ido amx-ido-get-text
   amx-completing-read-ivy amx-ivy-get-text
   amx-completing-read-helm amx-completing-read-selectrum
   amx-auto-select-backend amx-completing-read-auto
   amx-load-backend amx-set-backend amx-rebuild-cache
   amx-restore-history amx-sort-according-to-cache amx-update
   amx-detect-new-commands amx-update-if-needed amx-initialize
   amx-buffer-not-empty-p amx-load-save-file amx-save-history
   amx-pp* amx-pp amx-save-to-file amx-sorting-rules
   amx-rank amx-update-counter amx-sort-item-at
   amx-detect-position amx-remove-nth-cell amx-insert-cell
   amx-make-keybind-hash amx-augment-command-with-keybind
   amx-augment-commands-with-keybinds amx-clean-command-name
   amx-command-ignored-p amx-command-marked-ignored-p
   amx-command-obsolete-p amx-command-mouse-interactive-p
   amx-ignore-command amx-unignore-command amx-exit-minibuffer
   amx-do-with-selected-item amx-describe-function amx-where-is
   amx-find-function amx-extract-commands-from-keymap
   amx-parse-keymap amx-extract-commands-from-features
   amx-show-unbound-commands amx-post-eval-force-update
   amx-idle-update))
"##;
    let expect = expect![[
        r#"OK ((amx-mode (&optional arg) t (interactive #1=(list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "3f63583d96ea63ce360c7353977ac8e2dfa82e1affe7114cae6cfa66a1fdda91") (amx-debug-mode (&optional arg) t (interactive #1#) "15d4c455f438862d77f3cb53464fcb557bf337c67500d56442136a6da575c1e7") (amx--debug-message (format-string &rest args) nil nil "519fac4b22475ddd29fb75b2fd992343e4b67eea93876c4584a2e3343b454368") (amx-set-auto-update-interval (symbol value) nil nil "122a8811dabc76b4190802c5c6e563cd87853b4e8108cba57ec703ffe80d0681") (amx-set-save-file (symbol value) nil nil "ee5bf0f98f403a78d2040ffd020df5d467acd0421c4beab4dd05f6907f8a7eb4") (amx-get-command-name (cmd) nil nil "1492cbb1714adac4e7df8921e6ffb82d0d7c3089ee69b5c7b455959776c8c035") (amx-get-command-symbol (cmd &optional force) nil nil "f98ea44441f2bd4e08021c901bef05c1e71284ed3e13666442d50ac22187a70f") (amx-get-default (choices &optional bind-hash) nil nil "47d48dea1be6c77bbe3aa5637319ed1d798e7d33c5b3188358d79333bb61a713") (amx-active nil nil nil "f4c28a1f2632dd9e40401a4f200b28b53d1ba7d3cc484da070d3642a06ec581d") (amx nil t (interactive nil) "f3c268285df5f9b1ee63a5ceefdca8a94d2b6c3ede14a54ed73c666e9a1b1b84") (amx-update-and-rerun nil nil nil "b8edc562e6320f615260c40ff2c60dbe56711663523bd00a1ee933c3de6cb8f0") (amx-read-and-run (commands &optional initial-input) nil nil "dac95f4966b51a12cc95c680959897b8e9a5c2fbea9a2830f02341573963b5f4") (amx-major-mode-commands nil t (interactive nil) "990b95525acfef8769073ae24dd0d259a55d1621c2e80cedc6159821f807a111") (amx-prepare-ido-bindings nil nil nil "083821533337259933813acefa503feddd39764018e7f7772871ec1fa5f7f8ad") (amx-default-exit-minibuffer nil nil nil "cf2fad5b4dcfe13102f32bc187732ddd3b0fa78ca9f3dcc7a08e722a36eba909") (amx-completing-read (choices &rest --cl-rest--) nil nil "1b69d8e3a7856674f98f395bde25e9f0e2677d6541c1343d40f1a19fdf91f86b") (amx-prompt-with-prefix-arg nil nil nil "6f154055a3444dbbddb309a1d6db74a960cf155d7cb77e1b6a37b6083cc15b32") (amx-define-backend (&rest --cl-rest--) nil nil "f8dfde599360f67628fcbeda16af36c5bc86bc18677554da0094e04060c1fa68") (amx-get-backend (&rest --cl-rest--) nil nil "e59943124bc229e3a8ace443851e18399edb857ef91c36e1fd2339b74c3349f6") (amx-completing-read-default (choices &rest --cl-rest--) nil nil "1c4155b2ede2f684f40eb3044958ff9e257875c3d8791b9686fee6da46927e3e") (amx-default-get-text nil nil nil "853c61f0faa45ddc8c56fa74fc6842b82861d5f0df6d0df972d1d3e447ea4aba") (amx-completing-read-ido (choices &rest --cl-rest--) nil nil "5631de9a41c2f3624680ee2cb114438240f6f810f663c12d68ba1b27c77dad0d") (amx-ido-get-text nil nil nil "3c8767c52f4866c9c9565c64b30537a8108c1d7e3eea2092e7d4a58a256cc5f7") (amx-completing-read-ivy (choices &rest --cl-rest--) nil nil "ffc0d2cbfa7a96620124ceb24e21d3ade3868d78e2a3ccec8167e843965e50ac") (amx-ivy-get-text nil nil nil "ff44cf9adc8818dbddf1c2dceca3c41f6e8095c75d801023fe4c894d08c40c3e") (amx-completing-read-helm (choices &rest --cl-rest--) nil nil "d1986c16f2f64862140d032d498c1dc67d492c117817cb32595866c159c589e8") (amx-completing-read-selectrum (choices &rest --cl-rest--) nil nil "2571ef166a29ecbae1416913356e43b3419ebc8f97dbe4e4602a497eb0d68752") (amx-auto-select-backend nil nil nil nil) (amx-completing-read-auto (choices &rest --cl-rest--) nil nil "a5dd732f7b85c1da12110b44f3dfa66488c71e0e99fa869e9dbba453d6f31ed2") (amx-load-backend (backend) nil nil "37ec3aecb3f855aaf84be846aeac0c014bacc5c91464a281d5876d119fe81549") (amx-set-backend (symbol value) nil nil "10ad62e1432be46ea88882929c6673d2f8ef2513d68fd58bec77dbb819fad494") (amx-rebuild-cache nil nil nil "d6f85cca057be81d7295290a7f2d29bab6a24c35d3b317f0709c26cc7a447a65") (amx-restore-history nil nil nil "55229418c7b5f07e0a3e4aef3351747035f9b2a954d4d9ab32726420d385301b") (amx-sort-according-to-cache (list) nil nil "a475b922a73c0fe26bdee71765aaff75609f4f9086cd7342c687468e56369ef4") (amx-update nil t (interactive nil) "701eb2b56182a47e6ead95ed258475d4dc8aa42a983b86dff8ffe0f8d6601836") (amx-detect-new-commands nil nil nil "0c2e452cc000d211fbce161f7169dcc0ecce5cadf7e251a67ae74a36af0bccce") (amx-update-if-needed (&optional count-commands) nil nil "e8265111a492b026b9eb5d052eef81c17019cbb9772c60e15cfa2cd7c16cc1de") (amx-initialize (&optional reinit) t (interactive "P") "e8efbab0b5788fe1103fa747e5524001f35dca911eb67bcba23951a0aa48f731") (amx-buffer-not-empty-p nil nil nil "bdb3dd7365ba4f8dcc3c8efa5bf959e8230a4b12969e4fde61333c6faf636d3d") (amx-load-save-file nil nil nil "8d0fea789a5ea82813f4ef2aa2b354ec14b94a18ca33e07f4dca11933fef14dd") (amx-save-history nil nil nil "d3022f7804a3306a357995daaa51d814baa4726cf7f12f85d6e6d2ecbfdcea4c") (amx-pp* (list list-name) nil nil "02dd94ce35f00ce7f2e8e73c9159dde44d1cb2d5ba8e12b0596510e5ec940d5f") (amx-pp (list-var) nil nil "c61bf25d4bef4f237168fd88418ffc63779a7ced20c9a838d58bf76ec6a37147") (amx-save-to-file nil t (interactive nil) "565e01cd5aae4a19470958f717739a23535728f331e9057b8b5c1f56b91ab161") (amx-sorting-rules (command-item other-command-item) nil nil "9b2a2bfe664230a7ebfeb7284c0cda38a3229fad5f54971be25dc9d806103b9f") (amx-rank (command) nil nil "f2c7fea629246c0e2a88355d280babac3a54981ed085b823631d799533a673dc") (amx-update-counter (command-item) nil nil "27e978b3982d95ee948bc7d73ead716d96ff801d504c2ae0150d7c1e6e1a57a3") (amx-sort-item-at (n) nil nil "49f9df4d6371e912331fc2c83a9448935617cfc1e346d191bbcf2204af951d7f") (amx-detect-position (cell pred) nil nil "0a13c49f330b23bec8876c62a3ee81ea968087f1856b7b6e1e962e5a4343957c") (amx-remove-nth-cell (n list) nil nil "5b7d069149143a7533866d7fc4d3f4e2fb3dafe2f0d18c1247b89a74c6e6e017") (amx-insert-cell (new-cell n list) nil nil "8f9140429f9fc34b611da8f94634a3d06906924572e1881329c895f3d493072b") (amx-make-keybind-hash (&optional keymap) nil nil "9351a68190804f606a0f3318e3f3f69afa10e9ce1a0e52b544f36d6e2ec4029c") (amx-augment-command-with-keybind (command &optional bind-hash) nil nil "44fb9394e7d5e1f48a19d2864e1243c2f04acf489978d0f007e2b8d3b990c14d") (amx-augment-commands-with-keybinds (commands &optional bind-hash) nil nil "74d6f3810107ec7520208ffa56db866f1063d2c7f8ddce9e801eb1214a54407c") (amx-clean-command-name (command-name) nil nil "4d5fb768579be86c246f9093b4eecfae3728b07b9b05571024d4016c867755e1") (amx-command-ignored-p (command) nil nil "4a1cde4d74ff6280804668bac9888677f7a24a15c074b80d6006a9b65ec40d70") (amx-command-marked-ignored-p (command) nil nil "ee92461c274873f4d58c607fe39170cfdf21be73d94a72d510a07d9b23f7ef4c") (amx-command-obsolete-p (command) nil nil "2faf73313e59deaf77b499c03a141655897d3f3827c08303ae98a343838794d3") (amx-command-mouse-interactive-p (command) nil nil "856e6a1725d9c109b05dc23a81fc0f7b0a13bac5522f3708094a3fe4c36a8571") (amx-ignore-command (command &rest --cl-rest--) t (interactive (list (let ((amx-temp-prompt-string "Ignore command: ")) (amx-completing-read amx-cache :predicate #'(lambda (cmd) (not (amx-command-ignored-p cmd))))))) "4770ea186f5abe965bb053db4bc40c3b08753222ceeb977511f15db25533ace2") (amx-unignore-command (command) t (interactive (list (let ((amx-temp-prompt-string "Un-ignore command: ")) (amx-completing-read amx-cache :predicate #'amx-command-marked-ignored-p)))) "653d436f5b728d98a45aed0ba2f50059c04fde9245bc9847c41fe9bf2c577a22") (amx-exit-minibuffer nil t (interactive nil) "e1799bbce090e18a69a67a69d2e6752a59587b01b714ea451e3baf874f2b0f89") (amx-do-with-selected-item (fn) nil nil "a62b3a12bf09170f7ab787e207c80f76289a1095763f3644a1d66e271cc38a6f") (amx-describe-function nil t (interactive nil) "2f851d449122f3206e7ac0f51df32347d932ee5c03fbe41d7afa748647edf648") (amx-where-is nil t (interactive nil) "9b17fda9424820a486ea11a5187c897e1ec3d2395cd1267874a86542220c2efd") (amx-find-function nil t (interactive nil) "cd131719390edc0eb09f8e5884402b3309a2540c61c5c82b3de6fb452aed0365") (amx-extract-commands-from-keymap (keymap) nil nil nil) (amx-parse-keymap (keymap commands) nil nil nil) (amx-extract-commands-from-features (mode) nil nil nil) (amx-show-unbound-commands nil t (interactive nil) "7c0cbabb6581fac6560fafa8b4c5de3ebeeae9ba7aeaa9433057a2ba97d8a350") (amx-post-eval-force-update (&rest _args) nil nil "70901e59f8cce15152663a030d77a7e2e6b4e5689b5fa86fd2ab5f4273f8bbc8") (amx-idle-update (&optional force) nil nil "2b68e3de591b4635f07d2cac94c616c23525a376d5ebab31f89e7249e5e0d2f5"))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_expose_only_the_public_entry_commands() {
    let elisp_form = r##"
(list
 (featurep 'amx)
 (featurep 'amx-autoloads)
 (mapcar
  (lambda (function)
    (let ((definition (symbol-function function)))
      (list
       function
       (commandp function)
       (autoloadp definition)
       (and (autoloadp definition) (nth 1 definition))
       (and (autoloadp definition) (nth 3 definition))
       (and (autoloadp definition) (nth 4 definition)))))
  '(amx-mode amx amx-major-mode-commands amx-initialize))
 (mapcar
  (lambda (function)
    (list function (fboundp function)))
  '(amx-rank amx-completing-read amx-ignore-command)))
"##;
    let expect = expect![[
        r#"OK (nil t ((amx-mode t t "amx" t nil) (amx t t "amx" t nil) (amx-major-mode-commands t t "amx" t nil) (amx-initialize t t "amx" t nil)) ((amx-rank nil) (amx-completing-read nil) (amx-ignore-command nil)))"#
    ]];
    assert_amx_autoload_parity(elisp_form, expect);
}

#[test]
fn source_initialization_registers_backends_advice_and_one_short_idle_timer() {
    let elisp_form = r##"
(list
 (featurep 'amx)
 (featurep 's)
 (nreverse amx-test-timer-events)
 amx-short-idle-update-timer
 amx-long-idle-update-timer
 (mapcar
  (lambda (function)
    (list function
          (and
           (advice-member-p
            #'amx-post-eval-force-update function)
           t)))
  '(load eval-last-sexp eval-buffer eval-region
    eval-expression autoload-do-load))
 (mapcar
  (lambda (name)
    (amx-backend-name
     (plist-get amx-known-backends name)))
  '(standard ido ivy helm selectrum auto)))
"##;
    let expect = expect![
        "OK (t t ((schedule amx-test-timer-1 1 t amx-idle-update nil)) amx-test-timer-1 nil ((load t) (eval-last-sexp t) (eval-buffer t) (eval-region t) (eval-expression t) (autoload-do-load t)) (standard ido ivy helm selectrum auto))"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn global_mode_lifecycle_remaps_extended_command_and_manages_save_hook() {
    let elisp_form = r##"
(let ((amx-mode nil)
      (auto-save-hook nil)
      events)
  (cl-letf
      (((symbol-function 'amx-initialize)
        (lambda (&optional reinit)
          (push (list 'initialize reinit) events))))
    (amx-mode 1)
    (let ((enabled
           (list
            amx-mode
            (key-binding
             [remap execute-extended-command])
            (memq 'amx-save-to-file auto-save-hook))))
      (amx-mode -1)
      (list
       enabled
       amx-mode
       (key-binding
        [remap execute-extended-command])
       (memq 'amx-save-to-file auto-save-hook)
       (nreverse events)))))
"##;
    let expect = expect!["OK ((t amx (amx-save-to-file)) nil nil nil ((initialize nil)))"];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn debug_mode_emits_timestamped_messages_only_while_enabled() {
    let elisp_form = r##"
(let (events)
  (cl-letf
      (((symbol-function 'message)
        (lambda (&rest arguments)
          (push arguments events)))
       ((symbol-function 'format-time-string)
        (lambda (&rest _) "2026-07-27T12:34:56.123456-0400")))
    (let ((amx-debug-mode nil))
      (amx--debug-message "hidden %s" 'event))
    (let ((amx-debug-mode t))
      (amx--debug-message "visible %s" 'event))
    (list
     (nreverse events)
     (default-value 'amx-debug-mode)
     (custom-variable-p 'amx-debug-mode))))
"##;
    let expect = expect![[
        r#"OK ((("amx (%s): visible %s" "2026-07-27T12:34:56.123456-0400" event)) nil ((funcall #'#[nil (nil) (cl-struct-amx-backend-tags helm-comp-read-map ido-text ido-setup-hook ido-completion-map ido-ubiquitous-mode ivy-text ivy-mode smex-save-file amx-backend amx-history amx-data amx-cache t)])))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}
