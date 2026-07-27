use expect_test::expect;

use super::{assert_arview_autoload_parity, assert_arview_parity};

#[test]
fn arview_descriptor_pins_exact_release_metadata_and_dependency_contract() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'arview package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-summary descriptor)
                (package-desc-reqs descriptor)
                (package-desc-kind descriptor)
                (package-desc-archive descriptor)
                (package-desc-extras descriptor)))"##;
    let expect = expect![[
        r#"OK (arview "20160419.2109" "Extract and view archives in the temporary directory." nil nil nil ((:maintainers ("Andrey Fainer" . "fandrey@gmx.com")) (:authors ("Andrey Fainer" . "fandrey@gmx.com")) (:keywords "files") (:revdesc . "5437b4221b64") (:commit . "5437b4221b64b238c273a651d4792c577dba6d45") (:url . "https://github.com/afainer/arview")))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_installed_payload_has_exact_files_sizes_and_source_hashes() {
    let elisp_form = r##"(let* ((descriptor
                     (cadr
                      (assq 'arview package-alist)))
                    (directory
                     (package-desc-dir descriptor))
                    (files
                     (sort
                      (directory-files
                       directory t "^[^.].*")
                      #'string<)))
               (mapcar
                (lambda (file)
                  (list
                   (file-name-nondirectory file)
                   (file-attribute-size
                    (file-attributes file))
                   (and
                    (member
                     (file-name-nondirectory file)
                     '("README-elpa"
                       "arview-autoloads.el"
                       "arview-pkg.el"
                       "arview.el"))
                    (with-temp-buffer
                      (insert-file-contents-literally file)
                      (secure-hash
                       'sha256
                       (current-buffer))))))
                files))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 1203 "87015d1a374e6b3aa0eb1725dd0270715f7a0a3ea99ef3f816dcc32f443c2faa") ("arview-autoloads.el" 1003 "d329b1e02d36dbb45d89c6cf57a9e54458942cb5b931263933195e1af83cf401") ("arview-pkg.el" 410 "93babf137221c0829de952e0e3ae45b199355a16b637dfe4408d6f7d1b434d82") ("arview.el" 11099 "d4016c719ffbfdfe75fed31c04f4c8555332f700829135aaf38974c0ed8b2c0c") ("arview.elc" 7448 nil))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_public_function_surface_has_exact_arglists_commands_and_interactive_forms() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (fboundp function)
                  (macrop function)
                  (commandp function)
                  (help-function-arglist
                   function t)
                  (interactive-form function)
                  (documentation function)))
               '(arview-file-archive
                 arview-file-extension
                 arview-copy-remote-file
                 arview-archive-type
                 arview-process-file
                 arview-view
                 arview-kill-buffer-hook
                 arview-process-prefix-arg
                 arview-dired
                 arview))"##;
    let expect = expect![[
        r#"OK ((arview-file-archive t nil nil (filename) nil "Use the ‘file’ utility to determine the type of FILENAME.\nSee ‘arview-file-alist’.") (arview-file-extension t nil nil (filename) nil "Determine the type of FILENAME by its extension.") (arview-copy-remote-file t nil nil (filename tempdir) nil "Copy FILENAME to TEMPDIR.\nCopy only if FILENAME and TEMPDIR on different hosts.  Otherwise\nreturn FILENAME.") (arview-archive-type t nil nil (filename) nil "Determine FILENAME type using ‘arview-archive-type-functions’.") (arview-process-file t nil nil (arcmd arargs file log) nil "Run a shell process with ARCMD and ARARGS.\nThe filename FILE is a file for the archive command ARCMD.\nInsert output in the buffer LOG.") (arview-view t nil nil (filename &optional tempdir args) nil "Extract the archive FILENAME and open its dired buffer.\nThe type of the archive determined with the function\n‘arview-archive-type’.  The archive extracted using the archive\nprogram associated with the archive type (see ‘arview-types’).\nARGS is additional arguments fo the archive program.\n\nThe temporary directory where the archive is extracted to is\n\nTEMPDIR/arview-FILENAME.<random-string>\n\nSet ‘arview-buffer-p’ to t or FILENAME if FILENAME is a remote\nfile.  The variable is local to the temporary directory buffer.") (arview-kill-buffer-hook t nil nil nil nil "Remove the archive directory when its dired buffer is killed.\nAlso if archive is a remote file remove its local copy.  See\n‘arview-view’.") (arview-process-prefix-arg t nil nil (arg) nil "Read from the minibuffer a temp dir and additional args.\nWhen ‘arview’ or ‘arview-dired’ commands called with one prefix\nargument, prompt for another temporary directory, not the default\none.  With two prefix arguments also promt for additional\narguments for the archive command.\n\nARG is the value of the prefix argument ‘arview’ and\n‘arview-dired’ called with.") (arview-dired t nil t (arg) (interactive "P") "View the arview under point in the current dired buffer.\nProcess ARG using ‘arview-process-prefix-arg’.  See\n‘arview-view’.") (arview t nil t (arg filename) (interactive "P\nfArchive file name: ") "Ask for the archive FILENAME and view it.\nProcess ARG using ‘arview-process-prefix-arg’.  See\n‘arview-view’."))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_customization_surface_has_exact_defaults_types_groups_and_docs() {
    let elisp_form = r##"(mapcar
               (lambda (symbol)
                 (list
                  symbol
                  (boundp symbol)
                  (default-value symbol)
                  (get symbol 'standard-value)
                  (get symbol 'custom-type)
                  (get symbol 'custom-requests)
                  (get symbol 'variable-documentation)))
               '(arview-archive-type-functions
                 arview-types
                 arview-file-alist
                 arview-log-buffer-name
                 arview-buffer-p))"##;
    let expect = expect![[
        r#"OK ((arview-archive-type-functions t #1=(arview-file-archive arview-file-extension) ('#1#) list nil "The list of functions used to determine the archive file type.\nThe archive type is the value of the first function which returns\nnon-nil.  The functions must take one argument: the archive file\nname.") (arview-types t #2=((tar "tar" "-xf") (zip "unzip") (7z "7z" "x") (rar "unrar" "x")) ('#2#) alist nil "Archive types known to arview.\nEach element of the alist is\n\n  (ARCHIVE-TYPE EXECUTABLE ARGUMENTS)\n\nARCHIVE-TYPE - a symbol which designates the archive type.\nEXECUTABLE - a program to extract archives of this type.\nARGUMENTS - command-line arguments to the program.") (arview-file-alist t #3=((tar . ".*: .* tar archive") (zip . ".*: Zip archive data") (7z . ".*: 7-zip archive data") (rar . ".*: RAR archive data")) ('#3#) alist nil "Alist of archive type for the function `arview-file-archive'.\nThe element of the alist is a cons (ARCHIVE-TYPE . REGEXP), where\nARCHIVE-TYPE is a symbol which designates the archive type and\nREGEXP used match against the output from file utility.") (arview-log-buffer-name t "*arview-log*" nil nil nil "The name of buffer that contains output from the archive program.") (arview-buffer-p t nil nil nil nil "Buffers with non-nil value are temporary archive directories.\nSee `arview-view'."))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_library_registers_feature_hook_group_and_conditional_dired_binding() {
    let elisp_form = r##"(list
               (featurep 'arview)
               (get 'arview 'custom-group)
               (get 'arview 'group-documentation)
               (memq
                'arview-kill-buffer-hook
                kill-buffer-hook)
               (seq-count
                (lambda (function)
                  (eq function
                      'arview-kill-buffer-hook))
                kill-buffer-hook)
               (lookup-key
                dired-mode-map
                [C-return])
               arview-log-buffer-name)"##;
    let expect = expect![[
        r#"OK (t ((arview-archive-type-functions custom-variable) (arview-types custom-variable) (arview-file-alist custom-variable)) "The archive viewer customization group." (arview-kill-buffer-hook tramp-delete-temp-file-function tramp-flush-file-function uniquify-kill-buffer-function vc-kill-buffer-hook) 1 arview-dired "*arview-log*")"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_buffer_state_variable_is_automatically_buffer_local_and_isolated() {
    let elisp_form = r##"(let ((default
                    (default-value
                     'arview-buffer-p))
                   first
                   second)
               (with-temp-buffer
                 (setq arview-buffer-p
                       'first-buffer)
                 (setq first
                       (list
                        (local-variable-p
                         'arview-buffer-p)
                        arview-buffer-p
                        (default-value
                         'arview-buffer-p))))
               (with-temp-buffer
                 (setq second
                       (list
                        (local-variable-p
                         'arview-buffer-p)
                        arview-buffer-p
                        (default-value
                         'arview-buffer-p))))
               (list default first second))"##;
    let expect = expect!["OK (nil (t first-buffer nil) (nil nil nil))"];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_generated_autoloads_expose_only_commands_without_loading_runtime_state() {
    let elisp_form = r##"(list
               (featurep 'arview)
               (fboundp 'arview)
               (autoloadp
                (symbol-function
                 'arview))
               (fboundp 'arview-dired)
               (autoloadp
                (symbol-function
                 'arview-dired))
               (fboundp
                'arview-file-extension)
               (boundp
                'arview-archive-type-functions)
               (memq
                'arview-kill-buffer-hook
                kill-buffer-hook)
               (boundp 'dired-mode-map)
               (and
                (boundp 'dired-mode-map)
                (lookup-key
                 dired-mode-map
                 [C-return])))"##;
    let expect = expect!["OK (nil t t t t nil nil nil nil nil)"];
    assert_arview_autoload_parity(elisp_form, expect);
}
