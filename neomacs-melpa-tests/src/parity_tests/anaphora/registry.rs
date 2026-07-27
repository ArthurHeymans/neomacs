use expect_test::expect;

use super::{assert_anaphora_autoload_parity, assert_anaphora_parity};

#[test]
fn anaphora_package_descriptor_records_exact_pin_summary_kind_and_payload() {
    let elisp_form = r##"(let* ((description
                          (cadr
                           (assq 'anaphora
                                 package-alist)))
               (directory
                (package-desc-dir description)))
         (list
          (package-version-join
           (package-desc-version description))
          (package-desc-reqs description)
          (package-desc-summary description)
          (package-desc-kind description)
          (sort
           (mapcar
            #'file-name-nondirectory
            (directory-files
             directory t
             "\\.el\\'"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK ("20260720.903" nil "Anaphoric macros providing implicit temp variables." nil ("anaphora-autoloads.el" "anaphora-pkg.el" "anaphora.el"))"#
    ]];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_autoloads_publish_every_long_macro_without_loading_the_feature() {
    let elisp_form = r##"(list
         (featurep 'anaphora)
         (mapcar
          (lambda (symbol)
            (let ((definition
                   (and
                    (fboundp symbol)
                    (symbol-function
                     symbol))))
              (list
               symbol
               (fboundp symbol)
               (autoloadp definition)
               (and
                (autoloadp definition)
                (nth 4 definition))
               (macrop symbol))))
          '(anaphoric-if
            anaphoric-prog1
            anaphoric-prog2
            anaphoric-when
            anaphoric-while
            anaphoric-and
            anaphoric-cond
            anaphoric-lambda
            anaphoric-block
            anaphoric-case
            anaphoric-ecase
            anaphoric-typecase
            anaphoric-etypecase
            anaphoric-pcase
            anaphoric-let
            anaphoric-+
            anaphoric--
            anaphoric-*
            anaphoric-/)))"##;
    let expect = expect![
        "OK (nil ((anaphoric-if t t t #1=(t)) (anaphoric-prog1 t t t #1#) (anaphoric-prog2 t t t #1#) (anaphoric-when t t t #1#) (anaphoric-while t t t #1#) (anaphoric-and t t t #1#) (anaphoric-cond t t t #1#) (anaphoric-lambda t t t #1#) (anaphoric-block t t t #1#) (anaphoric-case t t t #1#) (anaphoric-ecase t t t #1#) (anaphoric-typecase t t t #1#) (anaphoric-etypecase t t t #1#) (anaphoric-pcase t t t #1#) (anaphoric-let t t t #1#) (anaphoric-+ t t t #1#) (anaphoric-- t t t #1#) (anaphoric-* t t t #1#) (anaphoric-/ t t t #1#)))"
    ];
    assert_anaphora_autoload_parity(elisp_form, expect);
}

#[test]
fn anaphora_autoloads_install_all_traditional_aliases_with_matching_targets() {
    let elisp_form = r##"(mapcar
         (lambda (pair)
           (let ((short
                  (car pair))
                 (long
                  (cdr pair)))
             (list
              short
              long
              (fboundp short)
              (macrop short)
              (eq
               (symbol-function short)
               (symbol-function long)))))
         '((aif . anaphoric-if)
           (aprog1 . anaphoric-prog1)
           (aprog2 . anaphoric-prog2)
           (awhen . anaphoric-when)
           (awhile . anaphoric-while)
           (aand . anaphoric-and)
           (acond . anaphoric-cond)
           (alambda . anaphoric-lambda)
           (ablock . anaphoric-block)
           (acase . anaphoric-case)
           (aecase . anaphoric-ecase)
           (atypecase . anaphoric-typecase)
           (aetypecase . anaphoric-etypecase)
           (apcase . anaphoric-pcase)
           (alet . anaphoric-let)
           (a+ . anaphoric-+)
           (a- . anaphoric--)
           (a* . anaphoric-*)
           (a/ . anaphoric-/)))"##;
    let expect = expect![
        "OK ((aif anaphoric-if t #1=(t) nil) (aprog1 anaphoric-prog1 t #1# nil) (aprog2 anaphoric-prog2 t #1# nil) (awhen anaphoric-when t #1# nil) (awhile anaphoric-while t #1# nil) (aand anaphoric-and t #1# nil) (acond anaphoric-cond t #1# nil) (alambda anaphoric-lambda t #1# nil) (ablock anaphoric-block t #1# nil) (acase anaphoric-case t #1# nil) (aecase anaphoric-ecase t #1# nil) (atypecase anaphoric-typecase t #1# nil) (aetypecase anaphoric-etypecase t #1# nil) (apcase anaphoric-pcase t #1# nil) (alet anaphoric-let t #1# nil) (a+ anaphoric-+ t #1# nil) (a- anaphoric-- t #1# nil) (a* anaphoric-* t #1# nil) (a/ anaphoric-/ t #1# nil))"
    ];
    assert_anaphora_autoload_parity(elisp_form, expect);
}

