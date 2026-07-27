use expect_test::expect;

use super::{assert_aqi_autoload_parity, assert_aqi_parity};

#[test]
fn aqi_exact_pin_descriptor_dependencies_origin_and_loaded_features_match() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'aqi
                    package-alist)))
                 (request-descriptor
                  (cadr
                   (assq
                    'request
                    package-alist)))
                 (let-alist-descriptor
                  (cadr
                   (assq
                    'let-alist
                    package-alist))))
         (list
          (package-desc-name
           descriptor)
          (package-version-join
           (package-desc-version
            descriptor))
          (package-desc-summary
           descriptor)
          (package-desc-kind
           descriptor)
          (package-desc-reqs
           descriptor)
          (package-desc-extras
           descriptor)
          (list
           (package-desc-name
            request-descriptor)
           (package-version-join
            (package-desc-version
             request-descriptor)))
          (and
           let-alist-descriptor
           (list
            (package-desc-name
             let-alist-descriptor)
            (package-version-join
             (package-desc-version
              let-alist-descriptor))))
          (package-built-in-p
           'let-alist)
          (let ((library
                 (locate-library
                  "let-alist")))
            (and
             library
             (file-name-nondirectory
              library)))
          (mapcar
           #'featurep
           '(aqi
             request
             let-alist))))"##;
    let expect = expect![[
        r#"OK (aqi "20230530.1204" "Air quality data from the World Air Quality Index." nil ((emacs (25 1)) (request (0 3)) (let-alist (0 0))) ((:maintainers ("nik gaffney" . "nik@fo.am")) (:authors ("nik gaffney" . "nik@fo.am")) (:keywords "air quality" "aqi" "pollution" "weather" "data") (:revdesc . "cbff3c6ce691") (:commit . "cbff3c6ce691a3a1d2f5636384e29d43f0e1d236") (:url . "https://github.com/zzkt/aqi")) (request "20250219.2213") nil t "let-alist.el" (t t t))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_installed_payload_has_exact_inventory_sizes_and_content_digests() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'aqi
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor)))
         (mapcar
          (lambda (file)
            (let ((path
                   (expand-file-name
                    file
                    directory)))
              (list
               file
               (file-attribute-size
                (file-attributes
                 path))
               (secure-hash
                'sha256
                path))))
          (sort
           (seq-filter
            (lambda (file)
              (file-regular-p
               (expand-file-name
                file
                directory)))
            (directory-files
             directory
             nil
             "\\`[^.]"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 584 "0c8704f0c7952c9b9b213496fa69a6c196867ff09d6f2c7da6bae9cb1246d272") ("aqi-autoloads.el" 1037 "1f66bcbed6796d555820db78bec0128af851fa25233aaef4d3c675ffb8d3591e") ("aqi-pkg.el" 486 "9988f1aedc3e68f5ff9037573bc896ec7cac91c2d3576b955f905da1e53fc51b") ("aqi.el" 9811 "51c9454d3413f835781a9c4620f4cdf5a8e8fcaffa9fe12dcb9556dad8cd0e97") ("aqi.elc" 7997 "90a1011eb490d5c7c23d3a7e3c753461efd44c7c176c887a8da36c4701e9cfc6"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_complete_callable_macro_command_arglist_documentation_and_source_surface_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (fboundp symbol)
            (macrop symbol)
            (commandp symbol)
            (interactive-form
             symbol)
            (help-function-arglist
             symbol
             t)
            (let ((doc
                   (documentation
                    symbol
                    t)))
              (and
               doc
               (secure-hash
                'sha256
                doc)))
            (let ((file
                   (symbol-file
                    symbol
                    'defun)))
              (and
               file
               (file-name-nondirectory
                file)))))
         '(aqi--city-cache-clear
           aqi--city-cache-update
           aqi--city-cache-get
           aqi--cached-city?
           aqi--make-city-raw-accessor
           aqi--make-city-format-accessor
           aqi-city-aqi
           aqi-city-lonlat
           aqi-request
           aqi-request-geo
           aqi-request-cached
           aqi-search
           aqi-report-brief
           aqi-report-full
           aqi-report))"##;
    let expect = expect![[
        r#"OK ((aqi--city-cache-clear t nil nil nil (&optional city) "1ba5c5ddfc68f45b571cb63f802518c7de6d3d8e3936a2354fd03b50d5bac73b" "aqi.el") (aqi--city-cache-update t nil nil nil (city) "4fd784e8eab7430f737ef581121087eda9b8b1958e9b7f70dbe91de9b533f9a8" "aqi.el") (aqi--city-cache-get t nil nil nil (city) "4fd784e8eab7430f737ef581121087eda9b8b1958e9b7f70dbe91de9b533f9a8" "aqi.el") (aqi--cached-city? t nil nil nil (city) "5185b25c73f1e973f311231ffc92753b7d7af84912e6760331acc5dd233e3513" "aqi.el") (aqi--make-city-raw-accessor t t nil nil (name aref) "25c6223b4cf54e479027f06172efe7860fe9516e9a9b21444e573403445a3d5e" "aqi.el") (aqi--make-city-format-accessor t t nil nil (name aref) "25c6223b4cf54e479027f06172efe7860fe9516e9a9b21444e573403445a3d5e" "aqi.el") (aqi-city-aqi t nil nil nil (city) nil nil) (aqi-city-lonlat t nil nil nil (city) nil nil) (aqi-request t nil nil nil (city) "447859fde165a994be4f0741458db06823553d54edb33ffe235d1699cc35feb7" "aqi.el") (aqi-request-geo t nil nil nil (latitude longitude) "380915851a1fece8bfd6a6dc13522c0da7196dbc92df62b2cf903796a193c10d" "aqi.el") (aqi-request-cached t nil nil nil (city) "a672b35a27ce2520809a06191e4c515c7a32bf1bb3cc4138892ad66c962d85a3" "aqi.el") (aqi-search t nil nil nil (name) "7e858ae8bef53dfc560a337bc4777f13eaf54f87d4c97002ec7fc28019763b2b" "aqi.el") (aqi-report-brief t nil nil nil (&optional place) "09bddd143a42fb79576b7da214b8d19712fb359bbd6c5b5de164d0a302f48e3e" "aqi.el") (aqi-report-full t nil nil nil (&optional place) "f156a604101433bc27ede1dcad96dc12f510ff79d75f80f05158097f4d45d8c4" "aqi.el") (aqi-report t nil t (interactive "sName of city or monitoring station (RET for \"here\"): ") (&optional place type) "630b1f522ca405cf815a0e3c20debe0b90fa26a0340996ccd2a2e1242e956c2e" "aqi.el"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_custom_group_and_complete_custom_variable_contracts_match() {
    let elisp_form = r##"(list
         (list
          (get
           'aqi
           'group-documentation)
          (get
           'aqi
           'custom-prefix)
          (get
           'aqi
           'custom-group)
          (get
           'aqi
           'custom-loads))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value
              symbol)
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (get symbol 'standard-value)
             (custom-variable-p
              symbol)
             (local-variable-if-set-p
              symbol)
             (documentation-property
              symbol
              'variable-documentation
              t)
             (let ((file
                    (symbol-file
                     symbol
                     'defvar)))
               (and
                file
                (file-name-nondirectory
                 file)))))
          '(aqi-api-key
            aqi-use-cache
            aqi-cache-refresh-period)))"##;
    let expect = expect![[
        r#"OK (("Fetch and display air quality data from WAQI." "aqi-" ((aqi-api-key custom-variable) (aqi-use-cache custom-variable) (aqi-cache-refresh-period custom-variable)) nil) ((aqi-api-key "demo" string nil #1=((funcall #'#[nil ("demo") #2=(t)])) #1# nil "A valid API key from http://aqicn.org/data-platform/token/ to access WAQI." "aqi.el") (aqi-use-cache nil boolean nil #3=((funcall #'#[nil (nil) #2#])) #3# nil "When set to t will use cached data, otherwise get new data on each call." "aqi.el") (aqi-cache-refresh-period 0 number nil #4=((funcall #'#[nil (0) #2#])) #4# nil "Cached data can be refreshed at a given interval (in minutes).\nSet to nil to never refresh." "aqi.el")))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_cache_variable_default_documentation_mutability_and_buffer_locality_match() {
    let elisp_form = r##"(let ((initial
                (copy-tree
                 aqi-cached-data)))
         (setq
          aqi-cached-data
          '(("Osaka" . 42)
            ("Taipei" . 17)))
         (list
          initial
          aqi-cached-data
          (default-boundp
           'aqi-cached-data)
          (special-variable-p
           'aqi-cached-data)
          (local-variable-if-set-p
           'aqi-cached-data)
          (documentation-property
           'aqi-cached-data
           'variable-documentation
           t)
          (let ((file
                 (symbol-file
                  'aqi-cached-data
                  'defvar)))
            (and
             file
             (file-name-nondirectory
              file)))
          (with-temp-buffer
            (setq-local
             aqi-cached-data
             'local)
            (list
             aqi-cached-data
             (local-variable-p
              'aqi-cached-data)))
          aqi-cached-data))"##;
    let expect = expect![[
        r#"OK ((("None" . "None")) #1=(("Osaka" . 42) ("Taipei" . 17)) t t nil "Data is cached as an alist of city names and results." "aqi.el" (local t) #1#)"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_autoloads_register_all_public_reports_without_loading_the_package() {
    let elisp_form = r##"(list
         (featurep
          'aqi)
         (mapcar
          (lambda (symbol)
            (let ((function
                   (symbol-function
                    symbol)))
              (list
               symbol
               (fboundp symbol)
               (autoloadp function)
               (and
                (autoloadp function)
                (nth 1 function))
               (and
                (autoloadp function)
                (nth 4 function))
               (commandp symbol)
               (interactive-form
                symbol))))
          '(aqi-report-brief
            aqi-report-full
            aqi-report)))"##;
    let expect = expect![[
        r#"OK (nil ((aqi-report-brief t t "aqi" nil nil nil) (aqi-report-full t nil nil nil nil nil) (aqi-report t nil nil nil t (interactive "sName of city or monitoring station (RET for \"here\"): "))))"#
    ]];

    assert_aqi_autoload_parity(elisp_form, expect);
}

#[test]
fn aqi_public_command_interactive_contract_and_feature_provision_match() {
    let elisp_form = r##"(list
         (featurep
          'aqi)
         (commandp
          'aqi-report)
         (interactive-form
          'aqi-report)
         (help-function-arglist
          'aqi-report
          t)
         (commandp
          'aqi-report-brief)
         (commandp
          'aqi-report-full)
         (provide
          'aqi)
         (length
          (seq-filter
           (lambda (feature)
             (eq feature 'aqi))
           features)))"##;
    let expect = expect![[
        r#"OK (t t (interactive "sName of city or monitoring station (RET for \"here\"): ") (&optional place type) nil nil aqi 1)"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}
