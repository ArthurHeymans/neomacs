use expect_test::expect;

use super::{assert_ast_grep_consult_batch, assert_ast_grep_helm_batch, assert_ast_grep_ivy_batch, assert_ast_grep_batch};

#[test]
fn backends_ast_grep_batch() {
    assert_ast_grep_batch(&[
        (
            "ast_grep_backend_selection_covers_forced_fallbacks_and_auto_ui_priority",
            r##"(let (consult ivy helm)
          (cl-letf (((symbol-function 'ast-grep--consult-backend-available-p)
                     (lambda () consult))
                    ((symbol-function 'ast-grep--ivy-backend-available-p)
                     (lambda () ivy))
                    ((symbol-function 'ast-grep--helm-backend-available-p)
                     (lambda () helm)))
            (mapcar
             (lambda (scenario)
               (setq ast-grep-search-backend (nth 0 scenario)
                     ivy-mode (nth 1 scenario)
                     helm-mode (nth 2 scenario)
                     consult (nth 3 scenario)
                     ivy (nth 4 scenario)
                     helm (nth 5 scenario))
               (list scenario
                     (ast-grep--select-backend)
                     (ast-grep--backend-description)))
             '((sync nil nil t t t)
               (consult nil nil t nil nil)
               (consult nil nil nil t t)
               (ivy nil nil t t nil)
               (ivy nil nil t nil t)
               (helm nil nil t nil t)
               (helm nil nil t t nil)
               (auto t nil t t t)
               (auto t nil t nil t)
               (auto nil t t t t)
               (auto nil t t t nil)
               (auto nil nil t t t)
               (auto nil nil nil t t)))))"##,
            true,
            expect![[
        r#"OK (((sync nil nil t t t) sync "ast-grep backend: sync") ((consult nil nil t nil nil) consult "ast-grep backend: consult") ((consult nil nil nil t t) sync "ast-grep backend: consult -> sync") ((ivy nil nil t t nil) ivy "ast-grep backend: ivy") ((ivy nil nil t nil t) sync "ast-grep backend: ivy -> sync") ((helm nil nil t nil t) helm "ast-grep backend: helm") ((helm nil nil t t nil) sync "ast-grep backend: helm -> sync") ((auto t nil t t t) ivy "ast-grep backend: auto -> ivy") ((auto t nil t nil t) sync "ast-grep backend: auto -> sync") ((auto nil t t t t) helm "ast-grep backend: auto -> helm") ((auto nil t t t nil) sync "ast-grep backend: auto -> sync") ((auto nil nil t t t) consult "ast-grep backend: auto -> consult") ((auto nil nil nil t t) sync "ast-grep backend: auto -> sync"))"#
    ]],
        ),
        (
            "ast_grep_backend_runner_dispatches_exact_adapter_and_directory",
            r##"(let (calls)
          (cl-letf (((symbol-function 'require)
                     (lambda (feature &rest _)
                       (push (list :require feature) calls)
                       t))
                    ((symbol-function 'ast-grep--search-consult)
                     (lambda (directory)
                       (push (list :consult directory) calls)
                       :consult-result))
                    ((symbol-function 'ast-grep--search-ivy)
                     (lambda (directory)
                       (push (list :ivy directory) calls)
                       :ivy-result))
                    ((symbol-function 'ast-grep--search-helm)
                     (lambda (directory)
                       (push (list :helm directory) calls)
                       :helm-result))
                    ((symbol-function 'ast-grep--search-sync)
                     (lambda (directory)
                       (push (list :sync directory) calls)
                       :sync-result)))
            (list
             (mapcar
              (lambda (backend)
                (ast-grep--run-search-backend
                 backend "/fixture/project/"))
              '(consult ivy helm sync unknown))
             (nreverse calls))))"##,
            true,
            expect![[
        r#"OK ((:consult-result :ivy-result :helm-result :sync-result nil) ((:require ast-grep-consult) (:consult "/fixture/project/") (:require ast-grep-ivy) (:ivy "/fixture/project/") (:require ast-grep-helm) (:helm "/fixture/project/") (:sync "/fixture/project/")))"#
    ]],
        ),
        (
            "ast_grep_describe_backend_returns_and_messages_the_same_resolution",
            r##"(let ((ast-grep-search-backend 'consult)
               messages)
          (cl-letf (((symbol-function 'ast-grep--select-backend)
                     (lambda () 'sync))
                    ((symbol-function 'message)
                     (lambda (format-string &rest args)
                       (push
                        (apply #'format format-string args)
                        messages))))
            (list
             (ast-grep-describe-backend)
             (nreverse messages))))"##,
            true,
            expect![[
        r#"OK ("ast-grep backend: consult -> sync" ("ast-grep backend: consult -> sync"))"#
    ]],
        ),
        (
            "ast_grep_main_backend_availability_adapters_require_modules_and_forward_results",
            r##"(let (calls)
          (cl-letf (((symbol-function 'require)
                     (lambda (feature &rest _)
                       (push (list :require feature) calls)
                       (not (eq feature 'ast-grep-ivy))))
                    ((symbol-function 'ast-grep--consult-available-p)
                     (lambda ()
                       (push :consult-probe calls)
                       :consult-ready))
                    ((symbol-function 'ast-grep--ivy-available-p)
                     (lambda ()
                       (push :ivy-probe calls)
                       :ivy-ready))
                    ((symbol-function 'ast-grep--helm-available-p)
                     (lambda ()
                       (push :helm-probe calls)
                       :helm-ready)))
            (list
             (ast-grep--consult-backend-available-p)
             (ast-grep--ivy-backend-available-p)
             (ast-grep--helm-backend-available-p)
             (nreverse calls))))"##,
            true,
            expect![
        "OK (:consult-ready nil :helm-ready ((:require ast-grep-consult) :consult-probe (:require ast-grep-ivy) (:require ast-grep-helm) :helm-probe))"
    ],
        ),
    ]);
}

#[test]
fn backends_ast_grep_consult_batch() {
    assert_ast_grep_consult_batch(&[
        (
            "ast_grep_consult_availability_probe_returns_exact_require_result",
            r##"(let (calls)
          (cl-letf (((symbol-function 'require)
                     (lambda (feature filename noerror)
                       (push
                        (list feature filename noerror)
                        calls)
                       (eq feature 'consult))))
            (list
             (ast-grep--consult-available-p)
             (nreverse calls))))"##,
            true,
            expect!["OK (t ((consult nil t)))"],
        ),
        (
            "ast_grep_consult_async_builder_enforces_minimum_and_builds_exact_argv",
            r##"(let ((ast-grep-async-min-input 3)
               (ast-grep-executable "sg"))
          (mapcar
           (lambda (input)
             (ast-grep--async-builder input "/fixture/root"))
           '(nil "" "a" "ab" "abc" "αβγ" "console.log($A)")))"##,
            true,
            expect![[
        r#"OK (nil nil nil nil ("sg" "run" "--pattern=abc" "--json=stream" "/fixture/root") ("sg" "run" "--pattern=αβγ" "--json=stream" "/fixture/root") ("sg" "run" "--pattern=console.log($A)" "--json=stream" "/fixture/root"))"#
    ]],
        ),
        (
            "ast_grep_consult_async_source_builds_real_pipeline_and_transforms_json_items",
            r##"(let (calls)
          (cl-letf (((symbol-function 'consult--async-throttle)
                     (lambda ()
                       (push :throttle calls)
                       'throttle-stage))
                    ((symbol-function 'consult--async-process)
                     (lambda (builder)
                       (push
                        (list :process
                              (funcall builder "ab")
                              (funcall builder "abc"))
                        calls)
                       'process-stage))
                    ((symbol-function 'consult--async-transform)
                     (lambda (transform)
                       (push
                        (list
                         :transform
                         (mapcar
                          #'substring-no-properties
                          (funcall
                           transform
                           '("{\"file\":\"a.rs\",\"range\":{\"start\":{\"line\":1,\"column\":2}},\"text\":\"one\"}"
                             "bad-json"
                             "{\"file\":\"b.rs\",\"range\":{\"start\":{\"line\":3,\"column\":4}},\"text\":\"two\"}"))))
                        calls)
                       'transform-stage))
                    ((symbol-function 'consult--async-pipeline)
                     (lambda (&rest stages)
                       (push (list :pipeline stages) calls)
                       (list :source stages))))
            (list
             (ast-grep--async-source "/fixture/")
             (nreverse calls)
             (hash-table-count ast-grep--candidate-table))))"##,
            true,
            expect![[
        r#"OK ((:source #1=(throttle-stage process-stage transform-stage)) (:throttle (:process nil ("ast-grep" "run" "--pattern=abc" "--json=stream" "/fixture/")) (:transform ("a.rs:2:2:one" "b.rs:4:4:two")) (:pipeline #1#)) 2)"#
    ]],
        ),
        (
            "ast_grep_consult_state_previews_structured_match_at_character_column_and_cleans_up",
            r##"(let* ((file
                (ast-grep-test-write-file
                 "consult/preview.rs"
                 "zero\n\tα界target()\n"))
               (buffer (find-file-noselect file))
               (candidate
                (ast-grep--format-candidate
                 (list :file file :start-line 1 :start-column 3
                       :text "target()")))
               calls)
          (unwind-protect
              (cl-letf (((symbol-function 'consult--file-preview)
                         (lambda ()
                           (lambda (action item)
                             (push (list action
                                         (if (equal item file) :file item))
                                   calls)
                             (when (and (eq action 'preview)
                                        (equal item file))
                               buffer))))
                        ((symbol-function 'pulse-momentary-highlight-one-line)
                         (lambda (position)
                           (push (list :pulse position) calls))))
                (let ((state (ast-grep--state)))
                  (list
                   (funcall state 'preview candidate)
                   (with-current-buffer buffer
                     (list
                      (line-number-at-pos)
                      (- (point) (line-beginning-position))
                      (char-after)))
                   (funcall state 'return candidate)
                   (funcall state 'exit nil)
                   (nreverse calls))))
            (ast-grep-test-kill-file-buffer file)))"##,
            true,
            expect!["OK (9 (2 3 116) nil nil ((preview :file) (:pulse 9) (return :file) (exit nil)))"],
        ),
        (
            "ast_grep_consult_search_wires_source_options_annotation_and_selected_jump",
            r##"(let* ((candidate
                (ast-grep--format-candidate
                 '(:file "src/a.rs" :start-line 2 :start-column 4
                   :text "target")))
               (ast-grep-use-nerd-icons nil)
               calls
               jumped)
          (cl-letf (((symbol-function 'ast-grep--async-source)
                     (lambda (directory)
                       (push (list :source directory) calls)
                       'fixture-source))
                    ((symbol-function 'ast-grep--state)
                     (lambda () 'fixture-state))
                    ((symbol-function 'consult--lookup-member)
                     #'identity)
                    ((symbol-function 'consult--read)
                     (lambda (source &rest options)
                       (push
                        (list
                         :read source
                         options
                         :annotation
                         (funcall
                          (plist-get options :annotate)
                          candidate))
                        calls)
                       candidate))
                    ((symbol-function 'ast-grep--goto-match)
                     (lambda (selection)
                       (setq jumped
                             (ast-grep-test-match-summary selection)))))
            (puthash "stale" 'old ast-grep--candidate-table)
            (list
             (ast-grep--search-consult "/fixture/")
             (nreverse calls)
             jumped
             (gethash "stale" ast-grep--candidate-table))))"##,
            true,
            expect![[
        r#"OK (#1=("src/a.rs" 2 4 nil nil "target" nil) ((:source "/fixture/") (:read fixture-source (:prompt "ast-grep: " :lookup consult--lookup-member :state fixture-state :annotate #[(cand) ((list cand (ast-grep--candidate-icon-prefix cand) "")) (t)] :category ast-grep :history ast-grep-history :require-match t) :annotation (#("src/a.rs:3:4:target" 0 19 (ast-grep-match (:file "src/a.rs" :start-line 2 :start-column 4 :text "target"))) "" ""))) #1# nil)"#
    ]],
        ),
    ]);
}

#[test]
fn backends_ast_grep_ivy_batch() {
    assert_ast_grep_ivy_batch(&[
        (
            "ast_grep_ivy_threshold_generation_and_shell_quoting_match_real_input_rules",
            r##"(let ((ast-grep-async-min-input 4)
               (ast-grep--ivy-generation 40))
          (list
           (mapcar
            #'ast-grep--ivy-more-chars
            '("" "a" "abc" "abcd" "α界"))
           (ast-grep--ivy-next-generation)
           (ast-grep--ivy-next-generation)
           (ast-grep--command-shell-string
            '("sg tool" "run" "--pattern=a b;nope" "/a dir"))))"##,
            true,
            expect![[
        r#"OK ((("" "4 chars more") ("" "3 chars more") ("" "1 chars more") nil ("" "2 chars more")) 41 42 "sg\\ tool run --pattern\\=a\\ b\\;nope /a\\ dir")"#
    ]],
        ),
        (
            "ast_grep_ivy_availability_probe_requires_both_packages_with_short_circuiting",
            r##"(let (available calls)
          (cl-letf (((symbol-function 'require)
                     (lambda (feature filename noerror)
                       (push
                        (list feature filename noerror)
                        calls)
                       (cdr (assq feature available)))))
            (setq available '((ivy . nil) (counsel . t)))
            (let ((ivy-missing
                   (list
                    (ast-grep--ivy-available-p)
                    (nreverse calls))))
              (setq available '((ivy . t) (counsel . nil))
                    calls nil)
              (let ((counsel-missing
                     (list
                      (ast-grep--ivy-available-p)
                      (nreverse calls))))
                (setq available '((ivy . t) (counsel . ready))
                      calls nil)
                (list
                 ivy-missing
                 counsel-missing
                 (list
                  (ast-grep--ivy-available-p)
                  (nreverse calls)))))))"##,
            true,
            expect![
        "OK ((nil ((ivy nil t))) (nil ((ivy nil t) (counsel nil t))) (ready ((ivy nil t) (counsel nil t))))"
    ],
        ),
        (
            "ast_grep_ivy_timer_and_process_lifecycle_cancels_work_and_rejects_stale_owners",
            r##"(let* ((ast-grep--ivy-generation 7)
               (process
                (start-process
                 ast-grep--ivy-process-name
                 nil
                 "sh" "-c"
                 "while read line; do :; done"))
               (other
                (start-process
                 "ast-grep-other-process"
                 nil
                 "sh" "-c"
                 "while read line; do :; done"))
               (timer (run-at-time 60 nil #'ignore))
               deleted)
          (unwind-protect
              (progn
                (setq counsel--async-timer timer)
                (let ((ownership
                       (list
                        (ast-grep--ivy-current-process-p process 7)
                        (ast-grep--ivy-current-process-p process 6)
                        (ast-grep--ivy-current-process-p other 7))))
                  (cl-letf (((symbol-function 'counsel-delete-process)
                             (lambda ()
                               (setq deleted t))))
                    (ast-grep--ivy-stop-process)
                    (list
                     ownership
                     counsel--async-timer
                     (timerp timer)
                     deleted))))
            (when (timerp counsel--async-timer)
              (cancel-timer counsel--async-timer))
            (when (process-live-p process)
              (delete-process process))
            (when (process-live-p other)
              (delete-process other))))"##,
            true,
            expect!["OK ((t nil nil) nil t t)"],
        ),
        (
            "ast_grep_ivy_async_filter_buffers_partial_json_and_forwards_complete_candidates",
            r##"(let* ((process
                (start-process
                 "ast-grep-ivy-filter-fixture"
                 nil
                 "sh" "-c" "exit 0"))
               forwarded)
          (unwind-protect
              (cl-letf (((symbol-function 'counsel--async-filter)
                         (lambda (proc output)
                           (push
                            (list
                             (eq proc process)
                             output)
                            forwarded))))
                (process-put process 'ast-grep--pending "")
                (ast-grep--ivy-async-filter
                 process
                 "{\"file\":\"a.rs\",\"range\":{\"start\":{\"line\":1,\"column\":2}},\"text\":\"one")
                (ast-grep--ivy-async-filter
                 process
                 "\"}\ninvalid\n{\"file\":\"b.rs\",\"range\":{\"start\":{\"line\":3,\"column\":4}},\"text\":\"two\"}\npartial")
                (list
                 (nreverse forwarded)
                 (process-get process 'ast-grep--pending)
                 (hash-table-count ast-grep--candidate-table)
                 (ast-grep-test-match-summary "a.rs:2:2:one")
                 (ast-grep-test-match-summary "b.rs:4:4:two")))
            (when (process-live-p process)
              (delete-process process))))"##,
            true,
            expect![[
        r#"OK (((t "a.rs:2:2:one\nb.rs:4:4:two\n")) "partial" 2 ("a.rs" 1 2 nil nil "one" nil) ("b.rs" 3 4 nil nil "two" nil))"#
    ]],
        ),
        (
            "ast_grep_ivy_collection_cancels_stale_work_and_starts_exact_async_command",
            r##"(let ((ast-grep-async-min-input 3)
               (ast-grep-executable "sg")
               (ast-grep--ivy-generation 0)
               calls)
          (cl-letf (((symbol-function 'ast-grep--ivy-stop-process)
                     (lambda ()
                       (push :stop calls)))
                    ((symbol-function 'counsel--async-command)
                     (lambda (command buffer filter)
                       (push
                        (list :async command buffer
                              (functionp filter))
                        calls)
                       :started)))
            (puthash "stale" 'old ast-grep--candidate-table)
            (let ((collection
                   (ast-grep--ivy-collection "/fixture/root")))
              (list
               (funcall collection "a")
               (hash-table-count ast-grep--candidate-table)
               (funcall collection "abc")
               ast-grep--ivy-generation
               (nreverse calls)))))"##,
            true,
            expect![[
        r#"OK (("" "2 chars more") 0 nil 2 (:stop :stop (:async "sg run --pattern\\=abc --json\\=stream /fixture/root" nil t)))"#
    ]],
        ),
        (
            "ast_grep_ivy_action_transformer_and_search_wire_real_candidate_contract",
            r##"(let* ((candidate
                (ast-grep--format-candidate
                 '(:file "src/a.rs" :start-line 2 :start-column 4
                   :text "target")))
               (ast-grep-use-nerd-icons nil)
               calls)
          (cl-letf (((symbol-function 'ast-grep--goto-match)
                     (lambda (selection)
                       (push
                        (list :goto
                              (ast-grep-test-match-summary selection))
                        calls)))
                    ((symbol-function 'pulse-momentary-highlight-one-line)
                     (lambda (position)
                       (push (list :pulse position) calls)))
                    ((symbol-function 'point)
                     (lambda () 77))
                    ((symbol-function 'ivy-set-display-transformer)
                     (lambda (&rest args)
                       (push (cons :transformer args) calls)))
                    ((symbol-function 'ivy-read)
                     (lambda (prompt collection &rest options)
                       (push
                        (list :read prompt
                              (functionp collection)
                              options)
                        calls)
                       :ivy-result))
                    ((symbol-function 'ast-grep--ivy-stop-process)
                     (lambda ()
                       (push :stop calls))))
            (let ((action (ast-grep--ivy-action candidate))
                  (display
                   (ast-grep--ivy-display-transformer candidate)))
              (puthash "stale" 'old ast-grep--candidate-table)
              (list
               action
               (substring-no-properties display)
               (ast-grep--search-ivy "/fixture/")
               (gethash "stale" ast-grep--candidate-table)
               (nreverse calls)))))"##,
            true,
            expect![[
        r#"OK (#1=((:pulse 77) (:transformer ast-grep-search ast-grep--ivy-display-transformer) (:read "ast-grep: " t (:dynamic-collection t :action ast-grep--ivy-action :update-fn auto :unwind #[nil ((ast-grep--ivy-next-generation) (ast-grep--ivy-stop-process)) (t)] :history ast-grep-history :require-match t :caller ast-grep-search))) "src/a.rs:3:4:target" :ivy-result nil ((:goto ("src/a.rs" 2 4 nil nil "target" nil)) . #1#))"#
    ]],
        ),
    ]);
}

#[test]
fn backends_ast_grep_helm_batch() {
    assert_ast_grep_helm_batch(&[
        (
            "ast_grep_helm_command_uses_display_width_so_cjk_matches_helm_gate",
            r##"(let ((ast-grep-async-min-input 3)
               (ast-grep-executable "sg"))
          (mapcar
           (lambda (input)
             (list
              input
              (and input (length input))
              (and input (string-width input))
              (ast-grep-test-error-data
               (lambda ()
                 (ast-grep--helm-command input "/fixture/")))))
           '(nil "" "ab" "abc" "界" "界a" "αβγ")))"##,
            true,
            expect![[
        r#"OK ((nil nil nil (:ok nil)) ("" 0 0 (:ok nil)) ("ab" 2 2 (:ok nil)) ("abc" 3 3 (:ok ("sg" "run" "--pattern=abc" "--json=stream" "/fixture/"))) ("界" 1 2 (:ok nil)) ("界a" 2 3 (:ok ("sg" "run" "--pattern=界a" "--json=stream" "/fixture/"))) ("αβγ" 3 3 (:ok ("sg" "run" "--pattern=αβγ" "--json=stream" "/fixture/"))))"#
    ]],
        ),
        (
            "ast_grep_helm_availability_probe_requires_module_and_checks_both_entry_points",
            r##"(let (load-result definitions calls)
          (cl-letf (((symbol-function 'require)
                     (lambda (feature filename noerror)
                       (push
                        (list feature filename noerror)
                        calls)
                       load-result))
                    ((symbol-function 'fboundp)
                     (lambda (function)
                       (memq function definitions))))
            (mapcar
             (lambda (scenario)
               (setq load-result (nth 0 scenario)
                     definitions (nth 1 scenario)
                     calls nil)
               (list
                scenario
                (ast-grep--helm-available-p)
                (nreverse calls)))
             '((nil (helm helm-make-source))
               (t (helm))
               (t (helm-make-source))
               (t (helm helm-make-source))))))"##,
            true,
            expect![
        "OK (((nil (helm helm-make-source)) nil ((helm nil t))) ((t (helm)) nil ((helm nil t))) ((t (helm-make-source)) nil ((helm nil t))) ((t (helm . #1=(helm-make-source))) #1# ((helm nil t))))"
    ],
        ),
        (
            "ast_grep_helm_candidates_process_runs_real_program_with_exact_argv",
            r##"(let* ((log (ast-grep-test-path "helm-argv.log"))
               (program
                (ast-grep-test-make-executable
                 "sg-helm"
                 (format
                  "printf '%%s\\n' \"$@\" > %s"
                  (shell-quote-argument log))))
               (ast-grep-executable program)
               (ast-grep-async-min-input 3)
               process)
          (setq helm-pattern "console.log($A)")
          (setq process
                (ast-grep--helm-candidates-process
                 "/fixture/project"))
          (unwind-protect
              (progn
                (while (process-live-p process)
                  (accept-process-output process 0.05))
                (list
                 (processp process)
                 (process-name process)
                 (process-status process)
                 (ast-grep-test-read-file log)
                 (hash-table-count ast-grep--candidate-table)))
            (makunbound 'helm-pattern)))"##,
            true,
            expect![[
        r#"OK (t "ast-grep-helm" exit "run\n--pattern=console.log($A)\n--json=stream\n/fixture/project\n" 0)"#
    ]],
        ),
        (
            "ast_grep_helm_filter_display_preview_and_cleanup_manage_real_file_buffers",
            r##"(let* ((file
                (ast-grep-test-write-file
                 "helm/preview.rs"
                 "zero\n  target()\n"))
               (line
                (format
                 "{\"file\":%S,\"range\":{\"start\":{\"line\":1,\"column\":2}},\"text\":\"target()\"}"
                 file))
               (ast-grep-use-nerd-icons nil)
               (pair (ast-grep--helm-filter-one-by-one line))
               pulses)
          (unwind-protect
              (cl-letf (((symbol-function 'pulse-momentary-highlight-one-line)
                         (lambda (position)
                           (push position pulses))))
                (ast-grep--helm-preview (cdr pair))
                (let ((during
                       (list
                        (buffer-live-p (find-buffer-visiting file))
                        (line-number-at-pos)
                        (- (point) (line-beginning-position))
                        (length ast-grep--helm-preview-buffers)
                        (nreverse pulses))))
                  (ast-grep--helm-cleanup)
                  (list
                   (substring-no-properties (car pair))
                   (substring-no-properties (cdr pair))
                   during
                   (find-buffer-visiting file)
                   ast-grep--helm-preview-buffers
                   (progn
                     (ast-grep--helm-action (cdr pair))
                     (list
                      (buffer-live-p (find-buffer-visiting file))
                      (line-number-at-pos)
                      (- (point) (line-beginning-position))
                      (nreverse pulses))))))
            (ast-grep-test-kill-file-buffer file)))"##,
            true,
            expect![[
        r#"OK ("[ORACLE-SANDBOX]/helm/preview.rs:2:2:target()" "[ORACLE-SANDBOX]/helm/preview.rs:2:2:target()" (t 2 2 1 #1=(8 8)) nil nil (t 2 2 #1#))"#
    ]],
        ),
        (
            "ast_grep_helm_source_and_search_wire_all_async_preview_contract_slots",
            r##"(let (calls)
          (cl-letf (((symbol-function 'ast-grep--helm-ensure-function)
                     (lambda (function)
                       (push (list :ensure function) calls)))
                    ((symbol-function 'helm-make-source)
                     (lambda (name class &rest options)
                       (push
                        (list :make name class options)
                        calls)
                       (list :source name class options)))
                    ((symbol-function 'helm)
                     (lambda (&rest options)
                       (push (list :helm options) calls)
                       :helm-result)))
            (puthash "stale" 'old ast-grep--candidate-table)
            (let ((source
                   (ast-grep--helm-source "/fixture/project/")))
              (list
               (mapcar
                (lambda (key)
                  (list key
                        (plist-get (nthcdr 3 source) key)))
                '(:follow :requires-pattern :nohighlight :nomark))
               (ast-grep--search-helm "/fixture/project/")
               (gethash "stale" ast-grep--candidate-table)
               (nreverse calls)))))"##,
            true,
            expect![[
        r#"OK (((:follow nil) (:requires-pattern nil) (:nohighlight nil) (:nomark nil)) :helm-result nil ((:ensure helm-make-source) (:make "ast-grep" helm-source-async (:candidates-process #[nil #1=((ast-grep--helm-candidates-process directory)) ((directory . "/fixture/project/"))] :filter-one-by-one ast-grep--helm-filter-one-by-one :action ast-grep--helm-action :persistent-action ast-grep--helm-preview :persistent-help "Preview match" :cleanup ast-grep--helm-cleanup :follow 1 :requires-pattern 3 :nohighlight t :nomark t)) (:ensure helm) (:ensure helm-make-source) (:make "ast-grep" helm-source-async #2=(:candidates-process #[nil #1# ((directory . "/fixture/project/"))] :filter-one-by-one ast-grep--helm-filter-one-by-one :action ast-grep--helm-action :persistent-action ast-grep--helm-preview :persistent-help "Preview match" :cleanup ast-grep--helm-cleanup :follow 1 :requires-pattern 3 :nohighlight t :nomark t)) (:helm (:sources (:source "ast-grep" helm-source-async #2#) :prompt "ast-grep: " :buffer "*helm ast-grep*" :history ast-grep-history))))"#
    ]],
        ),
        (
            "ast_grep_helm_short_pattern_and_missing_function_fail_with_precise_contract",
            r##"(let ((ast-grep-async-min-input 3))
          (setq helm-pattern "ab")
          (unwind-protect
              (list
               (ast-grep-test-error-data
                (lambda ()
                  (ast-grep--helm-candidates-process "/fixture/")))
               (cl-letf (((symbol-function 'require)
                          (lambda (&rest _) nil)))
                 (ast-grep-test-error-data
                  (lambda ()
                    (ast-grep--helm-ensure-function
                     'ast-grep-certainly-missing)))))
            (makunbound 'helm-pattern)))"##,
            true,
            expect![[
        r#"OK ((:error error ("Helm pattern is shorter than ‘ast-grep-async-min-input’")) (:error error ("Helm function ‘ast-grep-certainly-missing’ is not available")))"#
    ]],
        ),
    ]);
}
