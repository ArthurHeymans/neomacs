use expect_test::expect;

use super::assert_aio_parity;

#[test]
fn aio_select_add_remove_and_level_triggered_reuse_match() {
    let elisp_form = r##"(let* ((one (aio-promise))
                          (two (aio-promise))
                          (three (aio-promise))
                          (select (aio-make-select (list one two))))
                      (aio-select-add select three)
                      (aio-select-remove select two)
                      (aio-resolve two (lambda () :removed))
                      (aio-resolve three (lambda () :three))
                      (aio-wait-for (aio-sleep 0))
                      (let ((winner-one
                             (aio-wait-for (aio-select select))))
                        (aio-resolve one (lambda () :one))
                        (aio-wait-for (aio-sleep 0))
                        (let ((winner-two
                               (aio-wait-for (aio-select select))))
                          (list
                           (funcall (aio-result winner-one))
                           (funcall (aio-result winner-two))
                           (aio-select-promises select)
                           (hash-table-count
                            (aio-select-members select))))))"##;
    let expect = expect!["OK (:three :one nil 0)"];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_select_timer_race_reports_timeout_and_success_without_network() {
    let elisp_form = r##"(let* ((slow (aio-sleep 0.02 :slow))
                          (timeout (aio-timeout 0))
                          (first-select
                           (aio-make-select (list slow timeout)))
                          (first
                           (aio-wait-for
                            (aio-select first-select)))
                          (fast (aio-sleep 0 :fast))
                          (late-timeout (aio-timeout 0.02))
                          (second-select
                           (aio-make-select
                            (list fast late-timeout)))
                          (second
                           (aio-wait-for
                            (aio-select second-select))))
                      (list
                       (aio-wait-for (aio-catch first))
                       (aio-wait-for (aio-catch second))))"##;
    let expect = expect!["OK ((:error aio-timeout . 0) (:success . :fast))"];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_semaphore_releases_waiters_fifo_and_preserves_counter() {
    let elisp_form = r##"(let ((sem (aio-sem 1))
                          (events nil)
                          workers)
                      (dotimes (index 6)
                        (push
                         (aio-with-async
                           (aio-await (aio-sem-wait sem))
                           (push index events)
                           (aio-sem-post sem)
                           index)
                         workers))
                      (setq workers (nreverse workers))
                      (aio-wait-for
                       (aio-with-async
                         (aio-await (aio-all workers))))
                      (list
                       (nreverse events)
                       (mapcar
                        (lambda (promise)
                          (funcall (aio-result promise)))
                        workers)
                       (aio-sem-value sem)
                       (aio-sem-queue sem)))"##;
    let expect = expect!["OK ((0 1 2 3 4 5) (0 1 2 3 4 5) 1 (nil))"];
    assert_aio_parity(elisp_form, expect);
}
