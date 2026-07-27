use expect_test::expect;

use super::assert_apel_source_parity;

#[test]
fn installed_descriptor_and_complete_archive_source_set_are_exact() {
    let elisp_form = r##"(let* ((descriptor (cadr (assq 'apel package-alist)))
                           (directory (file-name-directory
                                       (getenv "NEOMACS_PACKAGE_SOURCE")))
                           (files (sort (mapcar #'file-name-nondirectory
                                                (directory-files directory t "\\.el\\'"))
                                        #'string<)))
                      (list (package-desc-name descriptor)
                            (package-version-join (package-desc-version descriptor))
                            (package-desc-reqs descriptor)
                            (package-desc-summary descriptor)
                            (package-desc-extras descriptor)
                            files))"##;
    let expect = expect![[
        r#"OK (apel "20250608.1806" ((emacs (24 5))) "Support for portable Emacs Lisp programs." ((:revdesc . "1b043cfea58e") (:commit . "1b043cfea58ea146356c237a5286ead69e97417b") (:url . "https://github.com/emacsmirror/apel")) ("alist.el" "apel-autoloads.el" "apel-pkg.el" "apel-ver.el" "apel.el" "atype.el" "broken.el" "calist.el" "emu.el" "filename.el" "inv-23.el" "invisible.el" "mcharset.el" "mcs-20.el" "mcs-e20.el" "mule-caesar.el" "path-util.el" "pccl-20.el" "pccl.el" "pces-20.el" "pces-e20.el" "pces.el" "pcustom.el" "poe.el" "poem-e20.el" "poem-e20_3.el" "poem.el" "product.el" "pym.el" "richtext.el" "static.el"))"#
    ]];
    assert_apel_source_parity("apel.el", elisp_form, expect);
}

#[test]
fn top_level_product_identity_and_runtime_version_are_exact() {
    let elisp_form = r##"(progn
                      (require 'apel-ver)
                      (list (featurep 'apel)
                      (apel-version)
                      (product-string-verbose 'apel-ver)
                      (mapcar (lambda (name)
                                (let ((product (product-find-by-name name)))
                                  (and product
                                       (list (product-name product)
                                             (product-family product)
                                             (product-version product)
                                             (product-code-name product)
                                             (product-features product)))))
                              '("APEL-LB" "apel"))))"##;
    let expect = expect![[
        r#"OK (t "APEL-LB/10.8" "APEL-LB/10.8" (("APEL-LB" nil (10 8) nil (apel-ver)) nil))"#
    ]];
    assert_apel_source_parity("apel.el", elisp_form, expect);
}

#[test]
fn collection_modules_expose_the_complete_callable_surface() {
    let elisp_form = r##"(progn
                      (mapc #'require '(alist atype calist))
                      (mapcar
                       (lambda (symbol)
                         (list symbol
                               (help-function-arglist symbol t)
                               (macrop symbol)))
                       '(put-alist del-alist set-alist remove-alist
                        modify-alist set-modified-alist vassoc
                        field-unifier-for-default field-unify assoc-unify
                        get-unified-alist delete-atype remove-atype replace-atype
                        set-atype find-calist-package
                        define-calist-field-match-method use-calist-package
                        make-calist-package in-calist-package
                        calist-default-field-match-method
                        calist-field-match-method calist-field-match
                        ctree-match-calist ctree-match-calist-partially
                        ctree-find-calist calist-to-ctree
                         ctree-add-calist-strictly ctree-add-calist-with-default
                         ctree-set-calist-strictly
                         ctree-set-calist-with-default)))"##;
    let expect = expect![
        "OK ((put-alist (key value alist) nil) (del-alist (key alist) nil) (set-alist (symbol key value) nil) (remove-alist (symbol key) nil) (modify-alist (modifier default) nil) (set-modified-alist (symbol modifier) nil) (vassoc (key avlist) nil) (field-unifier-for-default (a b) nil) (field-unify (a b) nil) (assoc-unify (class instance) nil) (get-unified-alist (db al) nil) (delete-atype (atl al) nil) (remove-atype (sym al) nil) (replace-atype (atl old-al new-al) nil) (set-atype (sym al &rest options) nil) (find-calist-package (name) nil) (define-calist-field-match-method (field-type function) nil) (use-calist-package (name) nil) (make-calist-package (name &optional use) nil) (in-calist-package (name) nil) (calist-default-field-match-method (calist field-type field-value) nil) (calist-field-match-method (field-type) nil) (calist-field-match (calist field-type field-value) nil) (ctree-match-calist (rule-tree alist) nil) (ctree-match-calist-partially (rule-tree alist) nil) (ctree-find-calist (rule-tree alist &optional all) nil) (calist-to-ctree (calist) nil) (ctree-add-calist-strictly (ctree calist) nil) (ctree-add-calist-with-default (ctree calist) nil) (ctree-set-calist-strictly (ctree-var calist) nil) (ctree-set-calist-with-default (ctree-var calist) nil))"
    ];
    assert_apel_source_parity("calist.el", elisp_form, expect);
}

#[test]
fn compatibility_macro_modules_expose_every_macro_and_function() {
    let elisp_form = r##"(progn
                      (mapc #'require '(static broken pym))
                      (mapcar
                       (lambda (symbol)
                         (list symbol
                               (help-function-arglist symbol t)
                               (macrop symbol)))
                       '(static-if static-when static-unless
                         static-condition-case static-defconst static-cond
                         broken-facility-internal broken-p
                         broken-facility-description
                         broken-facility if-broken when-broken unless-broken
                         check-broken-facility defun-maybe defmacro-maybe
                         defsubst-maybe defalias-maybe defvar-maybe
                         defconst-maybe defun-maybe-cond defmacro-maybe-cond
                         defsubst-maybe-cond def-edebug-spec subr-fboundp)))"##;
    let expect = expect![
        "OK ((static-if (cond then &rest else) t) (static-when (cond &rest body) t) (static-unless (cond &rest body) t) (static-condition-case (var bodyform &rest handlers) t) (static-defconst (symbol initvalue &optional docstring) t) (static-cond (&rest clauses) t) (broken-facility-internal (facility &optional docstring assertion) nil) (broken-p (facility) nil) (broken-facility-description (facility) nil) (broken-facility (facility &optional docstring assertion no-notice) t) (if-broken (facility then &rest else) t) (when-broken (facility &rest body) t) (unless-broken (facility &rest body) t) (check-broken-facility (facility) t) (defun-maybe (name &rest everything-else) t) (defmacro-maybe (name &rest everything-else) t) (defsubst-maybe (name &rest everything-else) t) (defalias-maybe (symbol definition) t) (defvar-maybe (name &rest everything-else) t) (defconst-maybe (name &rest everything-else) t) (defun-maybe-cond (name args &optional doc &rest clauses) t) (defmacro-maybe-cond (name args &optional doc &rest clauses) t) (defsubst-maybe-cond (name args &optional doc &rest clauses) t) (def-edebug-spec (symbol spec) t) (subr-fboundp (symbol) nil))"
    ];
    assert_apel_source_parity("pym.el", elisp_form, expect);
}

#[test]
fn product_module_exposes_every_accessor_mutator_and_workflow_api() {
    let elisp_form = r##"(mapcar
                      (lambda (symbol)
                        (list symbol
                              (help-function-arglist symbol t)
                              (macrop symbol)))
                      '(product-define product-name product-family product-version
                        product-code-name product-checkers
                        product-family-products product-features
                        product-version-string product-set-name
                        product-set-family product-set-version
                        product-set-code-name product-set-checkers
                        product-set-family-products product-set-features
                        product-set-version-string product-add-to-family
                        product-remove-from-family product-add-checkers
                        product-remove-checkers product-add-feature
                        product-remove-feature product-run-checkers
                        product-find-by-name product-find-by-feature product-find
                        product-provide product-version-as-string product-string-1
                        product-for-each product-string product-string-verbose
                        product-version-compare product-version>=
                        product-list-products product-parse-version-string))"##;
    let expect = expect![
        "OK ((product-define (name &optional family version code-name) nil) (product-name (product) nil) (product-family (product) nil) (product-version (product) nil) (product-code-name (product) nil) (product-checkers (product) nil) (product-family-products (product) nil) (product-features (product) nil) (product-version-string (product) nil) (product-set-name (product name) nil) (product-set-family (product family) nil) (product-set-version (product version) nil) (product-set-code-name (product code-name) nil) (product-set-checkers (product checkers) nil) (product-set-family-products (product products) nil) (product-set-features (product features) nil) (product-set-version-string (product version-string) nil) (product-add-to-family (family product-name) nil) (product-remove-from-family (family product-name) nil) (product-add-checkers (product &rest checkers) nil) (product-remove-checkers (product &rest checkers) nil) (product-add-feature (product feature) nil) (product-remove-feature (product feature) nil) (product-run-checkers (product version &optional force) nil) (product-find-by-name (name) nil) (product-find-by-feature (feature) nil) (product-find (product) nil) (product-provide (feature-def product-def) t) (product-version-as-string (product) nil) (product-string-1 (product &optional verbose) nil) (product-for-each (product all function &rest args) nil) (product-string (product) nil) (product-string-verbose (product) nil) (product-version-compare (v1 v2) nil) (product-version>= (product require-version) nil) (product-list-products nil nil) (product-parse-version-string (verstr) nil))"
    ];
    assert_apel_source_parity("product.el", elisp_form, expect);
}

#[test]
fn file_text_and_runtime_modules_expose_every_callable_api() {
    let elisp_form = r##"(progn
                      (mapc #'require
                            '(filename path-util invisible poe emu richtext))
                      (mapcar
                       (lambda (symbol)
                         (list symbol
                               (help-function-arglist symbol t)
                               (macrop symbol)))
                       '(poly-funcall filename-japanese-to-roman-string
                         filename-special-filter-1
                        filename-control-p filename-special-filter
                        filename-eliminate-top-low-lines
                        filename-canonicalize-low-lines
                        filename-maybe-truncate-by-size
                        filename-eliminate-bottom-low-lines replace-as-filename
                        add-path add-latest-path get-latest-path file-installed-p
                        exec-installed-p module-installed-p enable-invisible
                        disable-invisible end-of-invisible invisible-region
                        visible-region next-visible-point remassoc remassq
                        remrassoc remrassq save-selected-frame find-face
                         character-to-event event-to-character
                         next-command-event cancel-undo-boundary
                         char-list-to-string
                         insert-binary-file-contents-literally
                         insert-binary-file-contents
                         char-category richtext-encode
                         richtext-next-annotation richtext-decode)))"##;
    let expect = expect![
        "OK ((poly-funcall (functions argument) nil) (filename-japanese-to-roman-string (str) nil) (filename-special-filter-1 (string) t) (filename-control-p (character) nil) (filename-special-filter (string) nil) (filename-eliminate-top-low-lines (string) nil) (filename-canonicalize-low-lines (string) nil) (filename-maybe-truncate-by-size (string) nil) (filename-eliminate-bottom-low-lines (string) nil) (replace-as-filename (string) nil) (add-path (path &rest options) nil) (add-latest-path (pattern &optional all-paths) nil) (get-latest-path (pattern &optional all-paths) nil) (file-installed-p (file &optional paths) nil) (exec-installed-p (file &optional paths suffixes) nil) (module-installed-p (module &optional paths) nil) (enable-invisible nil nil) (disable-invisible nil nil) (end-of-invisible nil nil) (invisible-region (start end) nil) (visible-region (start end) nil) (next-visible-point (pos) nil) (remassoc (key alist) nil) (remassq (key alist) nil) (remrassoc (value alist) nil) (remrassq (value alist) nil) (save-selected-frame (&rest body) t) (find-face (face-or-name) nil) (character-to-event (ch) nil) (event-to-character (event) nil) (next-command-event (&optional _event prompt) nil) (cancel-undo-boundary nil nil) (char-list-to-string (char-list) nil) (insert-binary-file-contents-literally (filename &optional visit beg end replace) nil) (insert-binary-file-contents t nil) (char-category (character) nil) (richtext-encode (from to) nil) (richtext-next-annotation nil nil) (richtext-decode (from to) nil))"
    ];
    assert_apel_source_parity("richtext.el", elisp_form, expect);
}

#[test]
fn coding_modules_expose_every_callable_and_macro_api() {
    let elisp_form = r##"(progn
                      (mapc #'require
                            '(mcharset mcs-20 mcs-e20 mule-caesar
                              pces pces-20 pces-e20 poem poem-e20 poem-e20_3))
                      (mapcar
                       (lambda (symbol)
                         (list symbol
                               (help-function-arglist symbol t)
                               (macrop symbol)))
                       '(charsets-to-mime-charset
                         find-mime-charset-by-charsets
                        mime-charset-to-coding-system mime-charset-p
                        widget-mime-charset-prompt-value
                        widget-mime-charset-action detect-mime-charset-list
                        detect-mime-charset-from-coding-system
                        detect-mime-charset-string detect-mime-charset-region
                        write-region-as-mime-charset encode-mime-charset-region
                        decode-mime-charset-region encode-mime-charset-string
                        decode-mime-charset-string coding-system-to-mime-charset
                        mime-charset-list mule-caesar-region as-binary-process
                        as-binary-input-file as-binary-output-file
                        write-region-as-binary insert-file-contents-as-binary
                        insert-file-contents-as-raw-text
                        insert-file-contents-as-raw-text-CRLF
                        write-region-as-raw-text-CRLF
                        find-file-noselect-as-binary
                        find-file-noselect-as-raw-text
                        find-file-noselect-as-raw-text-CRLF
                        save-buffer-as-binary save-buffer-as-raw-text-CRLF
                        open-network-stream-as-binary
                        insert-file-contents-as-coding-system
                        write-region-as-coding-system
                        find-file-noselect-as-coding-system
                        save-buffer-as-coding-system find-coding-system
                        set-process-input-coding-system fontset-pixel-size
                        find-non-ascii-charset-string
                        find-non-ascii-charset-region char-length char-next-index
                        sset string-to-char-list string-to-int-list
                         looking-at-as-unibyte char-int int-char
                         char-or-char-int-p char-octet)))"##;
    let expect = expect![
        "OK ((charsets-to-mime-charset (charsets) nil) (find-mime-charset-by-charsets (charsets &optional mode &rest args) nil) (mime-charset-to-coding-system #1=(charset &optional lbt) nil) (mime-charset-p #1# nil) (widget-mime-charset-prompt-value (_widget prompt value _unbound) nil) (widget-mime-charset-action (widget &optional event) nil) (detect-mime-charset-list (chars) nil) (detect-mime-charset-from-coding-system (start end &optional string) nil) (detect-mime-charset-string (string) nil) (detect-mime-charset-region (start end) nil) (write-region-as-mime-charset (charset start end filename &optional append visit lockname) nil) (encode-mime-charset-region (start end charset &optional lbt) nil) (decode-mime-charset-region (start end charset &optional lbt) nil) (encode-mime-charset-string (string charset &optional lbt) nil) (decode-mime-charset-string (string charset &optional lbt) nil) (coding-system-to-mime-charset (coding-system) nil) (mime-charset-list nil nil) (mule-caesar-region (start end &optional stride-ascii) nil) (as-binary-process (&rest body) t) (as-binary-input-file (&rest body) t) (as-binary-output-file (&rest body) t) (write-region-as-binary (start end filename &optional append visit lockname) nil) (insert-file-contents-as-binary (filename &optional visit beg end replace) nil) (insert-file-contents-as-raw-text (filename &optional visit beg end replace) nil) (insert-file-contents-as-raw-text-CRLF (filename &optional visit beg end replace) nil) (write-region-as-raw-text-CRLF (start end filename &optional append visit lockname) nil) (find-file-noselect-as-binary (filename &optional nowarn rawfile) nil) (find-file-noselect-as-raw-text (filename &optional nowarn rawfile) nil) (find-file-noselect-as-raw-text-CRLF (filename &optional nowarn rawfile) nil) (save-buffer-as-binary (&optional args) nil) (save-buffer-as-raw-text-CRLF (&optional args) nil) (open-network-stream-as-binary (name buffer host service) nil) (insert-file-contents-as-coding-system (coding-system filename &optional visit beg end replace) nil) (write-region-as-coding-system (coding-system start end filename &optional append visit lockname) nil) (find-file-noselect-as-coding-system (coding-system filename &optional nowarn rawfile) nil) (save-buffer-as-coding-system (coding-system &optional args) nil) (find-coding-system (obj) nil) (set-process-input-coding-system (process &optional decoding encoding) nil) (fontset-pixel-size (fontset) nil) (find-non-ascii-charset-string (string) nil) (find-non-ascii-charset-region (start end) nil) (char-length (_char) nil) (char-next-index (_char index) t) (sset (string idx obj) nil) (string-to-char-list (string) nil) (string-to-int-list (string) nil) (looking-at-as-unibyte (regexp &optional inhibit-modify) nil) (char-int (argument) nil) (int-char (argument) nil) (char-or-char-int-p (object) nil) (char-octet (ch &optional n) nil))"
    ];
    assert_apel_source_parity("mcharset.el", elisp_form, expect);
}

