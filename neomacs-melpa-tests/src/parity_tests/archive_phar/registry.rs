use expect_test::expect;

use super::assert_archive_phar_parity;

#[test]
fn archive_phar_descriptor_records_exact_pin_dependencies_and_payload() {
    let elisp_form = r##"(let* ((desc
                (cadr (assq 'archive-phar package-alist)))
              (dir (package-desc-dir desc)))
         (list
          (package-version-join (package-desc-version desc))
          (package-desc-reqs desc)
          (package-desc-kind desc)
          (sort
           (mapcar #'file-name-nondirectory
                   (directory-files dir t "^[^.].*"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK ("20221009.2129" ((emacs (28 1)) (php-runtime (0 2)) (datetime-format (0 0 1))) nil ("README-elpa" "archive-phar-autoloads.el" "archive-phar-pkg.el" "archive-phar.el" "archive-phar.elc"))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_complete_callable_surface_arities_and_docs_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (macrop symbol)
                 (documentation symbol t)))
         '(archive-phar-find-type
           archive-phar-summarize
           archive-phar-extract))"##;
    let expect = expect![[
        r#"OK ((archive-phar-find-type nil nil nil "Added logic to `archive-find-type' to detect Phar.") (archive-phar-summarize nil nil nil "Summarize Phar archive file.") (archive-phar-extract (archive name) nil nil "Extract NAME file from Phar ARCHIVE."))"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_custom_group_executable_and_pattern_contract_is_exact() {
    let elisp_form = r##"(list
         archive-phar-php-executable
         (custom-variable-p
          'archive-phar-php-executable)
         (get 'archive-phar-php-executable 'custom-type)
         (get 'archive-phar-php-executable 'custom-tag)
         (get 'archive-phar-php-executable 'custom-group)
         (get 'archive-phar 'group-documentation)
         (get 'archive-phar 'custom-group)
         archive-phar-file-name-pattern)"##;
    let expect = expect![[
        r#"OK ("/usr/bin/php" ((funcall #'#[nil ((or (and (boundp 'php-executable) php-executable) (executable-find "php") "/usr/bin/php")) (t)])) string "Archive Phar PHP Executable" nil "Phar-specific options to archive." ((archive-phar-php-executable custom-variable)) "\\.phar\\'")"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_embedded_php_programs_are_byte_exact() {
    let elisp_form = r##"(list
         (length archive-phar--code-summarize-file)
         (secure-hash 'sha256
                      archive-phar--code-summarize-file)
         archive-phar--code-summarize-file
         (length archive-phar--code-extract-file)
         (secure-hash 'sha256
                      archive-phar--code-extract-file)
         archive-phar--code-extract-file)"##;
    let expect = expect![[
        r#"OK (494 "84096ab70624f1bdc64994abd40c788b3fdf07874f546b1a838d1366fb13ee7a" "declare(strict_types=1);\n\n$phar_path = trim(stream_get_contents(STDIN));\n$tr = [\"phar://{$phar_path}/\" => ''];\n$files = [];\n$p = new Phar($phar_path);\nforeach (new RecursiveIteratorIterator($p) as $f) {\n    $files[] = [\n        'pathname' => strtr($f->getPathname(), $tr),\n        'mtime' => $f->getMTime(),\n        'size' => $f->getSize(),\n        'perms' => $f->getPerms(),\n        'type' => $f->getType(),\n    ];\n}\n\necho json_encode($files, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);\n" 217 "ae6f3ae927e8eee2e1b0eaf017fdd6fb62d6f13b1859b168cca974ba180f1fe1" "declare(strict_types=1);\n\n$input = trim(stream_get_contents(STDIN));\nlist($phar_path, $filename) = explode(\"\11\", $input);\n$p = new Phar($phar_path, 0);\n$alias = $p->getAlias();\nreadfile(\"phar://{$alias}/{$filename}\");\n")"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_load_registers_advice_mode_rule_and_feature() {
    let elisp_form = r##"(list
         (featurep 'archive-phar)
         (advice-member-p
          #'archive-phar-find-type #'archive-find-type)
         (cl-remove-if-not
          (lambda (entry)
            (eq (cdr entry) 'archive-mode))
          auto-mode-alist)
         (assoc "\\.phar\\'" auto-mode-alist)
         (get 'archive-phar-find-type 'function-documentation)
         (get 'archive-phar-extract 'function-documentation))"##;
    let expect = expect![[
        r#"OK (t #[128 "����\2\"��\13\0����\2\"��" [archive-phar-find-type #[nil ((widen) (goto-char (point-min)) (let (case-fold-search) (cond ((looking-at "\\(?:PK\7\10\\|PK00\\)?[P]K\3\4") 'zip) ((looking-at "..-l[hz][0-9ds]-") 'lzh) ((looking-at "....................[��]������") 'zoo) ((and (looking-at "\32") (string-match "\\.[aA][rR][cC]\\'" (or buffer-file-name (buffer-name)))) 'arc) ((looking-at "MZ\\(.\\|\n\\)\\{34\\}LH[aA]'s SFX ") 'lzh-exe) ((looking-at "Rar!") 'rar) ((looking-at "!<arch>\n") 'ar) ((and (looking-at "MZ") (re-search-forward "Rar!" (+ (point) 100000) t)) 'rar-exe) ((looking-at "7z����'\34") '7z) ((looking-at "hsqs") 'squashfs) (t (error "Buffer format not recognized"))))) (cl-struct-archive--file-desc-tags t)] :before-until nil apply] 4 advice] (#1=("\\.phar\\'" . archive-mode) ("\\.\\(arc\\|zip\\|lzh\\|lha\\|zoo\\|[jew]ar\\|xpi\\|rar\\|cbr\\|7z\\|squashfs\\|ARC\\|ZIP\\|LZH\\|LHA\\|ZOO\\|[JEW]AR\\|XPI\\|RAR\\|CBR\\|7Z\\|SQUASHFS\\)\\'" . archive-mode) ("\\.oxt\\'" . archive-mode) ("\\.\\(deb\\|[oi]pk\\)\\'" . archive-mode)) #1# nil nil)"#
    ]];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_reload_is_idempotent_for_advice_and_mode_rule() {
    let elisp_form = r##"(let ((source
                (locate-library "archive-phar"))
               advice-functions)
         (load source nil 'nomessage)
         (load source nil 'nomessage)
         (load source nil 'nomessage)
         (advice-mapc
          (lambda (function _props)
            (push function advice-functions))
          #'archive-find-type)
         (list
          (cl-count 'archive-phar features)
          (cl-count #'archive-phar-find-type
                    advice-functions)
          (cl-count-if
           (lambda (entry)
             (and (equal (car entry) "\\.phar\\'")
                  (eq (cdr entry) 'archive-mode)))
           auto-mode-alist)))"##;
    let expect = expect!["OK (1 1 1)"];
    assert_archive_phar_parity(elisp_form, expect);
}

#[test]
fn archive_phar_required_dependency_features_are_loaded() {
    let elisp_form = r##"(mapcar
         (lambda (feature)
           (list feature (featurep feature)))
         '(arc-mode datetime-format nadvice php-runtime json))"##;
    let expect =
        expect!["OK ((arc-mode t) (datetime-format t) (nadvice t) (php-runtime t) (json t))"];
    assert_archive_phar_parity(elisp_form, expect);
}
