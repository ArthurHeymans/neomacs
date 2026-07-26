use expect_test::expect;

use super::assert_ac_c_headers_parity;

#[test]
fn ac_c_headers_exact_pin_surface_defaults_and_auto_complete_source_contracts_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-c-headers package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (featurep 'find-file)
                (featurep 'ac-c-headers)
                ac-c-headers-version
                (mapcar
                 #'fboundp
                 '(ac-c-headers--files-update
                   ac-c-headers--files-list
                   ac-c-headers--search-header-file
                   ac-c-headers--symbols-update
                   ac-c-headers--symbols-list))
                (mapcar
                 #'commandp
                 '(ac-c-headers--files-update
                   ac-c-headers--files-list
                   ac-c-headers--search-header-file
                   ac-c-headers--symbols-update
                   ac-c-headers--symbols-list))
                ac-c-headers--files-cache
                ac-c-headers--symbols-cache
                ac-source-c-headers
                ac-source-c-header-symbols))"##;
    let expect = expect![[
        r##"OK (ac-c-headers "20200816.1007" ((auto-complete (1 3 1))) t t "1.0.0" (t t t t t) (nil nil nil nil nil) nil nil ((prefix . "#include *[<\"][^<>\"]*?\\([^<>\"/]*\\)") (candidates . ac-c-headers--files-list) (action lambda nil (when (string-match "\\.h$" candidate) (ac-c-headers--symbols-update candidate) (cond ((looking-at "[>\"]") (forward-char 1) (newline-and-indent)) ((looking-back "#include *<\\([^<]*\\)") (insert ">\n")) (t (insert "\"\n"))))) (symbol . "h") (requires . 0) (cache)) ((candidates . ac-c-headers--symbols-list) (symbol . "h") (cache)))"##
    ]];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_filename_action_updates_header_cache_and_handles_existing_closer() {
    let elisp_form = r##"(let ((action
                    (cdr
                     (assq
                      'action
                      ac-source-c-headers)))
                   events)
               (cl-letf
                   (((symbol-function
                      'ac-c-headers--symbols-update)
                     (lambda (header)
                       (push header events)
                       'updated)))
                 (with-temp-buffer
                   (insert
                    "#include <stdio.h>tail")
                   (search-backward ">")
                   (cl-progv '(candidate) '("stdio.h")
                     (list
                      (funcall action)
                      (buffer-string)
                      (point)
                      (nreverse events))))))"##;
    let expect = expect![[r##"OK (nil "#include <stdio.h>\ntail" 20 ("stdio.h"))"##]];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_filename_action_inserts_missing_angle_or_quote_closer_and_skips_directories() {
    let elisp_form = r##"(let ((action
                    (cdr
                     (assq
                      'action
                      ac-source-c-headers)))
                   events)
               (cl-letf
                   (((symbol-function
                      'ac-c-headers--symbols-update)
                     (lambda (header)
                       (push header events)
                       'updated)))
                 (list
                  (with-temp-buffer
                    (insert
                     "#include <path/file.h")
                    (cl-progv '(candidate) '("file.h")
                      (list
                       (funcall action)
                       (buffer-string)
                       (point))))
                  (with-temp-buffer
                    (insert
                     "#include \"quoted.h")
                    (cl-progv '(candidate) '("quoted.h")
                      (list
                       (funcall action)
                       (buffer-string)
                       (point))))
                  (with-temp-buffer
                    (insert
                     "#include <sub/")
                    (cl-progv '(candidate) '("sub/")
                      (list
                       (funcall action)
                       (buffer-string)
                       (point))))
                  (nreverse events))))"##;
    let expect = expect![[
        r##"OK ((nil "#include <path/file.h>\n" 24) (nil "#include \"quoted.h\"\n" 21) (nil "#include <sub/" 15) ("file.h" "quoted.h"))"##
    ]];

    assert_ac_c_headers_parity(elisp_form, expect);
}