#[test]
fn all_documented_configuration_variables_have_exact_defaults_and_types() {
    let elisp_form = r##"(progn
                      (mapc #'require
                            '(broken calist emu filename mcharset mcs-20 mcs-e20
                              path-util poe product richtext))
                      (mapcar
                       (lambda (symbol)
                         (let ((value
                                (and (boundp symbol)
                                     (symbol-value symbol))))
                           (list symbol
                                 (boundp symbol)
                                 (cond
                                  ((eq symbol 'default-load-path)
                                   (mapcar
                                    (lambda (path)
                                      (file-name-nondirectory
                                       (directory-file-name path)))
                                    value))
                                  ((eq symbol 'product-obarray)
                                   (list :obarray
                                         (length value)
                                         (length
                                          (product-list-products))))
                                  (t value))
                                 (get symbol 'custom-type))))
                       '(calist-package-alist
                         calist-field-match-method-obarray
                         notice-non-obvious-broken-facility
                        running-emacs-18 running-xemacs
                        running-mule-merged-emacs running-xemacs-with-mule
                        running-emacs-19 running-emacs-19_29-or-later
                        running-xemacs-19 running-xemacs-20-or-later
                        running-xemacs-19_14-or-later mouse-button-1
                        mouse-button-2 mouse-button-3 filename-limit-length
                        filename-replacement-alist filename-filters
                        default-mime-charset-for-write
                        default-mime-charset-detect-method-for-write
                        mime-charset-coding-system-alist
                        mime-charset-to-coding-system-default-method
                        widget-mime-charset-prompt-value-history
                        default-mime-charset detect-mime-charset-from-coding-system
                        charsets-mime-charset-alist
                        coding-system-to-mime-charset-exclude-regexp
                         default-load-path exec-suffix-list buffer-file-type
                         *noconv* product-obarray product-ignore-checkers
                         richtext-initial-annotation
                         richtext-annotation-regexp richtext-translations)))"##;
    let expect = expect![[
        r#"OK ((calist-package-alist t ((standard . #1=[#<obarray n=1> 0 0 0 0 0 0])) nil) (calist-field-match-method-obarray t #1# nil) (notice-non-obvious-broken-facility t t nil) (running-emacs-18 t nil nil) (running-xemacs t nil nil) (running-mule-merged-emacs t t nil) (running-xemacs-with-mule t nil nil) (running-emacs-19 t nil nil) (running-emacs-19_29-or-later t t nil) (running-xemacs-19 t nil nil) (running-xemacs-20-or-later t nil nil) (running-xemacs-19_14-or-later t nil nil) (mouse-button-1 t [mouse-1] nil) (mouse-button-2 t [mouse-2] nil) (mouse-button-3 t [down-mouse-3] nil) (filename-limit-length t 21 nil) (filename-replacement-alist t (((32 9) . "_") ((33 34 35 36 37 38 39 40 41 42 47 58 59 60 62 63 91 92 93 96 123 124 125) . "_") (filename-control-p . "")) nil) (filename-filters t nil nil) (default-mime-charset-for-write t utf-8 mime-charset) (default-mime-charset-detect-method-for-write t nil (choice function (const nil))) (mime-charset-coding-system-alist t ((x-unknown . undecided) (unknown . undecided) (windows-874 . tis-620)) (repeat (cons symbol coding-system))) (mime-charset-to-coding-system-default-method t nil (choice function (const nil))) (widget-mime-charset-prompt-value-history t nil nil) (default-mime-charset t x-unknown mime-charset) (detect-mime-charset-from-coding-system t nil boolean) (charsets-mime-charset-alist t (((ascii) . us-ascii) ((ascii latin-iso8859-1) . iso-8859-1) ((ascii latin-iso8859-2) . iso-8859-2) ((ascii latin-iso8859-3) . iso-8859-3) ((ascii latin-iso8859-4) . iso-8859-4) ((ascii latin-iso8859-15) . iso-8859-15) ((ascii cyrillic-iso8859-5) . koi8-r) ((ascii arabic-iso8859-6) . iso-8859-6) ((ascii greek-iso8859-7) . iso-8859-7) ((ascii hebrew-iso8859-8) . iso-8859-8) ((ascii latin-iso8859-9) . iso-8859-9) ((ascii latin-iso8859-14) . iso-8859-14) ((ascii latin-jisx0201 japanese-jisx0208-1978 japanese-jisx0208) . iso-2022-jp) ((ascii latin-jisx0201 katakana-jisx0201 japanese-jisx0208) . shift_jis) ((ascii korean-ksc5601) . euc-kr) ((ascii chinese-gb2312) . gb2312) ((ascii chinese-big5-1 chinese-big5-2) . big5) ((ascii thai-tis620) . tis-620)) nil) (coding-system-to-mime-charset-exclude-regexp t "^unknown$\\|^x-" nil) (default-load-path t ("apel-20250608.1806" "lisp" "vc" "use-package" "url" "textmodes" "progmodes" "play" "org" "nxml" "net" "mh-e" "mail" "leim" "language" "international" "image" "gnus" "eshell" "erc" "emulation" "emacs-lisp" "cedet" "calendar" "calc" "obsolete") nil) (exec-suffix-list t ("") nil) (buffer-file-type t nil nil) (*noconv* t binary nil) (product-obarray t (:obarray 13 1) nil) (product-ignore-checkers t nil nil) (richtext-initial-annotation t #[nil ((format "Content-Type: text/richtext\nText-Width: %d\n\n" fill-column)) (t)] nil) (richtext-annotation-regexp t "[ \11\n]*\\(<\\(/\\)?\\([-A-za-z0-9]+\\)>\\)[ \11\n]*" nil) (richtext-translations t ((face (bold-italic "bold" "italic") (bold "bold") (italic "italic") (underline "underline") (fixed "fixed") (excerpt "excerpt") (default) (nil enriched-encode-other-face)) (invisible (t "comment")) (left-margin (4 "indent")) (right-margin (4 "indentright")) (justification (right "flushright") (left "flushleft") (full "flushboth") (center "center")) (FUNCTION (enriched-decode-foreground "x-color") (enriched-decode-background "x-bg-color")) (read-only (t "x-read-only")) (unknown (nil format-annotate-value))) nil))"#
    ]];
    assert_apel_source_parity("richtext.el", elisp_form, expect);
}
