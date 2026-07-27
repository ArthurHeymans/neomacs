use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_tempo_quote_passthrough_and_role_templates_wrap_empty_and_region_text() {
    let elisp_form = r##"(cl-labels
         ((render
           (template text region)
           (with-temp-buffer
             (adoc-mode)
             (insert text)
             (goto-char (point-max))
             (when region
               (set-mark (point-min))
               (activate-mark))
             (let ((this-command template)
                   (current-prefix-arg region))
               (funcall template region))
             (buffer-string))))
       (mapcar
        (lambda (template)
          (list template
                (render template "" nil)
                (render template "content" t)))
        '(tempo-template-adoc-emphasis
          tempo-template-adoc-bold
          tempo-template-adoc-typewriter-face
          tempo-template-adoc-monospace-literal
          tempo-template-adoc-double-curved-quote
          tempo-template-adoc-single-curved-quote
          tempo-template-adoc-attributed
          tempo-template-adoc-underline
          tempo-template-adoc-overline
          tempo-template-adoc-line-through
          tempo-template-adoc-nobreak
          tempo-template-adoc-nowrap
          tempo-template-adoc-pre-wrap
          tempo-template-adoc-emphasis-uc
          tempo-template-adoc-bold-uc
          tempo-template-adoc-monospace-uc
          tempo-template-adoc-superscript
          tempo-template-adoc-subscript
          tempo-template-adoc-pass
          tempo-template-adoc-asciimath
          tempo-template-adoc-latexmath
          tempo-template-adoc-pass-+++
          tempo-template-adoc-pass-$$)))"##;
    let expect = expect![[
        r#"OK ((tempo-template-adoc-emphasis "__" "_content_") (tempo-template-adoc-bold "**" "*content*") (tempo-template-adoc-typewriter-face "++" "+content+") (tempo-template-adoc-monospace-literal "``" "`content`") (tempo-template-adoc-double-curved-quote "\"``\"" "\"`content`\"") (tempo-template-adoc-single-curved-quote "'``'" "'`content`'") (tempo-template-adoc-attributed "[]##" "[]#content#") (tempo-template-adoc-underline "[.underline]##" "[.underline]#content#") (tempo-template-adoc-overline "[.overline]##" "[.overline]#content#") (tempo-template-adoc-line-through "[.line-through]##" "[.line-through]#content#") (tempo-template-adoc-nobreak "[.nobreak]##" "[.nobreak]#content#") (tempo-template-adoc-nowrap "[.nowrap]##" "[.nowrap]#content#") (tempo-template-adoc-pre-wrap "[.pre-wrap]##" "[.pre-wrap]#content#") (tempo-template-adoc-emphasis-uc "____" "__content__") (tempo-template-adoc-bold-uc "****" "**content**") (tempo-template-adoc-monospace-uc "++++" "++content++") (tempo-template-adoc-superscript "^^" "^content^") (tempo-template-adoc-subscript "~~" "~content~") (tempo-template-adoc-pass "pass:[]" "pass:[content]") (tempo-template-adoc-asciimath "asciimath:[]" "asciimath:[content]") (tempo-template-adoc-latexmath "latexmath:[]" "latexmath:[content]") (tempo-template-adoc-pass-+++ "++++++" "+++content+++") (tempo-template-adoc-pass-$$ "$$$$" "$$content$$"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_tempo_title_paragraph_break_list_table_and_block_templates_match() {
    let elisp_form = r##"(cl-labels
         ((render
           (template &optional style)
           (with-temp-buffer
             (adoc-mode)
             (let ((adoc-title-style
                    (or style 'adoc-title-style-one-line))
                   (this-command template))
               (funcall template))
             (buffer-string))))
       (list
        (mapcar
         (lambda (style)
           (list style
                 (render 'tempo-template-adoc-title-1 style)
                 (render 'tempo-template-adoc-title-3 style)))
         '(adoc-title-style-one-line
           adoc-title-style-one-line-enclosed
           adoc-title-style-two-line))
        (mapcar
         #'render
         '(tempo-template-adoc-line-break
           tempo-template-adoc-page-break
           tempo-template-adoc-ruler-line
           tempo-template-adoc-literal-paragraph
           tempo-template-adoc-paragraph-tip
           tempo-template-adoc-paragraph-note
           tempo-template-adoc-paragraph-important
           tempo-template-adoc-paragraph-warning
           tempo-template-adoc-paragraph-caution
           tempo-template-adoc-bulleted-list-item-1
           tempo-template-adoc-bulleted-list-item-2
           tempo-template-adoc-implicit-numbered-list-item-1
           tempo-template-adoc-list-item-continuation
           tempo-template-adoc-example-table))
        (mapcar
         #'render
         '(tempo-template-adoc-delimited-block-comment
           tempo-template-adoc-delimited-block-passthrough
           tempo-template-adoc-delimited-block-listing
           tempo-template-adoc-delimited-block-literal
           tempo-template-adoc-delimited-block-quote
           tempo-template-adoc-delimited-block-example
           tempo-template-adoc-delimited-block-sidebar
           tempo-template-adoc-delimited-block-open-block))))"##;
    let expect = expect![[
        r#"OK (((adoc-title-style-one-line "= " "=== ") (adoc-title-style-one-line-enclosed "=  =" "===  ===") (adoc-title-style-two-line "\n====" "\n~~~~")) (" +" "<<<" "---" "  " "TIP: " "NOTE: " "IMPORTANT: " "WARNING: " "CAUTION: " "      - " "\11     ** " "      . " "+" "|===\n| cell 11 | cell 12\n| cell 21 | cell 22\n|===\n") ("//////////////////////////////////////////////////\n\n//////////////////////////////////////////////////" "++++++++++++++++++++++++++++++++++++++++++++++++++\n\n++++++++++++++++++++++++++++++++++++++++++++++++++" "--------------------------------------------------\n\n--------------------------------------------------" "..................................................\n\n.................................................." "__________________________________________________\n\n__________________________________________________" "==================================================\n\n==================================================" "**************************************************\n\n**************************************************" "--\n\n--"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_tempo_macro_templates_and_handler_protocol_match() {
    let elisp_form = r##"(cl-labels
         ((render
           (template)
           (with-temp-buffer
             (adoc-mode)
             (let ((this-command template))
               (funcall template))
             (buffer-string))))
       (list
        (mapcar
         (lambda (template) (list template (render template)))
         '(tempo-template-adoc-url
           tempo-template-adoc-url-caption
           tempo-template-adoc-email
           tempo-template-adoc-email-caption
           tempo-template-adoc-anchor
           tempo-template-adoc-anchor-default-syntax
           tempo-template-adoc-xref
           tempo-template-adoc-xref-default-syntax
           tempo-template-adoc-image
           tempo-template-adoc-entity-reference
           tempo-template-adoc-copyright
           tempo-template-adoc-trademark
           tempo-template-adoc-registered-trademark
           tempo-template-adoc-right-arrow
           tempo-template-adoc-left-arrow))
        (mapcar
         (lambda (element)
           (copy-tree (adoc-tempo-handler element)))
         '(bol r-or-n (r-or-n "text" text)
           (tr "a" "b") unrelated))
        (mapcar
         (lambda (spec) (apply #'adoc-template-str-title spec))
         '((0 "Alpha") (2 "Beta") (4 "Gamma")))))"##;
    let expect = expect![[
        r#"OK (((tempo-template-adoc-url "http://foo.com") (tempo-template-adoc-url-caption "http://foo.com[]") (tempo-template-adoc-email "bob@foo.com") (tempo-template-adoc-email-caption "mailto:[]") (tempo-template-adoc-anchor "[[]]") (tempo-template-adoc-anchor-default-syntax "anchor:[]") (tempo-template-adoc-xref "<<,>>") (tempo-template-adoc-xref-default-syntax "xref:[]") (tempo-template-adoc-image "image:[]") (tempo-template-adoc-entity-reference "&;") (tempo-template-adoc-copyright "(C)") (tempo-template-adoc-trademark "(T)") (tempo-template-adoc-registered-trademark "(R)") (tempo-template-adoc-right-arrow "->") (tempo-template-adoc-left-arrow "<-")) ("" (tr p n) (tr p n) "" nil) ("Alpha\\n= " "Beta\\n=== " "Gamma\\n===== "))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}
