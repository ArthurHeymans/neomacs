use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_font_lock_titles_inline_quotes_passthroughs_escapes_and_roles_match() {
    let elisp_form = r##"(cl-labels
         ((face-at
           (needle)
           (goto-char (point-min))
           (search-forward needle)
           (let ((face (get-text-property (match-beginning 0) 'face)))
             (if (and (consp face) (null (cdr face)))
                 (car face)
               face))))
       (with-temp-buffer
         (insert
          "= Document Title\n"
          "== Section One\n"
          "=== Section Two\n"
          ".Block Caption\n"
          "a *boldword* and _emphword_ and `monoword`\n"
          "a #markedword# and ^superword^ and ~subword~\n"
          "a +plainpass+ and ++widepass++ and +++rawpass+++\n"
          "\"`doublecurved`\" and '`singlecurved`'\n"
          "\\*escapedbold* and normalword\n"
          "[.underline]#underlinedword# [.line-through]#struckword#\n")
         (adoc-mode)
         (font-lock-ensure)
         (mapcar
          #'face-at
          '("Document Title" "Section One" "Section Two"
            "Block Caption" "boldword" "emphword" "monoword"
            "markedword" "superword" "subword" "plainpass"
            "widepass" "rawpass" "doublecurved" "singlecurved"
            "escapedbold" "normalword" "underlinedword"
            "struckword"))))"##;
    let expect = expect![[
        r#"OK (adoc-title-0-face adoc-title-1-face adoc-title-2-face adoc-gen-face adoc-bold-face adoc-emphasis-face (adoc-typewriter-face adoc-verbatim-face) adoc-highlight-face adoc-superscript-face adoc-subscript-face nil nil (adoc-typewriter-face adoc-verbatim-face) nil nil nil nil (adoc-underline-face adoc-highlight-face) (adoc-strike-through-face adoc-highlight-face))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_font_lock_lists_blocks_tables_admonitions_and_attributes_match() {
    let elisp_form = r##"(cl-labels
         ((face-at
           (needle occurrence)
           (goto-char (point-min))
           (dotimes (_ occurrence) (search-forward needle))
           (let ((face (get-text-property (match-beginning 0) 'face)))
             (if (and (consp face) (null (cdr face)))
                 (car face)
               face))))
       (with-temp-buffer
         (insert
          "* bulletitem\n"
          "*** nesteditem\n"
          "1. numbereditem\n"
          "* [x] checkeditem\n"
          "term:: definition\n"
          "NOTE: admonition text\n"
          ":project: Demo\n"
          "Value is {project}.\n"
          "----\nlistingbody\n----\n"
          "....\nliteralbody\n....\n"
          "////\ncommentbody\n////\n"
          "++++\nrawbody\n++++\n"
          "____\nquotedbody\n____\n"
          "|===\n|tablecell\n|===\n"
          ",===\nCSVONE,CSVTWO\n,===\n"
          "ordinary, prose: remains\n")
         (adoc-mode)
         (font-lock-ensure)
         (list
          (face-at "*" 1)
          (face-at "***" 1)
          (face-at "1." 1)
          (face-at "[x]" 1)
          (face-at "term" 1)
          (face-at "NOTE:" 1)
          (face-at "project" 1)
          (face-at "{project}" 1)
          (face-at "listingbody" 1)
          (face-at "literalbody" 1)
          (face-at "commentbody" 1)
          (face-at "rawbody" 1)
          (face-at "quotedbody" 1)
          (face-at "tablecell" 1)
          (face-at "," 1)
          (face-at "," 3)
          (face-at ":" 3))))"##;
    let expect = expect![[
        r#"OK (adoc-list-face adoc-list-face adoc-list-face adoc-checkbox-face adoc-gen-face adoc-complex-replacement-face adoc-metadata-key-face adoc-replacement-face adoc-code-face adoc-verbatim-face adoc-comment-face adoc-passthrough-face adoc-blockquote-face nil adoc-table-face adoc-table-face adoc-complex-replacement-face)"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_font_lock_macros_xrefs_urls_footnotes_and_click_properties_match() {
    let elisp_form = r##"(cl-labels
         ((props-at
           (needle)
           (goto-char (point-min))
           (search-forward needle)
           (let ((position (match-beginning 0)))
             (list
              (get-text-property position 'face)
              (get-text-property position 'keymap)
              (get-text-property position 'mouse-face)
              (and (get-text-property position 'help-echo) t)))))
       (with-temp-buffer
         (insert
          "[[target]]\n"
          "anchor:other[Other]\n"
          "See <<target>> and xref:other[there].\n"
          "https://example.test[Site] bare https://bare.example.test/path\n"
          "image::diagram.png[Diagram]\n"
          "include::chapter.adoc[]\n"
          "kbd:[Ctrl+C] btn:[OK] menu:File[Open]\n"
          "footnote:[Anonymous note] footnote:id[Named note]\n"
          "----\n<<dead>> https://dead.example.test\n----\n")
         (adoc-mode)
         (font-lock-ensure)
         (mapcar
          #'props-at
          '("[[target]]" "anchor:other[Other]" "<<target>>"
            "xref:other[there]" "https://example.test[Site]"
            "bare.example.test" "image::diagram.png[Diagram]"
            "include::chapter.adoc[]" "kbd:[Ctrl+C]" "btn:[OK]"
            "menu:File[Open]" "footnote:[Anonymous note]"
            "footnote:id[Named note]" "<<dead>>"
            "dead.example.test"))))"##;
    let expect = expect![[
        r#"OK ((adoc-meta-face nil nil nil) (adoc-command-face nil nil nil) (adoc-meta-hide-face adoc-link-keymap adoc-link-mouse-face t) (adoc-command-face adoc-link-keymap adoc-link-mouse-face t) (adoc-url-face adoc-link-keymap adoc-link-mouse-face t) ((adoc-url-face) adoc-link-keymap adoc-link-mouse-face t) (adoc-complex-replacement-face adoc-image-link-map nil nil) (adoc-preprocessor-face adoc-link-keymap adoc-link-mouse-face t) (adoc-command-face nil nil nil) (adoc-command-face nil nil nil) (adoc-command-face nil nil nil) (adoc-footnote-marker-face nil nil nil) (adoc-footnote-marker-face nil nil nil) (adoc-code-face nil nil nil) (adoc-code-face nil nil nil))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_font_lock_native_code_replacements_and_idempotence_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "= Doc\n\n"
          "[source,emacs-lisp]\n"
          "----\n(defun hello (name) (message \"Hi %s\" name))\n----\n\n"
          "(C) (R) (TM) -> <- => <= ... --\n")
         (let ((adoc-insert-replacement t)
               (adoc-fontify-code-blocks-natively 5000))
           (adoc-mode)
           (font-lock-ensure)
           (let ((first
                  (buffer-substring (point-min) (point-max)))
                 (overlay-one
                  (mapcar
                   (lambda (overlay)
                     (list (overlay-start overlay)
                           (overlay-end overlay)
                           (overlay-get overlay 'after-string)))
                   (overlays-in (point-min) (point-max)))))
             (font-lock-flush)
             (font-lock-ensure)
             (list
              (equal-including-properties
               first (buffer-substring (point-min) (point-max)))
              overlay-one
              (mapcar
               (lambda (overlay)
                 (list (overlay-start overlay)
                       (overlay-end overlay)
                       (overlay-get overlay 'after-string)))
               (overlays-in (point-min) (point-max)))
              (progn
                (goto-char (point-min))
                (search-forward "defun")
                (get-text-property (match-beginning 0) 'face))))))"##;
    let expect = expect![[
        r#"OK (t ((86 86 "©") (90 90 "®") (95 95 "™") (98 98 "→") (101 101 "←") (104 104 "⇒") (107 107 "⇐") (111 111 "…") (114 114 "—")) ((86 86 "©") (90 90 "®") (95 95 "™") (98 98 "→") (101 101 "←") (104 104 "⇒") (107 107 "⇐") (111 111 "…") (114 114 "—")) (font-lock-keyword-face adoc-native-code-face))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_font_lock_directives_breaks_attribute_lists_nested_quotes_and_quoted_urls_match() {
    let elisp_form = r##"(cl-labels
         ((face-at
           (needle occurrence)
           (goto-char (point-min))
           (dotimes (_ occurrence)
             (search-forward needle))
           (let ((face
                  (get-text-property
                   (match-beginning 0) 'face)))
             (if (and (consp face) (null (cdr face)))
                 (car face)
               face))))
       (with-temp-buffer
         (insert
          "include::file.adoc[]\n"
          "ifdef::env[shown text]\n"
          "'''\n"
          "<<<\n"
          "first line +\n"
          "[hello]\n"
          "[hello world]\n"
          "[hello,world]\n"
          "*lorem _nested-bold-emphasis_ dolor*\n"
          "_lorem *nested-emphasis-bold* dolor_\n"
          "foo __ https://quoted.example.test/x.html __\n")
         (adoc-mode)
         (font-lock-ensure)
         (list
          (face-at "include::" 1)
          (face-at "file.adoc" 1)
          (face-at "ifdef::" 1)
          (face-at "env" 1)
          (face-at "'''" 1)
          (face-at "<<<" 1)
          (face-at "+" 1)
          (face-at "hello" 1)
          (face-at "hello world" 1)
          (face-at "hello" 3)
          (face-at "world" 2)
          (face-at "nested-bold-emphasis" 1)
          (face-at "nested-emphasis-bold" 1)
          (face-at "https://quoted.example.test/x.html" 1))))"##;
    let expect = expect![[
        r#"OK (adoc-preprocessor-face adoc-meta-face adoc-preprocessor-face adoc-meta-face adoc-complex-replacement-face adoc-meta-face adoc-meta-face adoc-value-face adoc-value-face adoc-value-face adoc-value-face (adoc-bold-face adoc-emphasis-face) (adoc-bold-face adoc-emphasis-face) (adoc-emphasis-face adoc-url-face))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_font_lock_whole_document_face_coverage_and_idempotence_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "= Document\n"
          ":project: Demo\n\n"
          "== Section\n\n"
          "* bullet with *bold* and _emphasis_\n"
          "NOTE: Read https://example.test[the docs].\n"
          "include::chapter.adoc[]\n"
          "[[target]]\n"
          "See <<target>>.\n"
          "[source,emacs-lisp]\n"
          "----\n(defun demo () t)\n----\n"
          "|===\n|cell\n|===\n"
          "// comment\n")
         (adoc-mode)
         (font-lock-ensure)
         (let ((first
                (buffer-substring
                 (point-min) (point-max)))
               faces)
           (goto-char (point-min))
           (while (< (point) (point-max))
             (let ((value
                    (get-text-property (point) 'face)))
               (dolist
                   (face
                    (if (listp value) value (list value)))
                 (when (and (symbolp face) face)
                   (cl-pushnew face faces))))
             (goto-char
              (next-single-property-change
               (point) 'face nil (point-max))))
           (font-lock-flush)
           (font-lock-ensure)
           (list
            (sort
             faces
             (lambda (left right)
               (string-lessp
                (symbol-name left)
                (symbol-name right))))
            (equal-including-properties
             first
             (buffer-substring
              (point-min) (point-max))))))"##;
    let expect = expect![[
        r#"OK ((adoc-align-face adoc-anchor-face adoc-bold-face adoc-comment-face adoc-complex-replacement-face adoc-emphasis-face adoc-list-face adoc-meta-face adoc-meta-hide-face adoc-metadata-key-face adoc-metadata-value-face adoc-native-code-face adoc-preprocessor-face adoc-reference-face adoc-table-face adoc-title-0-face adoc-title-1-face adoc-url-face font-lock-function-name-face font-lock-keyword-face) t)"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}
