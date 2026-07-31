use expect_test::expect;

use super::assert_magit_section_batch;

#[test]
fn matching_public_surface_batch() {
    assert_magit_section_batch(&[
        (
            "magit_section_lineage_and_match_conditions_cover_exact_and_recursive_forms",
            r##"(with-temp-buffer
               (magit-section-mode)
               (let ((inhibit-read-only t))
                 (magit-insert-section (root nil)
                   (magit-insert-heading "Root")
                   (magit-insert-section (group 'g)
                     (magit-insert-heading "Group")
                     (magit-insert-section (item 42)
                       (magit-insert-heading "Item"))))
                 (let* ((root magit-root-section)
                        (group (car (oref root children)))
                        (item (car (oref group children))))
                   (list
                    (magit-section-lineage item)
                    (mapcar (lambda (condition)
                              (and (magit-section-match condition item) t))
                            '(item
                              [item group root]
                              [item root]
                              [* group root]
                              [* root]
                              (missing item)
                              missing))
                    (magit-section-value-if
                     [item group root] item)
                    (magit-section-value-if
                     [item root] item)))))"##,
            true,
            expect![[r#"OK ((item group root) (t t nil t t t nil) 42 nil)"#]],
        ),
        (
            "magit_section_case_and_match_assoc_choose_first_matching_clause",
            r##"(with-temp-buffer
               (magit-section-mode)
               (let ((inhibit-read-only t))
                 (magit-insert-section (root nil)
                   (magit-insert-heading "Root")
                   (magit-insert-section (group 'g)
                     (magit-insert-heading "Group")
                     (magit-insert-section (item 42)
                       (magit-insert-heading "Item"))))
                 (let* ((item (car
                               (oref
                                (car (oref magit-root-section children))
                                children))))
                   (goto-char (oref item start))
                   (list
                    (magit-section-case
                      ([missing root] 'wrong)
                      ([item group] (list 'matched (oref it value)))
                      (t 'fallback))
                    (magit-section-match-assoc
                     item
                     '(([item group root] . exact)
                       ([* group] . recursive)
                       (item . type)))
                    (magit-section-match-assoc
                     item
                     '((missing . no)
                       ([* group] . recursive)))))))"##,
            true,
            expect![[r#"OK ((matched 42) exact recursive)"#]],
        ),
        (
            "magit_section_cancel_removes_partial_section_without_corrupting_siblings",
            r##"(with-temp-buffer
               (magit-section-mode)
               (let ((inhibit-read-only t))
                 (magit-insert-section (root nil)
                   (magit-insert-heading "Root")
                   (magit-insert-section (item 'kept)
                     (magit-insert-heading "Kept"))
                   (magit-insert-section (item 'canceled)
                     (magit-insert-heading "Canceled")
                     (magit-cancel-section))
                   (magit-insert-section (item 'also-kept)
                     (magit-insert-heading "Also kept")))
                 (list
                  (buffer-substring-no-properties
                   (point-min) (point-max))
                  (mapcar (lambda (section) (oref section value))
                          (oref magit-root-section children))
                  (magit-get-section
                   '((item . canceled) (root))))))"##,
            true,
            expect![[r#"OK ("Root\nKept\nAlso kept\n" (kept also-kept) nil)"#]],
        ),
    ]);
}
