use expect_test::expect;

use super::{assert_adoc_mode_autoload_parity, assert_adoc_mode_parity};

#[test]
fn adoc_mode_exact_pin_metadata_features_and_installed_payload_match() {
    let elisp_form = r##"(progn
         (require 'lisp-mnt)
         (let* ((descriptor (cadr (assq 'adoc-mode package-alist)))
                (package-dir (file-name-directory
                              (getenv "NEOMACS_PACKAGE_SOURCE")))
                (files (directory-files-recursively package-dir ".")))
           (list
            (package-desc-name descriptor)
            (package-version-join (package-desc-version descriptor))
            (package-desc-summary descriptor)
            (package-desc-kind descriptor)
            (package-desc-reqs descriptor)
            (package-desc-extras descriptor)
            adoc-mode-version
            (mapcar #'featurep
                    '(adoc-mode adoc-mode-image adoc-mode-tempo
                      adoc-asciidoctor))
            (with-temp-buffer
              (insert-file-contents (getenv "NEOMACS_PACKAGE_SOURCE"))
              (list (lm-header "version")
                    (lm-header "x-url")
                    (lm-header "package-requires")))
            (mapcar
             (lambda (file)
               (let ((relative (file-relative-name file package-dir)))
                 (if (string-suffix-p ".elc" relative)
                     (list relative 'generated-bytecode)
                   (list
                    relative
                    (with-temp-buffer
                      (set-buffer-multibyte nil)
                      (insert-file-contents-literally file)
                      (secure-hash 'sha256 (current-buffer)))))))
             files))))"##;
    let expect = expect![[
        r#"OK (adoc-mode "20260612.638" "A major-mode for editing AsciiDoc files." nil ((emacs (28 1))) ((:maintainers ("Bozhidar Batsov" . "bozhidar@batsov.dev")) (:authors ("Florian Kaufmann" . "sensorflo@gmail.com")) (:keywords "asciidoc" "text") (:revdesc . "5c1484b89828") (:commit . "5c1484b8982845845ccd0be02629e21f1d5bed81") (:url . "https://github.com/bbatsov/adoc-mode")) "0.9.0" (t t t t) (nil nil "((emacs \"28.1\"))") (("README-elpa" "ec3cedf92ad5a5e69af590cf0ee13c73fb44fb4c864ad9cbe288d7ad00dbac55") ("adoc-asciidoctor.el" "9fba95ea1e32c5f5bdb0b27f069b09b9dea4d262905da1632970929c3ddcec81") ("adoc-asciidoctor.elc" generated-bytecode) ("adoc-mode-autoloads.el" "2361acc40a92ff27bb12413d63ac606fb0c80ac1004af377edaa8ee6a22589bb") ("adoc-mode-image.el" "944205c09d9e711649932fc9429fa54d4d78dbf58c60bb41c8cf119e820db197") ("adoc-mode-image.elc" generated-bytecode) ("adoc-mode-pkg.el" "d440fd56e7607f234be3014b98dc3544f9358c89e0aeb2bd10fc7f4650e6bdef") ("adoc-mode-tempo.el" "02cc5a2fc305289455119bf0bbbe5d4fe22494635326f99a30e0cf1d4ac60c2b") ("adoc-mode-tempo.elc" generated-bytecode) ("adoc-mode.el" "854df6bcf913bf9ec0bc009d94ce2367ceff5160d8fd449cce0d75e078648ae2") ("adoc-mode.elc" generated-bytecode)))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_complete_prefix_callable_surface_matches() {
    let elisp_form = r##"(let (callables)
         (mapatoms
          (lambda (symbol)
            (when (and (string-prefix-p "adoc-" (symbol-name symbol))
                       (fboundp symbol)
                       (let ((file (symbol-file symbol 'defun)))
                         (and file
                              (string-match-p
                               "/adoc-\\(?:mode\\|asciidoctor\\)"
                               file))))
              (push
               (list symbol
                     (commandp symbol)
                     (string-remove-suffix
                      "c"
                      (file-name-nondirectory
                       (symbol-file symbol 'defun))))
               callables))))
         (sort callables
               (lambda (left right)
                 (string-lessp (symbol-name (car left))
                               (symbol-name (car right))))))"##;
    let expect = expect![[
        r#"OK ((adoc--anchor-id-at-point nil "adoc-mode.el") (adoc--antora-current-module nil "adoc-mode.el") (adoc--antora-current-page-targets nil "adoc-mode.el") (adoc--antora-p nil "adoc-mode.el") (adoc--antora-page-fragments nil "adoc-mode.el") (adoc--antora-page-targets nil "adoc-mode.el") (adoc--antora-page-xref-at-point nil "adoc-mode.el") (adoc--antora-references nil "adoc-mode.el") (adoc--antora-resolve-page nil "adoc-mode.el") (adoc--antora-root nil "adoc-mode.el") (adoc--asciidoctor-compile nil "adoc-asciidoctor.el") (adoc--asciidoctor-ensure nil "adoc-asciidoctor.el") (adoc--asciidoctor-render-preview nil "adoc-asciidoctor.el") (adoc--asciidoctor-source-file nil "adoc-asciidoctor.el") (adoc--back-to-heading nil "adoc-mode.el") (adoc--backward-heading nil "adoc-mode.el") (adoc--change-list-item-level nil "adoc-mode.el") (adoc--collect-anchor-ids nil "adoc-mode.el") (adoc--collect-attribute-names nil "adoc-mode.el") (adoc--collect-section-ids nil "adoc-mode.el") (adoc--collect-sections nil "adoc-mode.el") (adoc--completion-attribute-bounds nil "adoc-mode.el") (adoc--completion-include-bounds nil "adoc-mode.el") (adoc--completion-langs nil "adoc-mode.el") (adoc--completion-source-lang-bounds nil "adoc-mode.el") (adoc--completion-token-bounds nil "adoc-mode.el") (adoc--completion-xref-bounds nil "adoc-mode.el") (adoc--completion-xref-target-bounds nil "adoc-mode.el") (adoc--doc-attribute nil "adoc-mode.el") (adoc--explicit-item-on-line-p nil "adoc-mode.el") (adoc--explicit-marker nil "adoc-mode.el") (adoc--explicit-marker-kind nil "adoc-mode.el") (adoc--explicit-marker-value nil "adoc-mode.el") (adoc--face-memq nil "adoc-mode.el") (adoc--flymake-parse-output nil "adoc-asciidoctor.el") (adoc--fontified-as-title-p nil "adoc-mode.el") (adoc--forward-heading nil "adoc-mode.el") (adoc--get-remote-image nil "adoc-mode-image.el") (adoc--goto-id nil "adoc-mode.el") (adoc--heading-descriptor-at-point nil "adoc-mode.el") (adoc--imenu-build-tree nil "adoc-mode.el") (adoc--imenu-heading-level nil "adoc-mode.el") (adoc--imenu-nest nil "adoc-mode.el") (adoc--implicit-numbered-marker nil "adoc-mode.el") (adoc--increment-marker nil "adoc-mode.el") (adoc--inline-link-at-point nil "adoc-mode.el") (adoc--insert-markup nil "adoc-mode.el") (adoc--line-list-level nil "adoc-mode.el") (adoc--list-item-at-point nil "adoc-mode.el") (adoc--list-item-block-end nil "adoc-mode.el") (adoc--move-list-item nil "adoc-mode.el") (adoc--next-sibling-start nil "adoc-mode.el") (adoc--prev-sibling-start nil "adoc-mode.el") (adoc--preview-cleanup nil "adoc-asciidoctor.el") (adoc--preview-display nil "adoc-asciidoctor.el") (adoc--preview-resolve-backend nil "adoc-asciidoctor.el") (adoc--preview-update nil "adoc-asciidoctor.el") (adoc--quote-face nil "adoc-mode.el") (adoc--re-all-titles nil "adoc-mode.el") (adoc--re-xref-to nil "adoc-mode.el") (adoc--resolve-attribute-references nil "adoc-mode-image.el") (adoc--role-face-from-attribute nil "adoc-mode.el") (adoc--same-list-p nil "adoc-mode.el") (adoc--section-definitions nil "adoc-mode.el") (adoc--section-id nil "adoc-mode.el") (adoc--section-id-params nil "adoc-mode.el") (adoc--section-position nil "adoc-mode.el") (adoc--title-bounds nil "adoc-mode.el") (adoc--unordered-marker nil "adoc-mode.el") (adoc--xref-backend nil "adoc-mode.el") (adoc--xref-collect nil "adoc-mode.el") (adoc-adjust-title-del t "adoc-mode.el") (adoc-asciidoctor-menu t "adoc-asciidoctor.el") (adoc-backward-same-level t "adoc-mode.el") (adoc-bounds-of-image-link-at nil "adoc-mode-image.el") (adoc-calc t "adoc-mode.el") (adoc-completion-at-point nil "adoc-mode.el") (adoc-create-image-overlay nil "adoc-mode-image.el") (adoc-cycle t "adoc-mode.el") (adoc-cycle-buffer t "adoc-mode.el") (adoc-demote t "adoc-mode.el") (adoc-demote-title t "adoc-mode.el") (adoc-display-image-at t "adoc-mode-image.el") (adoc-display-images t "adoc-mode-image.el") (adoc-entity-to-string nil "adoc-mode.el") (adoc-export-docbook t "adoc-asciidoctor.el") (adoc-export-epub t "adoc-asciidoctor.el") (adoc-export-html t "adoc-asciidoctor.el") (adoc-export-pdf t "adoc-asciidoctor.el") (adoc-face-for-attribute nil "adoc-mode.el") (adoc-facespec-subscript nil "adoc-mode.el") (adoc-facespec-superscript nil "adoc-mode.el") (adoc-fill-nobreak-p nil "adoc-mode.el") (adoc-fill-paragraph nil "adoc-mode.el") (adoc-flf-first-whites-fixed-width nil "adoc-mode.el") (adoc-flf-meta-face-cleanup nil "adoc-mode.el") (adoc-flymake nil "adoc-asciidoctor.el") (adoc-flyspell-p nil "adoc-mode.el") (adoc-follow-thing-at-point t "adoc-mode.el") (adoc-font-lock-extend-after-change-region nil "adoc-mode.el") (adoc-font-lock-extend-region nil "adoc-mode.el") (adoc-font-lock-mark-block-function nil "adoc-mode.el") (adoc-fontify-code-block-natively nil "adoc-mode.el") (adoc-fontify-code-blocks nil "adoc-mode.el") (adoc-forward-same-level t "adoc-mode.el") (adoc-forward-xref nil "adoc-mode.el") (adoc-get-font-lock-keywords nil "adoc-mode.el") (adoc-get-lang-mode nil "adoc-mode.el") (adoc-goto-ref-label t "adoc-mode.el") (adoc-image-link-at nil "adoc-mode-image.el") (adoc-image-link-begin nil "adoc-mode-image.el") (adoc-image-link-begin--inliner nil "adoc-mode-image.el") (adoc-image-link-begin-attributes nil "adoc-mode-image.el") (adoc-image-link-begin-attributes--inliner nil "adoc-mode-image.el") (adoc-image-link-begin-uri nil "adoc-mode-image.el") (adoc-image-link-begin-uri--inliner nil "adoc-mode-image.el") (adoc-image-link-end nil "adoc-mode-image.el") (adoc-image-link-end--inliner nil "adoc-mode-image.el") (adoc-image-link-end-attributes nil "adoc-mode-image.el") (adoc-image-link-end-attributes--inliner nil "adoc-mode-image.el") (adoc-image-link-end-uri nil "adoc-mode-image.el") (adoc-image-link-end-uri--inliner nil "adoc-mode-image.el") (adoc-image-link-p nil "adoc-mode-image.el") (adoc-image-link-p--inliner nil "adoc-mode-image.el") (adoc-image-link-uri nil "adoc-mode-image.el") (adoc-image-link-uri--inliner nil "adoc-mode-image.el") (adoc-image-overlay-at nil "adoc-mode-image.el") (adoc-image-overlays nil "adoc-mode-image.el") (adoc-imenu-create-index nil "adoc-mode.el") (adoc-imenu-create-nested-index nil "adoc-mode.el") (adoc-insert-bold t "adoc-mode.el") (adoc-insert-highlight t "adoc-mode.el") (adoc-insert-indented nil "adoc-mode.el") (adoc-insert-italic t "adoc-mode.el") (adoc-insert-link t "adoc-mode.el") (adoc-insert-list-item t "adoc-mode.el") (adoc-insert-monospace t "adoc-mode.el") (adoc-insert-subscript t "adoc-mode.el") (adoc-insert-superscript t "adoc-mode.el") (adoc-kw-admonition-paragraph nil "adoc-mode.el") (adoc-kw-block-title nil "adoc-mode.el") (adoc-kw-checkbox nil "adoc-mode.el") (adoc-kw-csv-dsv-table nil "adoc-mode.el") (adoc-kw-delimited-block nil "adoc-mode.el") (adoc-kw-delimiter-line-fallback nil "adoc-mode.el") (adoc-kw-escaped-formatting nil "adoc-mode.el") (adoc-kw-first-whites-fixed-width nil "adoc-mode.el") (adoc-kw-inline-macro nil "adoc-mode.el") (adoc-kw-inline-macro-urls-attribute-list nil "adoc-mode.el") (adoc-kw-inline-macro-urls-no-attribute-list nil "adoc-mode.el") (adoc-kw-inline-passthrough nil "adoc-mode.el") (adoc-kw-list-continuation nil "adoc-mode.el") (adoc-kw-llisti nil "adoc-mode.el") (adoc-kw-one-line-title nil "adoc-mode.el") (adoc-kw-oulisti nil "adoc-mode.el") (adoc-kw-quote nil "adoc-mode.el") (adoc-kw-replacement nil "adoc-mode.el") (adoc-kw-standalone-urls nil "adoc-mode.el") (adoc-kw-two-line-title nil "adoc-mode.el") (adoc-kw-verbatim-paragraph-sequence nil "adoc-mode.el") (adoc-kwf-attribute-list nil "adoc-mode.el") (adoc-kwf-search nil "adoc-mode.el") (adoc-kwf-std nil "adoc-mode.el") (adoc-live-preview-mode t "adoc-asciidoctor.el") (adoc-make-one-line-title nil "adoc-mode.el") (adoc-make-title nil "adoc-mode.el") (adoc-make-two-line-title nil "adoc-mode.el") (adoc-make-two-line-title-underline nil "adoc-mode.el") (adoc-make-unichar-alist nil "adoc-mode.el") (adoc-make-uolisti nil "adoc-mode.el") (adoc-mode t "adoc-mode.el") (adoc-mode-menu t "adoc-mode.el") (adoc-mode-version t "adoc-mode.el") (adoc-modify-title nil "adoc-mode.el") (adoc-move-list-item-down t "adoc-mode.el") (adoc-move-list-item-up t "adoc-mode.el") (adoc-next-visible-heading t "adoc-mode.el") (adoc-preview t "adoc-asciidoctor.el") (adoc-previous-visible-heading t "adoc-mode.el") (adoc-promote t "adoc-mode.el") (adoc-promote-title t "adoc-mode.el") (adoc-re-anchor nil "adoc-mode.el") (adoc-re-aor nil "adoc-mode.el") (adoc-re-attribute-entry nil "adoc-mode.el") (adoc-re-block-macro nil "adoc-mode.el") (adoc-re-block-title nil "adoc-mode.el") (adoc-re-cell-specifier nil "adoc-mode.el") (adoc-re-constrained-quote nil "adoc-mode.el") (adoc-re-content nil "adoc-mode.el") (adoc-re-delimited-block nil "adoc-mode.el") (adoc-re-delimited-block-line nil "adoc-mode.el") (adoc-re-id nil "adoc-mode.el") (adoc-re-inline-macro nil "adoc-mode.el") (adoc-re-llisti nil "adoc-mode.el") (adoc-re-one-line-title nil "adoc-mode.el") (adoc-re-oulisti nil "adoc-mode.el") (adoc-re-paragraph-separate nil "adoc-mode.el") (adoc-re-paragraph-start nil "adoc-mode.el") (adoc-re-precond nil "adoc-mode.el") (adoc-re-quote nil "adoc-mode.el") (adoc-re-quote-precondition nil "adoc-mode.el") (adoc-re-ror nil "adoc-mode.el") (adoc-re-two-line-title nil "adoc-mode.el") (adoc-re-two-line-title-underline nil "adoc-mode.el") (adoc-re-unconstrained-quote nil "adoc-mode.el") (adoc-re-verbatim-paragraph-sequence nil "adoc-mode.el") (adoc-re-xref nil "adoc-mode.el") (adoc-remove-image-overlay-at t "adoc-mode-image.el") (adoc-remove-images t "adoc-mode-image.el") (adoc-renumber-list t "adoc-mode.el") (adoc-search-forward-code-block nil "adoc-mode.el") (adoc-show-version t "adoc-mode.el") (adoc-template-str-title nil "adoc-mode-tempo.el") (adoc-tempo-define nil "adoc-mode-tempo.el") (adoc-tempo-handler nil "adoc-mode-tempo.el") (adoc-tempo-insert-template-fix nil "adoc-mode-tempo.el") (adoc-tempo-on-region nil "adoc-mode-tempo.el") (adoc-title-descriptor nil "adoc-mode.el") (adoc-toggle-images t "adoc-mode-image.el") (adoc-toggle-title-type t "adoc-mode.el") (adoc-unfontify-region-function nil "adoc-mode.el") (adoc-unichar-by-name nil "adoc-mode.el") (adoc-up-heading t "adoc-mode.el") (adoc-update-title-faces nil "adoc-mode.el") (adoc-with-point-at-event nil "adoc-mode-image.el") (adoc-xref-id-at-point nil "adoc-mode.el"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_source_callable_arglist_contract_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (copy-tree
                  (help-function-arglist symbol t))))
         '(adoc-flymake
           adoc-image-link-begin
           adoc-image-link-begin--inliner))"##;
    let expect = expect![[
        r#"OK ((adoc-flymake (report-fn &rest _args)) (adoc-image-link-begin (x)) (adoc-image-link-begin--inliner (inline--form x)))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_complete_variable_face_hook_and_map_surface_matches() {
    let elisp_form = r##"(let (variables faces)
         (mapatoms
          (lambda (symbol)
            (when (and (string-prefix-p "adoc-" (symbol-name symbol))
                       (boundp symbol)
                       (let ((file (symbol-file symbol 'defvar)))
                         (and file
                              (string-match-p
                               "/adoc-\\(?:mode\\|asciidoctor\\)"
                               file))))
              (push
               (list symbol
                     (and (custom-variable-p symbol) t)
                     (local-variable-if-set-p symbol)
                     (string-remove-suffix
                      "c"
                      (file-name-nondirectory
                       (symbol-file symbol 'defvar))))
               variables))
            (when (and (string-prefix-p "adoc-" (symbol-name symbol))
                       (facep symbol))
              (push symbol faces))))
         (list
          (sort variables
                (lambda (left right)
                  (string-lessp (symbol-name (car left))
                                (symbol-name (car right)))))
          (sort faces
                (lambda (left right)
                  (string-lessp (symbol-name left)
                                (symbol-name right))))
          (keymapp adoc-mode-map)
          (keymapp adoc-image-link-map)
          (keymapp adoc-image-overlay-map)
          (memq #'adoc--preview-update after-save-hook)
          (memq #'adoc--preview-cleanup kill-buffer-hook)))"##;
    let expect = expect![[
        r#"OK (((adoc--completion-common-langs nil nil "adoc-mode.el") (adoc--flymake-diagnostic-re nil nil "adoc-asciidoctor.el") (adoc--flymake-proc nil t "adoc-asciidoctor.el") (adoc--preview-file nil t "adoc-asciidoctor.el") (adoc--remote-image-cache nil nil "adoc-mode-image.el") (adoc--table-cell-separator nil nil "adoc-mode.el") (adoc--title-faces nil nil "adoc-mode.el") (adoc-asciidoctor-command t nil "adoc-asciidoctor.el") (adoc-asciidoctor-extra-args t nil "adoc-asciidoctor.el") (adoc-attribute-face-alist nil nil "adoc-mode.el") (adoc-code-block-begin-regexp nil nil "adoc-mode.el") (adoc-code-lang-modes t nil "adoc-mode.el") (adoc-default-title-sub-type t nil "adoc-mode.el") (adoc-default-title-type t nil "adoc-mode.el") (adoc-delimited-block-del t nil "adoc-mode.el") (adoc-display-images t nil "adoc-mode.el") (adoc-display-remote-images t nil "adoc-mode-image.el") (adoc-enable-two-line-title t nil "adoc-mode.el") (adoc-font-lock-extend-after-change-max t nil "adoc-mode.el") (adoc-font-lock-keywords nil nil "adoc-mode.el") (adoc-fontify-code-block-default-mode t nil "adoc-mode.el") (adoc-fontify-code-blocks-natively t nil "adoc-mode.el") (adoc-help-anchor nil nil "adoc-mode.el") (adoc-help-asciimath nil nil "adoc-mode.el") (adoc-help-attributed nil nil "adoc-mode.el") (adoc-help-bold nil nil "adoc-mode.el") (adoc-help-bulleted-list nil nil "adoc-mode.el") (adoc-help-comment nil nil "adoc-mode.el") (adoc-help-constrained-quotes nil nil "adoc-mode.el") (adoc-help-delimited-block nil nil "adoc-mode.el") (adoc-help-delimited-block-comment nil nil "adoc-mode.el") (adoc-help-delimited-block-example nil nil "adoc-mode.el") (adoc-help-delimited-block-listing nil nil "adoc-mode.el") (adoc-help-delimited-block-literal nil nil "adoc-mode.el") (adoc-help-delimited-block-open-block nil nil "adoc-mode.el") (adoc-help-delimited-block-passthrouh nil nil "adoc-mode.el") (adoc-help-delimited-block-quote nil nil "adoc-mode.el") (adoc-help-delimited-block-sidebar nil nil "adoc-mode.el") (adoc-help-double-quote nil nil "adoc-mode.el") (adoc-help-emphasis nil nil "adoc-mode.el") (adoc-help-entity-reference nil nil "adoc-mode.el") (adoc-help-latexmath nil nil "adoc-mode.el") (adoc-help-line-break nil nil "adoc-mode.el") (adoc-help-line-through nil nil "adoc-mode.el") (adoc-help-list nil nil "adoc-mode.el") (adoc-help-list-item-continuation nil nil "adoc-mode.el") (adoc-help-literal-paragraph nil nil "adoc-mode.el") (adoc-help-local-doc-link nil nil "adoc-mode.el") (adoc-help-macros nil nil "adoc-mode.el") (adoc-help-monospace nil nil "adoc-mode.el") (adoc-help-monospace-literal nil nil "adoc-mode.el") (adoc-help-nobreak nil nil "adoc-mode.el") (adoc-help-nowrap nil nil "adoc-mode.el") (adoc-help-overline nil nil "adoc-mode.el") (adoc-help-page-break nil nil "adoc-mode.el") (adoc-help-pass nil nil "adoc-mode.el") (adoc-help-pass-$$ nil nil "adoc-mode.el") (adoc-help-pass-+++ nil nil "adoc-mode.el") (adoc-help-passthrough-macros nil nil "adoc-mode.el") (adoc-help-pre-wrap nil nil "adoc-mode.el") (adoc-help-ruler-line nil nil "adoc-mode.el") (adoc-help-single-quote nil nil "adoc-mode.el") (adoc-help-table nil nil "adoc-mode.el") (adoc-help-unconstrained-quotes nil nil "adoc-mode.el") (adoc-help-underline nil nil "adoc-mode.el") (adoc-help-url nil nil "adoc-mode.el") (adoc-help-xref nil nil "adoc-mode.el") (adoc-image-link-map nil nil "adoc-mode-image.el") (adoc-image-link-menu nil nil "adoc-mode-image.el") (adoc-image-menu nil nil "adoc-mode-image.el") (adoc-image-overlay-functions nil nil "adoc-mode-image.el") (adoc-image-overlay-map nil nil "adoc-mode-image.el") (adoc-image-overlay-menu nil nil "adoc-mode-image.el") (adoc-image-overlays nil t "adoc-mode-image.el") (adoc-imenu-create-index-function t nil "adoc-mode.el") (adoc-include-title-properties nil nil "adoc-mode.el") (adoc-insert-replacement t nil "adoc-mode.el") (adoc-intrinsic-attributes nil nil "adoc-mode.el") (adoc-language-info-properties nil nil "adoc-mode.el") (adoc-language-keyword-properties nil nil "adoc-mode.el") (adoc-link-keymap nil nil "adoc-mode.el") (adoc-live-preview-mode nil t "adoc-asciidoctor.el") (adoc-live-preview-mode-hook t nil "adoc-asciidoctor.el") (adoc-markup-properties nil nil "adoc-mode.el") (adoc-max-image-size t nil "adoc-mode.el") (adoc-mode-abbrev-table nil nil "adoc-mode.el") (adoc-mode-hook nil nil "adoc-mode.el") (adoc-mode-map nil nil "adoc-mode.el") (adoc-mode-menu nil nil "adoc-mode.el") (adoc-mode-syntax-table nil nil "adoc-mode.el") (adoc-mode-version nil nil "adoc-mode.el") (adoc-preview-backend t nil "adoc-asciidoctor.el") (adoc-re-attribute-list-elt nil nil "adoc-mode.el") (adoc-re-escaped-formatting nil nil "adoc-mode.el") (adoc-re-image nil nil "adoc-mode-image.el") (adoc-remote-image-protocols t nil "adoc-mode-image.el") (adoc-role-face-alist t nil "adoc-mode.el") (adoc-script-raise t nil "adoc-mode.el") (adoc-section-id-style t nil "adoc-mode.el") (adoc-style-map nil nil "adoc-mode.el") (adoc-summarize-re-llisti nil nil "adoc-mode.el") (adoc-summarize-re-olisti nil nil "adoc-mode.el") (adoc-summarize-re-uolisti nil nil "adoc-mode.el") (adoc-tempo-frwk t nil "adoc-mode-tempo.el") (adoc-title-max-level nil nil "adoc-mode.el") (adoc-title-scaling t nil "adoc-mode.el") (adoc-title-scaling-values t nil "adoc-mode.el") (adoc-title-style t nil "adoc-mode.el") (adoc-two-line-title-del t nil "adoc-mode.el") (adoc-unichar-alist nil nil "adoc-mode.el") (adoc-unichar-name-resolver t nil "adoc-mode.el") (adoc-uolist-max-level nil nil "adoc-mode.el")) (adoc-align-face adoc-anchor-face adoc-attribute-face adoc-blockquote-face adoc-bold-face adoc-checkbox-face adoc-code-face adoc-command-face adoc-comment-face adoc-complex-replacement-face adoc-emphasis-face adoc-footnote-marker-face adoc-footnote-text-face adoc-gen-face adoc-highlight-face adoc-internal-reference-face adoc-language-info-face adoc-language-keyword-face adoc-link-mouse-face adoc-link-title-face adoc-list-face adoc-markup-face adoc-meta-face adoc-meta-hide-face adoc-metadata-key-face adoc-metadata-value-face adoc-native-code-face adoc-overline-face adoc-passthrough-face adoc-preprocessor-face adoc-reference-face adoc-replacement-face adoc-secondary-text-face adoc-strike-through-face adoc-subscript-face adoc-superscript-face adoc-table-face adoc-title-0-face adoc-title-1-face adoc-title-2-face adoc-title-3-face adoc-title-4-face adoc-title-5-face adoc-title-face adoc-typewriter-face adoc-underline-face adoc-url-face adoc-value-face adoc-verbatim-face adoc-warning-face) t t t nil nil)"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_autoload_and_auto_mode_contract_matches() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list symbol
                  (autoloadp (symbol-function symbol))
                  (nth 1 (symbol-function symbol))
                  (commandp symbol)))
          '(adoc-mode
            adoc-export-html
            adoc-export-docbook
            adoc-export-pdf
            adoc-export-epub
            adoc-preview
            adoc-live-preview-mode
            adoc-asciidoctor-menu))
         (cdr (assoc "\\.a\\(?:scii\\)?doc\\'" auto-mode-alist)))"##;
    let expect = expect![[
        r#"OK (((adoc-mode t "adoc-mode" t) (adoc-export-html t "adoc-asciidoctor" t) (adoc-export-docbook t "adoc-asciidoctor" t) (adoc-export-pdf t "adoc-asciidoctor" t) (adoc-export-epub t "adoc-asciidoctor" t) (adoc-preview t "adoc-asciidoctor" t) (adoc-live-preview-mode t "adoc-asciidoctor" t) (adoc-asciidoctor-menu t "adoc-asciidoctor" t)) adoc-mode)"#
    ]];
    assert_adoc_mode_autoload_parity(elisp_form, expect);
}
