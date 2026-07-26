use expect_test::expect;

use super::{assert_ac_geiser_parity, assert_ac_geiser_signal_parity};

#[test]
fn ac_geiser_candidates_before_geiser_components_load_signals_void_function() {
    let elisp_form = r##"(let ((ac-prefix "fixture"))
               (ac-source-geiser-candidates))"##;
    let expect = expect!["ERR (void-function geiser-repl--live-p)"];

    assert_ac_geiser_signal_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_candidates_work_after_current_geiser_components_are_loaded() {
    let elisp_form = r##"(progn
               (require 'geiser-repl)
               (require 'geiser-completion)
               (let ((ac-prefix "fixture"))
                 (list
                  (mapcar
                   #'functionp
                   '(geiser-repl--live-p
                     geiser-completion--complete))
                  (ac-source-geiser-candidates))))"##;
    let expect = expect!["OK ((t t) nil)"];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_candidates_short_circuits_completion_when_repl_is_not_live() {
    let elisp_form = r##"(let ((ac-prefix
                    (propertize
                     "fixture"
                     'origin 'prefix))
                   calls)
               (cl-letf
                   (((symbol-function
                      'geiser-repl--live-p)
                     (lambda ()
                       (push 'live calls)
                       nil))
                    ((symbol-function
                      'geiser-completion--complete)
                     (lambda (&rest arguments)
                       (push
                        (cons 'complete arguments)
                        calls)
                       'unexpected)))
                 (list
                  (ac-source-geiser-candidates)
                  (nreverse calls)
                  ac-prefix
                  (text-properties-at
                   0 ac-prefix))))"##;
    let expect = expect![[r#"OK (nil (live) #("fixture" 0 7 (origin prefix)) (origin prefix))"#]];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_candidates_forwards_live_prefix_and_nil_module_exactly() {
    let elisp_form = r##"(let ((ac-prefix
                    (propertize
                     "sche"
                     'fixture '(prefix value)))
                   (candidates
                    (list
                     (propertize
                      "scheme"
                      'summary "language")
                     "schedule"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'geiser-repl--live-p)
                     (lambda ()
                       (push '(live) calls)
                       'connected))
                    ((symbol-function
                      'geiser-completion--complete)
                     (lambda (prefix module)
                       (push
                        (list
                         'complete
                         prefix
                         (text-properties-at
                          0 prefix)
                         module)
                        calls)
                       candidates)))
                 (let ((result
                        (ac-source-geiser-candidates)))
                   (list
                    result
                    (eq result candidates)
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((#("scheme" 0 6 (summary "language")) "schedule") t ((live) (complete #("sche" 0 4 (fixture (prefix value))) (fixture (prefix value)) nil)))"#
    ]];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_candidates_does_not_read_an_unbound_prefix_for_a_dead_repl() {
    let elisp_form = r##"(progn
               (makunbound 'ac-prefix)
               (cl-letf
                   (((symbol-function
                      'geiser-repl--live-p)
                     (lambda () nil))
                    ((symbol-function
                      'geiser-completion--complete)
                     (lambda (&rest _arguments)
                       'unexpected)))
                 (list
                  (ac-source-geiser-candidates)
                  (boundp 'ac-prefix))))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_candidates_propagates_completion_signals_from_a_live_repl() {
    let elisp_form = r##"(let ((ac-prefix "fixture"))
               (cl-letf
                   (((symbol-function
                      'geiser-repl--live-p)
                     (lambda () t))
                    ((symbol-function
                      'geiser-completion--complete)
                     (lambda (&rest arguments)
                       (signal
                        'error
                        (list
                         "fixture completion failure"
                         arguments)))))
                 (ac-source-geiser-candidates)))"##;
    let expect = expect![[r#"ERR (error "fixture completion failure" ("fixture" nil))"#]];

    assert_ac_geiser_signal_parity(elisp_form, expect);
}
