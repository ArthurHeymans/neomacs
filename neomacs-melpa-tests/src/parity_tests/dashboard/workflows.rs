use expect_test::expect;

use super::ParityBatchCase;

fn buffer_name_and_startupify_list_are_configured() -> ParityBatchCase {
    ParityBatchCase::value(
        "buffer_name_and_startupify_list_are_configured",
        r####"
(list :buffer-name dashboard-buffer-name
      :startupify-has-banner
      (and (memq 'dashboard-insert-banner dashboard-startupify-list) t)
      :startupify-has-items
      (and (memq 'dashboard-insert-items dashboard-startupify-list) t)
      :startupify-has-footer
      (and (memq 'dashboard-insert-footer dashboard-startupify-list) t))
"####,
        expect![[
            r#"OK (:buffer-name "*dashboard*" :startupify-has-banner t :startupify-has-items t :startupify-has-footer t)"#
        ]],
    )
}

fn separator_and_goto_line_mutate_dashboard_buffer() -> ParityBatchCase {
    ParityBatchCase::value(
        "separator_and_goto_line_mutate_dashboard_buffer",
        r####"
(let ((dashboard-buffer-name (generate-new-buffer-name "*dashboard-parity*")))
  (with-current-buffer (get-buffer-create dashboard-buffer-name)
    (erase-buffer)
    (insert "line1\nline2\nline3\n")
    (dashboard--goto-line 2)
    (list :line (line-number-at-pos)
          :sep (substring-no-properties (dashboard--separator))
          :point-bol (bolp))))
"####,
        expect![[r#"OK (:line 2 :sep "\n\n" :point-bol t)"#]],
    )
}

fn insert_newline_appends_blank_line() -> ParityBatchCase {
    ParityBatchCase::value(
        "insert_newline_appends_blank_line",
        r####"
(let ((dashboard-buffer-name (generate-new-buffer-name "*dashboard-nl*")))
  (with-current-buffer (get-buffer-create dashboard-buffer-name)
    (erase-buffer)
    (insert "x")
    (dashboard-insert-newline)
    (list :text (buffer-string)
          :ends-newline (and (string-suffix-p "\n" (buffer-string)) t))))
"####,
        expect![[r#"OK (:text "x\n" :ends-newline t)"#]],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        buffer_name_and_startupify_list_are_configured(),
        separator_and_goto_line_mutate_dashboard_buffer(),
        insert_newline_appends_blank_line(),
    ]
}
