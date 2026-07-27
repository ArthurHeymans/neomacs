use expect_test::expect;

use super::{assert_all_the_icons_autoload_parity, assert_all_the_icons_parity};

#[test]
fn all_the_icons_registry_exposes_exact_families_data_and_configuration() {
    let elisp_form = r##"(list
         (featurep 'all-the-icons)
         (featurep 'all-the-icons-faces)
         all-the-icons-font-families
         all-the-icons-font-names
         (mapcar
          (lambda (family)
            (let* ((data-function
                    (intern
                     (format "all-the-icons-%s-data" family)))
                   (data (funcall data-function)))
              (list family
                    (length data)
                    (car data)
                    (car (last data)))))
          all-the-icons-font-families)
         (list all-the-icons-color-icons
               all-the-icons-scale-factor
               all-the-icons-default-adjust
               all-the-icons-fonts-subdirectory
               all-the-icons--cache-limit)
         (mapcar #'length
                 (list all-the-icons-extension-icon-alist
                       all-the-icons-regexp-icon-alist
                       all-the-icons-dir-icon-alist
                       all-the-icons-weather-icon-alist
                       all-the-icons-mode-icon-alist
                       all-the-icons-url-alist)))"##;
    let expect = expect![[
        r#"OK (t t (material wicon octicon faicon fileicon alltheicon) ("material-design-icons.ttf" "weathericons.ttf" "octicons.ttf" "fontawesome.ttf" "file-icons.ttf" "all-the-icons.ttf") ((material 932 ("3d_rotation" . "") ("zoom_out_map" . "")) (wicon 587 ("alien" . "") ("yahoo-9" . "")) (octicon 158 ("alert" . "") ("zap" . "⚡")) (faicon 634 ("500px" . "") ("youtube-square" . "")) (fileicon 495 ("1c" . "ꗪ") ("zimpl" . "")) (alltheicon 62 ("apache" . "") ("wave-right" . ""))) (t 1.2 -0.2 nil 2048) (262 46 16 30 201 85))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_generated_family_api_has_exact_callable_contracts() {
    let elisp_form = r##"(mapcar
         (lambda (family)
           (let ((icon-f
                  (intern (format "all-the-icons-%s" family)))
                 (family-f
                  (intern
                   (format "all-the-icons-%s-family" family)))
                 (data-f
                  (intern
                   (format "all-the-icons-%s-data" family)))
                 (insert-f
                  (intern
                   (format "all-the-icons-insert-%s" family))))
             (list family
                   (help-function-arglist icon-f t)
                   (help-function-arglist family-f t)
                   (help-function-arglist data-f t)
                   (help-function-arglist insert-f t)
                   (commandp icon-f)
                   (commandp insert-f)
                   (get
                    (intern
                     (format "all-the-icons-%s-scale-factor"
                             family))
                    'custom-type)
                   (get
                    (intern
                     (format
                      "all-the-icons-default-%s-adjust"
                      family))
                    'custom-type))))
         all-the-icons-font-families)"##;
    let expect = expect![
        "OK ((material #1=(icon-name &rest args) nil nil #2=(&optional arg) nil t number number) (wicon #1# nil nil #2# nil t number number) (octicon #1# nil nil #2# nil t number number) (faicon #1# nil nil #2# nil t number number) (fileicon #1# nil nil #2# nil t number number) (alltheicon #1# nil nil #2# nil t number number))"
    ];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_faces_register_complete_palette_and_stable_specs() {
    let elisp_form = r##"(let* ((faces
                 (sort
                  (seq-filter
                   (lambda (symbol)
                     (and
                      (string-prefix-p
                       "all-the-icons-" (symbol-name symbol))
                      (facep symbol)))
                   (apropos-internal "^all-the-icons-"))
                  (lambda (left right)
                    (string-lessp
                     (symbol-name left)
                     (symbol-name right))))))
         (list
          (length faces)
          faces
          (mapcar
           (lambda (face)
             (list face
                   (get face 'face-documentation)
                   (get face 'face-defface-spec)
                   (get face 'custom-group)))
           '(all-the-icons-red
             all-the-icons-lgreen
             all-the-icons-blue-alt
             all-the-icons-purple
             all-the-icons-cyan-alt
             all-the-icons-dsilver))))"##;
    let expect = expect![[
        r##"OK (34 (all-the-icons-blue all-the-icons-blue-alt all-the-icons-cyan all-the-icons-cyan-alt all-the-icons-dblue all-the-icons-dcyan all-the-icons-dgreen all-the-icons-dmaroon all-the-icons-dorange all-the-icons-dpink all-the-icons-dpurple all-the-icons-dred all-the-icons-dsilver all-the-icons-dyellow all-the-icons-green all-the-icons-lblue all-the-icons-lcyan all-the-icons-lgreen all-the-icons-lmaroon all-the-icons-lorange all-the-icons-lpink all-the-icons-lpurple all-the-icons-lred all-the-icons-lsilver all-the-icons-lyellow all-the-icons-maroon all-the-icons-orange all-the-icons-pink all-the-icons-purple all-the-icons-purple-alt all-the-icons-red all-the-icons-red-alt all-the-icons-silver all-the-icons-yellow) ((all-the-icons-red "Face for red icons" ((((background dark)) :foreground "#AC4142") (((background light)) :foreground "#AC4142")) nil) (all-the-icons-lgreen "Face for lgreen icons" ((((background dark)) :foreground "#C6E87A") (((background light)) :foreground "#3D6837")) nil) (all-the-icons-blue-alt "Face for blue icons" ((((background dark)) :foreground "#2188b6") (((background light)) :foreground "#2188b6")) nil) (all-the-icons-purple "Face for purple icons" ((((background dark)) :foreground "#AA759F") (((background light)) :foreground "#68295B")) nil) (all-the-icons-cyan-alt "Face for cyan icons" ((((background dark)) :foreground "#61dafb") (((background light)) :foreground "#0595bd")) nil) (all-the-icons-dsilver "Face for dsilver icons" ((((background dark)) :foreground "#838484") (((background light)) :foreground "#838484")) nil)))"##
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_every_configured_icon_reference_is_callable_like_upstream_suite() {
    let elisp_form = r##"(let ((tables
                '(all-the-icons-extension-icon-alist
                  all-the-icons-regexp-icon-alist
                  all-the-icons-dir-icon-alist
                  all-the-icons-weather-icon-alist
                  all-the-icons-mode-icon-alist
                  all-the-icons-url-alist))
               totals failures)
         (dolist (table tables)
           (let ((checked 0))
             (dolist (config (symbol-value table))
               (when (nth 2 config)
                 (setq checked (1+ checked))
                 (condition-case error-data
                     (let ((icon
                            (apply (nth 1 config)
                                   (nthcdr 2 config))))
                       (unless
                           (and (stringp icon)
                                (= (length icon) 1)
                                (get-text-property 0 'face icon)
                                (get-text-property 0 'display icon))
                         (push
                          (list table (car config) 'bad-shape)
                          failures)))
                   (error
                    (push
                     (list table (car config) error-data)
                     failures)))))
             (push (list table checked) totals)))
         (list (nreverse totals)
               (length failures)
               (nreverse failures)))"##;
    let expect = expect![[
        r#"OK (((all-the-icons-extension-icon-alist 262) (all-the-icons-regexp-icon-alist 46) (all-the-icons-dir-icon-alist 16) (all-the-icons-weather-icon-alist 30) (all-the-icons-mode-icon-alist 200) (all-the-icons-url-alist 85)) 1 ((all-the-icons-url-alist "^\\(https?://\\)?\\(www\\.\\)?shirtsinbulk\\.com" (error "Unable to find icon with name ‘shitsinbulk’ in icon set ‘faicon’"))))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_autoload_contract_publishes_commands_without_loading_runtime() {
    let elisp_form = r##"(let ((symbols
                '(all-the-icons-icon-for-dir
                  all-the-icons-icon-for-file
                  all-the-icons-icon-for-mode
                  all-the-icons-icon-for-url
                  all-the-icons-install-fonts
                  all-the-icons-insert)))
         (list
          (featurep 'all-the-icons)
          (mapcar
           (lambda (symbol)
             (list symbol
                   (fboundp symbol)
                   (autoloadp
                    (and (fboundp symbol)
                         (symbol-function symbol)))
                   (commandp symbol)
                   (help-function-arglist symbol t)))
           symbols)
          (file-name-nondirectory
           (getenv "NEOMACS_PACKAGE_SOURCE"))))"##;
    let expect = expect![[
        r#"OK (nil ((all-the-icons-icon-for-dir t t nil "[Arg list not available until function definition is loaded.]") (all-the-icons-icon-for-file t t nil "[Arg list not available until function definition is loaded.]") (all-the-icons-icon-for-mode t t nil "[Arg list not available until function definition is loaded.]") (all-the-icons-icon-for-url t t nil "[Arg list not available until function definition is loaded.]") (all-the-icons-install-fonts t t t "[Arg list not available until function definition is loaded.]") (all-the-icons-insert t t t "[Arg list not available until function definition is loaded.]")) "all-the-icons-autoloads.el")"#
    ]];
    assert_all_the_icons_autoload_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_data_tables_are_content_addressed_and_duplicate_aware() {
    let elisp_form = r##"(mapcar
         (lambda (family)
           (let* ((data
                   (funcall
                    (intern
                     (format "all-the-icons-%s-data" family))))
                  (names (mapcar #'car data)))
             (list family
                   (length data)
                   (length (delete-dups (copy-sequence names)))
                   (secure-hash 'sha256
                                (prin1-to-string data)))))
         all-the-icons-font-families)"##;
    let expect = expect![[
        r#"OK ((material 932 932 "80623b4588351a85705b100a16facd495df3eb0a1eb8baddb525d064554cbfa7") (wicon 587 584 "849e2311390d326b3cccf0f011c78af0404f4c7a49ccf1008fcf02676db60e9f") (octicon 158 158 "328c36ceea59428c1aa0c1d354d2609ff97c4ea4f5b7cf44f1a9e74fbc300e06") (faicon 634 634 "809004440e307e16d29272f0b1ad184b2c5ba6e123ae186bde7fe557b275ae16") (fileicon 495 495 "15ccae7e582a3ae73894eb4d94cb61a6cbe121aba8c3e8e44d4389b24437eab3") (alltheicon 62 62 "7d22581e6bd3792b4507850151b29d6983194f2c7722e0cbfcd95f0e474c28f1"))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}
