use expect_test::expect;

use super::assert_all_the_icons_parity;

#[test]
fn all_the_icons_family_candidates_preserve_names_display_payload_and_icons() {
    let elisp_form = r##"(let* ((candidates
                 (all-the-icons--read-candidates-for-family
                  'alltheicon t))
                (selected
                 (seq-filter
                  (lambda (entry)
                    (member
                     (substring-no-properties (car entry))
                     '("rust\t[alltheicon]"
                       "javascript\t[alltheicon]"
                       "emacs\t[alltheicon]")))
                  candidates)))
         (list
          (length candidates)
          (mapcar
           (lambda (entry)
             (list
              (substring-no-properties (car entry))
              (get-text-property 0 'display (car entry))
              (string-to-list (cdr entry))
              (text-properties-at 0 (cdr entry))))
           selected)))"##;
    let expect = expect![[
        r#"OK (62 (("javascript\11[alltheicon]" #("\11j" 0 1 (rear-nonsticky t display (raise -0.24) font-lock-face #1=(:family #3="all-the-icons" :height 1.2) face #1#)) (59654) (face #2=(:family "all-the-icons" :height 1.2) font-lock-face #2# display (raise -0.24) rear-nonsticky t)) ("rust\11[alltheicon]" #("\11r" 0 1 (rear-nonsticky t display (raise -0.24) font-lock-face #4=(:family #3# :height 1.2) face #4#)) (59692) (face #5=(:family "all-the-icons" :height 1.2) font-lock-face #5# display (raise -0.24) rear-nonsticky t))))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_interactive_insert_uses_completion_selection_and_inserts_icon() {
    let elisp_form = r##"(with-temp-buffer
         (let (observed-prompt observed-require-match)
           (cl-letf
               (((symbol-function 'completing-read)
                 (lambda (prompt collection predicate require-match
                                  &rest _)
                   (setq observed-prompt prompt
                         observed-require-match require-match)
                   (car
                    (car
                     (seq-filter
                      (lambda (entry)
                        (equal
                         (substring-no-properties (car entry))
                         "rust"))
                      collection))))))
             (all-the-icons-insert nil 'alltheicon)
             (list observed-prompt
                   observed-require-match
                   (string-to-list (buffer-string))
                   (text-properties-at 1 (buffer-string))))))"##;
    let expect = expect![[r#"OK ("all-the-icons Icon: " t (59692) nil)"#]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_prefix_insert_prints_readable_propertized_icon_value() {
    let elisp_form = r##"(with-temp-buffer
         (cl-letf
             (((symbol-function 'completing-read)
               (lambda (_prompt collection &rest _)
                 (car
                  (car
                   (seq-filter
                    (lambda (entry)
                      (equal
                       (substring-no-properties (car entry))
                       "cogs"))
                    collection))))))
           (all-the-icons-insert t 'faicon)
           (list
            (buffer-string)
            (read (buffer-string)))))"##;
    let expect = expect![[
        r##"OK ("#(\"\" 0 1 (face #1=(:family \"FontAwesome\" :height 1.2) font-lock-face #1# display (raise -0.24) rear-nonsticky t))" #("" 0 1 (face #1=(:family "FontAwesome" :height 1.2) font-lock-face #1# display (raise -0.24) rear-nonsticky t)))"##
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_bulk_insertion_formats_every_family_entry_with_requested_height() {
    let elisp_form = r##"(with-temp-buffer
         (cl-letf
             (((symbol-function 'all-the-icons-material-data)
               (lambda ()
                 '(("settings" . "\ue8b8")
                   ("star" . "\ue838")))))
           (all-the-icons-insert-icons-for 'material 3)
           (let ((text (buffer-string)))
             (list
              (substring-no-properties text)
              (string-to-list text)
              (get-text-property 0 'face text)
              (get-text-property
               (1+ (string-match "\n" text))
               'face text)))))"##;
    let expect = expect![[
        r#"OK (" - settings\n - star\n" (59576 32 45 32 115 101 116 116 105 110 103 115 10 59448 32 45 32 115 116 97 114 10) (:family "Material Icons" :height 3.5999999999999996) (:family "Material Icons" :height 3.5999999999999996))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_global_candidates_cover_every_data_row_and_label_family() {
    let elisp_form = r##"(let* ((candidates (all-the-icons--read-candidates))
               (expected
                (apply
                 #'+
                 (mapcar
                  (lambda (family)
                    (length
                     (funcall
                      (intern
                       (format
                        "all-the-icons-%s-data"
                        family)))))
                  all-the-icons-font-families)))
               (families
                (mapcar
                 (lambda (family)
                   (cons
                    family
                    (seq-count
                     (lambda (entry)
                       (string-suffix-p
                        (format "\t[%s]" family)
                        (substring-no-properties
                         (car entry))))
                     candidates)))
                 all-the-icons-font-families)))
         (list expected (length candidates) families))"##;
    let expect = expect![
        "OK (2868 2868 ((material . 932) (wicon . 587) (octicon . 158) (faicon . 634) (fileicon . 495) (alltheicon . 62)))"
    ];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_font_install_workflow_downloads_exact_manifest_and_refreshes_cache() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (process-environment
                (cons (concat "XDG_DATA_HOME=" root)
                      process-environment))
               (all-the-icons-fonts-subdirectory "neomacs-icons")
               copies commands messages)
         (cl-letf
             (((symbol-function 'url-copy-file)
               (lambda (url destination overwrite)
                 (push
                  (list url
                        (file-relative-name destination root)
                        overwrite)
                  copies)))
              ((symbol-function 'shell-command-to-string)
               (lambda (command)
                 (push command commands)
                 "cache refreshed"))
              ((symbol-function 'message)
               (lambda (format-string &rest arguments)
                 (push
                  (apply #'format format-string arguments)
                  messages))))
           (all-the-icons-install-fonts t)
           (list
            (file-directory-p
             (expand-file-name "fonts/neomacs-icons" root))
            (nreverse copies)
            (nreverse commands)
            (mapcar #'substring-no-properties
                    (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (t (("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/material-design-icons.ttf" "fonts/neomacs-icons/material-design-icons.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/weathericons.ttf" "fonts/neomacs-icons/weathericons.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/octicons.ttf" "fonts/neomacs-icons/octicons.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/fontawesome.ttf" "fonts/neomacs-icons/fontawesome.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/file-icons.ttf" "fonts/neomacs-icons/file-icons.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/all-the-icons.ttf" "fonts/neomacs-icons/all-the-icons.ttf" t)) ("fc-cache -f -v") ("Fonts downloaded, updating font cache... <fc-cache -f -v> " " Successfully installed `all-the-icons' fonts to `[ORACLE-SANDBOX]/fonts/neomacs-icons'!"))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_font_install_decline_performs_no_filesystem_or_network_action() {
    let elisp_form = r##"(let (events)
         (cl-letf
             (((symbol-function 'yes-or-no-p)
               (lambda (prompt)
                 (push (list 'prompt prompt) events)
                 nil))
              ((symbol-function 'url-copy-file)
               (lambda (&rest arguments)
                 (push (cons 'copy arguments) events)))
              ((symbol-function 'make-directory)
               (lambda (&rest arguments)
                 (push (cons 'mkdir arguments) events))))
           (list
            (all-the-icons-install-fonts nil)
            (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil ((prompt "This will download and install fonts, are you sure you want to do this?")))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}
