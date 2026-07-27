use expect_test::expect;

use super::{assert_apdl_mode_parity, assert_apdl_mode_signal_parity};

#[test]
fn command_navigation_walks_each_statement_on_a_condensed_input_line() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert "prep7 $ et,1,solid186 $ keyopt,1,2,0 ! production mesh")
  (goto-char (point-min))
  (search-forward "solid")
  (list
   (progn
     (apdl-command-start)
     (list (point) (current-column)
           (buffer-substring-no-properties
            (point) (save-excursion (apdl-command-end) (point)))))
   (progn
     (apdl-command-end)
     (list (point) (current-column)
           (buffer-substring-no-properties
            (save-excursion (apdl-command-start) (point)) (point))))
   (progn
     (apdl-command-end 1)
     (list (point) (current-column)
           (buffer-substring-no-properties
            (save-excursion (apdl-command-start) (point)) (point))))))"##;
    let expect = expect![[
        r#"OK ((9 8 "et,1,solid186") (22 21 "et,1,solid186") (55 54 "keyopt,1,2,0 ! production mesh"))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn command_navigation_treats_multiline_formats_and_default_commands_as_units() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "*vwrite,node_id,x_coord\n"
   "(I8,F16.6) &\n"
   "(A8)\n"
   "mp,ex,1,210000\n"
   ",prxy,1,0.3\n"
   ",dens,1,7.85e-9\n")
  (list
   (progn
     (goto-char (point-min))
     (forward-line 1)
     (apdl-command-start)
     (list (line-number-at-pos) (current-column)))
   (progn
     (apdl-command-end)
     (list (line-number-at-pos) (current-column)))
   (progn
     (forward-line 1)
     (apdl-command-end)
     (list (line-number-at-pos) (current-column)))
   (progn
     (apdl-command-start)
     (list (line-number-at-pos) (current-column)))))"##;
    let expect = expect!["OK ((1 0) (3 4) (4 14) (4 0))"];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn code_line_navigation_skips_blank_lines_comments_and_preserves_goal_column() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "prep7\n"
   "! select production elements\n"
   "\n"
   "  esel,s,type,,1\n"
   "    nsle,s\n"
   "! another comment\n"
   "solve\n")
  (goto-char (point-min))
  (move-to-column 4)
  (list
   (progn
     (apdl-next-code-line 1)
     (list (line-number-at-pos) (current-column)
           (buffer-substring-no-properties
            (line-beginning-position) (line-end-position))))
   (progn
     (apdl-next-code-line 2)
     (list (line-number-at-pos) (current-column)
           (buffer-substring-no-properties
            (line-beginning-position) (line-end-position))))
   (progn
     (apdl-previous-code-line 2)
     (list (line-number-at-pos) (current-column)
           (buffer-substring-no-properties
            (line-beginning-position) (line-end-position))))))"##;
    let expect =
        expect![[r#"OK ((4 4 "  esel,s,type,,1") (7 4 "solve") (4 4 "  esel,s,type,,1"))"#]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn number_block_navigation_finds_complete_workbench_mesh_data_boundaries() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "nblock,3,solid\n"
   "(1i9,3e20.9e3)\n"
   "1,0.0,0.0,0.0\n"
   "2,1.0,0.0,0.0\n"
   "3,1.0,1.0,0.0\n"
   "-1\n"
   "type,1\n")
  (goto-char (point-min))
  (forward-line 3)
  (end-of-line)
  (list
   (progn
     (apdl-number-block-start)
     (list (line-number-at-pos) (current-column)
           (buffer-substring-no-properties
            (line-beginning-position) (line-end-position))))
   (progn
     (apdl-number-block-end)
     (list (line-number-at-pos) (current-column)
           (buffer-substring-no-properties
            (line-beginning-position) (line-end-position))))))"##;
    let expect = expect![[r#"OK ((2 0 "(1i9,3e20.9e3)") (6 2 "-1"))"#]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn balanced_block_navigation_handles_nested_if_do_else_and_marking_workflows() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "*if,active,eq,1,then\n"
   "  *do,index,1,3\n"
   "    solve\n"
   "  *enddo\n"
   "*else\n"
   "  /com,disabled\n"
   "*endif\n")
  (goto-char (point-min))
  (let ((skip-forward
         (progn
           (apdl-skip-block-forward)
           (list (line-number-at-pos) (current-column)))))
    (let ((skip-backward
           (progn
             (apdl-skip-block-backwards)
             (list (line-number-at-pos) (current-column)))))
      (forward-line 2)
      (let ((up
             (progn
               (apdl-up-block)
               (list (line-number-at-pos) (current-column)))))
        (goto-char (point-min))
        (apdl-mark-block)
        (list
         skip-forward skip-backward up
         (line-number-at-pos (region-beginning))
         (line-number-at-pos (region-end))
         (buffer-substring-no-properties
          (region-beginning) (region-end)))))))"##;
    let expect = expect![[
        r#"OK ((7 6) (1 0) (2 2) 1 8 "*if,active,eq,1,then\n  *do,index,1,3\n    solve\n  *enddo\n*else\n  /com,disabled\n*endif\n")"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn unmatched_block_scan_signals_the_exact_navigation_failure() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert "*if,active,eq,1,then\nsolve\n")
  (goto-char (point-min))
  (apdl-skip-block-forward))"##;
    let expect = expect![[r#"ERR (error "Can’t reach specified block level")"#]];
    assert_apdl_mode_signal_parity(elisp_form, expect);
}
