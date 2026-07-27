use expect_test::expect;

use super::{assert_apdl_mode_parity, assert_apdl_mode_signal_parity};

#[test]
fn indent_region_formats_a_nested_conditional_loop_and_else_workflow() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil)
        (apdl-block-offset 2))
    (apdl-mode)
    (insert
     "*if,load_case,eq,1,then\n"
     "f,all,fy,-1000\n"
     "*do,index,1,3\n"
     "time,index\n"
     "solve\n"
     "*enddo\n"
     "*else\n"
     "/com,Skipping load case\n"
     "*endif\n")
    (indent-region (point-min) (point-max))
    (buffer-string)))"##;
    let expect = expect![[
        r#"OK "*if,load_case,eq,1,then\n  f,all,fy,-1000\n  *do,index,1,3\n    time,index\n    solve\n  *enddo\n*else\n  /com,Skipping load case\n*endif\n""#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn default_command_continuations_align_with_the_previous_argument_column() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil))
    (apdl-mode)
    (insert
     "mp,ex,1,210000\n"
     ",prxy,1,0.3\n"
     ",dens,1,7.85e-9\n"
     "\n"
     "  et,1,solid186\n"
     ",2,beam188\n")
    (indent-region (point-min) (point-max))
    (list
     (buffer-string)
     (save-excursion
       (goto-char (point-min))
       (mapcar
        (lambda (_)
          (prog1 (current-indentation) (forward-line)))
        '(1 2 3 4 5 6))))))"##;
    let expect = expect![[
        r#"OK ("mp,ex,1,210000\n  ,prxy,1,0.3\n  ,dens,1,7.85e-9\n\net,1,solid186\n  ,2,beam188\n" (0 2 2 0 0 2))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn calculate_indent_handles_comments_condensed_commands_and_existing_offsets() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil)
        (apdl-code-comment-column 36))
    (apdl-mode)
    (insert
     "*if,active,eq,1,then\n"
     "    n,1,0,0,0 $ n,2,1,0,0\n"
     "! explanatory code comment\n"
     "*endif\n")
    (goto-char (point-min))
    (let (results)
      (dotimes (_ 4)
        (push
         (list (line-number-at-pos)
               (current-indentation)
               (apdl-calculate-indent))
         results)
        (forward-line))
      (nreverse results))))"##;
    let expect = expect!["OK ((1 0 0) (2 4 2) (3 0 4) (4 0 2))"];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn format_line_split_continues_comments_and_adds_ampersands_to_format_constructs() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (let ((apdl-mode-hook nil)
         (apdl-dynamic-highlighting-flag nil))
     (apdl-mode)
     (insert "! long engineering explanation")
     (goto-char 9)
     (apdl-indent-format-line)
     (buffer-string)))
 (with-temp-buffer
   (let ((apdl-mode-hook nil)
         (apdl-dynamic-highlighting-flag nil))
     (apdl-mode)
     (insert "*vwrite,node,x,y\n(I8,2F12.5)")
     (goto-char (point-min))
     (forward-line)
     (search-forward "I8")
     (apdl-indent-format-line)
     (buffer-string))))"##;
    let expect = expect![[
        r#"OK ("! long e\n! ngineering explanation" "*vwrite,node,x,y\n(I8 &\n    ,2F12.5)")"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn splitting_inside_a_single_quoted_apdl_string_signals_without_mutating_the_line() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil))
    (apdl-mode)
    (insert "jobname = 'production beam model'")
    (search-backward "beam")
    (apdl-indent-format-line)))"##;
    let expect = expect![[r#"ERR (error "Cannot split a code line inside a string")"#]];
    assert_apdl_mode_signal_parity(elisp_form, expect);
}
