use expect_test::expect;

use super::{assert_angry_police_captain_autoload_parity, assert_angry_police_captain_parity};

#[test]
fn angry_police_captain_registers_exact_feature_command_and_metadata() {
    let elisp_form = r##"(list
         (featurep 'angry-police-captain)
         (commandp 'angry-police-captain)
         (interactive-form
          'angry-police-captain)
         (documentation
          'angry-police-captain)
         (help-function-arglist
          'angry-police-captain t)
         (file-name-nondirectory
          (symbol-file
           'angry-police-captain
           'defun)))"##;
    let expect = expect![[
        r#"OK (t t (interactive nil) "Display a quote from \"http://theangrypolicecaptain.com\" in the minibuffer." nil "angry-police-captain.el")"#
    ]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_descriptor_records_exact_pin_and_single_source_payload() {
    let elisp_form = r##"(let* ((description
                          (cadr
                           (assq
                            'angry-police-captain
                            package-alist)))
               (directory
                (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join
           (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar #'file-name-nondirectory
                   (directory-files
                    directory t
                    "\\`[^.]"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (angry-police-captain "20120829.1252" nil "Show quote from http://theangrypolicecaptain.com in the minibuffer." nil ("README-elpa" "angry-police-captain-autoloads.el" "angry-police-captain-pkg.el" "angry-police-captain.el" "angry-police-captain.elc"))"#
    ]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_autoload_exposes_command_without_loading_feature() {
    let elisp_form = r##"(list
         (featurep 'angry-police-captain)
         (commandp 'angry-police-captain)
         (autoloadp
          (symbol-function
           'angry-police-captain))
         (symbol-function
          'angry-police-captain)
         (documentation
          'angry-police-captain))"##;
    let expect = expect![[
        r#"OK (nil t t (autoload "angry-police-captain" "Display a quote from \"http://theangrypolicecaptain.com\" in the minibuffer." t nil) "Display a quote from \"http://theangrypolicecaptain.com\" in the minibuffer.")"#
    ]];
    assert_angry_police_captain_autoload_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_source_reload_preserves_callable_contract() {
    let elisp_form = r##"(let* ((source
                          (getenv
                           "NEOMACS_PACKAGE_SOURCE"))
               (before
                (list
                 (featurep
                  'angry-police-captain)
                 (commandp
                  'angry-police-captain)
                 (help-function-arglist
                  'angry-police-captain t)
                 (documentation
                  'angry-police-captain))))
         (load source nil t t)
         (load source nil t t)
         (list
          before
          (featurep
           'angry-police-captain)
          (commandp
           'angry-police-captain)
          (help-function-arglist
           'angry-police-captain t)
          (documentation
           'angry-police-captain)))"##;
    let expect = expect![[
        r#"OK ((t t nil "Display a quote from \"http://theangrypolicecaptain.com\" in the minibuffer.") t t nil "Display a quote from \"http://theangrypolicecaptain.com\" in the minibuffer.")"#
    ]];
    assert_angry_police_captain_parity(elisp_form, expect);
}
