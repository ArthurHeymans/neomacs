use expect_test::expect;

use super::{assert_attrap_autoload_parity, assert_attrap_parity};

#[test]
fn attrap_exact_package_descriptor_dependency_activation_and_payload_contract_match() {
    let elisp_form = r##"(let* ((descriptor
                (cadr
                 (assq 'attrap package-alist)))
               (directory
                (package-desc-dir descriptor)))
          (list
           (package-desc-name descriptor)
           (package-version-join
            (package-desc-version descriptor))
           (package-desc-summary descriptor)
           (package-desc-kind descriptor)
           (package-desc-reqs descriptor)
           (package-desc-extras descriptor)
           (featurep 'attrap)
           (package-installed-p
            'attrap
            '(20260304 1504))
           (file-name-nondirectory
            (locate-library "attrap"))
           (mapcar
            (lambda (package)
              (let* ((dependency
                      (cadr
                       (assq package package-alist)))
                     (source
                      (locate-library
                       (symbol-name package))))
                (list
                 package
                 (package-version-join
                  (package-desc-version dependency))
                 (and source
                      (file-name-nondirectory source))
                 (and source
                      (with-temp-buffer
                        (insert-file-contents-literally
                         source)
                        (secure-hash
                         'sha256
                         (current-buffer)))))))
            '(dash f s))
           (mapcar
            (lambda (file)
              (let ((path
                     (expand-file-name
                      file
                      directory)))
                (list
                 file
                 (file-attribute-size
                  (file-attributes path))
                 (with-temp-buffer
                   (insert-file-contents-literally
                    path)
                   (secure-hash
                    'sha256
                    (current-buffer))))))
            '("attrap-pkg.el"
              "attrap.el"))))"##;
    let expect = expect![[
        r#"OK (attrap "20260304.1504" "ATtempt To Repair At Point." nil ((dash (2 12 0)) (emacs (25 1)) (f (0 19 0)) (s (1 11 0))) ((:maintainers ("Jean-Philippe Bernardy" . "jeanphilippe.bernardy@gmail.com")) (:authors ("Jean-Philippe Bernardy" . "jeanphilippe.bernardy@gmail.com")) (:keywords "programming" "tools") (:revdesc . "ad1d9443fcd9") (:commit . "ad1d9443fcd93e32f2aefadc5af2646701664581") (:url . "https://github.com/jyp/attrap")) t t "attrap.el" ((dash "20260221.1346" "dash.el" "ce8043bfcfe64bfe69a411ee29e4c704213abd93aaa9a6da8b6791d3110d7f48") (f "20241003.1131" "f.el" "6c50127cfb8ff86ada7667f0e6a4242002f41b4e132f11877de095be5cf3683e") (s "20220902.1511" "s.el" "fbb8ef1b861eef414fbb424ff3c55363f5b7a96866deec515c84a0523e61bed3")) (("attrap-pkg.el" 522 "9bb8e1e96a892b1c2983949eaddafd54fb3f987e91646f56755723f1755506c3") ("attrap.el" 29123 "4289ecabb49a8d7f365861f633a6ba08c4dc4a90d46404aaf6b5bee52cba1117")))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_complete_prefixed_function_macro_variable_and_source_inventory_matches() {
    let elisp_form = r##"(let (symbols)
          (mapatoms
           (lambda (symbol)
             (when
                 (and
                  (string-prefix-p
                   "attrap"
                   (symbol-name symbol))
                  (not
                   (string-prefix-p
                    "attrap-test-"
                    (symbol-name symbol))))
               (push
                (list
                 symbol
                 (fboundp symbol)
                 (macrop symbol)
                 (boundp symbol)
                 (and
                  (custom-variable-p symbol)
                  t)
                 (when
                     (fboundp symbol)
                   (copy-tree
                    (help-function-arglist
                     symbol
                     t)))
                 (when-let
                     ((source
                       (or
                        (symbol-file symbol 'defun)
                        (symbol-file symbol 'defvar))))
                   (file-name-nondirectory
                    source)))
                symbols))))
          (sort
           symbols
           (lambda (left right)
             (string<
              (symbol-name
               (car left))
              (symbol-name
               (car right))))))"##;
    let expect = expect![[
        r#"OK ((attrap nil nil nil nil nil nil) (attrap-LaTeX-fixer t nil nil nil (msg pos _end) "attrap.el") (attrap-add-operator-parens t nil nil nil (name) "attrap.el") (attrap-add-to-import t t nil nil (missing module line col) "attrap.el") (attrap-alternatives t t nil nil (&rest clauses) "attrap.el") (attrap-attrap t nil nil nil (pos) "attrap.el") (attrap-autoloads nil nil nil nil nil nil) (attrap-elisp-fixer t nil nil nil (msg _beg _end) "attrap.el") (attrap-flycheck t nil nil nil (pos) "attrap.el") (attrap-flycheck-checkers-alist nil nil t t nil "attrap.el") (attrap-flymake t nil nil nil (pos) "attrap.el") (attrap-flymake-backends-alist nil nil t t nil "attrap.el") (attrap-flymake-hlint nil nil nil nil nil nil) (attrap-ghc-fixer t nil nil nil (msg pos _end) "attrap.el") (attrap-haskell-extensions nil nil t t nil "attrap.el") (attrap-hlint-fixer t nil nil nil (msg pos end) "attrap.el") (attrap-insert-language-pragma t t nil nil (extension) "attrap.el") (attrap-one-option t t nil nil (description &rest body) "attrap.el") (attrap-option t t nil nil (description &rest body) "attrap.el") (attrap-select-and-apply-option t nil nil nil (options) "attrap.el"))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_all_function_macro_command_argument_documentation_and_declaration_contracts_match() {
    let elisp_form = r##"(mapcar
          (lambda (symbol)
            (list
             symbol
             (macrop symbol)
             (commandp symbol)
             (interactive-form symbol)
             (copy-tree
              (help-function-arglist
               symbol
               t))
             (documentation symbol t)
             (get symbol 'lisp-indent-function)
             (copy-tree
              (get symbol 'edebug-form-spec))
             (file-name-nondirectory
              (symbol-file symbol 'defun))))
          '(attrap-select-and-apply-option
            attrap-flymake
            attrap-flycheck
            attrap-attrap
            attrap-option
            attrap-one-option
            attrap-alternatives
            attrap-elisp-fixer
            attrap-insert-language-pragma
            attrap-add-to-import
            attrap-ghc-fixer
            attrap-add-operator-parens
            attrap-hlint-fixer
            attrap-LaTeX-fixer))"##;
    let expect = expect![[
        r#"OK ((attrap-select-and-apply-option nil nil nil (options) "Ask the user which of OPTIONS is best, then apply it." nil nil "attrap.el") (attrap-flymake nil t (interactive "d") (pos) "Attempt to repair the flymake error at POS." nil nil "attrap.el") (attrap-flycheck nil t (interactive "d") (pos) "Attempt to repair the flycheck error at POS." nil nil "attrap.el") (attrap-attrap nil t (interactive "d") (pos) "Attempt to repair the error at POS." nil nil "attrap.el") (attrap-option t nil nil (description &rest body) "Create an attrap option with DESCRIPTION and BODY.\nThe body is code that performs the fix." 1 nil "attrap.el") (attrap-one-option t nil nil (description &rest body) "Create an attrap option list with a single element of DESCRIPTION and BODY." 1 nil "attrap.el") (attrap-alternatives t nil nil (&rest clauses) "Append all succeeding clauses.\nEach clause looks like (CONDITION BODY...).  CONDITION is\nevaluated and, if the value is non-nil, this clause succeeds:\nthen the expressions in BODY are evaluated and the last one's\nvalue is a list which is appended to the result of\n`attrap-alternatives'.  Usage: (attrap-alternatives CLAUSES...)" nil nil "attrap.el") (attrap-elisp-fixer nil nil nil (msg _beg _end) "An `attrap' fixer for any elisp warning given as MSG." nil nil "attrap.el") (attrap-insert-language-pragma t nil nil (extension) "Action: Insert language language EXTENSION pragma at beginning of file." nil nil "attrap.el") (attrap-add-to-import t nil nil (missing module line col) "Action: insert MISSING to the import of MODULE.\nThe import ends at LINE and COL in the file." nil nil "attrap.el") (attrap-ghc-fixer nil nil nil (msg pos _end) "An `attrap' fixer for any GHC error or warning.\nError is given as MSG and reported between POS and END." nil nil "attrap.el") (attrap-add-operator-parens nil nil nil (name) "Add parens around a NAME if it refers to a Haskell operator." nil nil "attrap.el") (attrap-hlint-fixer nil nil nil (msg pos end) "Fixer for any hlint hint given as MSG and reported between POS and END." nil nil "attrap.el") (attrap-LaTeX-fixer nil nil nil (msg pos _end) nil nil nil "attrap.el"))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_custom_group_and_complete_variable_contract_match_exact_source_defaults() {
    let elisp_form = r##"(list
          (list
           (get 'attrap 'custom-group)
           (documentation-property
            'attrap
            'group-documentation
            t)
           (get 'attrap 'custom-prefix)
           (get 'attrap 'custom-links))
          (mapcar
           (lambda (symbol)
             (let ((standard-value
                    (copy-tree
                     (get symbol 'standard-value))))
               (list
                symbol
                (and
                 (custom-variable-p symbol)
                 t)
                (symbol-value symbol)
                (default-value symbol)
                standard-value
                (and
                 (=
                  (length standard-value)
                  1)
                 (equal
                  (eval
                   (car standard-value)
                   t)
                  (default-value symbol)))
                (copy-tree
                 (get symbol 'custom-type))
                (get symbol 'custom-group)
                (documentation-property
                 symbol
                 'variable-documentation
                 t)
                (special-variable-p symbol)
                (local-variable-if-set-p
                 symbol)
                (file-name-nondirectory
                 (symbol-file symbol 'defvar)))))
           '(attrap-flycheck-checkers-alist
             attrap-flymake-backends-alist
             attrap-haskell-extensions)))"##;
    let expect = expect![[
        r#"OK ((((attrap-flycheck-checkers-alist custom-variable) (attrap-flymake-backends-alist custom-variable) (attrap-haskell-extensions custom-variable)) nil nil nil) ((attrap-flycheck-checkers-alist t #1=((haskell-dante . attrap-ghc-fixer) (emacs-lisp . attrap-elisp-fixer)) #1# ((funcall #'#[nil ('((haskell-dante . attrap-ghc-fixer) (emacs-lisp . attrap-elisp-fixer))) #3=(t)])) t (alist :key-type symbol :value-type function) nil "An alist from flycheck checker symbol to attrap fixer." t nil "attrap.el") (attrap-flymake-backends-alist t #2=((dante-flymake . attrap-ghc-fixer) (LaTeX-flymake . attrap-LaTeX-fixer) (attrap-flymake-hlint . attrap-hlint-fixer) (elisp-flymake-byte-compile . attrap-elisp-fixer) (elisp-flymake-checkdoc . attrap-elisp-fixer)) #2# ((funcall #'#[nil ('((dante-flymake . attrap-ghc-fixer) (LaTeX-flymake . attrap-LaTeX-fixer) (attrap-flymake-hlint . attrap-hlint-fixer) (elisp-flymake-byte-compile . attrap-elisp-fixer) (elisp-flymake-checkdoc . attrap-elisp-fixer))) #3#])) t (alist :key-type symbol :value-type function) nil "An alist from flymake backend to attrap fixer." t nil "attrap.el") (attrap-haskell-extensions t #4=("AllowAmbiguousTypes" "BangPatterns" "ConstraintKinds" "ConstrainedClassMethods" "DataKinds" "DefaultSignatures" "DeriveAnyClass" "DeriveDataTypeable" "DeriveFoldable" "DeriveFunctor" "DeriveGeneric" "DeriveTraversable" "DerivingStrategies" "DerivingVia" "DisambiguateRecordFields" "EmptyCase" "EmptyDataDecls" "EmptyDataDeriving" "ExistentialQuantification" "ExplicitNamespaces" "FlexibleContexts" "FlexibleInstances" "FunctionalDependencies" "GADTs" "GeneralizedNewtypeDeriving" "ImportQualifiedPost" "InstanceSigs" "KindSignatures" "LambdaCase" "LinearTypes" "MonoLocalBinds" "MultiParamTypeClasses" "NamedFieldPuns" "ParallelListComp" "PartialTypeSignatures" "PatternGuards" "PatternSynonyms" "PolyKinds" "QuantifiedConstraints" "RankNTypes" "RecordWildCards" "ScopedTypeVariables" "StandaloneDeriving" "StandaloneKindSignatures" "TemplateHaskell" "TransformListComp" "TupleSections" "TypeAbstractions" "TypeApplications" "TypeFamilies" "TypeFamilyDependencies" "TypeInType" "TypeOperators" "TypeSynonymInstances" "UndecidableSuperClasses" "UndecidableInstances" "UnliftedNewtypes" "ViewPatterns") #4# ((funcall #'#[nil ('("AllowAmbiguousTypes" "BangPatterns" "ConstraintKinds" "ConstrainedClassMethods" "DataKinds" "DefaultSignatures" "DeriveAnyClass" "DeriveDataTypeable" "DeriveFoldable" "DeriveFunctor" "DeriveGeneric" "DeriveTraversable" "DerivingStrategies" "DerivingVia" "DisambiguateRecordFields" "EmptyCase" "EmptyDataDecls" "EmptyDataDeriving" "ExistentialQuantification" "ExplicitNamespaces" "FlexibleContexts" "FlexibleInstances" "FunctionalDependencies" "GADTs" "GeneralizedNewtypeDeriving" "ImportQualifiedPost" "InstanceSigs" "KindSignatures" "LambdaCase" "LinearTypes" "MonoLocalBinds" "MultiParamTypeClasses" "NamedFieldPuns" "ParallelListComp" "PartialTypeSignatures" "PatternGuards" "PatternSynonyms" "PolyKinds" "QuantifiedConstraints" "RankNTypes" "RecordWildCards" "ScopedTypeVariables" "StandaloneDeriving" "StandaloneKindSignatures" "TemplateHaskell" "TransformListComp" "TupleSections" "TypeAbstractions" "TypeApplications" "TypeFamilies" "TypeFamilyDependencies" "TypeInType" "TypeOperators" "TypeSynonymInstances" "UndecidableSuperClasses" "UndecidableInstances" "UnliftedNewtypes" "ViewPatterns")) #3#])) t (repeat string) nil "Language extensions that Attrap can use to fix errors." t nil "attrap.el")))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_delayed_flymake_flycheck_alias_activates_only_after_real_feature_load() {
    let elisp_form = r##"(let* ((root
                (file-name-as-directory
                 (expand-file-name
                  "attrap-delayed-alias"
                  (getenv "TMPDIR"))))
               (library
                (expand-file-name
                 "flymake-flycheck.el"
                 root)))
          (when
              (file-exists-p root)
            (delete-directory root t))
          (make-directory root t)
          (with-temp-file library
            (insert
             "(defun attrap-test-hlint-backend (&rest arguments)\n"
             "  (cons :backend arguments))\n"
             "(defun flymake-flycheck-diagnostic-function-for (checker)\n"
             "  (unless (eq checker 'haskell-hlint)\n"
             "    (error \"unexpected checker: %S\" checker))\n"
             "  'attrap-test-hlint-backend)\n"
             "(provide 'flymake-flycheck)\n"))
          (let ((before
                 (list
                  (featurep 'flymake-flycheck)
                  (fboundp 'attrap-flymake-hlint))))
            (load library nil nil t)
            (list
             before
             (featurep 'flymake-flycheck)
             (fboundp 'attrap-flymake-hlint)
             (symbol-function
              'attrap-flymake-hlint)
             (funcall
              'attrap-flymake-hlint
              :diagnostic)
             (when-let
                 ((source
                   (symbol-file
                    'attrap-flymake-hlint
                    'defun)))
               (file-name-nondirectory
                source)))))"##;
    let expect = expect!["OK ((nil nil) t t attrap-test-hlint-backend (:backend :diagnostic) nil)"];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_generated_autoload_contract_registers_all_commands_without_loading_source() {
    let elisp_form = r##"(let* ((history
                (seq-find
                 (lambda (entry)
                   (and
                    (stringp
                     (car entry))
                    (string-suffix-p
                     "attrap-autoloads.el"
                     (car entry))))
                 load-history))
               (events
                (mapcar
                 (lambda (event)
                   (list
                    (car event)
                    (cdr event)))
                 (seq-filter
                  (lambda (event)
                    (memq
                     (car-safe event)
                     '(defun provide)))
                  (cdr history))))
               (prefix-files
                (if
                    (hash-table-p definition-prefixes)
                    (gethash
                     "attrap-"
                     definition-prefixes)
                  (cdr
                   (assoc
                    "attrap-"
                    definition-prefixes)))))
          (list
           (featurep 'attrap-autoloads)
           (featurep 'attrap)
           (sort
            (delete-dups
             (copy-sequence prefix-files))
            #'string<)
           events
           (mapcar
            (lambda (symbol)
              (let ((definition
                     (and
                      (fboundp symbol)
                      (symbol-function symbol))))
                (list
                 symbol
                 (autoloadp definition)
                 (and
                  (autoloadp definition)
                  (nth 1 definition))
                 (commandp symbol)
                 (help-function-arglist
                  symbol
                  t))))
            '(attrap-flymake
              attrap-flycheck
              attrap-attrap))
           (mapcar
            (lambda (symbol)
              (list
               symbol
               (fboundp symbol)
               (boundp symbol)))
            '(attrap-ghc-fixer
              attrap-haskell-extensions
              attrap-flymake-backends-alist))))"##;
    let expect = expect![[
        r#"OK (t nil ("attrap") ((defun attrap-flymake) (defun attrap-flycheck) (defun attrap-attrap) (provide attrap-autoloads)) ((attrap-flymake t "attrap" t "[Arg list not available until function definition is loaded.]") (attrap-flycheck t "attrap" t "[Arg list not available until function definition is loaded.]") (attrap-attrap t "attrap" t "[Arg list not available until function definition is loaded.]")) ((attrap-ghc-fixer nil nil) (attrap-haskell-extensions nil nil) (attrap-flymake-backends-alist nil nil)))"#
    ]];

    assert_attrap_autoload_parity(elisp_form, expect);
}