#[test]
fn anaphora_source_registers_exact_feature_custom_group_and_callable_surface() {
    let elisp_form = r##"(list
         (featurep 'anaphora)
         anaphora-use-long-names-only
         (custom-variable-p
          'anaphora-use-long-names-only)
         (get
          'anaphora-use-long-names-only
          'standard-value)
         (get 'anaphora
              'custom-group)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (macrop symbol)
             (help-function-arglist
              symbol t)))
          '(anaphora-install-font-lock-keywords
            anaphora--install-traditional-aliases
            anaphoric-if
            anaphoric-prog1
            anaphoric-prog2
            anaphoric-when
            anaphoric-while
            anaphoric-and
            anaphoric-cond
            anaphoric-lambda
            anaphoric-block
            anaphoric-case
            anaphoric-ecase
            anaphoric-typecase
            anaphoric-etypecase
            anaphoric-pcase
            anaphoric-let
            anaphoric-+
            anaphoric--
            anaphoric-*
            anaphoric-/)))"##;
    let expect = expect![
        "OK (t nil #1=((funcall #'#[nil (nil) (t)])) #1# ((anaphora-use-long-names-only custom-variable)) ((anaphora-install-font-lock-keywords t nil nil) (anaphora--install-traditional-aliases t nil (&optional arg)) (anaphoric-if t t (cond then &rest else)) (anaphoric-prog1 t t (first &rest body)) (anaphoric-prog2 t t (form1 form2 &rest body)) (anaphoric-when t t (cond &rest body)) (anaphoric-while t t (test &rest body)) (anaphoric-and t t (&rest conditions)) (anaphoric-cond t t (&rest clauses)) (anaphoric-lambda t t (args &rest body)) (anaphoric-block t t (name &rest body)) (anaphoric-case t t (expr &rest clauses)) (anaphoric-ecase t t (expr &rest clauses)) (anaphoric-typecase t t (expr &rest clauses)) (anaphoric-etypecase t t (expr &rest clauses)) (anaphoric-pcase t t (expr &rest clauses)) (anaphoric-let t t (form &rest body)) (anaphoric-+ t t (&rest numbers-or-markers)) (anaphoric-- t t (&optional number-or-marker &rest numbers-or-markers)) (anaphoric-* t t (&rest numbers-or-markers)) (anaphoric-/ t t (dividend divisor &rest divisors))))"
    ];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_long_names_only_prevents_short_alias_installation_on_fresh_load() {
    let elisp_form = r##"(let* ((directory
                          (file-name-directory
                           (getenv
                            "NEOMACS_PACKAGE_SOURCE")))
               (source
                (expand-file-name
                 "anaphora.el"
                 directory))
               (shorts
                '(aif aprog1 aprog2
                  awhen awhile aand acond
                  alambda ablock acase
                  aecase atypecase
                  aetypecase apcase alet
                  a+ a- a* a/)))
         (mapc #'fmakunbound shorts)
         (setq anaphora-use-long-names-only
               t)
         (load source nil t t)
         (list
          anaphora-use-long-names-only
          (mapcar
           (lambda (symbol)
             (list
              symbol
              (fboundp symbol)))
           shorts)
          (mapcar
           #'macrop
           '(anaphoric-if
             anaphoric-lambda
             anaphoric-+))))"##;
    let expect = expect![
        "OK (t ((aif nil) (aprog1 nil) (aprog2 nil) (awhen nil) (awhile nil) (aand nil) (acond nil) (alambda nil) (ablock nil) (acase nil) (aecase nil) (atypecase nil) (aetypecase nil) (apcase nil) (alet nil) (a+ nil) (a- nil) (a* nil) (a/ nil)) (t t t))"
    ];
    assert_anaphora_autoload_parity(elisp_form, expect);
}

