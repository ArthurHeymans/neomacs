use expect_test::expect;

use super::assert_apdl_mode_parity;

#[test]
fn full_highlighting_fontifies_a_practical_structural_analysis_program() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil)
        (apdl-highlighting-level 2))
    (apdl-mode)
    (insert
     "/prep7\n"
     "et,1,solid186\n"
     "mp,ex,1,210000\n"
     "radius = 10\n"
     "area = acos(-1) * radius ** 2\n"
     "*if,area,gt,100,then\n"
     "  nsel,s,loc,x,0\n"
     "  f,all,fy,-1000\n"
     "*endif\n"
     "solve ! production solve\n")
    (apdl-find-user-variables)
    (font-lock-ensure)
    (mapcar
     (lambda (needle)
       (goto-char (point-min))
       (search-forward needle)
       (let ((start (- (point) (length needle))))
         (list needle
               (get-text-property start 'face)
               (get-text-property start 'font-lock-face))))
     '("/prep7" "et" "solid186" "mp" "radius" "acos"
       "*if" "then" "nsel" "solve" "production"))))"##;
    let expect = expect![[
        r#"OK (("/prep7" font-lock-keyword-face nil) ("et" font-lock-keyword-face nil) ("solid186" font-lock-builtin-face nil) ("mp" font-lock-keyword-face nil) ("radius" font-lock-variable-name-face nil) ("acos" font-lock-function-name-face nil) ("*if" font-lock-keyword-face nil) ("then" nil nil) ("nsel" font-lock-keyword-face nil) ("solve" font-lock-keyword-face nil) ("production" font-lock-comment-face nil))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn highlighting_levels_expose_progressively_richer_language_semantics() {
    let elisp_form = r##"(mapcar
 (lambda (level)
   (with-temp-buffer
     (let ((apdl-mode-hook nil)
           (apdl-dynamic-highlighting-flag nil)
           (apdl-highlighting-level level))
       (apdl-mode)
       (insert
        "et,1,solid186 $ keyopt,1,2,0\n"
        "value = sqrt(4)\n"
        "ARG1 = 7\n"
        ":retry\n")
       (apdl-find-user-variables)
       (font-lock-ensure)
       (cons
        level
        (mapcar
         (lambda (needle)
           (goto-char (point-min))
           (search-forward needle)
           (get-text-property (- (point) (length needle)) 'face))
         '("et" "solid186" "$" "value" "sqrt" "ARG1" ":retry"))))))
 '(0 1 2))"##;
    let expect = expect![
        "OK ((0 font-lock-keyword-face nil nil font-lock-variable-name-face nil font-lock-variable-name-face nil) (1 font-lock-keyword-face font-lock-builtin-face font-lock-type-face font-lock-variable-name-face font-lock-function-name-face apdl-arg-face font-lock-type-face) (2 font-lock-keyword-face font-lock-builtin-face font-lock-type-face font-lock-variable-name-face font-lock-function-name-face apdl-arg-face font-lock-type-face))"
    ];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn comments_titles_format_strings_arguments_and_operators_keep_distinct_faces() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil)
        (apdl-highlighting-level 2))
    (apdl-mode)
    (insert
     "/title,Cantilever production model\n"
     "/com,Generated for release verification\n"
     "! ordinary engineering note\n"
     "*msg,info\n"
     "Maximum displacement = %disp%\n"
     "*vwrite,node_id,disp\n"
     "(I8,E16.8) &\n"
     "(A8)\n"
     "ARG2 = 4\n")
    (apdl-find-user-variables)
    (font-lock-ensure)
    (mapcar
     (lambda (needle)
       (goto-char (point-min))
       (search-forward needle)
       (list needle
             (get-text-property (- (point) (length needle)) 'face)
             (get-text-property (1- (point)) 'face)))
     '("/title" "Cantilever" "/com" "Generated" "ordinary"
       "*msg" "Maximum displacement" "%disp%" "*vwrite"
       "I8" "&" "ARG2"))))"##;
    let expect = expect![[
        r#"OK (("/title" font-lock-keyword-face font-lock-keyword-face) ("Cantilever" font-lock-doc-face font-lock-doc-face) ("/com" font-lock-keyword-face font-lock-keyword-face) ("Generated" font-lock-doc-face font-lock-doc-face) ("ordinary" font-lock-comment-face font-lock-comment-face) ("*msg" font-lock-keyword-face font-lock-keyword-face) ("Maximum displacement" nil nil) ("%disp%" font-lock-type-face font-lock-type-face) ("*vwrite" font-lock-keyword-face font-lock-keyword-face) ("I8" nil nil) ("&" font-lock-type-face font-lock-type-face) ("ARG2" apdl-arg-face apdl-arg-face))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn keyword_corpus_supports_real_completion_across_commands_elements_and_functions() {
    let elisp_form = r##"(list
 (length apdl-completions)
 (mapcar
  (lambda (prefix)
    (let ((completion-ignore-case t))
      (all-completions prefix apdl-completions)))
  '("SOLID18" "*DOW" "/PRE" "ACOS" "UX"))
 (mapcar
  (lambda (pair)
    (let ((regexp (symbol-value (car pair)))
          (sample (cdr pair)))
      (list (car pair) sample
            (and (string-match regexp sample)
                 (match-string 0 sample)))))
  '((apdl-command-regexp . "SOLVE")
    (apdl-element-regexp . "SOLID186")
    (apdl-get-function-regexp . "ACTIVE")
    (apdl-parametric-function-regexp . "ACOS")
    (apdl-undocumented-command-regexp . "UNDO"))))"##;
    let expect = expect![[
        r#"OK (1877 (("SOLID185" "SOLID186" "SOLID187") ("*DOWHILE") ("/PREP7") ("ACOS()") ("UX()")) ((apdl-command-regexp "SOLVE" "SOLV") (apdl-element-regexp "SOLID186" "SOLID186") (apdl-get-function-regexp "ACTIVE" nil) (apdl-parametric-function-regexp "ACOS" "ACOS") (apdl-undocumented-command-regexp "UNDO" nil)))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}
