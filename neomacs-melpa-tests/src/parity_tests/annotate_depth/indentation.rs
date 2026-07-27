use expect_test::expect;

use super::assert_annotate_depth_parity;

#[test]
fn annotate_depth_determine_indent_obeys_exact_mode_variable_priority() {
    let elisp_form = r##"(let ((standard-indent 11)
               results)
         (push (annotate-depth--determine-indent) results)
         (let ((css-indent-level 7))
           (push (annotate-depth--determine-indent) results))
         (let ((js-indent-level 6)
               (css-indent-level 7))
           (push (annotate-depth--determine-indent) results))
         (let ((sh-indentation 5)
               (js-indent-level 6))
           (push (annotate-depth--determine-indent) results))
         (let ((lisp-body-indent 4)
               (sh-indentation 5))
           (push (annotate-depth--determine-indent) results))
         (let ((c-basic-offset 3)
               (lisp-body-indent 4))
           (push (annotate-depth--determine-indent) results))
         (nreverse results))"##;
    let expect = expect!["OK (2 2 2 2 4 4)"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_determine_indent_ignores_noninteger_candidates() {
    let elisp_form = r##"(let ((standard-indent 8)
               (c-basic-offset 'set-from-style)
               (lisp-body-indent nil)
               (sh-indentation 3.5)
               (js-indent-level "two")
               (cperl-indent-level 6))
         (annotate-depth--determine-indent))"##;
    let expect = expect!["OK 8"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_determine_indent_uses_tab_width_only_for_true_indent_flags() {
    let elisp_form = r##"(list
         (let ((standard-indent 9)
               (tab-width 4)
               (tab-always-indent t))
           (annotate-depth--determine-indent))
         (let ((standard-indent 9)
               (tab-width 4)
               (tab-always-indent 'complete))
           (annotate-depth--determine-indent))
         (let ((standard-indent 9)
               (tab-width 4)
               (tab-always-indent nil)
               (c-tab-always-indent t))
           (annotate-depth--determine-indent)))"##;
    let expect = expect!["OK (2 2 2)"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_marks_real_lisp_lines_at_and_beyond_threshold() {
    let elisp_form = r##"(with-temp-buffer
         (insert "top\n  level-one\n    level-two\n      level-three\n        level-four\n")
         (let ((annotate-depth-mode t)
               (annotate-depth-threshold 2)
               (lisp-body-indent 2)
               (annotate-depth-face 'warning))
           (annotate-depth--annotate)
           (mapcar
            (lambda (overlay)
              (list (overlay-start overlay)
                    (overlay-end overlay)
                    (line-number-at-pos (overlay-start overlay))
                    (current-indentation)
                    (buffer-substring-no-properties
                     (overlay-start overlay) (overlay-end overlay))
                    (overlay-get overlay 'face)))
            (sort (copy-sequence annotate-depth--overlays)
                  (lambda (a b) (< (overlay-start a) (overlay-start b)))))))"##;
    let expect = expect![[
        r#"OK ((21 30 3 0 "level-two" warning) (37 48 4 0 "level-three" warning) (57 67 5 0 "level-four" warning))"#
    ]];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_threshold_boundary_and_disabled_mode_are_exact() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert "first\n  one\n    two\n      three\n")
           (let ((annotate-depth-mode t)
                 (annotate-depth-threshold 3)
                 (standard-indent 2))
             (annotate-depth--annotate)
             (mapcar
              (lambda (overlay)
                (list (line-number-at-pos (overlay-start overlay))
                      (overlay-start overlay)
                      (overlay-end overlay)))
              annotate-depth--overlays)))
         (with-temp-buffer
           (insert "first\n        very-deep\n")
           (let ((annotate-depth-mode nil)
                 (annotate-depth-threshold 1)
                 (standard-indent 2))
             (annotate-depth--annotate)
             annotate-depth--overlays)))"##;
    let expect = expect!["OK (((4 27 32)) nil)"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_blank_and_whitespace_only_lines_follow_indent_rules() {
    let elisp_form = r##"(with-temp-buffer
         (insert "root\n    code\n        \n      nested\n\n")
         (let ((annotate-depth-mode t)
               (annotate-depth-threshold 2)
               (standard-indent 2))
           (annotate-depth--annotate)
           (mapcar
            (lambda (overlay)
              (list (line-number-at-pos (overlay-start overlay))
                    (buffer-substring-no-properties
                     (line-beginning-position)
                     (line-end-position))
                    (overlay-start overlay)
                    (overlay-end overlay)))
            (sort (copy-sequence annotate-depth--overlays)
                  (lambda (a b) (< (overlay-start a) (overlay-start b)))))))"##;
    let expect = expect![[r#"OK ((2 "" 10 14) (3 "" 23 23) (4 "" 30 36))"#]];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_add_overlay_uses_point_to_eol_and_front_rear_advancement() {
    let elisp_form = r##"(with-temp-buffer
         (insert "  deeply indented line\n")
         (goto-char 3)
         (let ((annotate-depth-face 'error))
           (annotate-depth--add-overlay)
           (let ((overlay (car annotate-depth--overlays)))
             (list (overlay-start overlay)
                   (overlay-end overlay)
                   (overlay-get overlay 'face)
                   (overlay-get overlay 'front-advance)
                   (overlay-get overlay 'rear-advance)
                   (buffer-substring-no-properties
                    (overlay-start overlay) (overlay-end overlay))))))"##;
    let expect = expect![[r#"OK (3 23 error nil nil "deeply indented line")"#]];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_reannotation_replaces_stale_overlays_after_edit() {
    let elisp_form = r##"(with-temp-buffer
         (insert "root\n    deep\n  shallow\n")
         (let ((annotate-depth-mode t)
               (annotate-depth-threshold 2)
               (standard-indent 2))
           (annotate-depth--annotate)
           (let ((old (copy-sequence annotate-depth--overlays)))
             (goto-char (point-min))
             (forward-line 1)
             (delete-horizontal-space)
             (annotate-depth--annotate)
             (list (mapcar #'overlay-buffer old)
                   (length annotate-depth--overlays)
                   (buffer-string)))))"##;
    let expect = expect![[r#"OK ((nil) 0 "root\ndeep\n  shallow\n")"#]];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_clear_overlays_deletes_only_tracked_overlays() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdef\n")
         (let ((other (make-overlay 1 3))
               tracked)
           (goto-char 4)
           (annotate-depth--add-overlay)
           (setq tracked (car annotate-depth--overlays))
           (annotate-depth--clear-overlays)
           (list annotate-depth--overlays
                 (overlay-buffer tracked)
                 (overlay-buffer other)
                 (overlay-start other)
                 (overlay-end other)
                 (buffer-string))))"##;
    let expect = expect![[r#"OK (nil nil (:buffer nil) 1 3 "abcdef\n")"#]];
    assert_annotate_depth_parity(elisp_form, expect);
}
