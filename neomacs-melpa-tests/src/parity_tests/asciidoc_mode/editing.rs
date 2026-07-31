use expect_test::expect;

use super::assert_asciidoc_mode_batch;

#[test]
fn editing_public_surface_batch() {
    assert_asciidoc_mode_batch(&[
        (
            "heading_commands_edit_nested_document_structure_and_preserve_point_semantics",
            r##"(with-temp-buffer
  (insert
   "= Document\n\n"
   "== Architecture\n\n"
   "=== Parser Pipeline\n\n"
   "====== Deep Limit\n\n"
   "Ordinary prose.\n")
  (asciidoc-mode)
  (let (results)
    (dolist
        (case
         '(("Architecture" . asciidoc-demote-heading)
           ("Parser Pipeline" . asciidoc-promote-heading)))
      (goto-char (point-min))
      (search-forward (car case))
      (let ((before (point)))
        (funcall (cdr case))
        (push
         (list
          (cdr case)
          before
          (point)
          (buffer-substring-no-properties
           (line-beginning-position)
           (line-end-position)))
         results)))
    (list
     (nreverse results)
     (buffer-string)
     (mapcar
      (lambda (case)
        (goto-char (point-min))
        (search-forward (car case))
        (condition-case error
            (progn
              (funcall (cdr case))
              'no-error)
          (error
           (list
            (car error)
            (cdr error)
            (buffer-substring-no-properties
             (line-beginning-position)
             (line-end-position))))))
      '(("Document" . asciidoc-promote-heading)
        ("Deep Limit" . asciidoc-demote-heading)
        ("Ordinary prose" . asciidoc-promote-heading))))))"##,
            true,
            expect![[
        r#"OK (((asciidoc-demote-heading 28 29 "=== Architecture") (asciidoc-promote-heading 50 49 "== Parser Pipeline")) "= Document\n\n=== Architecture\n\n== Parser Pipeline\n\n====== Deep Limit\n\nOrdinary prose.\n" ((user-error ("Already at the topmost heading level") "= Document") (user-error ("Already at the deepest heading level") "====== Deep Limit") (user-error ("Point is not on a heading") "Ordinary prose.")))"#
    ]],
        ),
        (
            "comment_and_uncomment_region_round_trip_realistic_asciidoc_without_touching_urls",
            r##"(with-temp-buffer
  (insert
   "= Operations\n\n"
   "Visit https://example.com/a/b for deployment details.\n"
   "Run the documented rollback when required.\n")
  (asciidoc-mode)
  (let ((original (buffer-string)))
    (goto-char (point-min))
    (forward-line 2)
    (let ((beg (point)))
      (forward-line 2)
      (comment-region beg (point))
      (let ((commented (buffer-string)))
        (uncomment-region beg (point-max))
        (list
         comment-start
         comment-start-skip
         original
         commented
         (buffer-string)
         (equal original (buffer-string))
         (string-match-p
          "^// Visit https://"
          commented)
         (string-match-p
          "https://example.com/a/b"
          (buffer-string)))))))"##,
            true,
            expect![[
        r#"OK ("// " "^//+\\s-*" "= Operations\n\nVisit https://example.com/a/b for deployment details.\nRun the documented rollback when required.\n" "= Operations\n\n// Visit https://example.com/a/b for deployment details.\n// Run the documented rollback when required.\n" "= Operations\n\nVisit https://example.com/a/b for deployment details.\nRun the documented rollback when required.\n" t 14 20)"#
    ]],
        ),
        (
            "paragraph_filling_wraps_prose_around_a_url_without_inventing_comment_prefixes",
            r##"(with-temp-buffer
  (insert
   "Visit https://example.com/a/b for more details about the practical "
   "deployment process and its rollback procedure.\n\n"
   "// A genuine comment remains a comment.\n")
  (asciidoc-mode)
  (setq-local fill-column 42)
  (goto-char (point-min))
  (fill-paragraph)
  (list
   (buffer-string)
   (count-lines (point-min) (point-max))
   (string-match-p
    "^[ \t]*//"
    (buffer-substring-no-properties
     (point-min)
     (save-excursion
       (goto-char (point-min))
       (search-forward "\n\n")
       (point))))
   (save-excursion
     (goto-char (point-max))
     (forward-line -1)
     (nth 4 (syntax-ppss)))))"##,
            true,
            expect![[
        r#"OK ("Visit https://example.com/a/b for more\ndetails about the practical deployment\nprocess and its rollback procedure.\n\n// A genuine comment remains a comment.\n" 5 nil nil)"#
    ]],
        ),
        (
            "inherited_text_indentation_normalizes_explicit_asciidoc_list_and_source_layout_exactly",
            r##"(with-temp-buffer
  (insert
   "* Parent item\n"
   "  continuation aligned by the author\n"
   "** Nested item\n\n"
   "[source,emacs-lisp]\n"
   "----\n"
   "(let ((value 1))\n"
   "  (+ value 2))\n"
   "----\n")
  (asciidoc-mode)
  (setq-local indent-tabs-mode nil)
  (let ((before (buffer-string)))
    (indent-region (point-min) (point-max))
    (list
     indent-line-function
     before
     (buffer-string)
     (equal before (buffer-string))
     (mapcar
      (lambda (line)
        (save-excursion
          (goto-char (point-min))
          (forward-line line)
          (current-indentation)))
      '(0 1 2 4 5 6 7 8)))))"##,
            true,
            expect![[
        r#"OK (indent-relative "* Parent item\n  continuation aligned by the author\n** Nested item\n\n[source,emacs-lisp]\n----\n(let ((value 1))\n  (+ value 2))\n----\n" "* Parent item\ncontinuation aligned by the author\n** Nested item\n\n[source,emacs-lisp]\n----\n(let ((value 1))\n(+ value 2))\n----\n" nil (0 0 0 0 0 0 0 0))"#
    ]],
        ),
    ]);
}
