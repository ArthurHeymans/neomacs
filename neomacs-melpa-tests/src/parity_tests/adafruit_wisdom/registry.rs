use expect_test::expect;

use super::{assert_adafruit_wisdom_autoload_parity, assert_adafruit_wisdom_parity};

#[test]
fn adafruit_wisdom_exact_pin_metadata_dependencies_features_and_dependency_resolution_match() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'adafruit-wisdom
                    package-alist)))
                 (request-descriptor
                  (cadr
                   (assq
                    'request
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
          (package-desc-name
           request-descriptor)
          (package-version-join
           (package-desc-version
            request-descriptor))
          (mapcar
           #'featurep
           '(adafruit-wisdom
             request
             dom
             xml))))"##;
    let expect = expect![[
        r#"OK (adafruit-wisdom "20200217.306" "Get/display adafruit.com quotes." nil ((emacs (25 1)) (request (0 3 1))) ((:maintainers ("Neil Okamoto" . "neil.okamoto+melpa@gmail.com")) (:authors ("Neil Okamoto" . "neil.okamoto+melpa@gmail.com")) (:keywords "games") (:revdesc . "c4ae0db35d0b") (:commit . "c4ae0db35d0be94f0e9c50977758224d7e00234a") (:url . "https://github.com/gonewest818/adafruit-wisdom.el")) request "20250219.2213" (t t t t))"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_complete_constant_surface_values_docs_sources_and_locality_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (symbol-value
             symbol)
            (default-boundp
             symbol)
            (local-variable-if-set-p
             symbol)
            (special-variable-p
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
         '(adafruit-wisdom-quote-url
           adafruit-wisdom-cache-file
           adafruit-wisdom-cache-ttl))"##;
    let expect = expect![[
        r#"OK ((adafruit-wisdom-quote-url "https://www.adafruit.com/feed/quotes.xml" t nil t "URL for the RSS quote feed served on adafruit.com." "adafruit-wisdom.el") (adafruit-wisdom-cache-file "~/.emacs.d/adafruit-wisdom.cache" t nil t "Location for the local copy of the quotes file.\nWhen `no-littering' is available put the cache file in the\nspecified var directory.  Otherwise the default location for the\ncache file is `user-emacs-directory'." "adafruit-wisdom.el") (adafruit-wisdom-cache-ttl 86400.0 t nil t "Time-to-live for the local cache file." "adafruit-wisdom.el"))"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_complete_callable_command_and_documentation_surface_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (fboundp
             symbol)
            (help-function-arglist
             symbol
             t)
            (commandp
             symbol)
            (interactive-form
             symbol)
            (documentation
             symbol
             t)
            (let ((file
                   (symbol-file
                    symbol
                    'defun)))
              (and
               file
               (file-name-nondirectory
                file)))))
         '(adafruit-wisdom-cached-get
           adafruit-wisdom-select
           adafruit-wisdom))"##;
    let expect = expect![[
        r#"OK ((adafruit-wisdom-cached-get t nil nil nil "Retrieves RSS from adafruit.com, or from cache if TTL hasn't expired.\nReturns the parsed XML." "adafruit-wisdom.el") (adafruit-wisdom-select t nil nil nil "Select a quote at random and return as a string.\n\nParse assuming the following RSS format:\n     ((rss (channel (item ...) (item ...) (item ...) ...)))\nwhere each item contains:\n     (item (title nil \"the quote\") ...)\nand we  need just \"the quote\"." "adafruit-wisdom.el") (adafruit-wisdom t (&optional arg) t (interactive "P") "Display one of Adafruit's quotes in the minibuffer.\nIf ARG is non-nil the joke will be inserted into the current\nbuffer rather than shown in the minibuffer." "adafruit-wisdom.el"))"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_installed_package_inventory_sizes_and_sha256_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'adafruit-wisdom
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
               (with-temp-buffer
                 (set-buffer-multibyte nil)
                 (insert-file-contents-literally path)
                 (secure-hash
                  'sha256
                  (current-buffer))))))
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
           #'string-lessp)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 249 "9a3cb7c84f85ed88e4319694b37796403e8eca87d002bb7fee1121f982f1c20f") ("adafruit-wisdom-autoloads.el" 1259 "0f07ae834ee455182f09cd6b129d4788daf303b53e31f20e028c48bd1e8e1832") ("adafruit-wisdom-pkg.el" 476 "68d021b7ada3f721594054b28ff37bd4f54d45be3d149a975dade72493580202") ("adafruit-wisdom.el" 3919 "5b5aec0a2d55908cfa3343b4889f0ddd14087ba9f5c01b1212aa3dcd9ee6b452") ("adafruit-wisdom.elc" 2776 "934c46f7537a5d3b05b84e8d6c574f692f8b42cdd9768583205d6d386bda847f"))"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_generated_autoload_surface_registers_both_commands_without_runtime_load() {
    let elisp_form = r##"(list
         (featurep
          'adafruit-wisdom-autoloads)
         (featurep
          'adafruit-wisdom)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp
              symbol)
             (autoloadp
              (symbol-function
               symbol))
             (commandp
              symbol)
             (help-function-arglist
              symbol
              t)
             (nth
              1
              (symbol-function
               symbol))))
          '(adafruit-wisdom-select
            adafruit-wisdom)))"##;
    let expect = expect![[
        r#"OK (t nil ((adafruit-wisdom-select t t nil "[Arg list not available until function definition is loaded.]" "adafruit-wisdom") (adafruit-wisdom t t t "[Arg list not available until function definition is loaded.]" "adafruit-wisdom")))"#
    ]];
    assert_adafruit_wisdom_autoload_parity(elisp_form, expect);
}
