use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn extracts_multiple_real_file_edits_and_location_hints_from_a_tool_call() {
    let elisp_form = r##"
(agent-shell--make-diff-infos
 :acp-tool-call
 '((title . "Refactor parity runner")
   (content .
    [((type . "text") (text . "Applied two edits"))
     ((type . "diff")
      (path . "src/lib.rs")
      (oldText . "fn old() {\n    1\n}\n")
      (newText . "fn current() {\n    2\n}\n"))
     ((type . "diff")
      (path . "tests/parity.rs")
      (newText . "#[test]\nfn strict_case() {}\n"))])
   (locations .
    [((path . "tests/parity.rs") (line . 88))
     ((path . "src/lib.rs") (line . 240))])))
"##;
    let expect = expect![[
        r##"OK (((:old . "fn old() {\n    1\n}\n") (:new . "fn current() {\n    2\n}\n") (:file . "src/lib.rs") (:line . 240)) ((:old . "") (:new . "#[test]\nfn strict_case() {}\n") (:file . "tests/parity.rs") (:line . 88)))"##
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn computes_and_formats_aggregate_stats_for_mixed_repository_changes() {
    let elisp_form = r##"
(let ((diffs
       '(((:file . "src/lib.rs")
          (:old . "a\nb\nc\nd")
          (:new . "a\nB\nc\nD\nextra"))
         ((:file . "src/new.rs")
          (:old . "")
          (:new . "one\ntwo\nthree"))
         ((:file . "src/obsolete.rs")
          (:old . "old\ncontent")
          (:new . "")))))
  (list
   (mapcar #'agent-shell--diff-line-stats diffs)
   (agent-shell--diffs-line-stats diffs)
   (substring-no-properties
    (agent-shell--format-diffs-line-stats diffs))
   (mapcar
    (lambda (diff)
      (and (agent-shell--format-diff-line-stats diff)
           (substring-no-properties
            (agent-shell--format-diff-line-stats diff))))
    diffs)))
"##;
    let expect = expect![[
        r#"OK ((((:added . 3) (:removed . 2)) ((:added . 3) (:removed . 0)) ((:added . 0) (:removed . 2))) ((:added . 6) (:removed . 4)) "+6 -4" ("+3 -2" "+3" "-2"))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn renders_clean_unified_diffs_without_temporary_file_headers() {
    let elisp_form = r##"
(let* ((diff
        '((:file . "src/config.rs")
          (:old . "pub fn config() {\n    old();\n    keep();\n}\n")
          (:new . "pub fn config() {\n    new();\n    keep();\n    validate();\n}\n")))
       (text (agent-shell--format-diff-as-text diff)))
  (list (substring-no-properties text)
        (string-match-p "^---" text)
        (string-match-p "^\\+\\+\\+" text)
        (seq-some
         (lambda (position)
           (get-text-property position 'font-lock-face text))
         (number-sequence 0 (1- (length text))))))
"##;
    let expect = expect![[
        r#"OK ("@@ -1,4 +1,5 @@\n pub fn config() {\n-    old();\n+    new();\n     keep();\n+    validate();\n }\n" nil nil diff-hunk-header)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn renders_multi_file_review_text_with_each_filename_and_change() {
    let elisp_form = r##"
(substring-no-properties
 (agent-shell--format-diffs-as-text
  '(((:file . "Cargo.toml")
     (:old . "lto = true\n")
     (:new . "lto = false\n"))
    ((:file . ".github/workflows/ci.yml")
     (:old . "runs-on: ubuntu-latest\n")
     (:new . "runs-on: ubuntu-22.04\n"))
    ((:file . "src/svg.rs")
     (:old . "use librsvg::Renderer;\n")
     (:new . "use resvg::Tree;\n")))))
"##;
    let expect = expect![[
        r#"OK "╭────────────╮\n│ Cargo.toml │\n╰────────────╯\n\n@@ -1 +1 @@\n-lto = true\n+lto = false\n\n\n╭──────────────────────────╮\n│ .github/workflows/ci.yml │\n╰──────────────────────────╯\n\n@@ -1 +1 @@\n-runs-on: ubuntu-latest\n+runs-on: ubuntu-22.04\n\n\n╭────────────╮\n│ src/svg.rs │\n╰────────────╯\n\n@@ -1 +1 @@\n-use librsvg::Renderer;\n+use resvg::Tree;\n""#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn derives_old_and_new_search_anchors_for_a_changed_hunk() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "@@ -10,5 +10,6 @@ fn run()\n"
          " context-before\n"
          "-old-call();\n"
          "+new-call();\n"
          "+validate();\n"
          " context-after\n"
          "\\ No newline at end of file\n")
  (goto-char (point-min))
  (let ((header (point)))
    (list
     (agent-shell-diff--hunk-anchor header nil)
     (progn
       (forward-line 2)
       (agent-shell-diff--hunk-anchor header (line-beginning-position)))
     (progn
       (forward-line 2)
       (agent-shell-diff--hunk-anchor header (line-beginning-position))))))
"##;
    let expect = expect![[
        r#"OK (((:old-block . "context-before\nold-call();\ncontext-after") (:new-block . "context-before\nnew-call();\nvalidate();\ncontext-after") (:offset . 1)) ((:old-block . "context-before\nold-call();\ncontext-after") (:new-block . "context-before\nnew-call();\nvalidate();\ncontext-after") (:offset . 1)) ((:old-block . "context-before\nold-call();\ncontext-after") (:new-block . "context-before\nnew-call();\nvalidate();\ncontext-after") (:offset . 2)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn target_at_point_carries_file_line_hint_and_body_offset() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "@@ -1,4 +1,4 @@\n context\n-old\n+new\n tail\n")
  (put-text-property (point-min) (point-max)
                     'agent-shell-diff-file "/workspace/src/lib.rs")
  (put-text-property (point-min) (point-max)
                     'agent-shell-diff-line 73)
  (goto-char (point-min))
  (forward-line 2)
  (list
   (agent-shell-diff--target-at-point)
   (progn
     (goto-char (point-max))
     (insert "\noutside")
     (agent-shell-diff--target-at-point))))
"##;
    let expect = expect![[
        r#"OK (((:file . "/workspace/src/lib.rs") (:hint-line . 73) (:old-block . "context\nold\ntail") (:new-block . "context\nnew\ntail") (:offset . 1)) ((:file . "/workspace/src/lib.rs") (:hint-line . 73) (:old-block . "context\nold\ntail") (:new-block . "context\nnew\ntail") (:offset . 1)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn anchor_search_uses_the_location_hint_to_disambiguate_duplicate_code() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "fn duplicate() {\n  old();\n}\n\n"
          "intervening\n\n"
          "fn duplicate() {\n  old();\n}\n")
  (mapcar
   (lambda (hint)
     (let ((position
            (agent-shell-diff--search-block
             "fn duplicate() {\n  old();\n}" hint)))
       (and position (line-number-at-pos position))))
   '(nil 2 8 100)))
"##;
    let expect = expect!["OK (1 1 7 7)"];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn inserts_multiple_diff_sections_with_navigation_metadata() {
    let elisp_form = r##"
(with-temp-buffer
  (agent-shell-diff--insert-diffs
   '(((:file . "one.rs") (:line . 12)
      (:old . "fn one() { old(); }\n")
      (:new . "fn one() { new(); }\n"))
     ((:file . "two.rs") (:line . 91)
      (:old . "let value = 1;\n")
      (:new . "let value = 2;\n")))
   (current-buffer))
  (let ((plain (substring-no-properties (buffer-string))))
    (goto-char (point-min))
    (list (and (string-match-p "one\\.rs" plain) t)
          (and (string-match-p "fn one() { old(); }" plain) t)
          (and (string-match-p "fn one() { new(); }" plain) t)
          (and (string-match-p "two\\.rs" plain) t)
          (and (string-match-p "let value = 1;" plain) t)
          (and (string-match-p "let value = 2;" plain) t)
          (get-text-property (point) 'agent-shell-diff-file)
          (get-text-property (point) 'agent-shell-diff-line)
          (progn
            (search-forward "two.rs")
            (get-text-property (1- (point))
                               'agent-shell-diff-file))
          (get-text-property (1- (point))
                             'agent-shell-diff-line))))
"##;
    let expect = expect![[r#"OK (t t t t t t "one.rs" 12 "two.rs" 91)"#]];
    assert_agent_shell_parity(elisp_form, expect);
}
