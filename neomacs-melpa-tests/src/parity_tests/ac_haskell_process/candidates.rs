use expect_test::expect;

use super::{assert_ac_haskell_process_parity, assert_ac_haskell_process_signal_parity};

#[test]
fn ac_haskell_process_candidates_short_circuits_every_other_callback_without_session() {
    let elisp_form = r##"(let ((ac-prefix
                    (propertize
                     "fixture"
                     'origin 'prefix))
                   calls)
               (cl-letf
                   (((symbol-function
                      'haskell-session-maybe)
                     (lambda ()
                       (push 'session calls)
                       nil))
                    ((symbol-function
                      'haskell-process)
                     (lambda ()
                       (push 'process calls)
                       'unexpected))
                    ((symbol-function
                      'haskell-process-get-repl-completions)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       'unexpected)))
                 (list
                  (ac-haskell-process-candidates)
                  (nreverse calls)
                  ac-prefix)))"##;
    let expect = expect![[r#"OK (nil (session) #("fixture" 0 7 (origin prefix)))"#]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_candidates_forwards_non_import_prefix_process_and_result_identity() {
    let elisp_form = r##"(with-temp-buffer
               (insert "value")
               (goto-char
                (point-max))
               (let ((ac-prefix
                      (propertize
                       "val"
                       'fixture '(prefix value)))
                     (session
                      (list 'session))
                     (process
                      (list 'process))
                     (candidates
                      (list
                       (propertize
                        "value"
                        'summary "candidate")))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'haskell-session-maybe)
                       (lambda ()
                         (push
                          '(session)
                          calls)
                         session))
                      ((symbol-function
                        'haskell-process)
                       (lambda ()
                         (push
                          '(process)
                          calls)
                         process))
                      ((symbol-function
                        'haskell-process-get-repl-completions)
                       (lambda
                           (actual-process prefix)
                         (push
                          (list
                           'complete
                           actual-process
                           prefix
                           (text-properties-at
                            0 prefix))
                          calls)
                         candidates)))
                   (let ((result
                          (ac-haskell-process-candidates)))
                     (list
                      result
                      (eq result candidates)
                      (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK ((#("value" 0 5 (summary "candidate"))) t ((session) (process) (complete (process) #("val" 0 3 (fixture (prefix value))) (fixture (prefix value)))))"#
    ]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_candidates_uses_line_prefix_for_exact_import_space_and_tab_forms() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert
                    (car fixture))
                   (goto-char
                    (or
                     (cdr fixture)
                     (point-max)))
                   (let ((ac-prefix
                          "fallback")
                         observed)
                     (cl-letf
                         (((symbol-function
                            'haskell-session-maybe)
                           (lambda ()
                             'session))
                          ((symbol-function
                            'haskell-process)
                           (lambda ()
                             'process))
                          ((symbol-function
                            'haskell-process-get-repl-completions)
                           (lambda
                               (_process prefix)
                             (setq observed prefix)
                             '(candidate))))
                       (list
                        (car fixture)
                        (point)
                        (ac-haskell-process-candidates)
                        observed)))))
               '(("import Data.List")
                 ("import\tData.Map")
                 ("imported Data.Set")
                 (" import Data.Text")
                 ("Import Data.Char")
                 ("import Data.ByteString"
                  . 12)))"##;
    let expect = expect![[
        r#"OK (("import Data.List" 17 #1=(candidate) "import Data.List") ("import\11Data.Map" 16 #1# "import\11Data.Map") ("imported Data.Set" 18 #1# "fallback") (" import Data.Text" 18 #1# "fallback") ("Import Data.Char" 17 #1# "Import Data.Char") ("import Data.ByteString" 12 #1# "import Data"))"#
    ]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_import_prefix_preserves_buffer_text_properties_and_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                (propertize
                 "import Data.Li"
                 'fixture '(buffer prefix)))
               (goto-char
                (point-max))
               (let ((original-point
                      (point))
                     (ac-prefix "fallback")
                     observed)
                 (cl-letf
                     (((symbol-function
                        'haskell-session-maybe)
                       (lambda () 'session))
                      ((symbol-function
                        'haskell-process)
                       (lambda () 'process))
                      ((symbol-function
                        'haskell-process-get-repl-completions)
                       (lambda (_process prefix)
                         (setq observed prefix)
                         nil)))
                   (list
                    (ac-haskell-process-candidates)
                    observed
                    (text-properties-at
                     0 observed)
                    (point)
                    original-point))))"##;
    let expect = expect![[
        r#"OK (nil #("import Data.Li" 0 14 (fixture (buffer prefix))) (fixture (buffer prefix)) 15 15)"#
    ]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_candidates_propagates_completion_signals_with_exact_arguments() {
    let elisp_form = r##"(with-temp-buffer
               (insert "plain")
               (let ((ac-prefix "fixture"))
                 (cl-letf
                     (((symbol-function
                        'haskell-session-maybe)
                       (lambda () 'session))
                      ((symbol-function
                        'haskell-process)
                       (lambda () 'process))
                      ((symbol-function
                        'haskell-process-get-repl-completions)
                       (lambda (&rest arguments)
                         (signal
                          'error
                          (list
                           "fixture completion failure"
                           arguments)))))
                   (ac-haskell-process-candidates))))"##;
    let expect = expect![[r#"ERR (error "fixture completion failure" (process "fixture"))"#]];

    assert_ac_haskell_process_signal_parity(elisp_form, expect);
}
