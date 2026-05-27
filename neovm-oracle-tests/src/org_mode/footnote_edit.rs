use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_footnote_renumber_delete_sort_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Notes\n")
    (insert "B ref[fn:7], A ref[fn:3], B again[fn:7].\n\n")
    (insert "[fn:7] Bee definition\n")
    (insert "[fn:3] Aye definition\n")
    (insert "[fn:9] Unused definition\n")
    (org-footnote-renumber-fn:N)
    (let ((after-renumber
           (buffer-substring-no-properties (point-min) (point-max))))
      (org-footnote-sort)
      (let ((after-sort
             (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "[fn:1]")
        (let ((deleted (org-footnote-delete)))
          (list after-renumber
                after-sort
                deleted
                (org-footnote-all-labels)
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_footnote_inline_normalize_section_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Alpha\n")
    (insert "Text [fn::Inline *bold* note] and named [fn:name].\n")
    (insert "* Footnotes\n")
    (insert "[fn:name] Named note\n")
    (let ((org-footnote-section "Footnotes")
          (org-footnote-fill-after-inline-note-extraction nil))
      (org-footnote-normalize)
      (list (org-footnote-all-labels)
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_footnote_reference_definition_navigation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* One\n")
    (insert "First [fn:a] and second [fn:b].\n")
    (insert "* Footnotes\n")
    (insert "[fn:b] Bee\n")
    (insert "[fn:a] Aye\n")
    (goto-char (point-min))
    (search-forward "[fn:a]")
    (let ((ref-a (org-footnote-at-reference-p)))
      (org-footnote-goto-definition "a")
      (let ((def-a (list (line-number-at-pos)
                         (org-footnote-at-definition-p))))
        (org-footnote-goto-previous-reference "a")
        (let ((back-a (list (line-number-at-pos)
                            (org-footnote-at-reference-p))))
          (goto-char (point-max))
          (let ((pos (org-footnote-create-definition "c")))
            (list ref-a
                  def-a
                  back-a
                  pos
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_footnote_auto_label_inline_adjust_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Body\n")
    (insert "Alpha sentence. Beta sentence.\n")
    (goto-char (point-min))
    (search-forward "Alpha")
    (let ((org-footnote-auto-label t)
          (org-footnote-define-inline t)
          (org-footnote-auto-adjust 'sort)
          (org-footnote-fill-after-inline-note-extraction nil))
      (org-footnote-new)
      (insert "Inline A")
      (goto-char (point-min))
      (search-forward "Beta")
      (let ((org-footnote-define-inline nil))
        (org-footnote-new)
        (insert "Definition B"))
      (org-footnote-normalize)
      (list (org-footnote-all-labels)
            (org-footnote--collect-references 'anonymous)
            (org-footnote--collect-definitions)
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_footnote_delete_label_references_definitions_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Text\n")
    (insert "A[fn:keep] B[fn:drop] C[fn:drop] D[fn::Anon]\n\n")
    (insert "[fn:keep] Keep definition\n")
    (insert "[fn:drop] Drop definition 1\n")
    (insert "[fn:drop] Drop definition 2\n")
    (let ((refs (org-footnote-delete-references "drop"))
          (defs (org-footnote-delete-definitions "drop")))
      (goto-char (point-min))
      (search-forward "Anon")
      (let ((anon-deleted (org-footnote-delete)))
        (list refs
              defs
              anon-deleted
              (org-footnote-all-labels)
              (org-footnote--collect-references 'anonymous)
              (org-footnote--collect-definitions)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_footnote_missing_duplicate_normalize_sort_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* H\n")
    (insert "First[fn:z] missing[fn:missing] again[fn:z].\n")
    (insert "** Local\n")
    (insert "Nested[fn:local]\n")
    (insert "[fn:local] Local def\n")
    (insert "* Footnotes\n")
    (insert "[fn:z] First Z\n")
    (insert "[fn:z] Duplicate Z\n")
    (insert "[fn:unused] Unused def\n")
    (let ((org-footnote-section "Footnotes")
          (org-footnote-fill-after-inline-note-extraction nil))
      (org-footnote-normalize)
      (org-footnote-sort)
      (list (org-footnote-all-labels)
            (org-footnote--collect-references 'anonymous)
            (org-footnote--collect-definitions)
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_footnote_action_context_matrix_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Body\n")
    (insert "Paragraph anchor and old ref[fn:old].\n")
    (insert "Link [[https://example.org][link anchor]] text.\n")
    (insert "| table anchor | value |\n")
    (insert "#+begin_src emacs-lisp\n")
    (insert "src anchor\n")
    (insert "#+end_src\n")
    (insert "#+begin_verse\n")
    (insert "verse anchor\n")
    (insert "#+end_verse\n")
    (insert "* Footnotes\n")
    (insert "[fn:old] Old definition\n")
    (let ((probe
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (search-forward needle)
               (goto-char (match-beginning 0))
               (let ((context (org-element-context)))
                 (list needle
                       (org-element-type context)
                       (org-footnote-in-valid-context-p)
                       (org-footnote--allow-reference-p)
                       (org-footnote-at-reference-p)
                       (org-footnote-at-definition-p)))))))
          (org-footnote-auto-label 'confirm)
          (org-footnote-define-inline nil)
          (org-footnote-auto-adjust t)
          (org-footnote-section "Footnotes")
          (org-footnote-fill-after-inline-note-extraction nil))
      (let ((before (mapcar probe
                            '("Paragraph" "old ref" "link anchor"
                              "table anchor" "src anchor" "verse anchor"
                              "Old definition"))))
        (goto-char (point-min))
        (search-forward "Paragraph")
        (cl-letf (((symbol-function 'read-string)
                   (lambda (&rest _) "custom-label")))
          (org-footnote-new))
        (insert "Custom definition")
        (let ((after-new (buffer-substring-no-properties
                          (point-min) (point-max))))
          (goto-char (point-min))
          (search-forward "[fn:custom-label]")
          (org-footnote-action)
          (let ((after-action-def
                 (list (line-number-at-pos)
                       (org-footnote-at-definition-p))))
            (cl-letf (((symbol-function 'read-char-exclusive)
                       (lambda (&rest _) ?S)))
              (org-footnote-action t))
            (list before
                  after-new
                  after-action-def
                  (org-footnote-all-labels)
                  (org-footnote--collect-references 'anonymous)
                  (org-footnote--collect-definitions)
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_footnote_local_normalize_nested_missing_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Chapter A\n")
    (insert "Top A[fn:a] inline[fn::Anon A with nested [fn:nested]].\n")
    (insert "** Child A\n")
    (insert "Child ref[fn:missing] and repeat[fn:a].\n")
    (insert "[fn:a] Local A definition\n")
    (insert "[fn:nested] Nested definition\n")
    (insert "* Chapter B\n")
    (insert "Top B[fn:b] and anonymous[fn::Anon B].\n")
    (insert "[fn:unused] Unused B definition\n")
    (insert "[fn:b] Bee definition\n")
    (let ((org-footnote-section nil)
          (org-footnote-fill-after-inline-note-extraction nil))
      (let ((before (list (org-footnote-all-labels)
                          (org-footnote--collect-references 'anonymous)
                          (org-footnote--collect-definitions))))
        (org-footnote-normalize)
        (let ((after-normalize
               (buffer-substring-no-properties (point-min) (point-max)))
              (labels-after-normalize (org-footnote-all-labels))
              (defs-after-normalize (org-footnote--collect-definitions)))
          (org-footnote-sort)
          (goto-char (point-min))
          (search-forward "Chapter B")
          (let ((next-b (org-footnote-get-next-reference nil nil
                                                         (save-excursion
                                                           (outline-next-heading)
                                                           (point))))
                (prev-global (org-footnote-get-next-reference nil t)))
            (list before
                  labels-after-normalize
                  defs-after-normalize
                  after-normalize
                  (org-footnote-all-labels)
                  (org-footnote--collect-references 'anonymous)
                  (org-footnote--collect-definitions)
                  (and next-b
                       (list (car next-b)
                             (line-number-at-pos (nth 1 next-b))))
                  (and prev-global
                       (list (car prev-global)
                             (line-number-at-pos (nth 1 prev-global))))
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_footnote_label_definition_section_adjust_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Body\n")
    (insert "A[fn:1] B[fn:3] inline[fn::Inline note].\n")
    (insert "* Footnotes\n")
    (insert "[fn:1] One definition\n")
    (insert "[fn:2] Unused two\n")
    (insert "[fn:3] Three definition\n")
    (insert "* Footnotes\n")
    (insert "[fn:dup] Duplicate section definition\n")
    (let ((org-footnote-section "Footnotes")
          (org-footnote-auto-adjust 'renumber)
          (org-footnote-fill-after-inline-note-extraction nil))
      (let ((initial
             (list (mapcar #'org-footnote-normalize-label
                           '("fn:abc" "  fn:spaced  " " plain " "   "))
                   (org-footnote-unique-label)
                   (org-footnote-unique-label '("1" "2" "4"))
                   (org-footnote-get-definition "fn:1")
                   (org-footnote-get-definition "3")
                   (org-footnote-get-definition "missing")
                   (org-footnote-all-labels)
                   (org-footnote--collect-definitions))))
        (let ((deleted-defs (org-footnote--collect-definitions t))
              (after-delete-defs
               (buffer-substring-no-properties (point-min) (point-max))))
          (org-footnote--clear-footnote-section)
          (let ((after-clear
                 (buffer-substring-no-properties (point-min) (point-max))))
            (goto-char (point-min))
            (search-forward "B")
            (org-footnote-new)
            (insert "New auto-adjust definition")
            (org-footnote-auto-adjust-maybe)
            (let ((after-adjust
                   (buffer-substring-no-properties (point-min) (point-max))))
              (list initial
                    deleted-defs
                    after-delete-defs
                    after-clear
                    (org-footnote-all-labels)
                    (org-footnote--collect-references 'anonymous)
                    (org-footnote--collect-definitions)
                    after-adjust)))))))"##,
    );
}
