use expect_test::expect;

use super::ParityBatchCase;

/// The mode and the indent command align pipeline continuation lines.
fn the_mode_and_indent_command_align_pipeline_lines() -> ParityBatchCase {
    ParityBatchCase::value(
        "the_mode_and_indent_command_align_pipeline_lines",
        r####"(let ((buf (powershell--test-buffer
                       (concat "Get-ChildItem |\n"
                               "Where-Object Length -gt 0 |\n"
                               "Select-Object Name\n"))))
  (unwind-protect
      (with-current-buffer buf
        (goto-char (point-min))
        (while (not (eobp))
          (powershell-indent-line)
          (forward-line 1))
        (list :source (powershell--test-source-state)
              :mode major-mode
              :indent-line-function indent-line-function
              :comment-start comment-start
              :text (buffer-substring-no-properties (point-min) (point-max))
              :columns
              (save-excursion
                (goto-char (point-min))
                (let (cols)
                  (while (not (eobp))
                    (push (current-column) cols)
                    (forward-line 1))
                  (nreverse cols)))))
    (kill-buffer buf)))"####,
        expect![[
            r##"OK (:source (:upstream-tree "7fe94817c4ca016ba7e6e9c02658d234af0f9ac8" :feature t :version "20251122.1430") :mode powershell-mode :indent-line-function powershell-indent-line :comment-start "#" :text "  Get-ChildItem |\n    Where-Object Length -gt 0 |\n    Select-Object Name\n" :columns (0 0 0))"##
        ]],
    )
}

/// Quoting and unquoting a selection doubles and undoubles embedded
/// single quotes.
fn the_quote_and_unquote_selection_round_trip() -> ParityBatchCase {
    ParityBatchCase::value(
        "the_quote_and_unquote_selection_round_trip",
        r####"(let ((buf (powershell--test-buffer "it's a test")))
  (unwind-protect
      (with-current-buffer buf
        (powershell--test-select-region (point-min) (point-max))
        (powershell-quote-selection (region-beginning) (region-end))
        (let ((quoted (buffer-substring-no-properties
                       (point-min) (point-max))))
          (powershell--test-select-region (point-min) (point-max))
          (powershell-unquote-selection (region-beginning) (region-end))
          (list :quoted quoted
                :unquoted (buffer-substring-no-properties
                           (point-min) (point-max)))))
    (kill-buffer buf)))"####,
        expect![[r#"OK (:quoted "'it''s a test'" :unquoted "it's a test")"#]],
    )
}

/// The mode's auxiliary setups: imenu expressions, the eldoc function,
/// and the font-lock keywords.
fn the_mode_auxiliary_setups_are_registered() -> ParityBatchCase {
    ParityBatchCase::value(
        "the_mode_auxiliary_setups_are_registered",
        r####"(let ((buf (powershell--test-buffer "Get-ChildItem\n")))
  (unwind-protect
      (with-current-buffer buf
        (list :imenu imenu-generic-expression
              :eldoc-function eldoc-documentation-function
              :font-lock-keywords
              (mapcar (lambda (kw)
                        (if (consp (cdr kw)) (car kw) (car kw)))
                      font-lock-keywords)))
    (kill-buffer buf)))"####,
        expect![[
            r#"OK (:imenu (("Functions" "function \\_<\\(?:\\(global\\|local\\|private\\|script\\):\\)?\\([A-Z][a-zA-Z0-9]*-[A-Z0-9][a-zA-Z0-9]*\\)\\_>" 2) ("Filters" "filter \\_<\\(?:\\(global\\|local\\|private\\|script\\):\\)?\\([A-Z][a-zA-Z0-9]*-[A-Z0-9][a-zA-Z0-9]*\\)\\_>" 2) ("Top variables" "^\\(\\[\\(?:[a-zA-Z_][a-zA-Z0-9]*\\)\\(?:\\.[a-zA-Z_][a-zA-Z0-9]*\\)*\\]\\)?\\(\\_<$\\(?:{\\(?:\\(alias\\|env\\|function\\|global\\|hk\\(?:cu\\|lm\\)\\|local\\|private\\|script\\|variable\\|wsman\\):\\)?[^}]+}\\|\\(?:\\(alias\\|env\\|function\\|global\\|hk\\(?:cu\\|lm\\)\\|local\\|private\\|script\\|variable\\|wsman\\):\\)?[a-zA-Z0-9_]+\\_>\\)\\)\\s-*=" 2)) :eldoc-function eldoc-documentation-default :font-lock-keywords nil)"#
        ]],
    )
}

pub(super) fn workflows_public_surface_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        the_mode_and_indent_command_align_pipeline_lines(),
        the_quote_and_unquote_selection_round_trip(),
        the_mode_auxiliary_setups_are_registered(),
    ]
}
