use expect_test::expect;

use super::assert_ac_html_parity;

#[test]
fn ac_html_default_data_paths_match_the_packaged_completion_tree() {
    let elisp_form = r##"(progn
               (require
                'ac-html-default-data-provider)
               (let ((relative
                      (lambda (path)
                        (file-relative-name
                         path
                         web-completion-data-package-dir))))
                 (list
                  (mapcar
                   #'featurep
                   '(ac-html-default-data-provider
                     f))
                  (funcall relative
                           web-completion-data-html-source-dir)
                  (funcall relative
                           web-completion-data-tag-list-file)
                  (funcall relative
                           web-completion-data-tag-doc-dir)
                  (funcall relative
                           (web-completion-data-tag-doc-file
                            "div"))
                  (funcall relative
                           web-completion-data-attr-list-dir)
                  (funcall relative
                           web-completion-data-attr-global-list-file)
                  (funcall relative
                           (web-completion-data-attr-list-file
                            "meta"))
                  (funcall relative
                           web-completion-data-attr-doc-dir)
                  (funcall relative
                           (web-completion-data-attr-global-doc-file
                            "id"))
                  (funcall relative
                           (web-completion-data-attr-doc-file
                            "div"
                            "class"))
                  (funcall relative
                           web-completion-data-attrv-list-dir)
                  (funcall relative
                           (web-completion-data-attrv-list-file
                            "a"
                            "target"))
                  (funcall relative
                           (web-completion-data-attrv-global-list-file
                            "lang"))
                  (funcall relative
                           web-completion-data-attrv-doc-dir)
                  (funcall relative
                           (web-completion-data-attrv-global-doc-file
                            "lang"
                            "en"))
                  (funcall relative
                           (web-completion-data-attrv-doc-file
                            "script"
                            "type"
                            "text/javascript")))))"##;
    let expect = expect![[
        r#"OK ((t t) "completion-data" "completion-data/html-tag-list" "completion-data/html-tag-short-docs" "completion-data/html-tag-short-docs/div" "completion-data/html-attributes-list" "completion-data/html-attributes-list/global" "completion-data/html-attributes-list/meta" "completion-data/html-attributes-short-docs" "completion-data/html-attributes-short-docs/global-id" "completion-data/html-attributes-short-docs/div-class" "completion-data/html-attrv-list" "completion-data/html-attrv-list/a-target" "completion-data/html-attrv-list/global-lang" "completion-data/html-attrv-docs" "completion-data/html-attrv-docs/global-lang-en" "completion-data/html-attrv-docs/script-type-text%2Fjavascript")"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_default_list_helpers_handle_missing_empty_and_trailing_newlines() {
    let elisp_form = r##"(progn
               (require
                'ac-html-default-data-provider)
               (let* ((root
                       (make-temp-file
                        "ac-html-data-" t))
                      (missing
                       (expand-file-name
                        "missing" root))
                      (empty
                       (expand-file-name
                        "empty" root))
                      (values
                       (expand-file-name
                        "values" root)))
                 (unwind-protect
                     (progn
                       (with-temp-file empty)
                       (with-temp-file values
                         (insert
                          "alpha\n\nβeta\n"))
                       (list
                        (ac-html--load-list-from-file
                         missing)
                        (ac-html--load-list-from-file
                         empty)
                        (ac-html--load-list-from-file
                         values)
                        (ac-html--read-file
                         missing)
                        (ac-html--read-file
                         empty)
                        (ac-html--read-file
                         values)
                        (get-file-buffer values)))
                   (delete-directory root t))))"##;
    let expect = expect![[r#"OK (nil nil ("alpha" "βeta") nil "" "alpha\n\nβeta\n" nil)"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_default_tags_port_upstream_boundaries_and_cache_identity() {
    let elisp_form = r##"(progn
               (require
                'ac-html-default-data-provider)
               (let ((ac-html--tags-list nil))
                 (let ((first
                        (ac-html-default-tags))
                       (second
                        (ac-html-default-tags)))
                   (list
                    (car first)
                    (length first)
                    (nth 73 first)
                    (nth 146 first)
                    (eq first second)
                    (eq first
                        ac-html--tags-list)))))"##;
    let expect = expect![[r#"OK ("a" 147 "keygen" "xmp" t t)"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_default_attrs_port_upstream_values_and_global_cache_behavior() {
    let elisp_form = r##"(progn
               (require
                'ac-html-default-data-provider)
               (let ((ac-html--global-attributes
                      nil))
                 (let ((anchor
                        (ac-html-default-attrs
                         "a"))
                       (unknown
                        (ac-html-default-attrs
                         "not-a-real-tag")))
                   (list
                    (car anchor)
                    (length anchor)
                    (nth 13 anchor)
                    (nth 14 anchor)
                    (nth 34 anchor)
                    unknown
                    (length
                     ac-html--global-attributes)
                    (equal
                     (last anchor
                           (length
                            ac-html--global-attributes))
                     ac-html--global-attributes)))))"##;
    let expect = expect![[
        r#"OK ("download" 35 "type" "urn" "title" ("accesskey" "class" "contenteditable" "contextmenu" "data-" "dir" "draggable" "dropzone" "hidden" "id" "itemid" "itemprop" "itemref" "itemscope" "itemtype" "lang" "spellcheck" "style" "tabindex" "title") 20 t)"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_default_attr_values_merge_tag_specific_and_global_data() {
    let elisp_form = r##"(progn
               (require
                'ac-html-default-data-provider)
               (list
                (ac-html-default-attrvs
                 "a"
                 "target")
                (ac-html-default-attrvs
                 "div"
                 "dir")
                (ac-html-default-attrvs
                 "not-a-tag"
                 "lang")
                (ac-html-default-attrvs
                 "not-a-tag"
                 "not-an-attr")))"##;
    let expect = expect![[
        r#"OK (("_blank" "_parent" "_self" "_top") ("auto" "ltr" "rtl") ("aa" "ab" "af" "am" "an" "ar" "as" "ay" "az" "ba" "be" "bg" "bh" "bi" "bn" "bo" "br" "ca" "co" "cs" "cy" "da" "de" "dz" "el" "en" "eo" "es" "et" "eu" "fa" "fi" "fj" "fo" "fr" "fy" "ga" "gd" "gl" "gn" "gu" "gv" "ha" "he" "hi" "hr" "ht" "hu" "hy" "ia" "id" "ie" "ii" "ik" "in" "io" "is" "it" "iu" "iw" "ja" "ji" "jv" "ka" "kk" "kl" "km" "kn" "ko" "ks" "ku" "ky" "la" "li" "ln" "lo" "lt" "lv" "mg" "mi" "mk" "ml" "mn" "mo" "mr" "ms" "mt" "my" "na" "ne" "nl" "no" "oc" "om" "or" "pa" "pl" "ps" "pt" "qu" "rm" "rn" "ro" "ru" "rw" "sa" "sd" "sg" "sh" "si" "sk" "sl" "sm" "sn" "so" "sq" "sr" "ss" "st" "su" "sv" "sw" "ta" "te" "tg" "th" "ti" "tk" "tl" "tn" "to" "tr" "ts" "tt" "tw" "ug" "uk" "ur" "uz" "vi" "vo" "wa" "wo" "xh" "yi" "yo" "zh" "zh-Hans" "zh-Hant" "zu") nil)"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_default_documentation_prefers_tag_specific_then_global_files() {
    let elisp_form = r##"(progn
               (require
                'ac-html-default-data-provider)
               (list
                (ac-html-default-tag-doc
                 "a")
                (ac-html-default-tag-doc
                 "not-a-tag")
                (ac-html-default-attr-doc
                 "a"
                 "href")
                (ac-html-default-attr-doc
                 "div"
                 "id")
                (ac-html-default-attr-doc
                 "not-a-tag"
                 "not-an-attr")
                (ac-html-default-attrv-doc
                 "a"
                 "target"
                 "_blank")
                (ac-html-default-attrv-doc
                 "not-a-tag"
                 "lang"
                 "en")
                (ac-html-default-attrv-doc
                 "not-a-tag"
                 "not-an-attr"
                 "not-a-value")))"##;
    let expect = expect![[
        r##"OK ("The HTML <a> Element (or the HTML Anchor Element) defines a hyperlink, the named target destination for a hyperlink, or both.\n\nContent categories:\nFlow content, phrasing content, interactive content, palpable content.\n\nPermitted content:\nTransparent, containing either flow content or phrasing content.\n\nTag omission:\nNone, both the starting and ending tag are mandatory.\n\nPermitted parent elements:\nAny element that accepts phrasing content, or any element that accepts flow content.\n\nDOM interface:\nHTMLAnchorElement" nil "This was the single required attribute for anchors defining a hypertext source link, but is no longer required in HTML5. Omitting this attribute creates a placeholder link. The href attribute indicates the link target, either a URL or a URL fragment. A URL fragment is a name preceded by a hash mark (#), which specifies an internal target location (an ID) within the current document. URLs are not restricted to Web (HTTP)-based documents. URLs might use any protocol supported by the browser. For example, file, ftp, and mailto work in most user agents.\n\nNote: \n\nYou can use the special fragment \"top\" to create a link back to the top of the page; for example <a href=\"#top\">Return to top</a>. This behavior is specified by HTML5." "id\n\nThis attribute defines a unique identifier (ID) which must be unique in the whole document. Its purpose is to identify the element when linking (using a fragment identifier), scripting, or styling (with CSS).\n\nUsage note:\nThis attribute's value is an opaque string: this means that web author must not use it to convey any information. Particular meaning, for example semantic meaning, must not be derived from the string.\nThis attribute's value must not contain white spaces. Browsers treat non-conforming IDs that contains white spaces as if the white space is part of the ID. In contrast to the class attribute, which allows space-separated values, elements can only have one single ID defined through the id attribute. Note that an element may have several IDs, but the others should be set by another means, such as via a script interfacing with the DOM interface of the element.\nUsing characters except ASCII letters and digits, '_', '-' and '.' may cause compatibility problems, as they weren't allowed in HTML 4. Though this restriction has been lifted in HTML 5, an ID should start with a letter for compatibility." nil "Load in a new window\n" "Description: EnglishnAdded: 2005-10-16nSuppress-Script: Latnn\n" nil)"##
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_default_provider_registers_exact_supported_callbacks() {
    let elisp_form = r##"(progn
               (require
                'ac-html-default-data-provider)
               (list
                ac-html-data-providers
                (mapcar
                 (lambda (key)
                   (cons
                    key
                    (get
                     'ac-html-default-data-provider
                     key)))
                 '(:tag-func
                   :attr-func
                   :attrv-func
                   :id-func
                   :class-func
                   :tag-doc-func
                   :attr-doc-func
                   :attrv-doc-func
                   :id-doc-func
                   :class-doc-func))))"##;
    let expect = expect![
        "OK ((ac-html-default-data-provider) ((:tag-func . ac-html-default-tags) (:attr-func . ac-html-default-attrs) (:attrv-func . ac-html-default-attrvs) (:id-func) (:class-func) (:tag-doc-func . ac-html-default-tag-doc) (:attr-doc-func . ac-html-default-attr-doc) (:attrv-doc-func . ac-html-default-attrv-doc) (:id-doc-func) (:class-doc-func)))"
    ];

    assert_ac_html_parity(elisp_form, expect);
}
