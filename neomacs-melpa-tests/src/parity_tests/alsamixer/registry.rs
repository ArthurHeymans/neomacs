use expect_test::expect;

use super::{assert_alsamixer_autoload_parity, assert_alsamixer_parity};

#[test]
fn exact_release_descriptor_and_source_identity_are_stable() {
    let elisp_form = r##"
(let* ((descriptor (cadr (assq 'alsamixer package-alist)))
       (extras (package-desc-extras descriptor)))
  (list
   (package-desc-name descriptor)
   (package-version-join (package-desc-version descriptor))
   (package-desc-summary descriptor)
   (package-desc-reqs descriptor)
   (alist-get :url extras)
   (alist-get :commit extras)
   (alist-get :revdesc extras)
   (alist-get :keywords extras)
   (featurep 'alsamixer)
   (file-name-nondirectory (locate-library "alsamixer"))))
"##;
    let expect = expect![[
        r#"OK (alsamixer "20250106.1025" "Functions to call out to amixer." nil "https://codeberg.org/rwv/alsamixer-el" "5f5a1f26637ca1b2a8ac964fc86a59522e3f778e" "5f5a1f26637c" ("convenience") t "alsamixer.el")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn customization_group_and_every_user_option_keep_exact_contracts() {
    let elisp_form = r##"
(list
 (get 'alsamixer 'custom-group)
 (get 'alsamixer 'group-documentation)
 (get 'alsamixer 'custom-prefix)
 (mapcar
  (lambda (variable)
    (list
     variable
     (default-value variable)
     (custom-variable-p variable)
     (get variable 'standard-value)
     (get variable 'custom-type)
     (get variable 'custom-group)
     (documentation-property
      variable 'variable-documentation)))
  '(alsamixer-default-volume-increment
    alsamixer-amixer-command
    alsamixer-card
    alsamixer-device
    alsamixer-control)))
"##;
    let expect = expect![[
        r#"OK (((alsamixer-default-volume-increment custom-variable) (alsamixer-amixer-command custom-variable) (alsamixer-card custom-variable) (alsamixer-device custom-variable) (alsamixer-control custom-variable)) "Functions to call out to amixer." "alsamixer-" ((alsamixer-default-volume-increment 5 #1=(5) #1# integer nil "Default percentage to increment (or decrement) the volume of master.") (alsamixer-amixer-command "amixer" #2=("amixer") #2# string nil "Name of amixer command.") (alsamixer-card nil #3=(nil) #3# string nil "Card number to control.") (alsamixer-device nil #4=(nil) #4# string nil "Device name to control.") (alsamixer-control "Master" #5=("Master") #5# string nil "Name of control.")))"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn complete_function_signatures_docs_and_interactive_contracts_are_stable() {
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
 '(alsamixer-command
   alsamixer-get-volume
   alsamixer-set-volume
   alsamixer-up-volume
   alsamixer-down-volume
   alsamixer-toggle-mute))
"##;
    let expect = expect![[
        r#"OK ((alsamixer-command (args &rest objs) nil nil "436740ed6b3b9abd369ca6bd5b2caf4e78d062b52060c7bd74f617f874b98416") (alsamixer-get-volume nil nil nil "46af44c868edaf092d1f1feddf338e3dd079e63cd9d15cbbc264cc13eb0b9958") (alsamixer-set-volume (perc) t (interactive "nVolume (percentage): ") "6d274fa173d261f2057d9f4110208f174ec6d21355569eec853b0cfbab6bc85f") (alsamixer-up-volume (&optional perc) t (interactive "P") "26c9b5bab9f145ebd3998480847c79a4ce5fd4e7548d4699af359d7e460cb463") (alsamixer-down-volume (&optional perc) t (interactive "P") "26c9b5bab9f145ebd3998480847c79a4ce5fd4e7548d4699af359d7e460cb463") (alsamixer-toggle-mute nil t (interactive nil) "8c7deb7bc8e6c4a00b1eb39b5aa80871681eef1b6194dbdd9cbfb48fde65152f"))"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn source_load_registers_feature_and_complete_callable_surface() {
    let elisp_form = r##"
(list
 (featurep 'alsamixer)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (fboundp symbol)
     (and (fboundp symbol)
          (autoloadp (symbol-function symbol)))))
  '(alsamixer-command
    alsamixer-get-volume
    alsamixer-set-volume
    alsamixer-up-volume
    alsamixer-down-volume
    alsamixer-toggle-mute))
 (seq-filter
  (lambda (feature)
    (string-prefix-p "alsamixer"
                     (symbol-name feature)))
  features))
"##;
    let expect = expect![
        "OK (t ((alsamixer-command t nil) (alsamixer-get-volume t nil) (alsamixer-set-volume t nil) (alsamixer-up-volume t nil) (alsamixer-down-volume t nil) (alsamixer-toggle-mute t nil)) (alsamixer alsamixer-autoloads))"
    ];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_exposes_four_commands_without_loading_source() {
    let elisp_form = r##"
(list
 (featurep 'alsamixer)
 (mapcar
  (lambda (symbol)
    (let ((definition
           (and (fboundp symbol)
                (symbol-function symbol))))
      (list
       symbol
       (commandp symbol)
       (and (autoloadp definition) t)
       (and (autoloadp definition)
            (nth 1 definition))
       (and (autoloadp definition)
            (nth 3 definition))
       (and (autoloadp definition)
            (nth 4 definition)))))
  '(alsamixer-set-volume
    alsamixer-up-volume
    alsamixer-down-volume
    alsamixer-toggle-mute
    alsamixer-command
    alsamixer-get-volume)))
"##;
    let expect = expect![[
        r#"OK (nil ((alsamixer-set-volume t t "alsamixer" t nil) (alsamixer-up-volume t t "alsamixer" t nil) (alsamixer-down-volume t t "alsamixer" t nil) (alsamixer-toggle-mute t t "alsamixer" t nil) (alsamixer-command nil nil nil nil nil) (alsamixer-get-volume nil nil nil nil nil)))"#
    ]];
    assert_alsamixer_autoload_parity(elisp_form, expect);
}

#[test]
fn autoloaded_set_volume_loads_source_and_executes_real_fake_amixer() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "")
  (let ((before
         (list
          (featurep 'alsamixer)
          (autoloadp
           (symbol-function 'alsamixer-set-volume)))))
    (let ((result (alsamixer-set-volume 64)))
      (list
       before
       result
       (featurep 'alsamixer)
       (autoloadp
        (symbol-function 'alsamixer-set-volume))
       (alsamixer-test-log)))))
"##;
    let expect = expect![[
        r#"OK ((nil t) "Volume set to 64%" t nil "<sset>\n<Master>\n<playback>\n<64%>\n")"#
    ]];
    assert_alsamixer_autoload_parity(elisp_form, expect);
}
