use expect_test::expect;

use super::{assert_apheleia_autoload_parity, assert_apheleia_parity};

#[test]
fn apheleia_exact_pin_descriptor_dependencies_and_origin_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'apheleia
                      package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-reqs descriptor)
          (package-desc-summary descriptor)
          (copy-tree
           (package-desc-extras descriptor))
          (featurep 'apheleia)
          (mapcar
           #'featurep
           '(apheleia-utils
             apheleia-dp
             apheleia-formatter-context
             apheleia-log
             apheleia-formatters
             apheleia-rcs))))"##;
    let expect = expect![[
        r#"OK (apheleia "20260619.1935" ((emacs (27))) "Reformat buffer stably." ((:maintainers ("Radian LLC" . "contact+apheleia@radian.codes")) (:authors ("Radian LLC" . "contact+apheleia@radian.codes")) (:keywords "tools") (:revdesc . "14a0bb4454fb") (:commit . "14a0bb4454fb2cc3b5b377619288b742ce117da5") (:url . "https://github.com/radian-software/apheleia")) t (t t t t t t))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_installed_payload_has_exact_inventory_sizes_and_content_digests() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'apheleia
                    package-alist)))
                 (directory
                  (package-desc-dir descriptor))
                 (selected
                  '("apheleia-dp.el"
                    "apheleia-formatter-context.el"
                    "apheleia-formatters.el"
                    "apheleia-log.el"
                    "apheleia-pkg.el"
                    "apheleia-rcs.el"
                    "apheleia-utils.el"
                    "apheleia.el"
                    "scripts/formatters/apheleia-docformatter"
                    "scripts/formatters/apheleia-from-project-root"
                    "scripts/formatters/apheleia-mix-format"
                    "scripts/formatters/apheleia-npx"
                    "scripts/formatters/apheleia-phpcs"
                    "scripts/formatters/apheleia-pkl"
                    "scripts/formatters/pnp-bin.js"))
                 (all-files
                  (sort
                   (mapcar
                    (lambda (path)
                      (file-relative-name
                       path
                       directory))
                    (directory-files-recursively
                     directory
                     ".*"
                     nil))
                   #'string<)))
         (list
          all-files
          (mapcar
           (lambda (relative)
             (let ((path
                    (expand-file-name
                     relative
                     directory)))
               (list
                relative
                (nth
                 7
                 (file-attributes path))
                (secure-hash
                 'sha256
                 path)
                (file-executable-p path))))
           selected)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" "apheleia-autoloads.el" "apheleia-dp.el" "apheleia-dp.elc" "apheleia-formatter-context.el" "apheleia-formatter-context.elc" "apheleia-formatters.el" "apheleia-formatters.elc" "apheleia-log.el" "apheleia-log.elc" "apheleia-pkg.el" "apheleia-rcs.el" "apheleia-rcs.elc" "apheleia-utils.el" "apheleia-utils.elc" "apheleia.el" "apheleia.elc" "scripts/formatters/apheleia-docformatter" "scripts/formatters/apheleia-from-project-root" "scripts/formatters/apheleia-mix-format" "scripts/formatters/apheleia-npx" "scripts/formatters/apheleia-phpcs" "scripts/formatters/apheleia-pkl" "scripts/formatters/pnp-bin.js") (("apheleia-dp.el" 7340 "c657e51033e8fcae3e5f33cfa4a3f8f5e001e15e786855e9df4d2e61f158c186" nil) ("apheleia-formatter-context.el" 1998 "36089cb489ecd0249d4ef0bf80b9569e3c959227954c1b7d362f9393b81566f3" nil) ("apheleia-formatters.el" 64846 "645faa2c29f25828f3b60cb28e902a2673a7f54e2b6d357ea7b7762e9706699c" nil) ("apheleia-log.el" 5563 "ee73f9859fddcf1c3465e200abb808df3fbc1d697d74b9cd4e95b5cef66d02d2" nil) ("apheleia-pkg.el" 427 "815308b542be18ec68ac057efb410303afc888b155dc1dc30490af6696de0530" nil) ("apheleia-rcs.el" 9997 "a5daff0e94ac3902f377537ff29ad8be3ba1d609e66112917fb1321a43a96421" nil) ("apheleia-utils.el" 5322 "46634b1c4f13ccfe0522b2abbdf8ef7fcf5072193deae6d428a3eb37f895522b" nil) ("apheleia.el" 12490 "0480ffd6e1aab40fa846b4bbb7f59a846c7ee4595bde2827e923f31c75a5dd45" nil) ("scripts/formatters/apheleia-docformatter" 77 "bbee117c6ad27a3845a5d66c3e9e054c244d33e2e36eafe5b4e5f0b976fed96f" t) ("scripts/formatters/apheleia-from-project-root" 1238 "46ae68baee5fa1f3f5b2ced0ad88baac39f8b69f7b1e290b1f12aeb1b3bc7293" t) ("scripts/formatters/apheleia-mix-format" 782 "28bb82da1b4b969dc698c643fac84a18a299c0364dca3e9eed0b3ab670646c49" t) ("scripts/formatters/apheleia-npx" 2534 "120d531f6fb2e81ba3029d053016ca908a29e48d8ed994b3e353e9c23ece7635" t) ("scripts/formatters/apheleia-phpcs" 60 "6611dce5c86e04190a9300795185cac91aea295c79813035859f002b29879520" t) ("scripts/formatters/apheleia-pkl" 58 "6c817cd139b5a0bb849babff1947f54a3a3f88aeb3bb00765e7a6cba8f79e700" t) ("scripts/formatters/pnp-bin.js" 298002 "37c807c7894603a1176bee41cb49de73af03e96ad74d5c325598cbbae1a53a58" t)))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_callable_surface_has_exact_commands_arglists_and_documentation_digests() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((documentation
                  (documentation
                   symbol
                   t)))
             (list
              symbol
              (and
               (fboundp symbol)
               t)
              (commandp symbol)
              (help-function-arglist
               symbol
               t)
              (and
               documentation
               (secure-hash
                'sha256
                documentation)))))
         '(apheleia-format-buffer
           apheleia-format-after-save
           apheleia-mode
           apheleia-mode-maybe
           apheleia-global-mode
           apheleia-goto-error
           apheleia--buffer-hash
           apheleia--disallowed-p
           apheleia--with-on-error
           apheleia--map-rcs-patch
           apheleia--apply-rcs-patch
           apheleia--edit-distance-table
           apheleia--align-point
           apheleia-log--buffer-name
           apheleia-log--formatter-result
           apheleia--log
           apheleia-formatters-indent
           apheleia-formatters-js-indent
           apheleia-formatters-fill-column
           apheleia-formatters-locate-file
           apheleia-formatters-extension-p
           apheleia-formatters-mode-extension
           apheleia-formatters-local-buffer-file-name
           apheleia-mhtml-mode-predicate
           apheleia-script--formatter-not-available-p
           apheleia--make-process
           apheleia--call-process
           apheleia--execute-formatter-process
           apheleia--write-region-silently
           apheleia--save-buffer-silently
           apheleia--make-temp-file
           apheleia--create-rcs-patch
           apheleia--safe-buffer-name
           apheleia--replq
           apheleia--formatter-context
           apheleia--run-formatter-process
           apheleia--run-formatter-function
           apheleia-indent-lisp-buffer
           apheleia-reformat-bibtex-buffer
           apheleia--run-formatters
           apheleia--formatter-safe-p
           apheleia--ensure-list
           apheleia--get-mode-chain
           apheleia--get-formatters))"##;
    let expect = expect![[
        r#"OK ((apheleia-format-buffer t t (formatter &optional success-callback &rest --cl-rest--) "1163b218900eaf196e70b2a07d162aa2ae5ba07c7c3239ca4bcdc2569d16d46e") (apheleia-format-after-save t nil nil "771da4858ffe2bc6c28f32a3f24e2ada6cc1d4acbea9442a5c5c87ca499c402e") (apheleia-mode t t (&optional arg) "ec4cd150316677be53408a94c9118f9b4785891ba8a1c770c9ddb32d21710607") (apheleia-mode-maybe t nil nil "11b78747bc17e290ee034e2b4d4afbe586fc6b926e9fdc10ceabb604713b46d0") (apheleia-global-mode t t (&optional arg) "e4cd374ebd9188792673da07d7702f1506e6bb34d748e01044e987d1119a1fbf") (apheleia-goto-error t t nil "c6b8c3660b90689115f57587bfd613a06523b9230d90d6fbbf97519a5fca979d") (apheleia--buffer-hash t nil nil "5576f96ebccbdfe3cc23099acda2f3bceb519a2e4aef3d0f316dd4a125a48f62") (apheleia--disallowed-p t nil nil "dd0e777a08409235793176a4841d15fcdd808b2b3dc23952ed151cdf78de7eaf") (apheleia--with-on-error t nil (on-error &rest body) "a9b0f972674c2c26948557086abef4d4f5c3bf16d59a7a11b1299bfbc27a1884") (apheleia--map-rcs-patch t nil (func) "d3659367d8acf0c889b751796f6f881f03d703bc27bae04a2f4de2d6c4ca728d") (apheleia--apply-rcs-patch t nil (content-buffer patch-buffer) "2d313560f8240b4ebd62c0c2baacb1d914ba9c00d5c94924a142767779cbc387") (apheleia--edit-distance-table t nil (s1 s2) "f0a2a9c65d888b8e90845a3be4d9aea5baa3fe9b86de357212a187f3d4b45a97") (apheleia--align-point t nil (s1 s2 p1) "e4b0fcf3b7b94e6fdac884a555f00a8142e271d65deba60a214a71a87336d659") (apheleia-log--buffer-name t nil (formatter) "2b05504509992a59d4fddae00d2cb7c3754a3aaeefecb82a3acd802aafb0f476") (apheleia-log--formatter-result t nil (ctx log-buffer exit-ok directory stderr-string) "7f45f7b9bbc724feb63e63bdde01c51e333b69eb1dee5ebd171139c494d87bd6") (apheleia--log t nil (category message &rest args) "4e7a579e51043a60983d24ca526ab8a8fc2654ea4ed81f6d89bd04e59995ab09") (apheleia-formatters-indent t nil #1=(tab-flag indent-flag &optional indent-var) "b4d64308492ba3813b75f3cbfb88d06d3e9a094c3b33e8b71d4df19cc79b2082") (apheleia-formatters-js-indent t nil #1# "b4d64308492ba3813b75f3cbfb88d06d3e9a094c3b33e8b71d4df19cc79b2082") (apheleia-formatters-fill-column t nil (fill-flag) "eaddd9a6ca8f02fa62cf67289d8ecec376a511ea1ec399a300ae507bc948bafa") (apheleia-formatters-locate-file t nil (file-flag file-name) "d466fd2465ef079d47b897b54c99e8149bd4d1009fa1cfc456c80ee08ba11de8") (apheleia-formatters-extension-p t nil (&rest exts) "3758b8ee98fbbed4daf704ae0267245d0260e7d48cacd5e763a5b49a590a67fb") (apheleia-formatters-mode-extension t nil (&optional flag) "386100afe188fa772aa75a44442807d8643546c76562d05225e76fad4ea1676d") (apheleia-formatters-local-buffer-file-name t nil (&optional file-name) "2ebca38c27c2e7021f2e9978f83983689f07555bcddcd316ee68d65cbe976617") (apheleia-mhtml-mode-predicate t nil nil "0a30a5faf6917e132967c9cc93ea766b9341bfedd0712691e67bfa9635827cc7") (apheleia-script--formatter-not-available-p t nil (ctx stderr) "208e83f68f457fb06cd61eaec50a9f9866def475c11379b0f173867dd4d4bbbc") (apheleia--make-process t nil (&rest --cl-rest--) "563530b4d238272d4606862d66c98e5b820f212bd5b00afa72d3ea3bf22f55fb") (apheleia--call-process t nil (&rest --cl-rest--) "46e67e19d6a0b53e69803bd3fd46f84f19d42a0cbfd44e4f40d5f5d272f2bd14") (apheleia--execute-formatter-process t nil (&rest --cl-rest--) "90468e811c43946cb7cbec926cb07cae520b6f4b8a9072e826c6307bf48baf08") (apheleia--write-region-silently t nil (start end filename &optional append visit lockname mustbenew write-region) "e2678e13f59d885082e3e0dee03798a9386e17054d07aad7b690c3b9f3936fe1") (apheleia--save-buffer-silently t nil nil "e1f2e2829cf8978a11e56129027e0cdea7cec793e3d7af015cc13ef36a837262") (apheleia--make-temp-file t nil (remote prefix &optional dir-flag suffix) "5e1bc81e9f7621277e866d5e52f6a5bd583cdfa608ac15764b68f1f7dbce7162") (apheleia--create-rcs-patch t nil (old-buffer new-buffer remote callback) "8de22f1cb753aa8755506231255a1b9c5688a637ea4b3a3fac432721640d3362") (apheleia--safe-buffer-name t nil nil "c610436041d45feba7de2f6b3c45cc9184d95a99f36197541d2d9c33b2ec4150") (apheleia--replq t nil (dest in out) "c2177706e0ba30ee4719fc99381383765004d9536d06193764d0741095f5686a") (apheleia--formatter-context t nil (name command remote &optional stdin-buffer) "27cea183965ee3a5cd56b761fd5862cb4d2911ef2d61064903573b81a42872f2") (apheleia--run-formatter-process t nil (command buffer remote callback stdin formatter) "b0953219a72d65b7f4d5afb259d24eea049a455ab91b13a766027db0fbbbd5af") (apheleia--run-formatter-function t nil (func buffer remote callback stdin formatter) "a55cd048f7ddfd3ba80175b2e4762da1b22af04ed84caadef4bc620eb48f0b16") (apheleia-indent-lisp-buffer t nil (&rest --cl-rest--) "40e26f4e1ac1937e6e193d8ec0d2be5db854af0c952ddc2c96ef828a4c6387fb") (apheleia-reformat-bibtex-buffer t nil (&rest --cl-rest--) "452386456c409db8c06221f5bd44290008d9057c8b09b1edde53b90950789121") (apheleia--run-formatters t nil (formatters buffer remote callback &optional stdin) "ff92d9883fe830a9d4391ff2a0f1757d7f97aa891bc3615f65847f312ed39fc0") (apheleia--formatter-safe-p t nil (val) "7bea5052310774f05c370856ce9da4f4f87d32b24a19928cd85c6d3a19084506") (apheleia--ensure-list t nil (arg) "ba53c8afb26cc79d84e2232f973f67fa1f0ddfbda831160f2f6081b1d0fc8595") (apheleia--get-mode-chain t nil nil "2e2c20b82e3e25994f2f209faae8237a48b3c0e0e68251411efc046a1ca020d3") (apheleia--get-formatters t nil (&optional interactive) "afbb35caddfaa7929d998fbf086fed943c6c6c0819d2eff0211fd0f5de624665"))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_customization_surface_defaults_types_groups_and_safety_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((value
                  (and
                   (boundp symbol)
                   (symbol-value symbol))))
             (list
              symbol
              (cond
               ((eq
                 symbol
                 'apheleia-formatters)
                (list
                 (length value)
                 (secure-hash
                  'sha256
                  (prin1-to-string value))))
               ((eq
                 symbol
                 'apheleia-mode-alist)
                (list
                 (length value)
                 (secure-hash
                  'sha256
                  (prin1-to-string value))))
               (t value))
              (get symbol 'custom-type)
              (get symbol 'custom-group)
              (get symbol 'risky-local-variable)
              (get symbol 'safe-local-variable)
              (local-variable-if-set-p symbol))))
         '(apheleia-mode-lighter
           apheleia-skip-functions
           apheleia-post-format-hook
           apheleia-inhibit-functions
           apheleia-inhibit
           apheleia-max-alignment-size
           apheleia-hide-log-buffers
           apheleia-log-only-errors
           apheleia-debug-info-buffer
           apheleia-log-debug-info
           apheleia-formatters-respect-indent-level
           apheleia-formatters-respect-fill-column
           apheleia-formatters-mode-extension-assoc
           apheleia-formatters
           apheleia-mode-alist
           apheleia-mode-predicates
           apheleia-formatter-exited-hook
           apheleia-remote-algorithm
           apheleia-formatter))"##;
    let expect = expect![[
        r#"OK ((apheleia-mode-lighter " Apheleia" (choice :tag "Lighter" (const :tag "No lighter" nil) string) nil t nil nil) (apheleia-skip-functions nil (repeat function) nil nil nil nil) (apheleia-post-format-hook nil hook nil nil nil nil) (apheleia-inhibit-functions nil (repeat function) nil nil nil nil) (apheleia-inhibit nil nil nil nil booleanp t) (apheleia-max-alignment-size 400 integer nil nil nil nil) (apheleia-hide-log-buffers nil boolean nil nil nil nil) (apheleia-log-only-errors t boolean nil nil nil nil) (apheleia-debug-info-buffer "*apheleia-debug-log*" string nil nil nil nil) (apheleia-log-debug-info nil boolean nil nil nil nil) (apheleia-formatters-respect-indent-level t boolean nil nil booleanp nil) (apheleia-formatters-respect-fill-column nil boolean nil nil booleanp nil) (apheleia-formatters-mode-extension-assoc ((c-mode . ".c") (c-ts-mode . ".c") (c++-mode . ".cpp") (c++-ts-mode . ".cpp") (glsl-mode . ".glsl") (java-mode . ".java") (java-ts-mode . ".java")) alist nil nil nil nil) (apheleia-formatters (112 "775f1c3a9ec7bc1bb7b4585765ad91786e72087e977bcfc5525e6d9ab3758796") (alist :key-type symbol :value-type (choice (repeat (choice (string :tag "Argument") (const :tag "Look for command in node_modules/.bin" npx) (const :tag "TODO: docstring" inplace) (const :tag "Name of file being formatted" filepath) (const :tag "Name of real file used for input" file) (const :tag "Name of temporary file used for input" input) (const :tag "Name of temporary file used for output" output))) (function :tag "Formatter function"))) nil nil nil nil) (apheleia-mode-alist (106 "cc7449200d79580e1e6eb49377d9d99d2b7efd71efe01f4b0c23217d0d5fda84") (alist :key-type (choice (symbol :tag "Major mode") (string :tag "Buffer name regexp")) :value-type (choice (symbol :tag "Formatter") (repeat (symbol :tag "Formatter")))) nil nil nil nil) (apheleia-mode-predicates (apheleia-mhtml-mode-predicate) (repeat function) nil nil nil nil) (apheleia-formatter-exited-hook nil hook nil nil nil nil) (apheleia-remote-algorithm cancel (choice (const :tag "Run the formatter on the local machine" local) (const :tag "Run the formatter on the remote machine" remote) (const :tag "Disable formatting for remote buffers" cancel)) nil nil nil nil) (apheleia-formatter nil nil nil nil apheleia--formatter-safe-p t))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_autoloads_expose_modes_commands_options_and_safe_local_contracts() {
    let elisp_form = r##"(list
         (featurep 'apheleia)
         (mapcar
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
          '(apheleia-format-buffer
            apheleia-format-after-save
            apheleia-mode
            apheleia-mode-maybe
            apheleia-global-mode
            apheleia-goto-error))
         apheleia-inhibit-functions
         apheleia-inhibit
         (get
          'apheleia-inhibit
          'safe-local-variable)
         (get
          'apheleia-mode
          'safe-local-variable)
         (assq
          'apheleia-mode
          minor-mode-alist))"##;
    let expect = expect![
        "OK (nil ((apheleia-format-buffer nil t t t) (apheleia-format-after-save nil t t nil) (apheleia-mode t t nil t) (apheleia-mode-maybe nil t nil nil) (apheleia-global-mode t t nil t) (apheleia-goto-error nil t t t)) nil nil booleanp booleanp (apheleia-mode apheleia-mode-lighter))"
    ];

    assert_apheleia_autoload_parity(elisp_form, expect);
}

#[test]
fn apheleia_formatter_context_class_slots_accessors_and_mutation_match() {
    let elisp_form = r##"(let ((context
                (apheleia-formatter--context)))
         (setf
          (apheleia-formatter--name context)
          'demo
          (apheleia-formatter--arg1 context)
          "formatter"
          (apheleia-formatter--argv context)
          '("--check" "-")
          (apheleia-formatter--remote context)
          "/ssh:demo:"
          (apheleia-formatter--stdin context)
          (current-buffer)
          (apheleia-formatter--input-fname context)
          "input.tmp"
          (apheleia-formatter--output-fname context)
          "output.tmp"
          (apheleia-formatter--exit-status context)
          17)
         (list
          (eieio-object-class-name context)
          (apheleia-formatter--name context)
          (apheleia-formatter--arg1 context)
          (apheleia-formatter--argv context)
          (apheleia-formatter--remote context)
          (eq
           (apheleia-formatter--stdin context)
           (current-buffer))
          (apheleia-formatter--input-fname context)
          (apheleia-formatter--output-fname context)
          (apheleia-formatter--exit-status context)))"##;
    let expect = expect![[
        r#"OK (apheleia-formatter--context demo "formatter" ("--check" "-") "/ssh:demo:" t "input.tmp" "output.tmp" 17)"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}
