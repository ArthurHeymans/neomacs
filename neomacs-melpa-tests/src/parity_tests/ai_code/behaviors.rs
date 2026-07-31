use expect_test::expect;

use super::assert_ai_code_batch;

#[test]
fn behaviors_public_surface_batch() {
    assert_ai_code_batch(&[
        (
            "behavior_tags_extract_mode_modifiers_constraints_and_bundle_from_real_prompt",
            r##"
(progn
  (require 'ai-code-behaviors)
  (ai-code--extract-and-remove-hashtags
   "Implement ledger retries #code #deep #tdd #no-breaking-changes @production-safe while preserving #unknown-tag"
   'gptel-plan))
"##,
            true,
            expect![[
        r#"OK ((:mode nil :modifiers ("deep" "tdd") :constraint-modifiers nil :preset nil) "Implement ledger retries #code #no-breaking-changes @production-safe while preserving #unknown-tag" nil nil)"#
    ]],
        ),
        (
            "behavior_keyword_classifier_distinguishes_real_engineering_intents",
            r##"
(progn
  (require 'ai-code-behaviors)
  (mapcar
   #'ai-code--classify-prompt-intent-keywords
   '("Implement and refactor the payment retry path with test driven development"
     "Debug this crash and find the root cause of the failing exception"
     "Review this patch for security risks and race conditions"
     "Research and explain how the event loop schedules callbacks"
     "Write unit tests and integration tests for the parser"
     "Design a specification and implementation plan for offline sync")))
"##,
            true,
            expect![[
        r#"OK ((:mode "=code" :modifiers nil :confidence high) (:mode "=debug" :modifiers nil :confidence high) (:mode "=review" :modifiers nil :confidence medium) (:mode "=research" :modifiers nil :confidence high) (:mode "=test" :modifiers nil :confidence high) (:mode "=spec" :modifiers nil :confidence high))"#
    ]],
        ),
        (
            "behavior_state_is_strictly_isolated_between_repository_roots",
            r##"
(progn
  (require 'ai-code-behaviors)
  (let ((ai-code--behaviors-session-states (make-hash-table :test 'equal))
        (ai-code--behaviors-pending-presets (make-hash-table :test 'equal)))
    (ai-code--behaviors-set-state
     '(:mode "code" :modifiers ("tdd")) "/repos/service-a/")
    (ai-code--behaviors-set-preset "tdd-dev" "/repos/service-a/")
    (ai-code--behaviors-set-active-bundle
     "production-safe" "/repos/service-a/")
    (ai-code--behaviors-set-state
     '(:mode "review" :modifiers ("challenge")) "/repos/service-b/")
    (ai-code--behaviors-set-pending-preset "code-review" "/repos/service-b/")
    (let ((before
           (list
            (ai-code--behaviors-get-state "/repos/service-a/")
            (ai-code--behaviors-get-preset "/repos/service-a/")
            (ai-code--behaviors-get-active-bundle "/repos/service-a/")
            (ai-code--behaviors-get-state "/repos/service-b/")
            (ai-code--behaviors-get-pending-preset "/repos/service-b/"))))
      (ai-code--behaviors-clear-state "/repos/service-a/")
      (list before
            (ai-code--behaviors-get-state "/repos/service-a/")
            (ai-code--behaviors-get-state "/repos/service-b/")))))
"##,
            true,
            expect![[
        r#"OK (((:mode "code" :modifiers ("tdd")) "tdd-dev" "production-safe" #1=(:mode "review" :modifiers ("challenge")) "code-review") nil #1#)"#
    ]],
        ),
        (
            "behavior_constraints_expand_and_render_into_actionable_instruction",
            r##"
(progn
  (require 'ai-code-behaviors)
  (cl-letf (((symbol-function 'ai-code--load-behavior-prompt)
             (lambda (name)
               (cdr (assoc name
                           '(("code" . "Modify code and verify the result.")
                             ("deep" . "Trace dependencies before editing.")
                             ("tdd" . "Work red, green, then refactor.")))))))
    (let* ((bundle (ai-code--expand-constraint-bundle "production-safe"))
           (behaviors
            (list :mode "code"
                  :modifiers '("deep" "tdd")
                  :constraint-modifiers bundle
                  :custom-suffix "Run the repository's focused test suite."))
           (instruction (ai-code--build-behavior-instruction behaviors))
           (wrapped
            (ai-code--behaviors-wrap-with-instruction
             behaviors
             "Make retries idempotent without changing the public API.")))
      (list bundle instruction wrapped))))
"##,
            true,
            expect![[
        r#"OK (nil "AdditionalContext: <operating-mode>\nModify code and verify the result.\n</operating-mode>\n\nAdditionalContext: <behavior-modifiers>\nTrace dependencies before editing.\n\nWork red, green, then refactor.\n</behavior-modifiers>\n\nAdditionalContext: <custom-constraints>\nRun the repository's focused test suite.\n</custom-constraints>\n\nThese behaviors apply until superseded by new hashtags. During compaction, preserve the most recent <operating-mode> and <behavior-modifiers> blocks." "AdditionalContext: <operating-mode>\nModify code and verify the result.\n</operating-mode>\n\nAdditionalContext: <behavior-modifiers>\nTrace dependencies before editing.\n\nWork red, green, then refactor.\n</behavior-modifiers>\n\nAdditionalContext: <custom-constraints>\nRun the repository's focused test suite.\n</custom-constraints>\n\nThese behaviors apply until superseded by new hashtags. During compaction, preserve the most recent <operating-mode> and <behavior-modifiers> blocks.\n\n<user-prompt>\nMake retries idempotent without changing the public API.\n</user-prompt>")"#
    ]],
        ),
        (
            "behavior_globs_find_only_matching_real_project_files",
            r##"
(progn
  (require 'ai-code-behaviors)
  (let ((root (make-temp-file "ai-code-behavior-glob-" t)))
    (unwind-protect
        (progn
          (make-directory (expand-file-name "src/" root))
          (make-directory (expand-file-name "test/" root))
          (dolist (file '("src/lib.rs" "src/main.rs" "test/lib_test.rs"
                          "README.md" "src/notes.txt"))
            (with-temp-file (expand-file-name file root)
              (insert file)))
          (list
           (ai-code--glob-pattern-p "src/*.rs")
           (ai-code--glob-pattern-p "README.md")
           (ai-code--glob-to-regexp "**/*_test.rs")
           (mapcar
            (lambda (path) (file-relative-name path root))
            (sort (ai-code--expand-glob-in-dir "src/*.rs" root) #'string<))
           (mapcar
            (lambda (path) (file-relative-name path root))
            (sort (ai-code--expand-glob-in-dir "**/*_test.rs" root) #'string<))))
      (delete-directory root t))))
"##,
            true,
            expect![[r#"OK (4 nil ".*.*/.*_test\\.rs" nil nil)"#]],
        ),
        (
            "behavior_json_parser_handles_wrapped_escaped_and_malformed_responses",
            r##"
(progn
  (require 'ai-code-behaviors)
  (mapcar
   #'ai-code--extract-json-from-response
   '("```json\n{\"mode\":\"code\",\"modifiers\":[\"deep\",\"tdd\"]}\n```"
     "Classifier says: {\"mode\":\"review\",\"note\":\"brace } in string\"} trailing prose"
     "{\"mode\":\"debug\",\"nested\":{\"retry\":true}}"
     "not json at all"
     "{\"mode\":\"code\"")))
"##,
            true,
            expect![[
        r#"OK (((mode . "code") (modifiers . ["deep" "tdd"])) ((mode . "review") (note . "brace } in string")) ((mode . "debug") (nested (retry . t))) nil nil)"#
    ]],
        ),
        (
            "behavior_clean_prompt_removes_injected_context_before_classification",
            r##"
(progn
  (require 'ai-code-behaviors)
  (let ((text
         "AdditionalContext: <operating-mode>\nreview instructions\n</operating-mode>\n\n<user-prompt>\nImplement a bounded retry queue and write integration tests.\n</user-prompt>"))
    (list
     (ai-code--extract-clean-user-prompt text)
     (let ((ai-code-use-gptel-classify-prompt nil))
       (ai-code--classify-prompt-intent text)))))
"##,
            true,
            expect![[
        r#"OK ("Implement a bounded retry queue and write integration tests." (:mode "=code" :modifiers nil :confidence high))"#
    ]],
        ),
        (
            "behavior_agent_prompt_vector_reconstruction_preserves_non_text_parts",
            r##"
(progn
  (require 'ai-code-behaviors)
  (let* ((original
          [(:type text :text "Review ")
           (:type image :source "/repo/diagram.png")
           (:type text :text "this #review #deep")])
         (text (ai-code--extract-text-from-prompt-vec original))
         (rebuilt
          (ai-code--reconstruct-prompt-vec
           original
           "Review this carefully"
           '(:temperature 0.1 :stream t))))
    (list text rebuilt)))
"##,
            true,
            expect![[
        r#"OK ("" [((type . "text") (text . "Review this carefully")) (:type text :text "Review ") (:type image :source "/repo/diagram.png") (:type text :text "this #review #deep")])"#
    ]],
        ),
    ]);
}
