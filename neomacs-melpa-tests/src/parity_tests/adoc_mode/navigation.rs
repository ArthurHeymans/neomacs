use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_heading_navigation_covers_next_previous_same_level_parent_and_errors() {
    let elisp_form = r##"(cl-labels
         ((heading
           ()
           (buffer-substring-no-properties
            (line-beginning-position) (line-end-position)))
          (goto-heading
           (text)
           (goto-char (point-min))
           (search-forward text)
           (beginning-of-line))
          (step
           (function argument)
           (condition-case error
               (progn (funcall function argument) (heading))
             (error (list (car error) (cadr error) (heading))))))
       (with-temp-buffer
         (insert
          "= Doc Title\n\nintro\n\n"
          "== Section A\n\ntext a\n\n"
          "=== Sub A1\n\ntext\n\n"
          "=== Sub A2\n\nmore\n\n"
          "== Section B\n\nend\n")
         (adoc-mode)
         (font-lock-ensure)
         (let (results)
           (goto-char (point-min))
           (push (step #'adoc-next-visible-heading 1) results)
           (push (step #'adoc-next-visible-heading 1) results)
           (push (step #'adoc-next-visible-heading 2) results)
           (push (step #'adoc-next-visible-heading 1) results)
           (goto-heading "Section B")
           (push (step #'adoc-previous-visible-heading 1) results)
           (push (step #'adoc-previous-visible-heading 2) results)
           (goto-heading "Section A")
           (push (step #'adoc-forward-same-level 1) results)
           (goto-heading "Sub A2")
           (push (step #'adoc-backward-same-level 1) results)
           (push (step #'adoc-up-heading 1) results)
           (goto-heading "Sub A2")
           (push (step #'adoc-up-heading 2) results)
           (nreverse results))))"##;
    let expect = expect![[
        r#"OK ("== Section A" "=== Sub A1" "== Section B" (user-error "No following section title" "== Section B") "=== Sub A2" "== Section A" "== Section B" "=== Sub A1" "== Section A" "= Doc Title")"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_heading_navigation_handles_setext_macro_titles_and_verbatim_blocks() {
    let elisp_form = r##"(cl-labels
         ((collect
           (text setext)
           (with-temp-buffer
             (insert text)
             (adoc-mode)
             (setq-local adoc-enable-two-line-title setext)
             (font-lock-flush)
             (font-lock-ensure)
             (goto-char (point-min))
             (let (headings done)
               (while (not done)
                 (condition-case nil
                     (progn
                       (adoc-next-visible-heading 1)
                       (push (buffer-substring-no-properties
                              (line-beginning-position)
                              (line-end-position))
                             headings))
                   (user-error (setq done t))))
               (nreverse headings)))))
       (list
        (collect
         "Doc Title\n=========\n\nSection A\n---------\n\nSection B\n---------\n" t)
        (collect
         "Doc Title\n=========\n\nSection A\n---------\n\nSection B\n---------\n" nil)
        (collect
         "= Doc\n\n== Real A\n\n"
         nil)
        (collect
         (concat
          "= Doc\n\n"
          "== Real A\n\n"
          "----\n== fake listing heading\n----\n\n"
          "====\n== fake example heading\n====\n\n"
          "== https://example.test[Home]\n\n"
          "== image:logo.png[Logo]\n\n"
          "== Real B\n")
         nil)))"##;
    let expect = expect![[
        r#"OK (("Section A" "Section B") nil ("== Real A") ("== Real A" "== https://example.test[Home]" "== image:logo.png[Logo]" "== Real B"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_anchor_xref_and_inline_link_resolution_cover_syntax_and_prefix_collisions() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "[[foo]]\n"
          "= Document\n\n"
          "See <<foo>> and <<foo ,Caption>>.\n"
          "Also xref:foo[here] and https://example.test[Site].\n"
          "A link:other.adoc[document].\n"
          "[[foobar]]\n"
          "<<foobar>> xref:foobar[x]\n"
          "[style#short]\n")
         (adoc-mode)
         (font-lock-ensure)
         (let ((completion
                (sort (copy-sequence
                       (xref-backend-identifier-completion-table 'adoc))
                      #'string<))
               observations)
           (dolist (needle
                    '("<<foo>>" "<<foo ,Caption>>" "xref:foo[here]"
                      "https://example.test[Site]"
                      "link:other.adoc[document]"))
             (goto-char (point-min))
             (search-forward needle)
             (push
              (list
               needle
               (adoc-xref-id-at-point)
               (adoc--inline-link-at-point)
               (get-text-property (match-beginning 0) 'keymap)
               (get-text-property (match-beginning 0) 'mouse-face))
              observations))
           (list
            (xref-find-backend)
            completion
            (mapcar #'xref-item-summary
                    (xref-backend-definitions 'adoc "foo"))
            (mapcar #'xref-item-summary
                    (xref-backend-references 'adoc "foo"))
            (length (xref-backend-references 'adoc "foobar"))
            (progn (adoc-goto-ref-label "short")
                   (line-number-at-pos))
            (nreverse observations))))"##;
    let expect = expect![[
        r#"OK (adoc ("foo" "foobar" "short") ("[[foo]]") ("See <<foo>> and <<foo ,Caption>>." "See <<foo>> and <<foo ,Caption>>." "Also xref:foo[here] and https://example.test[Site].") 2 9 (("<<foo>>" "foo" nil adoc-link-keymap adoc-link-mouse-face) ("<<foo ,Caption>>" "foo" nil adoc-link-keymap adoc-link-mouse-face) ("xref:foo[here]" "foo" nil adoc-link-keymap adoc-link-mouse-face) ("https://example.test[Site]" nil "https://example.test" adoc-link-keymap adoc-link-mouse-face) ("link:other.adoc[document]" nil "other.adoc" adoc-link-keymap adoc-link-mouse-face)))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_outline_cycle_and_link_keymap_contract_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert "= Top\n\n== A\n\nbody a\n\n== B\n\nbody b\n")
         (adoc-mode)
         (font-lock-ensure)
         (goto-char (point-min))
         (search-forward "== A")
         (beginning-of-line)
         (let ((body (save-excursion
                       (search-forward "body a")
                       (point)))
               states)
           (push (outline-invisible-p body) states)
           (adoc-cycle)
           (push (outline-invisible-p body) states)
           (adoc-cycle)
           (push (outline-invisible-p body) states)
           (list
            (nreverse states)
            (lookup-key adoc-link-keymap [mouse-2])
            (lookup-key adoc-link-keymap [follow-link])
            (face-attribute 'adoc-link-mouse-face :underline nil t))))"##;
    let expect = expect![[r#"OK ((nil t nil) adoc-follow-thing-at-point mouse-face t)"#]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_outline_cycle_off_heading_prefix_and_global_state_machine_match() {
    let elisp_form = r##"(cl-labels
         ((make-document
           ()
           (insert
            "= Top\n\n== A\n\nbody a\n\n"
            "=== Child\n\nchild body\n\n"
            "== B\n\nbody b\n")
           (adoc-mode)
           (font-lock-ensure)))
       (list
        (with-temp-buffer
          (make-document)
          (goto-char (point-min))
          (search-forward "body a")
          (let ((body (point)))
            (adoc-cycle)
            (outline-invisible-p body)))
        (with-temp-buffer
          (make-document)
          (goto-char (point-min))
          (let ((body
                 (save-excursion
                   (search-forward "body a")
                   (point)))
                states)
            (push (outline-invisible-p body) states)
            (adoc-cycle '(4))
            (push (outline-invisible-p body) states)
            (adoc-cycle-buffer)
            (push (outline-invisible-p body) states)
            (adoc-cycle-buffer)
            (push (outline-invisible-p body) states)
            (nreverse states)))))"##;
    let expect = expect![[r#"OK (nil (nil t t nil))"#]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_section_auto_ids_drive_xref_definitions_goto_and_completion() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "[[explicit]]\n"
          "= Doc\n\n"
          "== Clojure CLI Setup\n\n"
          "text\n\n"
          "== Second Section\n\n"
          "more\n")
         (adoc-mode)
         (let ((definitions
                (mapcar
                 (lambda (definition)
                   (substring-no-properties
                    (xref-item-summary definition)))
                 (xref-backend-definitions
                  'adoc "_clojure_cli_setup")))
               (completion
                (sort
                 (copy-sequence
                  (xref-backend-identifier-completion-table
                   'adoc))
                 #'string<)))
           (goto-char (point-min))
           (adoc-goto-ref-label "_second_section")
           (list
            definitions
            (line-number-at-pos)
            completion)))"##;
    let expect = expect![[
        r#"OK (("Clojure CLI Setup") 8 ("_clojure_cli_setup" "_second_section" "explicit"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}
