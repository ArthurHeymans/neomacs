//! Oracle parity tests for GNU help and documentation semantics.
//!
//! These tests cover `documentation`, `documentation-property`,
//! `substitute-command-keys`, `help-split-fundoc`, `help-add-fundoc-usage`,
//! and `help-function-arglist` behavior studied from GNU `src/doc.c` and
//! `lisp/help.el`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_help_split_and_add_fundoc_usage() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'help)
  (list
   (help-split-fundoc "Doc body.\n\n(fn ARG &optional B)" 'neomacs-oracle-help-fn)
   (help-split-fundoc "Doc body.\n\n(fn ARG &optional B)" 'neomacs-oracle-help-fn 'usage)
   (help-split-fundoc "Doc body.\n\n(fn ARG &optional B)" 'neomacs-oracle-help-fn 'doc)
   (help-split-fundoc "No usage here" 'neomacs-oracle-help-fn)
   (help-split-fundoc "No usage here" 'neomacs-oracle-help-fn t)
   (help-add-fundoc-usage "Doc." '(arg &optional opt &rest rest))
   (help-add-fundoc-usage "Doc.\n\n(fn OLD)" '(arg))
   (help-add-fundoc-usage nil '(x y))
   (condition-case err
       (help-add-fundoc-usage "Doc." "(bad-usage")
     (error (list (car err) (cadr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_help_function_arglist_symbols_functions_and_autoloads() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'help)
  (defalias 'neomacs-oracle-help-alias
    (lambda (alpha &optional beta &rest gamma)
      "Doc."
      (list alpha beta gamma)))
  (defmacro neomacs-oracle-help-macro (x &optional y)
    "Macro doc."
    `(list ,x ,y))
  (defalias 'neomacs-oracle-help-autoload
    '(autoload "missing-file" "Autoload doc." t nil))
  (list
   (help-function-arglist 'neomacs-oracle-help-alias)
   (help-function-arglist (symbol-function 'neomacs-oracle-help-alias))
   (help-function-arglist 'neomacs-oracle-help-macro)
   (help-function-arglist 'neomacs-oracle-help-autoload)
   (help-function-arglist 'car)
   (help-function-arglist 'car t)
   (help-function-arglist 'apply)
   (help-function-arglist 'apply t)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_documentation_property_eval_raw_and_substitution() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'help)
  (let ((text-quoting-style 'grave))
    (fset 'neomacs-oracle-doc-command (lambda () (interactive)))
    (let ((map (make-sparse-keymap)))
      (define-key map (kbd "C-c d") 'neomacs-oracle-doc-command)
      (setq overriding-local-map map))
    (put 'neomacs-oracle-doc 'function-documentation
         "Doc for \\[neomacs-oracle-doc-command] and `quoted'.")
    (put 'neomacs-oracle-var 'variable-documentation
         '(concat "Dynamic " "doc"))
    (defvaralias 'neomacs-oracle-var-alias 'neomacs-oracle-var)
    (list
     (documentation-property 'neomacs-oracle-doc 'function-documentation t)
     (documentation-property 'neomacs-oracle-doc 'function-documentation nil)
     (documentation-property 'neomacs-oracle-var 'variable-documentation t)
     (documentation-property 'neomacs-oracle-var-alias 'variable-documentation t)
     (documentation 'neomacs-oracle-doc t)
     (documentation 'neomacs-oracle-doc nil))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_substitute_command_keys_keymap_quote_and_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'help)
  (let ((text-quoting-style 'grave)
        (map (make-sparse-keymap)))
    (define-key map (kbd "C-c x") 'neomacs-oracle-help-command)
    (fset 'neomacs-oracle-help-command (lambda () (interactive)))
    (let ((overriding-local-map map))
      (let ((plain (substitute-command-keys
                    "Run \\[neomacs-oracle-help-command], missing \\[neomacs-oracle-missing-command], key \\=`C-c x', quote `a' and \\==\\[literal]."
                    t))
            (faced (substitute-command-keys
                    "\\=`C-c x' \\[neomacs-oracle-help-command]"
                    nil)))
        (list
         plain
         faced
         (text-properties-at 0 faced)
         (eq plain (substitute-command-keys plain t)))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
