use super::assert_ace_jump_mode_parity;
use expect_test::expect;

#[test]
fn ace_jump_mode_populates_leaf_overlays_with_exact_properties() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-overlays*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "abcd"))
               (let* ((area
                       (make-aj-visual-area
                        :buffer buffer
                        :window (selected-window)
                        :frame (selected-frame)))
                      (positions
                       (mapcar
                        (lambda (offset)
                          (make-aj-position
                           :offset offset
                           :visual-area area))
                        '(1 2 3)))
                      (tree
                       (ace-jump-tree-breadth-first-construct
                        3 3)))
                 (ace-jump-populate-overlay-to-search-tree
                  tree positions)
                 (mapcar
                  (lambda (node)
                    (let ((overlay (cdr node)))
                      (list
                       (overlayp overlay)
                       (overlay-start overlay)
                       (overlay-end overlay)
                       (eq
                        (overlay-buffer overlay)
                        buffer)
                       (overlay-get overlay 'face)
                       (eq
                        (overlay-get overlay 'window)
                        (selected-window))
                       (aj-position-offset
                        (overlay-get overlay 'aj-data)))))
                  (cdr tree))))
           (kill-buffer buffer)))"##;
    let expect = expect![
        "OK ((t 1 2 t ace-jump-face-foreground t 1) (t 2 3 t ace-jump-face-foreground t 2) (t 3 4 t ace-jump-face-foreground t 3))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_delete_tree_overlays_clears_payload_and_detaches_overlays() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abc")
         (let* ((one (make-overlay 1 2))
                (two (make-overlay 2 3))
                (tree
                 (list
                  'branch
                  (cons 'leaf one)
                  (cons 'leaf two))))
           (ace-jump-delete-overlay-in-search-tree tree)
           (list
            tree
            (overlay-buffer one)
            (overlay-buffer two)
            (overlays-in (point-min) (point-max)))))"##;
    let expect = expect!["OK ((branch (leaf) (leaf)) nil nil nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_buffer_substring_reads_ascii_tab_newline_and_wide_character() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-substring*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "a\t\n界"))
               (let ((area
                      (make-aj-visual-area
                       :buffer buffer
                       :window (selected-window)
                       :frame (selected-frame))))
                 (mapcar
                  (lambda (offset)
                    (ace-jump-buffer-substring
                     (make-aj-position
                      :offset offset
                      :visual-area area)))
                  '(1 2 3 4))))
           (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK ("a" "\11" "\n" "界")"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_update_leaf_overlay_display_handles_character_widths() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-display*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "a\t\n界"))
               (let* ((area
                       (make-aj-visual-area
                        :buffer buffer
                        :window (selected-window)
                        :frame (selected-frame)))
                      (positions
                       (mapcar
                        (lambda (offset)
                          (make-aj-position
                           :offset offset
                           :visual-area area))
                        '(1 2 3 4)))
                      (tree
                       (ace-jump-tree-breadth-first-construct
                        4 4))
                      (tab-width 4))
                 (ace-jump-populate-overlay-to-search-tree
                  tree positions)
                 (ace-jump-update-overlay-in-search-tree
                  tree '(?w ?x ?y ?z))
                 (mapcar
                  (lambda (node)
                    (overlay-get
                     (cdr node)
                     'display))
                  (cdr tree))))
           (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK ("w" "x   " "y\n" "z ")"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_update_branch_overlay_display_reuses_branch_key() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-branch-display*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "abcdef"))
               (let* ((area
                       (make-aj-visual-area
                        :buffer buffer
                        :window (selected-window)
                        :frame (selected-frame)))
                      (positions
                       (mapcar
                        (lambda (offset)
                          (make-aj-position
                           :offset offset
                           :visual-area area))
                        '(1 2 3 4 5)))
                      (tree
                       (ace-jump-tree-breadth-first-construct
                        5 2))
                      displays)
                 (ace-jump-populate-overlay-to-search-tree
                  tree positions)
                 (ace-jump-update-overlay-in-search-tree
                  tree '(?a ?b))
                 (ace-jump-tree-preorder-traverse
                  tree
                  (lambda (node)
                    (setq displays
                          (cons
                           (overlay-get
                            (cdr node)
                            'display)
                           displays))))
                 (nreverse displays)))
           (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK ("a" "a" "a" "b" "b")"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
