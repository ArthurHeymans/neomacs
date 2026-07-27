use expect_test::expect;

use super::{assert_archive_region_autoload_parity, assert_archive_region_parity};

#[test]
fn archive_region_exact_pin_descriptor_dependency_and_origin_metadata_match() {
    let elisp_form = r##"(let ((descriptor
                (cadr
                 (assq
                  'archive-region
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
           descriptor)))"##;
    let expect = expect![[
        r#"OK (archive-region "20200316.1425" "Move region to archive file instead of killing." nil ((emacs (24 4))) ((:maintainers ("rubikitch" . "rubikitch@ruby-lang.org")) (:authors ("rubikitch" . "rubikitch@ruby-lang.org")) (:keywords "languages") (:revdesc . "53cd2d96ea7c") (:commit . "53cd2d96ea7c33f320353982b36854f25c900c2e") (:url . "http://www.emacswiki.org/cgi-bin/wiki/download/archive-region.el")))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_installed_payload_has_exact_inventory_sizes_and_content_digests() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'archive-region
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
        r#"OK (("README-elpa" 180 "3b28884f0675683382f49da4664f2e71679ce7f80d34967cb5d3c05d6b3912a7") ("archive-region-autoloads.el" 750 "5c6a09671583f12c3740f91aae8e52db290d8fce9c68fbdcd63790ace94afc8b") ("archive-region-pkg.el" 470 "f15abbd74f475ad7259650e75577a42029dd8bb50cd771a950caeae1b2af4bb9") ("archive-region.el" 5339 "2738fa68be2b2699ee794ffac560a033512bf2fa35e0dc3c06d9f79fe0c308b6") ("archive-region.elc" 2956 "b8427e0ec4c0997d64551d0bb2ebc7f985673aed7b01879bdb84d7ae186254e2"))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_complete_callable_command_arglist_interactive_doc_and_source_surface_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (fboundp symbol)
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
         '(archive-region
           archive-region-add-header
           archive-region-link-to-original
           archive-region-pos
           archive-region-current-archive-file
           archive-region-current-original-file
           archive-region-open-archive-file
           archive-region-open-archive-file-other-window
           kill-region-or-archive-region))"##;
    let expect = expect![[
        r#"OK ((archive-region t t (interactive "r") (s e) "fe925b4a0d5d5f3477e194cf4029a655540d9947479b4d2906ad8be8e4f0163f" "archive-region.el") (archive-region-add-header t nil nil nil nil "archive-region.el") (archive-region-link-to-original t nil nil nil nil "archive-region.el") (archive-region-pos t nil nil (line) nil "archive-region.el") (archive-region-current-archive-file t nil nil nil nil "archive-region.el") (archive-region-current-original-file t nil nil nil nil "archive-region.el") (archive-region-open-archive-file t t (interactive nil) (&optional func) "d579347231a75152d363ca3dd76a76c739a795f51ab2f18440416db684f3a06a" "archive-region.el") (archive-region-open-archive-file-other-window t t (interactive nil) nil "d579347231a75152d363ca3dd76a76c739a795f51ab2f18440416db684f3a06a" "archive-region.el") (kill-region-or-archive-region t t (interactive "p\nr") (arg s e) "930b1a0222e586f37587413f6e7d01f53a0152dfd15cc3f64f09328669f450d7" "archive-region.el"))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_group_suffix_and_date_variables_have_exact_contracts() {
    let elisp_form = r##"(list
         (list
          (get
           'archive-region
           'group-documentation)
          (get
           'archive-region
           'custom-group)
          (get
           'archive-region
           'custom-loads))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value
              symbol)
             (default-boundp
              symbol)
             (special-variable-p
              symbol)
             (local-variable-if-set-p
              symbol)
             (get symbol 'custom-type)
             (get symbol 'custom-group)
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
          '(archive-region-filename-suffix
            archive-region-date-format)))"##;
    let expect = expect![[
        r#"OK (("archive-region" nil nil) ((archive-region-filename-suffix "_archive" t t nil nil nil nil "archive-region.el") (archive-region-date-format "[%Y/%m/%d]" t t nil nil nil nil "archive-region.el")))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_load_replaces_global_kill_region_bindings_with_dispatch_command() {
    let elisp_form = r##"(list
         (lookup-key
          global-map
          (kbd
           "C-w"))
         (key-binding
          (kbd
           "C-w")
          t)
         (where-is-internal
          'kill-region-or-archive-region
          global-map)
         (where-is-internal
          'kill-region
          global-map)
         (eq
          (lookup-key
           global-map
           (kbd
            "C-w"))
          'kill-region-or-archive-region)
         (commandp
          (lookup-key
           global-map
           (kbd
            "C-w"))))"##;
    let expect = expect![
        "OK (kill-region-or-archive-region kill-region-or-archive-region ([23] [S-delete] [cut] [menu-bar edit cut]) nil t t)"
    ];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_generated_autoload_file_registers_prefix_without_public_command_autoloads() {
    let elisp_form = r##"(list
         (featurep
          'archive-region)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (and
              (fboundp symbol)
              (autoloadp
               (symbol-function
                symbol)))
             (commandp symbol)))
          '(archive-region
            archive-region-open-archive-file
            archive-region-open-archive-file-other-window
            kill-region-or-archive-region))
         (let ((entry
                (assq
                 'archive-region
                 load-history)))
           (and
            entry
            (mapcar
             (lambda (item)
               (cond
                ((stringp item)
                 (file-name-nondirectory
                  item))
                (t item)))
             entry))))"##;
    let expect = expect![
        "OK (nil ((archive-region nil nil nil) (archive-region-open-archive-file nil nil nil) (archive-region-open-archive-file-other-window nil nil nil) (kill-region-or-archive-region nil nil nil)) nil)"
    ];

    assert_archive_region_autoload_parity(elisp_form, expect);
}

#[test]
fn archive_region_feature_dependencies_and_repeated_require_are_idempotent() {
    let elisp_form = r##"(let ((before-binding
                (lookup-key
                 global-map
                 (kbd
                  "C-w")))
               (before-count
                (length
                 (seq-filter
                  (lambda (feature)
                    (eq
                     feature
                     'archive-region))
                  features))))
         (list
          (mapcar
           #'featurep
           '(archive-region
             cl-lib
             newcomment))
          (require
           'archive-region)
          before-binding
          (lookup-key
           global-map
           (kbd
            "C-w"))
          before-count
          (length
           (seq-filter
            (lambda (feature)
              (eq
               feature
               'archive-region))
            features))
          (let ((file
                 (symbol-file
                  'archive-region
                  'defun)))
            (and
             file
             (file-name-nondirectory
              file)))))"##;
    let expect = expect![[
        r#"OK ((t t t) archive-region kill-region-or-archive-region kill-region-or-archive-region 1 1 "archive-region.el")"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}
