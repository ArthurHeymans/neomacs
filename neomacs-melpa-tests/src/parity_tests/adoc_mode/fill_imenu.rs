use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_fill_preserves_hard_breaks_joins_soft_lines_and_indents_continuations() {
    let elisp_form = r##"(cl-labels
         ((fill
           (text column)
           (with-temp-buffer
             (insert text)
             (adoc-mode)
             (setq-local fill-column column)
             (goto-char (point-min))
             (adoc-fill-paragraph)
             (list
              (buffer-string)
              (let (hard)
                (dotimes (index (buffer-size))
                  (when (get-text-property (1+ index) 'hard)
                    (push (1+ index) hard)))
                (nreverse hard))))))
       (mapcar
        (lambda (case) (apply #'fill case))
        '(("first line +\nsecond line\n" 72)
          ("first soft line\nsecond soft line\n" 72)
          ("* a list item with several words\n+\ncontinuation text with words\n" 24)
          ("stale break +\nnext line\n" 12))))"##;
    let expect = expect![[
        r#"OK ((#("first line +\nsecond line\n" 12 13 (hard t)) (13)) ("first soft line second soft line\n" nil) ("* a list item with\n  several words\n+\ncontinuation text with words\n" nil) (#("stale break\n+\nnext line\n" 13 14 (hard t)) (14)))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_flat_and_nested_imenu_indexes_cover_heading_hierarchy_and_setext_toggle() {
    let elisp_form = r##"(cl-labels
         ((normalise
           (tree)
           (mapcar
            (lambda (item)
              (cond
               ((or (markerp (cdr item))
                    (integerp (cdr item)))
                (cons (and (car item)
                           (substring-no-properties (car item)))
                      (if (markerp (cdr item))
                          (marker-position (cdr item))
                        (cdr item))))
               ((listp (cdr item))
                (cons (and (car item)
                           (substring-no-properties (car item)))
                      (normalise (cdr item))))
               (t item)))
            tree)))
       (with-temp-buffer
         (insert
          "= Document\n\n"
          "intro\n\n"
          "== First\n"
          "=== Child A\n"
          "=== Child B\n"
          "==== Grandchild\n"
          "== Second\n"
          "Setext\n------\n")
         (adoc-mode)
         (list
          (normalise (adoc-imenu-create-index))
          (normalise (adoc-imenu-create-nested-index))
          (let ((adoc-enable-two-line-title t))
            (font-lock-flush)
            (font-lock-ensure)
            (normalise (adoc-imenu-create-index))))))"##;
    let expect = expect![[
        r#"OK ((("Document" . 1) ("First" . 20) ("Child A" . 29) ("Child B" . 41) ("Grandchild" . 53) ("Second" . 69)) (("Document" (nil . 1) ("First" (nil . 20) ("Child A" . 29) ("Child B" (nil . 41) ("Grandchild" . 53)))) ("Second" . 69)) (("Document" . 1) ("First" . 20) ("Child A" . 29) ("Child B" . 41) ("Grandchild" . 53) ("Second" . 69) ("Setext" . 79)))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_mode_initialization_syntax_maps_fill_flyspell_and_local_hooks_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert "= Heading\n\nplain *bold* `code`\n")
         (adoc-mode)
         (goto-char (point-min))
         (let ((title-no-break (adoc-fill-nobreak-p)))
           (forward-line 2)
           (search-forward "plain")
           (font-lock-ensure)
           (list
            major-mode
            mode-name
            title-no-break
            (eq fill-paragraph-function #'adoc-fill-paragraph)
            (string-match-p paragraph-start "")
            (string-match-p paragraph-separate "")
            (eq imenu-create-index-function
                #'adoc-imenu-create-nested-index)
            (and (string-match-p outline-regexp "== Heading") t)
            (functionp outline-level)
            (and (memq #'adoc--xref-backend
                       xref-backend-functions) t)
            (and (memq #'adoc-font-lock-extend-region
                       font-lock-extend-region-functions) t)
            (eq #'adoc-font-lock-mark-block-function
                font-lock-mark-block-function)
            (key-binding (kbd "C-c C-n"))
            (key-binding (kbd "C-c C-p"))
            (key-binding (kbd "C-c C-c"))
            (adoc-flyspell-p)
            (char-syntax ?_)
            (char-syntax ?-))))"##;
    let expect = expect![[
        r#"OK (adoc-mode "adoc" t t 0 0 t t t t t t adoc-next-visible-heading adoc-previous-visible-heading adoc-asciidoctor-menu t 46 95)"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}
