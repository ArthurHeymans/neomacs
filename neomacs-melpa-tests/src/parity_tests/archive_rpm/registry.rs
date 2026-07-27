use expect_test::expect;

use super::{assert_archive_rpm_autoload_parity, assert_archive_rpm_parity};

#[test]
fn package_descriptor_preserves_the_exact_frozen_release_and_provenance() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'archive-rpm package-alist)))
       (directory (package-desc-dir description)))
  (list
   (package-installed-p 'archive-rpm)
   (package-desc-name description)
   (package-version-join (package-desc-version description))
   (package-desc-summary description)
   (package-desc-reqs description)
   (package-desc-extras description)
   (file-name-nondirectory
    (directory-file-name directory))))"##;
    let expect = expect![[
        r#"OK (t archive-rpm "20220527.632" "RPM and CPIO support for archive-mode." ((emacs (24 4))) ((:maintainers ("Magnus Henoch" . "magnus.henoch@gmail.com")) (:authors ("Magnus Henoch" . "magnus.henoch@gmail.com")) (:keywords "files") (:revdesc . "cb48fee04cb0") (:commit . "cb48fee04cb0cbb26f760a3b95649f7dac78c6ec") (:url . "https://github.com/nbarrientos/archive-rpm")) "archive-rpm-20220527.632")"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn installed_archive_contains_only_both_runtime_libraries_and_descriptor() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'archive-rpm package-alist)))
       (directory (package-desc-dir description)))
  (mapcar
   (lambda (name)
     (let ((path (expand-file-name name directory)))
       (list name
             (file-attribute-size (file-attributes path)))))
   (sort
    (seq-remove
     (lambda (name)
       (or (member name '("." ".." "README-elpa"))
           (string-suffix-p ".elc" name)
           (string-suffix-p "-autoloads.el" name)))
     (directory-files directory))
    #'string-lessp)))"##;
    let expect = expect![[
        r#"OK (("archive-cpio.el" 8686) ("archive-rpm-pkg.el" 439) ("archive-rpm.el" 10180))"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn installed_runtime_and_descriptor_match_the_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'archive-rpm package-alist)))
       (directory (package-desc-dir description)))
  (mapcar
   (lambda (name)
     (let ((file (expand-file-name name directory)))
       (list
        name
        (file-attribute-size (file-attributes file))
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert-file-contents-literally file)
          (secure-hash 'sha256 (current-buffer))))))
   '("archive-cpio.el"
     "archive-rpm.el"
     "archive-rpm-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("archive-cpio.el" 8686 "ac89a3baef2ace9a742f2c1a8dbf206190989abdfb6754ffe950605021ec78d6") ("archive-rpm.el" 10180 "47efcbd261018a7c85ad9e9fb9b72d1d727f5956d81a221570e0063467b82e6b") ("archive-rpm-pkg.el" 439 "0f0f47b65cce083588a336560e76fde89239b44689cc768d6a1e650bb051e7c5"))"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_preserves_arguments_commands_and_source_ownership() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (macrop symbol)
    (commandp symbol)
    (copy-tree (help-function-arglist symbol t))
    (interactive-form symbol)
    (file-name-nondirectory
     (symbol-file symbol 'defun))))
 '(archive-cpio-find-type
   archive-cpio--parse-mode
   archive-cpio-summarize
   archive-cpio-extract
   archive-cpio-extract-from-buffer
   archive-rpm-find-type
   archive-rpm-summarize
   archive-rpm-extract
   archive-rpm--decompress-payload
   archive-rpm--insert-interesting-information
   archive-rpm--get-header-data
   archive-rpm--find-index-entry-data
   archive-rpm--parse-header))"##;
    let expect = expect![[
        r#"OK ((archive-cpio-find-type t nil nil nil nil "archive-cpio.el") (archive-cpio--parse-mode t nil nil (mode) nil "archive-cpio.el") (archive-cpio-summarize t nil nil (&optional archive-buffer) nil "archive-cpio.el") (archive-cpio-extract t nil nil (archive name) nil "archive-cpio.el") (archive-cpio-extract-from-buffer t nil nil (name archivebuf destbuf) nil "archive-cpio.el") (archive-rpm-find-type t nil nil nil nil "archive-rpm.el") (archive-rpm-summarize t nil nil nil nil "archive-rpm.el") (archive-rpm-extract t nil nil (archive name) nil "archive-rpm.el") (archive-rpm--decompress-payload t nil nil (payload header-entries) nil "archive-rpm.el") (archive-rpm--insert-interesting-information t nil nil (header-entries) nil "archive-rpm.el") (archive-rpm--get-header-data t nil nil (tag header-entries) nil "archive-rpm.el") (archive-rpm--find-index-entry-data t nil nil (index-entry data-starts-at data-len) nil "archive-rpm.el") (archive-rpm--parse-header t nil nil (align-after) nil "archive-rpm.el"))"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn binary_format_constants_preserve_complete_parser_contracts() {
    let elisp_form = r##"(list
 (list
  (length archive-cpio-entry-header-re)
  (secure-hash 'sha256 archive-cpio-entry-header-re)
  (get 'archive-cpio-entry-header-re
       'variable-documentation))
 (copy-tree archive-rpm--header-bindat-spec)
 (get 'archive-rpm--header-bindat-spec
      'variable-documentation)
 (copy-tree archive-rpm--index-entry-bindat-spec)
 (get 'archive-rpm--index-entry-bindat-spec
      'variable-documentation)
 (copy-tree archive-rpm--interesting-fields)
 (get 'archive-rpm--interesting-fields
      'variable-documentation))"##;
    let expect = expect![[
        r#"OK ((284 "ad65feaf7e32126b86b37e61d7119549f0f912f7b0de055157911dc53bd7cc7c" "Regular expression matching a CPIO entry.\nThe matched groups are:\n\n1. ino\n2. mode\n3. uid\n4. gid\n5. nlink\n6. mtime\n7. filesize\n8. devmajor\n9. devminor\n10. rdevmajor\n11. rdevminor\n12. namesize\n\nThe name starts at the end of the match, and goes on for namesize\nbytes.  It is padded with NUL bytes so that the start of file\ndata is aligned to four bytes.  File data is also padded, so that\nthe next header is aligned to four bytes.") ((:magic u24) (:version u8) (:reserved u32) (:n-index-entries u32) (:data-len u32)) "Bindat spec for RPM header.\n\nAs per <http://ftp.rpm.org/max-rpm/s1-rpm-file-format-rpm-file-format.html>:\n\nThe header structure header always starts with a three-byte magic\nnumber: 8e ad e8. Following this is a one-byte version\nnumber.  Next are four bytes that are reserved for future\nexpansion.  After the reserved bytes, there is a four-byte number\nthat indicates how many index entries exist in this header\nstructure, followed by another four-byte number indicating how\nmany bytes of data are part of the header structure." ((:tag u32) (:type u32) (:offset u32) (:count u32)) "Bindat spec for index entry in RPM header." ((1000 . "Name") (1001 . "Version") (1002 . "Release") (1004 . "Summary") (1010 . "Distribution") (1011 . "Vendor") (1014 . "License") (1016 . "Group") (1020 . "URL") (1021 . "OS") (1022 . "Architecture") (1124 . "Format") (1125 . "Compression")) "Fields to output at top of RPM archive buffer.")"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn loading_runtime_registers_both_archive_detectors_as_idempotent_advice() {
    let elisp_form = r##"(list
 (featurep 'archive-cpio)
 (featurep 'archive-rpm)
 (not
  (null
   (advice-member-p
    #'archive-cpio-find-type 'archive-find-type)))
 (not
  (null
   (advice-member-p
    #'archive-rpm-find-type 'archive-find-type)))
 (progn
   (load (getenv "NEOMACS_PACKAGE_SOURCE") nil t t)
   (list
    (not
     (null
      (advice-member-p
       #'archive-cpio-find-type 'archive-find-type)))
    (not
     (null
      (advice-member-p
       #'archive-rpm-find-type 'archive-find-type))))))"##;
    let expect = expect!["OK (t t t t (t t))"];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_expose_both_detectors_and_exact_rpm_magic_mode() {
    let elisp_form = r##"(list
 (featurep 'archive-cpio)
 (featurep 'archive-rpm)
 (featurep 'archive-rpm-autoloads)
 (mapcar
  (lambda (symbol)
    (list symbol
          (fboundp symbol)
          (and (fboundp symbol)
               (autoloadp (symbol-function symbol)))
          (commandp symbol)))
  '(archive-cpio-find-type
    archive-rpm-find-type
    archive-rpm-extract))
 (seq-filter
  (lambda (entry)
    (eq (cdr entry) 'archive-mode))
  magic-mode-alist))"##;
    let expect = expect![[
        r#"OK (nil nil t ((archive-cpio-find-type t t nil) (archive-rpm-find-type t t nil) (archive-rpm-extract nil nil nil)) (("��������\3\0" . archive-mode)))"#
    ]];
    assert_archive_rpm_autoload_parity(elisp_form, expect);
}

#[test]
fn public_features_and_library_lookup_resolve_to_the_installed_package() {
    let elisp_form = r##"(list
 (mapcar
  (lambda (feature)
    (list feature
          (featurep feature)
          (file-name-nondirectory
           (locate-library
            (symbol-name feature)))))
  '(archive-cpio archive-rpm))
 (mapcar
  (lambda (symbol)
    (list symbol
          (file-name-nondirectory
           (symbol-file symbol))))
  '(archive-cpio-entry-header-re
    archive-rpm--header-bindat-spec
    archive-rpm--interesting-fields)))"##;
    let expect = expect![[
        r#"OK (((archive-cpio t "archive-cpio.el") (archive-rpm t "archive-rpm.el")) ((archive-cpio-entry-header-re "archive-cpio.el") (archive-rpm--header-bindat-spec "archive-rpm.el") (archive-rpm--interesting-fields "archive-rpm.el")))"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}
