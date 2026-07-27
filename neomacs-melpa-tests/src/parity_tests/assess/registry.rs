use super::{assert_assess_autoload_parity, assert_assess_parity};
use expect_test::{Expect, expect};

#[test]
fn package_loads_with_its_runtime_dependency_and_registers_core_feature() {
    let elisp_form = r##"
(list
 (featurep 'assess)
 (featurep 'm-buffer)
 (mapcar
  #'fboundp
  '(assess=
    assess-with-temp-buffers
    assess-with-filesystem
    assess-indentation=
    assess-face-at=)))
"##;
    let expect: Expect = expect!["OK (t t (t t t t t))"];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn core_callable_registry_matches_the_complete_pinned_source_surface_and_arglists() {
    let elisp_form = r##"
(mapcar
 (lambda (symbol)
   (list
    symbol
    (if (macrop symbol)
        'macro
      (if (commandp symbol)
          'command
        'function))
    (help-function-arglist
     symbol t)))
 '(assess--ert-pp-with-indentation-and-newline
   assess-with-preserved-buffer-list
   assess--temp-buffer-let-form
   assess-with-temp-buffers
   assess-as-temp-buffer
   assess-ensure-string
   assess-buffer
   assess-file
   assess--write-file-silently
   assess--explainer-diff-string=
   assess--explainer-simple-string=
   assess=
   assess-explain=
   assess--make-related-file-1
   assess-make-related-file
   assess-with-find-file
   assess-with-filesystem--make-parent
   assess-with-filesystem--init
   assess-with-filesystem
   assess--indent-buffer
   assess--indent-in-mode
   assess-indentation=
   assess-explain-indentation=
   assess--buffer-unindent
   assess--roundtrip-1
   assess-roundtrip-indentation=
   assess-explain-roundtrip-indentation=
   assess--file-roundtrip-1
   assess-file-roundtrip-indentation=
   assess-explain-file-roundtrip-indentation=
   assess--face-at-location=
   assess--face-at=
   assess--face-at=-1
   assess-face-at=
   assess-explain-face-at=
   assess--file-face-at=-1
   assess-file-face-at=
   assess-explain-file-face-at=))
"##;
    let expect: Expect = expect![
        "OK ((assess--ert-pp-with-indentation-and-newline function (orig object)) (assess-with-preserved-buffer-list macro (&rest body)) (assess--temp-buffer-let-form function (item)) (assess-with-temp-buffers macro (varlist &rest body)) (assess-as-temp-buffer macro (x &rest body)) (assess-ensure-string function (x)) (assess-buffer function (arg1 &optional arg2)) (assess-file function (f)) (assess--write-file-silently function (filename)) (assess--explainer-diff-string= function (a b)) (assess--explainer-simple-string= function (a b)) (assess= function (a b)) (assess-explain= function (a b)) (assess--make-related-file-1 function (file &optional directory)) (assess-make-related-file function (file &optional directory)) (assess-with-find-file macro (file &rest body)) (assess-with-filesystem--make-parent function (spec path)) (assess-with-filesystem--init function (spec &optional path)) (assess-with-filesystem macro (spec &rest forms)) (assess--indent-buffer function (&optional column)) (assess--indent-in-mode function (mode unindented)) (assess-indentation= function (mode unindented indented)) (assess-explain-indentation= function (mode unindented indented)) (assess--buffer-unindent function (buffer)) (assess--roundtrip-1 function (comp mode indented)) (assess-roundtrip-indentation= function (mode indented)) (assess-explain-roundtrip-indentation= function (mode indented)) (assess--file-roundtrip-1 function (comp file)) (assess-file-roundtrip-indentation= function (file)) (assess-explain-file-roundtrip-indentation= function (file)) (assess--face-at-location= function (location face property throw-on-nil)) (assess--face-at= function (buffer locations faces property throw-on-nil)) (assess--face-at=-1 function (x mode locations faces property throw-on-nil)) (assess-face-at= function (x mode locations faces &optional property)) (assess-explain-face-at= function (x mode locations faces &optional property)) (assess--file-face-at=-1 function (file locations faces property throw-on-nil)) (assess-file-face-at= function (file locations faces &optional property)) (assess-explain-file-face-at= function (file locations faces &optional property)))"
    ];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn core_runtime_registrations_include_error_metadata_explainers_and_advice() {
    let elisp_form = r##"
(list
 (get
  'assess-deliberate-error
  'error-conditions)
 (get
  'assess-deliberate-error
  'error-message)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (get symbol
          'ert-explainer)))
  '(assess=
    assess-indentation=
    assess-roundtrip-indentation=
    assess-file-roundtrip-indentation=
    assess-face-at=
    assess-file-face-at=))
 (not
  (null
   (advice-member-p
    #'assess--ert-pp-with-indentation-and-newline
    'ert--pp-with-indentation-and-newline))))
"##;
    let expect: Expect = expect![[
        r#"OK ((assess-deliberate-error error) "An error deliberately caused during testing." ((assess= assess-explain=) (assess-indentation= assess-explain-indentation=) (assess-roundtrip-indentation= assess-explain-roundtrip-indentation=) (assess-file-roundtrip-indentation= assess-explain-file-roundtrip-indentation=) (assess-face-at= assess-explain-face-at=) (assess-file-face-at= assess-explain-file-face-at=)) t)"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn installed_archive_metadata_and_all_four_library_hashes_match_the_exact_pin() {
    let elisp_form = r##"
(let* ((description
        (cadr
         (assq
          'assess
          package-alist)))
       (directory
        (package-desc-dir
         description)))
  (list
   (package-version-join
    (package-desc-version
     description))
   (mapcar
    (lambda (dependency)
      (list
       (car dependency)
       (package-version-join
        (cadr dependency))))
    (package-desc-reqs
     description))
   (mapcar
    (lambda (filename)
      (list
       filename
       (secure-hash
        'sha256
        (with-temp-buffer
          (insert-file-contents-literally
           (expand-file-name
            filename directory))
          (buffer-string)))))
    '("assess.el"
      "assess-call.el"
      "assess-discover.el"
      "assess-robot.el"))))
"##;
    let expect: Expect = expect![[
        r#"OK ("20240303.1454" ((emacs "24.4") (m-buffer "0.15")) (("assess.el" "1e73d320ce3db6e83d99b00adfbf6ad559f5642c9063797337f755eebd88e6fa") ("assess-call.el" "1b7f0278c34de7bebd7687214ad02b7cb062147b667e65c95b5cb446d7620514") ("assess-discover.el" "425c9bb021775fe4961116fdeab18b365ab44f322ed0278d21b445b0e5482fb7") ("assess-robot.el" "a710f573030e78482735d14edfb78816299ba35de9299ce170b41f9fde5b9a0a")))"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn core_source_owns_public_and_internal_definitions_and_records_feature_provenance() {
    let elisp_form = r##"
(list
 (mapcar
  (lambda (symbol)
    (let ((source
           (symbol-file
            symbol 'defun)))
      (and source
           (file-name-nondirectory
            source))))
  '(assess=
    assess-with-temp-buffers
    assess-with-filesystem
    assess-roundtrip-indentation=
    assess-face-at=))
 (featurep 'assess)
 (let ((entry
        (seq-find
         (lambda (history)
           (member
            '(provide . assess)
            (cdr history)))
         load-history)))
   (file-name-nondirectory
    (or (car entry) ""))))
"##;
    let expect: Expect = expect![[
        r#"OK (("assess.el" "assess.el" "assess.el" "assess.el" "assess.el") t "assess.el")"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_file_registers_only_published_batch_discovery_commands() {
    let elisp_form = r##"
(list
 (mapcar
  (lambda (symbol)
    (let ((definition
           (symbol-function symbol)))
      (list
       symbol
       (autoloadp definition)
       (nth 1 definition)
       (nth 3 definition)
       (nth 4 definition))))
  '(assess-discover-run-batch
    assess-discover-run-and-exit-batch))
 (featurep 'assess-autoloads)
 (boundp
  'register-definition-prefixes))
"##;
    let expect: Expect = expect![[
        r#"OK (((assess-discover-run-batch t "assess-discover" nil nil) (assess-discover-run-and-exit-batch t "assess-discover" nil nil)) t nil)"#
    ]];
    assert_assess_autoload_parity(elisp_form, expect);
}
