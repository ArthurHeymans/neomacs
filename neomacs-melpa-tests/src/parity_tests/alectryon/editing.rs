use expect_test::expect;

use super::assert_alectryon_parity;

#[test]
fn alectryon_inserts_block_markers_for_coq_and_lean_at_real_editing_points() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (let ((alectryon--winding-down t))
       (funcall (car case)))
     (setq-local alectryon-prog-mode (car case))
     (insert (cadr case))
     (goto-char (caddr case))
     (alectryon-insert-literate-markers)
     (list (car case) (buffer-string) (point)
           (buffer-substring-no-properties
            (line-beginning-position) (line-end-position)))))
 '((coq-mode "Check nat." 7)
   (lean4-mode "#check Nat" 1)
   (coq-mode "" 1)
   (lean4-mode "αβγ" 3)))"##;
    let expect = expect![[
        r#"OK ((coq-mode "Check (*|\n\n|*)nat." 11 "") (lean4-mode "/-|\n\n|-/#check Nat" 5 "") (coq-mode "(*|\n\n|*)" 5 "") (lean4-mode "αβ/-|\n\n|-/γ" 7 ""))"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_block_marker_insertion_splits_existing_literate_comments_into_code_islands() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (let ((alectryon--winding-down t))
       (funcall (car case)))
     (setq-local alectryon-prog-mode (car case))
     (insert (cadr case))
     (goto-char (caddr case))
     (let ((before (alectryon--in-literate-comment-p)))
       (alectryon-insert-literate-markers)
       (list (car case) before (buffer-string) (point)
             (alectryon--in-literate-comment-p)))))
 '((coq-mode "(*|A paragraph with prose.|*)" 14)
   (lean4-mode "/-|A paragraph with prose.|-/" 15)))"##;
    let expect = expect![[
        r#"OK ((coq-mode t "(*|A paragrap|*)\n\n\n\n(*|h with prose.|*)" 19 nil) (lean4-mode t "/-|A paragraph|-/\n\n\n\n/-| with prose.|-/" 20 nil))"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_dafny_gutter_insertion_handles_blank_code_and_prose_lines() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (let ((alectryon--winding-down t))
       (dafny-mode))
     (setq-local alectryon-prog-mode 'dafny-mode)
     (insert (car case))
     (goto-char (cadr case))
     (let ((inside (alectryon--in-literate-comment-p)))
       (alectryon-insert-literate-markers)
       (list inside (buffer-string) (point)))))
 '(("" 1)
   ("method Main() {}" 8)
   ("/// prose line" 8)
   ("method A() {}\nmethod B() {}" 4)))"##;
    let expect = expect![[
        r#"OK ((nil "/// " 5) (nil "method Main() {}\n\n/// \n" 23) (t "/// prose line\n\n\n" 17) (nil "method A() {}\n\n/// \n\nmethod B() {}" 20))"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_newline_preserves_dafny_literate_gutters_but_not_code_or_block_comments() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (let ((alectryon--winding-down t))
     (dafny-mode))
   (setq-local alectryon-prog-mode 'dafny-mode)
   (insert "/// alpha beta")
   (goto-char 10)
   (alectryon-newline nil)
   (list (buffer-string) (point)))
 (with-temp-buffer
   (let ((alectryon--winding-down t))
     (dafny-mode))
   (setq-local alectryon-prog-mode 'dafny-mode)
   (insert "method Main() {}")
   (goto-char 8)
   (alectryon-newline nil)
   (list (buffer-string) (point)))
 (with-temp-buffer
   (let ((alectryon--winding-down t))
     (coq-mode))
   (setq-local alectryon-prog-mode 'coq-mode)
   (insert "(*|alpha beta|*)")
   (goto-char 9)
   (alectryon-newline nil)
   (list (buffer-string) (point))))"##;
    let expect = expect![[
        r#"OK (("/// alpha\n/// beta" 15) ("method \nMain() {}" 9) ("(*|alpha\n beta|*)" 10))"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_font_lock_marks_real_block_delimiters_gutters_and_prose_with_properties() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (let ((alectryon--winding-down t))
     (coq-mode))
   (setq-local alectryon-prog-mode 'coq-mode)
   (insert "(*|\nDocumentation\n|*)\nCheck nat.")
   (alectryon--prog-mode 1)
   (font-lock-ensure)
   (mapcar
    (lambda (position)
      (list position
            (get-text-property position 'face)
            (get-text-property position 'display)
            (get-text-property position 'wrap-prefix)
            (get-text-property position 'modification-hooks)))
    '(1 3 5 10 19 20 24)))
 (with-temp-buffer
   (let ((alectryon--winding-down t))
     (dafny-mode))
   (setq-local alectryon-prog-mode 'dafny-mode)
   (insert "/// prose wraps here\nmethod Main() {}")
   (alectryon--prog-mode 1)
   (font-lock-ensure)
   (mapcar
    (lambda (position)
      (list position
            (get-text-property position 'face)
            (get-text-property position 'display)
            (get-text-property position 'wrap-prefix)
            (get-text-property position 'modification-hooks)))
    '(1 3 4 5 10 20 21))))"##;
    let expect = expect![[
        r#"OK (((1 #1=(alectryon-comment alectryon-comment-marker) #2=(space :align-to right) nil nil) (3 #1# #2# nil nil) (5 alectryon-comment nil nil nil) (10 alectryon-comment nil nil nil) (19 #3=(alectryon-comment alectryon-comment-marker) #2# nil nil) (20 #3# #2# nil nil) (24 nil nil nil nil)) ((1 #4=(alectryon-gutter alectryon-comment) #5=(space :width (+ (0) 0.5)) nil #6=(alectryon--gutter-marker-modification-hook)) (3 #4# #5# nil #6#) (4 alectryon-comment (space :width (+ 0.5 (0))) nil #6#) (5 alectryon-comment nil #("/// " 0 3 (display #7=(space :width (+ (0) 0.5)) face alectryon-gutter) 3 4 (display #8=(space :width (+ 0.5 (0))) face nil)) nil) (10 alectryon-comment nil #("/// " 0 3 (display #7# face alectryon-gutter) 3 4 (display #8# face nil)) nil) (20 alectryon-comment nil #("/// " 0 3 (display #7# face alectryon-gutter) 3 4 (display #8# face nil)) nil) (21 alectryon-comment nil nil nil)))"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_gutter_backspace_hook_removes_the_whole_visual_marker_only_at_boundaries() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (insert (car case))
     (goto-char (cadr case))
     (alectryon--gutter-marker-modification-hook
      (caddr case) (cadddr case))
     (list (buffer-string) (point))))
 '(("/// prose" 5 4 5)
   ("///prose" 4 3 4)
   ("///" 4 3 4)
   ("x/// prose" 6 5 6)
   ("/// prose" 7 6 7)))"##;
    let expect =
        expect![[r#"OK ((" prose" 2) ("/prose" 2) ("/" 2) ("x/// prose" 6) ("/// prose" 7))"#]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_buffer_utilities_widen_narrowed_documents_and_choose_collision_free_point_markers() {
    let elisp_form = r##"(with-temp-buffer
  (insert "prefix\nbody\nsuffix")
  (let ((first (alectryon--point-marker)))
    (goto-char (point-min))
    (insert first)
    (let ((second (alectryon--point-marker)))
      (narrow-to-region (+ (point-min) (length first) 8)
                        (+ (point-min) (length first) 12))
      (list
       (buffer-string)
       (alectryon--buffer-string)
       first second
       (equal first second)
       (string-match-p (regexp-quote first) second)
       (point-min) (point-max)))))"##;
    let expect = expect![[
        r#"OK ("ody\n" "￼127919￼prefix\nbody\nsuffix" "￼127919￼" "￼127920￼" nil nil 21 25)"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_presentation_mode_hides_real_annotations_and_rejects_markup_views() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (let ((alectryon--winding-down t))
     (coq-mode))
   (setq-local alectryon-prog-mode 'coq-mode)
   (insert "(* .unfold .fails *)\nCheck nat.")
   (alectryon--prog-mode 1)
   (alectryon-presentation-mode 1)
   (font-lock-ensure)
   (list alectryon-presentation-mode
         alectryon--prog-presentation-font-lock-keywords
         (get-text-property 1 'face)
         (get-text-property 1 'display)
         (get-text-property 8 'display)))
 (with-temp-buffer
   (let ((alectryon--winding-down t))
     (rst-mode))
   (setq-local alectryon-prog-mode 'coq-mode
               alectryon-text-mode 'rst-mode)
   (condition-case err
       (alectryon-presentation-mode 1)
     (error (list (car err) (error-message-string err)
                  alectryon-presentation-mode)))))"##;
    let expect = expect![[
        r#"OK ((t (("([*]\\(\\(?:\\s-*[.][-a-z]+\\)+\\)\\s-*[*])" 0 '(face #1='(:height 0.5) display "👻") append)) (font-lock-comment-delimiter-face . #1#) "👻" "👻") (user-error "‘alectryon-presentation-mode’ needs Alectryon in programming mode" nil))"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}
