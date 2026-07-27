use expect_test::expect;

use super::assert_annotate_parity;

#[test]
fn annotate_create_single_line_annotation_builds_exact_overlay_contract() {
    let elisp_form = r##"(with-temp-buffer
         (insert "The quick brown fox jumps.\n")
         (let ((annotate-highlight-faces '((:underline "gold")))
               (annotate-annotation-text-faces '((:background "gold" :foreground "black")))
               (annotate-use-echo-area t))
           (annotate-create-annotation
            5 10 "animal under review" "quick" 0 :by-length "fixed-id")
           (let ((overlay (car (annotate-all-annotations))))
             (list
              (buffer-string)
              (length (annotate-all-annotations))
              (overlay-start overlay)
              (overlay-end overlay)
              (buffer-substring-no-properties
               (overlay-start overlay) (overlay-end overlay))
              (annotationp overlay)
              (annotate-annotation-id overlay)
              (annotate-annotation-get-annotation-text overlay)
              (annotate-annotation-face overlay)
              (annotate-annotation-property-annotation-face overlay)
              (annotate-annotation-get-position overlay)
              (annotate-annotation-get-chain-position overlay)
              (annotate-overlay-get-echo-help overlay)))))"##;
    let expect = expect![[
        r#"OK ("The quick brown fox jumps.\n" 1 5 10 "quick" "animal under review" "fixed-id" "animal under review" (:underline "gold") (:background "gold" :foreground "black") :by-length -1 "animal under review")"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_create_multiline_annotation_builds_ordered_chain_without_newline_overlays() {
    let elisp_form = r##"(with-temp-buffer
         (insert "alpha beta\ngamma delta\nepsilon\n")
         (let ((annotate-highlight-faces '((:underline "a") (:underline "b") (:underline "c")))
               (annotate-annotation-text-faces
                '((:background "a") (:background "b") (:background "c"))))
           (annotate-create-annotation
            7 29 "spans lines" "beta\ngamma delta\nepsilon"
            1 :new-line "chain-id")
           (let ((overlays
                  (sort (annotate-all-annotations)
                        (lambda (a b) (< (overlay-start a) (overlay-start b))))))
             (list
              (mapcar
               (lambda (overlay)
                 (list (overlay-start overlay)
                       (overlay-end overlay)
                       (buffer-substring-no-properties
                        (overlay-start overlay) (overlay-end overlay))
                       (annotate-annotation-get-chain-position overlay)
                       (annotate-annotation-id overlay)
                       (annotate-annotation-get-position overlay)))
               overlays)
              (mapcar
               (lambda (overlay)
                 (mapcar
                  (lambda (ring)
                    (list (overlay-start ring) (overlay-end ring)))
                  (annotate-find-chain overlay)))
               overlays)))))"##;
    let expect = expect!["OK (nil nil)"];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_describe_annotations_serializes_chain_once_with_exact_metadata() {
    let elisp_form = r##"(with-temp-buffer
         (insert "first line\nsecond line\n")
         (let ((annotate-highlight-faces '((:underline "a")))
               (annotate-annotation-text-faces '((:background "a"))))
           (annotate-create-annotation
            1 23 "whole thought" "first line\nsecond line"
            0 :by-length "root-id")
           (list
            (annotate-describe-annotations)
            (length (annotate-all-annotations))
            (annotate-buffer-checksum))))"##;
    let expect = expect![[
        r#"OK (((1 23 "whole thought" "first line\nsecond line" 0 :by-length "root-id" nil)) 2 "7565a01bd35f31ba82ab55c978c1b755")"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_navigation_cycles_between_practical_annotations() {
    let elisp_form = r##"(with-temp-buffer
         (insert "alpha beta gamma delta\n")
         (let ((annotate-highlight-faces '((:underline "a") (:underline "b") (:underline "c")))
               (annotate-annotation-text-faces
                '((:background "a") (:background "b") (:background "c"))))
           (annotate-create-annotation 1 6 "first" "alpha" 0 nil "a")
           (annotate-create-annotation 12 17 "second" "gamma" 1 nil "b")
           (goto-char (point-min))
           (let (visits)
             (dotimes (_ 5)
               (annotate-goto-next-annotation)
               (push (list (point) (annotate-annotation-id
                                    (annotate-annotation-at (point))))
                     visits))
             (dotimes (_ 3)
               (annotate-goto-previous-annotation)
               (push (list 'back (point)
                           (annotate-annotation-id
                            (annotate-annotation-at (point))))
                     visits))
             (nreverse visits))))"##;
    let expect = expect![[
        r#"OK ((12 "b") (12 "b") (12 "b") (12 "b") (12 "b") (back 5 "a") (back 5 "a") (back 5 "a"))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_clear_annotations_preserves_unrelated_overlays_and_source_text() {
    let elisp_form = r##"(with-temp-buffer
         (insert "alpha beta gamma\n")
         (let ((other (make-overlay 1 6))
               (annotate-highlight-faces '((:underline "a")))
               (annotate-annotation-text-faces '((:background "a"))))
           (overlay-put other 'category 'unrelated)
           (annotate-create-annotation 7 11 "note" "beta" 0 nil "id")
           (let ((before (length (overlays-in (point-min) (point-max)))))
             (annotate-clear-annotations)
             (list before
                   (length (overlays-in (point-min) (point-max)))
                   (overlay-get other 'category)
                   (overlay-buffer other)
                   (buffer-string)))))"##;
    let expect = expect![[r#"OK (2 1 unrelated (:buffer nil) "alpha beta gamma\n")"#]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_integrates_real_annotation_as_comments_without_mutating_source_buffer() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert "(defun add (a b)\n  (+ a b))\n")
         (let ((annotate-highlight-faces '((:underline "a")))
               (annotate-annotation-text-faces '((:background "a")))
               (annotate-integrate-marker " ANNOTATION: ")
               (annotate-integrate-highlight ?~))
           (annotate-create-annotation
            9 12 "Consider clearer argument names." "add"
            0 :by-length "review-id")
           (let ((source (buffer-string))
                 (result-buffer
                  (annotate--integrate-annotations
                   :use-annotation-marker t
                   :as-new-buffer t
                   :switch-to-new-buffer nil)))
             (unwind-protect
                 (list source
                       (buffer-string)
                       (with-current-buffer result-buffer
                         (buffer-string))
                       (buffer-name result-buffer))
               (kill-buffer result-buffer)))))"##;
    let expect = expect![[
        r#"OK (#("(defun add (a b)\n  (+ a b))\n" 1 6 (face font-lock-keyword-face) 7 10 (face font-lock-function-name-face)) #("(defun add (a b)\n  (+ a b))\n" 1 6 (face font-lock-keyword-face) 7 10 (face font-lock-function-name-face)) "(defun add (a b)\n;      ~~~\n; ANNOTATION: \n;Consider clearer argument names.\n  (+ a b))\n" ".annotated.diff")"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_bounds_selects_region_symbol_and_whole_line_at_newline() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (emacs-lisp-mode)
           (insert "alpha beta\n")
           (goto-char 2)
           (annotate-bounds))
         (with-temp-buffer
           (transient-mark-mode 1)
           (insert "alpha beta\n")
           (goto-char 2)
           (push-mark 7 t t)
           (setq mark-active t)
           (annotate-bounds))
         (with-temp-buffer
           (insert "alpha beta\nnext\n")
           (goto-char 11)
           (let ((annotate-endline-annotate-whole-line t))
             (annotate-bounds)))
         (with-temp-buffer
           (insert "alpha beta\n")
           (goto-char 11)
           (let ((annotate-endline-annotate-whole-line nil))
             (condition-case err
                 (annotate-bounds)
               (error (list (car err) (cdr err)))))))"##;
    let expect = expect!["OK ((1 6) (2 7) (11 12) (11 12))"];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_mode_lifecycle_installs_and_removes_hooks_without_touching_text() {
    let elisp_form = r##"(with-temp-buffer
         (insert "ordinary file content\n")
         (let ((annotate-use-messages nil)
               (annotate-file
                (expand-file-name "missing-annotations" (getenv "TMPDIR"))))
           (annotate-mode 1)
           (let ((enabled
                  (list annotate-mode
                        (memq #'annotate-before-change-fn before-change-functions)
                        (memq #'annotate-save-annotations after-save-hook)
                        (memq #'annotate-shutdown kill-buffer-hook)
                        (string-match-p
                         "annotate--font-lock-matcher"
                         (prin1-to-string font-lock-keywords)))))
             (annotate-mode -1)
             (list enabled
                   annotate-mode
                   (memq #'annotate-before-change-fn before-change-functions)
                   (memq #'annotate-save-annotations after-save-hook)
                   (memq #'annotate-shutdown kill-buffer-hook)
                   (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ((t (annotate-before-change-fn) nil nil 8) nil nil nil nil "ordinary file content\n")"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_blacklisted_major_mode_refuses_activation_and_runs_shutdown() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (let ((annotate-blacklist-major-mode '(prog-mode))
               initialize-calls shutdown-calls)
           (cl-letf (((symbol-function 'annotate-initialize)
                      (lambda () (push 'initialize initialize-calls)))
                     ((symbol-function 'annotate-shutdown)
                      (lambda () (push 'shutdown shutdown-calls))))
             (annotate-mode 1)
             (list annotate-mode initialize-calls shutdown-calls))))"##;
    let expect = expect!["OK (nil nil (shutdown))"];
    assert_annotate_parity(elisp_form, expect);
}
