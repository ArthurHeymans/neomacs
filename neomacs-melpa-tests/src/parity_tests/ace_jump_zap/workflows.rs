use super::assert_ace_jump_zap_parity;
use expect_test::expect;

#[test]
fn ace_jump_zap_forward_up_to_deletes_before_selected_character() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 3)
         (let ((ajz/zapping t)
               (ajz/to-char nil)
               (ajz/saved-point 3)
               (ajz/zap-function
                'delete-region))
           (ajz/maybe-zap-start)
           (goto-char 8)
           (ajz/maybe-zap-end)
           (list
            (buffer-string)
            (point)
            (mark t)
            mark-active
            ajz/zapping
            ajz/saved-point
            ajz/to-char)))"##;
    let expect = expect![[r#"OK ("abhij" 3 3 t nil nil nil)"#]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_forward_to_deletes_through_selected_character() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 3)
         (let ((ajz/zapping t)
               (ajz/to-char t)
               (ajz/saved-point 3)
               (ajz/zap-function
                'delete-region))
           (ajz/maybe-zap-start)
           (goto-char 8)
           (ajz/maybe-zap-end)
           (list
            (buffer-string)
            (point)
            (mark t)
            mark-active
            ajz/zapping
            ajz/saved-point
            ajz/to-char)))"##;
    let expect = expect![[r#"OK ("abij" 3 3 t nil nil nil)"#]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_backward_up_to_deletes_after_selected_character() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 9)
         (let ((ajz/zapping t)
               (ajz/to-char nil)
               (ajz/saved-point 9)
               (ajz/zap-function
                'delete-region))
           (ajz/maybe-zap-start)
           (goto-char 3)
           (ajz/maybe-zap-end)
           (list
            (buffer-string)
            (point)
            (mark t)
            mark-active
            ajz/zapping
            ajz/saved-point
            ajz/to-char)))"##;
    let expect = expect![[r#"OK ("abcij" 4 4 t nil nil nil)"#]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_backward_to_deletes_selected_character_too() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 9)
         (let ((ajz/zapping t)
               (ajz/to-char t)
               (ajz/saved-point 9)
               (ajz/zap-function
                'delete-region))
           (ajz/maybe-zap-start)
           (goto-char 3)
           (ajz/maybe-zap-end)
           (list
            (buffer-string)
            (point)
            (mark t)
            mark-active
            ajz/zapping
            ajz/saved-point
            ajz/to-char)))"##;
    let expect = expect![[r#"OK ("abij" 3 3 t nil nil nil)"#]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_kill_region_updates_buffer_and_kill_ring() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 3)
         (let ((ajz/zapping t)
               (ajz/to-char t)
               (ajz/saved-point 3)
               (ajz/zap-function
                'kill-region)
               (kill-ring nil)
               (kill-ring-yank-pointer nil))
           (ajz/maybe-zap-start)
           (goto-char 8)
           (ajz/maybe-zap-end)
           (list
            (buffer-string)
            kill-ring
            (eq kill-ring-yank-pointer
                kill-ring)
            ajz/zapping)))"##;
    let expect = expect![[r#"OK ("abij" ("cdefgh") t nil)"#]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_public_command_composes_installed_hooks_with_kill_region() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 3)
         (let ((ajz/zap-function
                'kill-region)
               (ajz/forward-only t)
               (overriding-local-map nil)
               (kill-ring nil)
               (kill-ring-yank-pointer nil)
               observed)
           (cl-letf (((symbol-function 'call-interactively)
                      (lambda (function &optional record keys)
                        (setq observed
                              (list
                               function
                               record
                               keys
                               ace-jump-mode-scope
                               ace-jump-search-filter
                               ajz/zapping
                               ajz/saved-point))
                        (run-hooks
                         'ace-jump-mode-before-jump-hook)
                        (goto-char 8)
                        (run-hooks
                         'ace-jump-mode-end-hook)
                        'jumped)))
             (ace-jump-zap-up-to-char))
           (list
            observed
            (buffer-string)
            kill-ring
            (eq kill-ring-yank-pointer
                kill-ring)
            ajz/zapping
            ajz/saved-point
            ajz/to-char)))"##;
    let expect = expect![[
        r#"OK ((ace-jump-char-mode nil nil window ajz/forward-query t 3) "abhij" ("cdefg") t nil nil nil)"#
    ]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_real_tree_and_populate_advices_compose_limit_sort_and_overlay_assignment() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-zap-compose*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert
                  (make-string 80 ?x)))
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
                        (number-sequence 1 60)))
                      (ajz/zapping t)
                      (ajz/saved-point 60)
                      (ajz/sort-by-closest t)
                      (ajz/52-character-limit t)
                      (tree
                       (ace-jump-tree-breadth-first-construct
                        (length positions)
                        4))
                      offsets)
                 (ace-jump-populate-overlay-to-search-tree
                  tree positions)
                 (ace-jump-tree-preorder-traverse
                  tree
                  (lambda (node)
                    (setq offsets
                          (cons
                           (overlay-start
                            (cdr node))
                           offsets))))
                 (setq offsets
                       (nreverse offsets))
                 (list
                  (length offsets)
                  (seq-take offsets 5)
                  (last offsets 5))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK (52 (60 59 58 57 56) (13 12 11 10 9))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}
