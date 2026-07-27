use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn renders_nested_inline_markup_with_exact_semantic_face_runs() {
    let elisp_form = r##"
(agent-shell-markdown--deconstruct
 (agent-shell-markdown-convert
  "Review **the _critical_ `unsafe` path**, ~~remove legacy~~, then read [the docs](https://example.com/a?q=1)."))
"##;
    let expect = expect![[
        r#"OK (("Review " nil) ("the " (agent-shell-markdown-bold)) ("critical" (agent-shell-markdown-bold agent-shell-markdown-italic)) (" " (agent-shell-markdown-bold)) ("unsafe" (agent-shell-markdown-inline-code agent-shell-markdown-bold)) (" path" (agent-shell-markdown-bold)) (", " nil) ("remove legacy" (agent-shell-markdown-strikethrough)) (", then read " nil) ("the docs" (agent-shell-markdown-link)) ("." nil))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn incomplete_streaming_markup_stays_literal_until_the_closing_chunk() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "Start **bold and `code")
  (agent-shell-markdown-replace-markup :render-images nil)
  (let ((first (agent-shell-markdown--deconstruct (buffer-string))))
    (goto-char (point-max))
    (insert "` inside** done.\n")
    (agent-shell-markdown-replace-markup :render-images nil)
    (list first
          (agent-shell-markdown--deconstruct (buffer-string))
          (agent-shell-markdown-reconstruct (point-min) (point-max)))))
"##;
    let expect = expect![[
        r#"OK ((("Start **bold and `code" nil)) (("Start " nil) ("bold and " (agent-shell-markdown-bold)) ("code" (agent-shell-markdown-inline-code agent-shell-markdown-bold)) (" inside" (agent-shell-markdown-bold)) (" done.\n" nil)) "Start **bold and `code` inside** done.\n")"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn fenced_code_keeps_markdown_literal_and_exposes_the_real_source_body() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "Before **bold**.\n\n```rust\nfn main() {\n    println!(\"**literal**\");\n}\n```\n\nAfter _italic_.\n")
  (agent-shell-markdown-replace-markup :force t :render-images nil)
  (goto-char (point-min))
  (search-forward "println!")
  (list (substring-no-properties (buffer-string))
        (agent-shell-markdown--deconstruct (buffer-string))
        (agent-shell-markdown-source-block-at-point (point))
        (agent-shell-markdown-reconstruct (point-min) (point-max))))
"##;
    let expect = expect![[
        r#"OK ("Before bold.\n\n\nrust ⧉\n\nfn main() {\n    println!(\"**literal**\");\n}\n\n\nAfter italic.\n" (("Before " nil) ("bold" (agent-shell-markdown-bold)) (".\n\n" nil) ("\n" (agent-shell-markdown-source-block)) ("rust ⧉" (agent-shell-markdown-source-block-language)) ("\n\nfn main() {\n    println!(\"**literal**\");\n}\n\n" (agent-shell-markdown-source-block)) ("\nAfter " nil) ("italic" (agent-shell-markdown-italic)) (".\n" nil)) "fn main() {\n    println!(\"**literal**\");\n}" "Before **bold**.\n\n```rust\nfn main() {\n    println!(\"**literal**\");\n}\n```\n\nAfter _italic_.\n")"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn complex_document_round_trips_after_rendering() {
    let elisp_form = r##"
(let ((markdown
       "# Release review\n\n> Verify **both editors** before shipping.\n\n- inspect `Cargo.toml`\n- compare [CI logs](https://example.com/run/42)\n\n| Runtime | Result |\n|---|---|\n| GNU Emacs | pass |\n| Neomacs | pending |\n\n```elisp\n(message \"not _markup_\")\n```\n"))
  (with-temp-buffer
    (insert markdown)
    (agent-shell-markdown-replace-markup :force t :render-images nil)
    (list (substring-no-properties (buffer-string))
          (agent-shell-markdown-reconstruct (point-min) (point-max))
          (equal markdown
                 (agent-shell-markdown-reconstruct (point-min) (point-max))))))
"##;
    let expect = expect![[
        r##"OK ("Release review\n\n> Verify both editors before shipping.\n\n- inspect Cargo.toml\n- compare CI logs\n\n│ Runtime   │ Result  │\n├───────────┼─────────┤\n│ GNU Emacs │ pass    │\n│ Neomacs   │ pending │\n\n\nelisp ⧉\n\n(message \"not _markup_\")\n\n" "# Release review\n\n> Verify **both editors** before shipping.\n\n- inspect `Cargo.toml`\n- compare [CI logs](https://example.com/run/42)\n\n| Runtime | Result |\n|---|---|\n| GNU Emacs | pass |\n| Neomacs | pending |\n\n```elisp\n(message \"not _markup_\")\n```\n" t)"##
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn rendered_links_retain_destination_interaction_and_source() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "See [ACP specification](<https://agentclientprotocol.com/protocol/session config>) and [issue](https://example.com/163).")
  (agent-shell-markdown-replace-markup :force t :render-images nil)
  (goto-char (point-min))
  (search-forward "ACP specification")
  (let ((first (1- (point))))
    (search-forward "issue")
    (let ((second (1- (point))))
      (list (substring-no-properties (buffer-string))
            (agent-shell-markdown-link-url-at-point first)
            (keymapp (get-text-property first 'keymap))
            (agent-shell-markdown-link-url-at-point second)
            (agent-shell-markdown-reconstruct (point-min) (point-max))))))
"##;
    let expect = expect![[
        r#"OK ("See ACP specification and issue." "https://agentclientprotocol.com/protocol/session config" t "https://example.com/163" "See [ACP specification](<https://agentclientprotocol.com/protocol/session config>) and [issue](https://example.com/163).")"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn remote_images_fall_back_to_links_and_reconstruct_original_markdown() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "Architecture: ![pipeline](https://example.com/pipeline.png)\nEmpty alt: ![](<https://example.com/a b.png>)")
  (agent-shell-markdown-replace-markup :force t :render-images t)
  (let ((visible (substring-no-properties (buffer-string)))
        (runs (agent-shell-markdown--deconstruct (buffer-string)))
        (source (agent-shell-markdown-reconstruct (point-min) (point-max))))
    (list visible runs source)))
"##;
    let expect = expect![[
        r#"OK ("Architecture: pipeline\nEmpty alt: https://example.com/a b.png" (("Architecture: " nil) ("pipeline" (agent-shell-markdown-link)) ("\nEmpty alt: " nil) ("https://example.com/a b.png" (agent-shell-markdown-link))) "Architecture: ![pipeline](https://example.com/pipeline.png)\nEmpty alt: ![](<https://example.com/a b.png>)")"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn tables_render_rich_cells_and_round_trip_exact_source() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "| Check | Outcome | Notes |\n|:---|:---:|---:|\n| **build** | pass | 24 threads |\n| parity | pending | 日本語とEnglish |\n")
  (let ((source (buffer-string)))
    (agent-shell-markdown-replace-markup :force t :render-images nil)
    (list (substring-no-properties (buffer-string))
          (agent-shell-markdown--deconstruct (buffer-string))
          (agent-shell-markdown-reconstruct (point-min) (point-max))
          (equal source
                 (agent-shell-markdown-reconstruct
                  (point-min) (point-max))))))
"##;
    let expect = expect![[
        r#"OK ("│ Check  │ Outcome │ Notes           │\n├────────┼─────────┼─────────────────┤\n│ build  │ pass    │ 24 threads      │\n│ parity │ pending │ 日本語とEnglish │\n" (("│" (agent-shell-markdown-table-border)) (" Check  " (agent-shell-markdown-table-header)) ("│" (agent-shell-markdown-table-border)) (" Outcome " (agent-shell-markdown-table-header)) ("│" (agent-shell-markdown-table-border)) (" Notes           " (agent-shell-markdown-table-header)) ("│" (agent-shell-markdown-table-border)) ("\n" nil) ("├────────┼─────────┼─────────────────┤" (agent-shell-markdown-table-border)) ("\n" nil) ("│" (agent-shell-markdown-table-border)) (" " nil) ("build" (agent-shell-markdown-bold)) ("  " nil) ("│" (agent-shell-markdown-table-border)) (" pass    " nil) ("│" (agent-shell-markdown-table-border)) (" 24 threads      " nil) ("│" (agent-shell-markdown-table-border)) ("\n" nil) ("│" (agent-shell-markdown-table-border)) (" parity " (agent-shell-markdown-table-zebra)) ("│" (agent-shell-markdown-table-border)) (" pending " (agent-shell-markdown-table-zebra)) ("│" (agent-shell-markdown-table-border)) (" 日本語とEnglish " (agent-shell-markdown-table-zebra)) ("│" (agent-shell-markdown-table-border)) ("\n" nil)) #("| Check | Outcome | Notes |\n|:---|:---:|---:|\n| build | pass | 24 threads |\n| parity | pending | 日本語とEnglish |\n" 48 53 (face agent-shell-markdown-bold agent-shell-markdown-source "**build**")) nil)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn blockquotes_headers_dividers_and_inline_code_compose_in_one_transcript() {
    let elisp_form = r##"
(agent-shell-markdown--deconstruct
 (agent-shell-markdown-convert
  (concat "#" "# Findings\n\n> **Important:** keep `./tmp`, never a system temp directory.\n>> Nested rationale with _emphasis_.\n\n---\n\nFinal ~~old~~ answer.\n")))
"##;
    let expect = expect![[
        r#"OK (("Findings" (agent-shell-markdown-header-2)) ("\n\n" nil) ("> " (agent-shell-markdown-blockquote)) ("Important:" (agent-shell-markdown-blockquote agent-shell-markdown-bold)) (" keep " (agent-shell-markdown-blockquote)) ("./tmp" (agent-shell-markdown-blockquote agent-shell-markdown-inline-code)) (", never a system temp directory." (agent-shell-markdown-blockquote)) ("\n" nil) (">> Nested rationale with " (agent-shell-markdown-blockquote)) ("emphasis" (agent-shell-markdown-blockquote agent-shell-markdown-italic)) ("." (agent-shell-markdown-blockquote)) ("\n\n---\n\nFinal " nil) ("old" (agent-shell-markdown-strikethrough)) (" answer.\n" nil))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn custom_renderer_receives_complete_and_streaming_source_descriptors() {
    let elisp_form = r##"
(mapcar
 (lambda (markdown)
   (with-temp-buffer
     (let (seen)
       (let ((agent-shell-markdown-render-functions
              (list
               (lambda (context)
                 (setq seen
                       (mapcar
                        (lambda (block)
                          (list
                           (map-elt block :language)
                           (buffer-substring-no-properties
                            (map-nested-elt block '(:block :start))
                            (map-nested-elt block '(:block :end)))
                           (map-elt block :body)
                           (map-elt block :complete)))
                        (map-elt context :source-blocks)))))))
         (insert markdown)
         (agent-shell-markdown-replace-markup)
         seen))))
 '("text\n```math\n\\frac{a}{b}\n```\n"
   "```latex\n\\alpha + \\beta"
   "```python\nprint(\"$$not math$$\")\n```\n"))
"##;
    let expect = expect![[
        r#"OK ((("math" "```math\n\\frac{a}{b}\n```\n" "\\frac{a}{b}" t)) (("latex" "```latex\n\\alpha + \\beta" nil nil)) (("python" "```python\nprint(\"$$not math$$\")\n```\n" "print(\"$$not math$$\")" t)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn custom_renderer_can_claim_math_without_corrupting_code_markup() {
    let elisp_form = r##"
(with-temp-buffer
  (let ((agent-shell-markdown-render-functions
         (list
          (lambda (context)
            (dolist (block (reverse (map-elt context :source-blocks)))
              (when (and (member (map-elt block :language) '("math" "latex"))
                         (map-elt block :complete))
                (let ((start (map-nested-elt block '(:block :start)))
                      (end (map-nested-elt block '(:block :end)))
                      (body (map-elt block :body)))
                  (delete-region start end)
                  (goto-char start)
                  (insert (format "[MATH:%s]\n" body))
                  (put-text-property start (point)
                                     'agent-shell-markdown-frozen t))))
            nil))))
    (insert "```python\nvalue = \"**literal**\"\n```\n```math\nx_y ** z\n```\n")
    (agent-shell-markdown-replace-markup)
    (list (substring-no-properties (buffer-string))
          (agent-shell-markdown--deconstruct (buffer-string))
          (text-property-any (point-min) (point-max)
                             'agent-shell-markdown-frozen t))))
"##;
    let expect = expect![[
        r#"OK ("\npython ⧉\n\nvalue = \"**literal**\"\n\n[MATH:x_y ** z]\n" (("\n" (agent-shell-markdown-source-block)) ("python ⧉" (agent-shell-markdown-source-block-language)) ("\n\n" (agent-shell-markdown-source-block)) ("value" (font-lock-variable-name-face agent-shell-markdown-source-block)) (" " (agent-shell-markdown-source-block)) ("=" (font-lock-operator-face agent-shell-markdown-source-block)) (" " (agent-shell-markdown-source-block)) ("\"**literal**\"" (font-lock-string-face agent-shell-markdown-source-block)) ("\n\n" (agent-shell-markdown-source-block)) ("[MATH:x_y ** z]\n" nil)) 2)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn forced_rerender_clears_a_stale_streaming_watermark() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "**complete** and _ready_\n")
  (with-silent-modifications
    (put-text-property (point-min) (1+ (point-min))
                       'agent-shell-markdown-watermark (point-max)))
  (agent-shell-markdown-replace-markup)
  (let ((stale (buffer-string)))
    (agent-shell-markdown-replace-markup :force t)
    (list stale
          (buffer-string)
          (agent-shell-markdown--deconstruct (buffer-string))
          (get-text-property (point-min)
                             'agent-shell-markdown-watermark))))
"##;
    let expect = expect![[
        r#"OK (#("**complete** and _ready_\n" 0 1 (agent-shell-markdown-watermark 26)) #("complete and ready\n" 0 1 (agent-shell-markdown-watermark 20 fontified t yank-handler #1=(#[(s) ((insert (substring-no-properties s))) (t)]) font-lock-face agent-shell-markdown-bold agent-shell-markdown-source #2="**complete**" face agent-shell-markdown-bold) 1 8 (fontified t yank-handler #1# font-lock-face agent-shell-markdown-bold agent-shell-markdown-source #2# face agent-shell-markdown-bold) 8 13 (fontified t yank-handler #1#) 13 18 (fontified t yank-handler #1# font-lock-face agent-shell-markdown-italic agent-shell-markdown-source "_ready_" face agent-shell-markdown-italic) 18 19 (fontified t yank-handler #1#)) (("complete" (agent-shell-markdown-bold)) (" and " nil) ("ready" (agent-shell-markdown-italic)) ("\n" nil)) 20)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn partial_selection_reconstruction_does_not_invent_missing_delimiters() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "Prefix **bold _and italic_ text** suffix.\n")
  (agent-shell-markdown-replace-markup :force t :render-images nil)
  (list (substring-no-properties (buffer-string))
        (agent-shell-markdown-reconstruct (point-min) (point-max))
        (agent-shell-markdown-reconstruct
         (+ (point-min) 10) (- (point-max) 3))))
"##;
    let expect = expect![[
        r#"OK ("Prefix bold and italic text suffix.\n" "Prefix **bold _and italic_ text** suffix.\n" "d and italic text suffi")"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}
