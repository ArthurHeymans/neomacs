use expect_test::expect;

use super::{assert_ac_haskell_process_parity, assert_ac_haskell_process_signal_parity};

#[test]
fn ac_haskell_process_doc_short_circuits_shell_work_when_hoogle_is_missing() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'executable-find)
                     (lambda (executable)
                       (push
                        (list
                         'find executable)
                        calls)
                       nil))
                    ((symbol-function
                      'shell-command-to-string)
                     (lambda (command)
                       (push
                        (list
                         'shell command)
                        calls)
                       "unexpected")))
                 (list
                  (ac-haskell-process-doc
                   'non-string-is-not-read)
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil ((find "hoogle")))"#]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_doc_shell_quotes_symbol_and_returns_shell_result_identity() {
    let elisp_form = r##"(let ((output
                    (propertize
                     "fixture docs\n"
                     'face 'documentation))
                   calls)
               (cl-letf
                   (((symbol-function
                      'executable-find)
                     (lambda (executable)
                       (push
                        (list
                         'find executable)
                        calls)
                       "/fixture/bin/hoogle"))
                    ((symbol-function
                      'shell-command-to-string)
                     (lambda (command)
                       (push
                        (list
                         'shell command)
                        calls)
                       output)))
                 (let ((result
                        (ac-haskell-process-doc
                         "Data.List map'; rm -rf ./fixture")))
                   (list
                    result
                    (eq result output)
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (#("fixture docs\n" 0 13 (face documentation)) t ((find "hoogle") (shell "hoogle --info Data.List\\ map\\'\\;\\ rm\\ -rf\\ ./fixture")))"#
    ]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_doc_runs_hoogle_for_empty_output_and_empty_symbol() {
    let elisp_form = r##"(let (commands)
               (cl-letf
                   (((symbol-function
                      'executable-find)
                     (lambda (_executable)
                       t))
                    ((symbol-function
                      'shell-command-to-string)
                     (lambda (command)
                       (push command commands)
                       "")))
                 (list
                  (ac-haskell-process-doc "")
                  (ac-haskell-process-doc "map")
                  (nreverse commands))))"##;
    let expect = expect![[r#"OK ("" "" ("hoogle --info ''" "hoogle --info map"))"#]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_doc_rejects_non_string_symbol_only_when_hoogle_exists() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'executable-find)
                 (lambda (_executable)
                   t))
                ((symbol-function
                  'shell-command-to-string)
                 (lambda (_command)
                   "unexpected")))
               (ac-haskell-process-doc
                'not-a-string))"##;
    let expect = expect!["ERR (wrong-type-argument sequencep not-a-string)"];

    assert_ac_haskell_process_signal_parity(elisp_form, expect);
}
