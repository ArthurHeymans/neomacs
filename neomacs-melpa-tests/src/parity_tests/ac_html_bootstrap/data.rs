use expect_test::expect;

use super::assert_ac_html_bootstrap_parity;

#[test]
fn ac_html_bootstrap_tag_data_preserves_exact_order_and_no_final_newline() {
    let elisp_form = r##"(let ((file
                    (expand-file-name
                     "html-tag-list"
                     ac-html-bootstrap-source-dir)))
               (with-temp-buffer
                 (insert-file-contents
                  file)
                 (let ((contents
                        (buffer-string)))
                   (list
                    (split-string
                     contents "\n" t)
                    (length
                     (split-string
                      contents "\n" t))
                    (string-suffix-p
                     "\n"
                     contents)
                    (secure-hash
                     'sha256
                     contents)))))"##;
    let expect = expect![[
        r#"OK (("abbr" "address" "blockquote" "cite" "code" "del" "em" "footer" "ins" "kbd" "mark" "pre" "s" "samp" "section" "small" "strong" "u" "var") 19 nil "0bdcce4630dcf7aa0cbcd2c3bd1dede68e8ea63aa371bb9cf254dc6bfbd6a523")"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_attribute_lists_preserve_every_file_shape_and_digest() {
    let elisp_form = r##"(let* ((directory
                     (expand-file-name
                      "html-attributes-list"
                      ac-html-bootstrap-source-dir))
                    (files
                     (directory-files
                      directory t
                      "^[^.]")))
               (mapcar
                (lambda (file)
                  (with-temp-buffer
                    (insert-file-contents
                     file)
                    (let* ((contents
                            (buffer-string))
                           (rows
                            (split-string
                             contents "\n" t)))
                      (list
                       (file-name-nondirectory
                        file)
                       (length rows)
                       (car rows)
                       (car
                        (last rows))
                       (string-suffix-p
                        "\n"
                        contents)
                       (secure-hash
                        'sha256
                        contents)))))
                files))"##;
    let expect = expect![[
        r##"OK (("a" 5 "data-dismiss .alert-dismissible. Close alert" "data-toggle Toggle facility" nil "b2ee348c5f983a92f02413f07f5dddd29256786523e5fc6e598f4158980f5337") ("button" 5 "data-dismiss .alert-dismissible. Close alert" "data-toggle Toggle facility" nil "b2ee348c5f983a92f02413f07f5dddd29256786523e5fc6e598f4158980f5337") ("div" 9 "data-backdrop Includes a modal-backdrop element.\\nDefault true\\nAlternatively, specify static for a backdrop which doesn't close the modal on click.\\n<div id=\"myModal\" class=\"modal fade\" role=\"dialog\" data-backdrop=\"true\">" "data-wrap Whether the carousel should cycle continuously or have hard stops.\\nDefault true." nil "e3ac5bbf83f8ae4997e1a49ed7f7c4e5a9918546638946b371b8b54436c9263a") ("fieldset" 1 "disabled" "disabled" nil "17eb3c0168d0d7b21ede5481150f17233427d89833ec121b4dbc4fb96cfab71e") ("global" 14 "data-animation Animation for tooltip/popover." "data-viewport String or object.\\nKeeps the tooltip/popover within the bounds of this element.\\nExample: viewport: '#viewport' or { \"selector\": \"#viewport\", \"padding\": 0 }" nil "7a0fe2e364bd75a117042d7c82776d43d71b0ae3afd6ae08cdc71d917c50b4da") ("input" 5 "data-dismiss .alert-dismissible. Close alert" "data-toggle Toggle facility" nil "b2ee348c5f983a92f02413f07f5dddd29256786523e5fc6e598f4158980f5337") ("label" 5 "data-dismiss .alert-dismissible. Close alert" "data-toggle Toggle facility" nil "b2ee348c5f983a92f02413f07f5dddd29256786523e5fc6e598f4158980f5337") ("li" 1 "data-slide-to Carousel controll" "data-slide-to Carousel controll" nil "8ae8b48b3779ed94d630857540709206a2b1972eaeb141f650a7316e265dc4b8"))"##
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_representative_completion_lists_preserve_docs_and_boundaries() {
    let elisp_form = r##"(mapcar
               (lambda (relative)
                 (let ((file
                        (expand-file-name
                         relative
                         ac-html-bootstrap-source-dir)))
                   (with-temp-buffer
                     (insert-file-contents
                      file)
                     (let* ((contents
                             (buffer-string))
                            (rows
                             (split-string
                              contents "\n" t)))
                       (list
                        relative
                        (length rows)
                        (car rows)
                        (nth
                         (/ (length rows) 2)
                         rows)
                        (car
                         (last rows))
                        (string-suffix-p
                         "\n"
                         contents)
                        (secure-hash
                         'sha256
                         contents))))))
               '("html-attributes-complete/global-class"
                 "html-attributes-complete/i-class"
                 "html-attributes-complete/div-class"
                 "html-attributes-complete/global-data-placement"
                 "html-attributes-complete/a-data-toggle"))"##;
    let expect = expect![[
        r##"OK (("html-attributes-complete/global-class" 224 "col-lg-1 1170px.\\nColumn width ~97px" "col-sm-offset-2 Increase the left margin of a column" "visible-xs-inline-block Extra small devices (<768px).\\nSet Visible of component" nil "c7c92d717750fcc47d0cfe96df2aef135fc31e3d0cf2e1132c7be9329384fd4a") ("html-attributes-complete/i-class" 201 "glyphicon Base class\\nBe sure to leave a space between the icon and text for proper padding." "glyphicon-lock" "glyphicon-zoom-out" nil "fa492447c28c878edda0ba94d822fda7cb57ced76e3badc865cc347a6ff5704b") ("html-attributes-complete/div-class" 115 "active Animate progress-bar" "media-middle" "well-sm" nil "61b60d8a7493141bff69414be62b0919f92f6dd81d7f372cfa8cf6a5283e4780") ("html-attributes-complete/global-data-placement" 5 "auto" "left" "top" nil "bc5dc8a1ee541e21deea9314a1770aae9e575bbc0d61efd8428f348fa16fe795") ("html-attributes-complete/a-data-toggle" 5 "button Single toggle button\\nPre-toggled buttons need .active and aria-pressed=\"true\"\\n<button type=\"button\" class=\"btn btn-primary\" data-toggle=\"button\"\\n        aria-pressed=\"false\" autocomplete=\"off\">\\n  Single toggle\\n</button>" "collapse" "modal Activate a modal without writing JavaScript. Set data-toggle=\"modal\" on a controller element, like a button, along with a data-target=\"#foo\" or href=\"#foo\" to target a specific modal to toggle.\\n\\n<button type=\"button\" data-toggle=\"modal\" data-target=\"#myModal\">Launch modal</button>" nil "3aa499429912c7be6d579359ee8c7659b2c19754e68f63d13278dad3b01875d1"))"##
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_fa_class_data_preserves_all_icons_utilities_and_digest() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (let ((file
                    (expand-file-name
                     "html-attributes-complete/i-class"
                     ac-html-fa-source-dir)))
               (with-temp-buffer
                 (insert-file-contents
                  file)
                 (let* ((contents
                         (buffer-string))
                        (rows
                         (split-string
                          contents "\n" t)))
                   (list
                    (length rows)
                    (seq-take rows 12)
                    (nth 307 rows)
                    (last rows 12)
                    (string-suffix-p
                     "\n"
                     contents)
                    (secure-hash
                     'sha256
                     contents))))))"##;
    let expect = expect![[
        r#"OK (616 ("fa" "fa-2x" "fa-3x" "fa-4x" "fa-5x" "fa-adjust" "fa-adn" "fa-align-center" "fa-align-justify" "fa-align-left" "fa-align-right" "fa-ambulance") "fa-lastfm-square" ("fa-wordpress" "fa-wrench" "fa-xing" "fa-xing-square" "fa-yahoo" "fa-yelp" "fa-yen" "fa-youtube" "fa-youtube-play" "fa-youtube-square" "pull-left" "pull-right") t "84dc4c5deebacb338f8e27e763d07e3a9739bcf72a440975115b1f08bcb6460c")"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_documentation_files_preserve_exact_inventory_and_content() {
    let elisp_form = r##"(let ((directories
                    '("html-tag-short-docs"
                      "html-attributes-short-docs")))
               (mapcar
                (lambda (relative)
                  (let* ((directory
                          (expand-file-name
                           relative
                           ac-html-bootstrap-source-dir))
                         (files
                          (directory-files
                           directory t
                           "^[^.]")))
                    (list
                     relative
                     (mapcar
                      #'file-name-nondirectory
                      files)
                     (mapcar
                      (lambda (file)
                        (with-temp-buffer
                          (insert-file-contents
                           file)
                          (list
                           (file-name-nondirectory
                            file)
                           (buffer-size)
                           (secure-hash
                            'sha256
                            (buffer-string)))))
                      files))))
                directories))"##;
    let expect = expect![[
        r#"OK (("html-tag-short-docs" ("abbr" "address" "blockquote" "cite" "del" "em" "footer" "ins" "mark" "s" "small" "strong" "u") (("abbr" 293 "f2f681763457bec60cd1658a313f6e53c65b75c1d4138de1b24b761095fe8375") ("address" 300 "3629954640b778ef7786580c57c6cc35218384da0da916d037745f80d5bc2ef5") ("blockquote" 174 "8de2ab2ed2816cce6aeadb8b638278b9eb12df3a7b0cb9cff06a4e9cf133a147") ("cite" 31 "b7087dbb6be17cb17e6b5f36f5e172c5a0f1a5899d3ff4e25fd1a55876cf2ac6") ("del" 48 "d73f7c91e7b29d405f31db8ae51a382c9da6aa3919c4f9d13245c42ca16f8c49") ("em" 7 "545f94624d86dabaa33c6eefe413b418a2e3b217dc664c86b88fc2bb162d6d56") ("footer" 54 "c9624d3f9a6062d6ec834074a95e1f42d5ecbf64456adc57981e77f9574323f9") ("ins" 36 "b05c60e93ef329b6c0355298d02d72f47a46a7dc422461ebffd83e5ca2bad1ce") ("mark" 17 "1b1bc9984e6273eb8f4301bfbec06557dfc201c22831f848357ee8a43c7dcf57") ("s" 53 "57c2e4f265263e176ab5f48c714d78716e381660ddd41cad92c6977c29e9d4b7") ("small" 183 "bc9394d0ae469be8c7618231945eb93fd3b83b6dc62163febed2c5a4acee8061") ("strong" 63 "240faf837dc6965f756343d98463ac380339a4564a10376e6eca88d8e76906a6") ("u" 14 "4567954b66397c1b86a6c9fc2cb517f3c2164aa39c6619334c06ea3a54ba58d9"))) ("html-attributes-short-docs" ("div-data-offset-bottom" "div-data-offset-top" "div-data-ride" "fieldset-disabled" "global-data-container" "global-data-spy") (("div-data-offset-bottom" 348 "8b865bcfa66e5874a5aaa3fbe56f39ba562b068bfe603fe262b5088e711c6044") ("div-data-offset-top" 348 "8b865bcfa66e5874a5aaa3fbe56f39ba562b068bfe603fe262b5088e711c6044") ("div-data-ride" 255 "cca37b0cf128c42df126f20a39e779ba4392624932e313c42ed7b314069015da") ("fieldset-disabled" 336 "5cea57df79c68aca6f73f3d427fc08b3912659bd0b10d19a418a8865dca407e7") ("global-data-container" 278 "46ce492ec91dfca7a2589583cced2c8b99f6b92f618e37a154dc242c62e547b3") ("global-data-spy" 282 "46182d11c5932431ae5d460b1e8109f63507eb930b71ed7bfeb0bcacdb266b23"))))"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_complete_data_inventory_and_whole_tree_digest_match() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (let* ((bootstrap-directories
                     '("html-attributes-complete"
                       "html-attributes-list"
                       "html-attributes-short-docs"
                       "html-tag-short-docs"))
                    (bootstrap-files
                     (cons
                      (expand-file-name
                       "html-tag-list"
                       ac-html-bootstrap-source-dir)
                      (apply
                       #'append
                       (mapcar
                        (lambda (relative)
                          (directory-files
                           (expand-file-name
                            relative
                            ac-html-bootstrap-source-dir)
                           t "^[^.]"))
                        bootstrap-directories))))
                    (fa-files
                     (directory-files
                      (expand-file-name
                       "html-attributes-complete"
                       ac-html-fa-source-dir)
                      t "^[^.]"))
                    (files
                     (sort
                      (append
                       bootstrap-files
                       fa-files)
                      #'string<))
                    (library-directory
                     (file-name-directory
                      (locate-library
                       "ac-html-bootstrap")))
                    (payload
                     (mapconcat
                      (lambda (file)
                        (with-temp-buffer
                          (insert
                           (file-relative-name
                            file
                            library-directory)
                           "\0")
                          (insert-file-contents
                           file)
                          (buffer-string)))
                      files
                      "\0")))
               (list
                (mapcar
                 (lambda (relative)
                   (list
                    relative
                    (length
                     (directory-files
                      (expand-file-name
                       relative
                       ac-html-bootstrap-source-dir)
                      nil "^[^.]"))))
                 bootstrap-directories)
                (length fa-files)
                (length files)
                (secure-hash
                 'sha256
                 payload))))"##;
    let expect = expect![[
        r#"OK ((("html-attributes-complete" 71) ("html-attributes-list" 8) ("html-attributes-short-docs" 6) ("html-tag-short-docs" 13)) 1 100 "9b7d87624f88334e149ca286e4b52643c3117a3c2f23315e7557ea9cce42079f")"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}
