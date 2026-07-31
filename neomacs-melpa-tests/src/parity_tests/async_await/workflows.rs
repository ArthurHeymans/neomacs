use expect_test::expect;

use super::assert_async_await_batch;

#[test]
fn workflows_public_surface_batch() {
    assert_async_await_batch(&[
        (
            "delayed_promises_resume_sequential_work_in_exact_source_order",
            r##"(let (events)
          (async-defun parity-sequential-delays ()
            (dolist
                (entry
                 '((0.01 :first)
                   (0.02 :second)
                   (0.01 :third)))
              (push
               (await
                (async-await-test-delay
                 (car entry)
                 (cadr entry)))
               events))
            (nreverse events))
          (async-await-test-settle
           (parity-sequential-delays)))"##,
            true,
            expect!["OK (fulfilled (:fullfilled (:first :second :third)))"],
        ),
        (
            "concurrent_invocations_complete_by_delay_without_cross_contaminating_results",
            r##"(let (completion-order)
          (async-defun parity-concurrent-task
              (label delay multiplier)
            (let ((value
                   (await
                    (async-await-test-delay
                     delay multiplier))))
              (push label completion-order)
              (list label
                    (* value 10))))
          (let* ((slow
                  (parity-concurrent-task
                   :slow 0.06 1))
                 (fast
                  (parity-concurrent-task
                   :fast 0.01 2))
                 (middle
                  (parity-concurrent-task
                   :middle 0.03 3))
                 (outcomes
                  (mapcar
                   #'async-await-test-settle
                   (list slow fast middle))))
            (list
             outcomes
             (nreverse
              completion-order))))"##,
            true,
            expect![
        "OK (((fulfilled (:fullfilled (:slow 10))) (fulfilled (:fullfilled (:fast 20))) (fulfilled (:fullfilled (:middle 30)))) (:fast :middle :slow))"
    ],
        ),
        (
            "async_function_awaits_real_subprocess_stdout_and_preserves_newlines",
            r##"(progn
          (async-defun parity-process-output ()
            (let ((output
                   (await
                    (promise:make-process-string
                     (list
                      "sh" "-c"
                      "printf 'alpha\\nbeta gamma\\n'")))))
              (list
               output
               (split-string
                output "\n" t)
               (length output))))
          (async-await-test-settle
           (parity-process-output)))"##,
            true,
            expect![[
        r#"OK (fulfilled (:fullfilled ("alpha\nbeta gamma\n" ("alpha" "beta gamma") 17)))"#
    ]],
        ),
        (
            "async_function_sends_real_multiline_input_to_subprocess_and_uses_result",
            r##"(progn
          (async-defun parity-process-input (input)
            (let ((result
                   (await
                    (promise:make-process-send-string
                     (list
                      "sh" "-c"
                      "tr '[:lower:]' '[:upper:]'")
                     input))))
              (list
               result
               (car result)
               (cadr result))))
          (async-await-test-settle
           (parity-process-input
            "one two\nthree\n")))"##,
            true,
            expect![[
        r#"OK (fulfilled (:fullfilled (("ONE TWO\nTHREE\n" "") "ONE TWO\nTHREE\n" "")))"#
    ]],
        ),
        (
            "failed_real_subprocess_is_caught_and_normalized_inside_async_function",
            r##"(progn
          (async-defun parity-process-failure ()
            (condition-case reason
                (await
                 (promise:make-process-string
                  (list
                   "sh" "-c"
                   "printf 'diagnostic' >&2; exit 7")))
              (error
               (let ((event
                      (cadr reason)))
                 (list
                  :caught
                  (car reason)
                  (stringp event)
                  (not
                   (null
                    (and
                     (stringp event)
                     (string-match-p
                      "code 7" event)))))))))
          (async-await-test-settle
           (parity-process-failure)))"##,
            true,
            expect!["OK (fulfilled (:fullfilled (:caught error t t)))"],
        ),
        (
            "delayed_filesystem_producer_is_awaited_before_exact_consumer_transform",
            r##"(let ((path
                 (async-await-test-path
                  "async-await-records.txt")))
          (async-defun parity-file-pipeline ()
            (let ((produced
                   (await
                    (promise-new
                     (lambda (resolve _reject)
                       (run-at-time
                        0.01 nil
                        (lambda ()
                          (with-temp-file path
                            (insert
                             "alice,3\n"
                             "bob,8\n"
                             "carol,5\n"))
                          (funcall resolve
                                   path))))))))
              (with-temp-buffer
                (insert-file-contents
                 produced)
                (let (records)
                  (dolist
                      (line
                       (split-string
                        (buffer-string)
                        "\n" t))
                    (pcase-let
                        ((`(,name ,score)
                          (split-string
                           line ",")))
                      (push
                       (list
                        name
                        (string-to-number
                         score))
                       records)))
                  (sort
                   records
                   (lambda (left right)
                     (>
                      (cadr left)
                      (cadr right))))))))
          (let ((outcome
                 (async-await-test-settle
                  (parity-file-pipeline))))
            (list
             outcome
             (file-exists-p path)
             (file-attribute-size
              (file-attributes path)))))"##,
            true,
            expect![[r#"OK ((fulfilled (:fullfilled (("bob" 8) ("carol" 5) ("alice" 3)))) t 22)"#]],
        ),
        (
            "awaited_resume_can_mutate_captured_buffer_then_return_exact_text_properties",
            r##"(let ((buffer
                 (generate-new-buffer
                  " *async-await-parity*")))
          (unwind-protect
              (progn
                (with-current-buffer buffer
                  (insert "start"))
                (async-defun parity-buffer-edit ()
                  (await
                   (async-await-test-delay
                    0.01 :resume))
                  (with-current-buffer buffer
                    (goto-char (point-max))
                    (insert " -> finished")
                    (add-text-properties
                     10 18
                     '(face bold category parity))
                    (list
                     (buffer-string)
                     (get-text-property
                      10 'face)
                     (get-text-property
                      10 'category)
                     (buffer-modified-p))))
                (async-await-test-settle
                 (parity-buffer-edit)))
            (kill-buffer buffer)))"##,
            true,
            expect![[
        r#"OK (fulfilled (:fullfilled (#("start -> finished" 9 17 (category parity face bold)) bold parity t)))"#
    ]],
        ),
        (
            "promise_all_composes_multiple_async_functions_and_preserves_input_order",
            r##"(progn
          (async-defun parity-map-one
              (value delay)
            (let ((resolved
                   (await
                    (async-await-test-delay
                     delay value))))
              (list
               resolved
               (* resolved resolved))))
          (async-defun parity-map-all ()
            (await
             (promise-all
              (vector
               (parity-map-one 2 0.03)
               (parity-map-one 3 0.01)
               (parity-map-one 4 0.02)))))
          (async-await-test-settle
           (parity-map-all)))"##,
            true,
            expect!["OK (fulfilled (:fullfilled [(2 4) (3 9) (4 16)]))"],
        ),
        (
            "async_lambda_forms_a_real_parse_filter_aggregate_pipeline",
            r##"(let* ((parse
                  (async-lambda (line)
                    (pcase-let
                        ((`(,name ,amount)
                          (split-string
                           line ":")))
                      (list
                       name
                       (string-to-number
                        (await amount))))))
                 (pipeline
                  (async-lambda (lines)
                    (let (records)
                      (dolist (line lines)
                        (let ((record
                               (await
                                (funcall
                                 parse line))))
                          (when
                              (>=
                               (cadr record)
                               5)
                            (push record
                                  records))))
                      (let ((ordered
                             (nreverse records)))
                        (list
                         ordered
                         (apply
                          #'+
                          (mapcar
                           #'cadr
                           ordered))))))))
          (async-await-test-settle
           (funcall
            pipeline
            '("alpha:3"
              "beta:8"
              "gamma:5"
              "delta:2"))))"##,
            true,
            expect![[r#"OK (fulfilled (:fullfilled ((("beta" 8) ("gamma" 5)) 13)))"#]],
        ),
        (
            "zero_await_and_empty_iteration_remain_pending_while_nonempty_iteration_resumes",
            r##"(progn
          (async-defun parity-zero-await
              (value)
            (list :sync value))
          (async-defun parity-empty-loop
              (values)
            (let (result)
              (dolist (value values)
                (push
                 (await value)
                 result))
              (nreverse result)))
          (list
           (async-await-test-settle
            (parity-zero-await 9))
           (async-await-test-settle
            (parity-empty-loop nil))
           (async-await-test-settle
            (parity-empty-loop
             '(1 2 3)))))"##,
            true,
            expect!["OK ((rejected #1=(:timeouted)) (rejected #1#) (fulfilled (:fullfilled (1 2 3))))"],
        ),
        (
            "two_stateful_async_workers_keep_independent_lexical_accumulators",
            r##"(cl-labels
          ((make-worker
            (label)
            (let ((total 0))
              (async-lambda (values)
                (let (snapshots)
                  (dolist (value values)
                    (setq total
                          (+
                           total
                           (await
                            (promise-resolve
                             value))))
                    (push
                     (list label total)
                     snapshots))
                  (nreverse
                   snapshots))))))
          (let ((left
                 (make-worker :left))
                (right
                 (make-worker :right)))
            (list
             (async-await-test-settle
              (funcall left '(1 2)))
             (async-await-test-settle
              (funcall right '(10)))
             (async-await-test-settle
              (funcall left '(3)))
             (async-await-test-settle
              (funcall right '(5 5))))))"##,
            true,
            expect![
        "OK ((fulfilled (:fullfilled ((:left 1) (:left 3)))) (fulfilled (:fullfilled ((:right 10)))) (fulfilled (:fullfilled ((:left 6)))) (fulfilled (:fullfilled ((:right 15) (:right 20)))))"
    ],
        ),
    ]);
}
