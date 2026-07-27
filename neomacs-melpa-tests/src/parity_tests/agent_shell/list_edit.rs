use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn parses_real_markdown_list_items_and_rejects_lookalikes() {
    let elisp_form = r##"
(mapcar
 (lambda (text)
   (with-temp-buffer
     (insert text)
     (goto-char (point-min))
     (agent-shell-list-edit--at-item)))
 '("- ship release"
   "    * nested **markup**"
   "	+ tabbed"
   "12. retry deployment"
   "-missing-space"
   "1.missing-space"
   "ordinary prose"))
"##;
    let expect = expect![[
        r#"OK ((#1=(:type . bullet) (:indent . "") (:marker . "-") (:content . "ship release")) (#1# (:indent . "    ") (:marker . "*") (:content . "nested **markup**")) (#1# (:indent . "\11") (:marker . "+") (:content . "tabbed")) ((:type . numbered) (:indent . "") (:marker . "12") (:content . "retry deployment")) nil nil nil)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn edits_a_mixed_nested_task_list_as_a_user_would() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "- prepare\n  1. inspect\n  2. patch\n- verify")
  (goto-char (point-min))
  (search-forward "prepare")
  (agent-shell-list-edit-newline)
  (insert "run checks")
  (agent-shell-list-edit-indent-line)
  (search-forward "inspect")
  (end-of-line)
  (agent-shell-list-edit-newline)
  (insert "compare GNU and Neomacs")
  (buffer-string))
"##;
    let expect = expect![[
        r#"OK "- prepare\n  - run checks\n  1. inspect\n  2. compare GNU and Neomacs\n  2. patch\n- verify""#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn numbered_list_continues_multi_digit_markers_and_preserves_indent() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "    98. download\n    99. install")
  (goto-char (point-max))
  (agent-shell-list-edit-newline)
  (insert "exercise package")
  (agent-shell-list-edit-newline)
  (insert "compare transcript")
  (list (buffer-string)
        (agent-shell-list-edit--at-item)
        (point)))
"##;
    let expect = expect![[
        r#"OK ("    98. download\n    99. install\n    100. exercise package\n    101. compare transcript" ((:type . numbered) (:indent . "    ") (:marker . "101") (:content . "compare transcript")) 87)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn empty_items_break_out_without_damaging_neighboring_content() {
    let elisp_form = r##"
(mapcar
 (lambda (text)
   (with-temp-buffer
     (insert text)
     (goto-char (point-min))
     (search-forward "|")
     (delete-char -1)
     (agent-shell-list-edit-newline)
     (insert "|")
     (buffer-string)))
 '("- done\n- |\nafter"
   "  +   |\nafter"
   "7.    |\nafter"))
"##;
    let expect = expect![[r#"OK ("- done\n\n|\nafter" "\n|\nafter" "\n|\nafter")"#]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn indent_and_dedent_round_trip_nested_bullets_and_numbers() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "- alpha\n  * beta\n    3. gamma\nplain")
  (goto-char (point-min))
  (agent-shell-list-edit-indent-line)
  (forward-line 1)
  (agent-shell-list-edit-indent-line)
  (agent-shell-list-edit-dedent-line)
  (forward-line 1)
  (agent-shell-list-edit-dedent-line)
  (agent-shell-list-edit-dedent-line)
  (forward-line 1)
  (agent-shell-list-edit-dedent-line)
  (buffer-string))
"##;
    let expect = expect![[r#"OK "  - alpha\n  * beta\n3. gamma\nplain""#]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn minor_mode_installs_and_removes_the_editing_key_contract() {
    let elisp_form = r##"
(with-temp-buffer
  (agent-shell-list-edit-mode 1)
  (let ((enabled (list agent-shell-list-edit-mode
                       (key-binding (kbd "RET"))
                       (key-binding (kbd "TAB"))
                       (key-binding (kbd "<backtab>")))))
    (agent-shell-list-edit-mode -1)
    (list enabled
          agent-shell-list-edit-mode
          (key-binding (kbd "<backtab>")))))
"##;
    let expect = expect![
        "OK ((t agent-shell-list-edit-newline agent-shell-list-edit-indent-line agent-shell-list-edit-dedent-line) nil nil)"
    ];
    assert_agent_shell_parity(elisp_form, expect);
}