#[test]
fn anaphora_traditional_alias_lifecycle_is_reversible_and_preserves_user_overrides() {
    let elisp_form = r##"(let ((shorts
                        '(aif aprog1 aprog2
                          awhen awhile aand
                          acond alambda ablock
                          acase aecase
                          atypecase aetypecase
                          apcase alet a+ a-
                          a* a/)))
         (anaphora--install-traditional-aliases
          -1)
         (let ((removed
                (mapcar #'fboundp
                        shorts)))
           (anaphora--install-traditional-aliases)
           (let ((installed
                  (mapcar
                   (lambda (symbol)
                     (list
                      symbol
                      (macrop symbol)))
                   shorts)))
             (fset
              'aif
              (lambda (&rest _)
                :user-override))
             (anaphora--install-traditional-aliases
              -1)
             (list
              removed
              installed
              (fboundp 'aif)
              (funcall 'aif)
              (mapcar
               #'fboundp
               (cdr shorts))))))"##;
    let expect = expect![
        "OK ((nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil) ((aif t) (aprog1 t) (aprog2 t) (awhen t) (awhile t) (aand t) (acond t) (alambda t) (ablock t) (acase t) (aecase t) (atypecase t) (aetypecase t) (apcase t) (alet t) (a+ t) (a- t) (a* t) (a/ t)) t :user-override (nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil))"
    ];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_aliases_copy_exact_indentation_and_edebug_metadata() {
    let elisp_form = r##"(mapcar
         (lambda (pair)
           (list
            (car pair)
            (get
             (car pair)
             'lisp-indent-function)
            (get
             (car pair)
             'edebug-form-spec)
            (get
             (cdr pair)
             'lisp-indent-function)
            (get
             (cdr pair)
             'edebug-form-spec)))
         '((aif . anaphoric-if)
           (aprog1 . anaphoric-prog1)
           (aprog2 . anaphoric-prog2)
           (awhen . anaphoric-when)
           (awhile . anaphoric-while)
           (aand . anaphoric-and)
           (acond . anaphoric-cond)
           (alambda . anaphoric-lambda)
           (ablock . anaphoric-block)
           (acase . anaphoric-case)
           (aecase . anaphoric-ecase)
           (atypecase . anaphoric-typecase)
           (aetypecase . anaphoric-etypecase)
           (apcase . anaphoric-pcase)
           (alet . anaphoric-let)))"##;
    let expect = expect![
        "OK ((aif 2 t 2 t) (aprog1 1 t 1 t) (aprog2 2 t 2 t) (awhen 1 when 1 when) (awhile 1 t 1 t) (aand nil t nil t) (acond nil cond nil cond) (alambda defun lambda defun lambda) (ablock nil block 1 block) (acase nil case 1 case) (aecase nil ecase 1 ecase) (atypecase nil typecase 1 typecase) (aetypecase nil etypecase 1 etypecase) (apcase nil pcase 1 pcase) (alet 1 let 1 let))"
    ];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_font_lock_marks_it_and_self_in_real_recursive_elisp() {
    let elisp_form = r##"(progn
         (anaphora-install-font-lock-keywords)
         (with-temp-buffer
           (emacs-lisp-mode)
           (insert
            "(alambda (tree)\n"
            "  (if (consp tree)\n"
            "      (self (cdr tree))\n"
            "    it))")
           (font-lock-ensure)
           (mapcar
            (lambda (needle)
              (goto-char
               (point-min))
              (let ((end
                     (search-forward
                      needle)))
                (list
                 needle
                 (get-text-property
                  (-
                   end
                   (length needle))
                  'face)
                 (get-text-property
                  (1- end)
                  'face))))
            '("alambda"
              "self"
              "it"))))"##;
    let expect = expect![[
        r#"OK (("alambda" font-lock-keyword-face font-lock-keyword-face) ("self" font-lock-variable-name-face font-lock-variable-name-face) ("it" font-lock-variable-name-face font-lock-variable-name-face))"#
    ]];
    assert_anaphora_parity(elisp_form, expect);
}
