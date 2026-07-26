use expect_test::expect;

use super::assert_ac_clang_parity;

#[test]
fn ac_clang_exact_pin_versions_features_dependencies_sources_aliases_and_keymap_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-clang package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(clang-server
                   auto-complete
                   pos-tip
                   yasnippet
                   flymake
                   ac-clang))
                ac-clang-version
                clang-server-version
                ac-source-clang-async
                ac-source-clang-template
                (mapcar
                 (lambda (pair)
                   (eq
                    (indirect-function
                     (car pair))
                    (indirect-function
                     (cdr pair))))
                 '((ac-clang-reparse-buffer
                    . clang-server-reparse-buffer)
                   (ac-clang-update-cflags
                    . clang-server-update-cflags)
                   (ac-clang-set-cflags
                    . clang-server-set-cflags)
                   (ac-clang-set-cflags-from-shell-command
                    . clang-server-set-cflags-from-shell-command)
                   (ac-clang-update-clang-parameters
                    . clang-server-update-clang-parameters)
                   (ac-clang-reset-server
                    . clang-server-reset)
                   (ac-clang-reboot-server
                    . clang-server-reboot)))
                (mapcar
                 (lambda (key)
                   (lookup-key
                    ac-clang--mode-key-map
                    (kbd key)))
                 '("." ">" ":" "<tab>"))))"##;
    let expect = expect![[
        r#"OK (ac-clang "20180710.546" ((emacs (24)) (cl-lib (0 5)) (auto-complete (1 4 0)) (pos-tip (0 4 6)) (yasnippet (0 8 0))) (t t t t t t) "2.1.3" "2.1.1" ((candidates . ac-clang--candidates) (candidate-face . ac-clang-candidate-face) (selection-face . ac-clang-selection-face) (prefix . ac-clang--prefix) (requires . 0) (action . ac-clang--action) (document . ac-clang--document) (cache) (symbol . "c")) ((candidates . ac-clang--template-candidates) (prefix . ac-clang--template-prefix) (requires . 0) (action . ac-clang--template-action) (document . ac-clang--document) (cache) (symbol . "t")) (t t t t t t t) (ac-clang-async-autocomplete-autotrigger ac-clang-async-autocomplete-autotrigger ac-clang-async-autocomplete-autotrigger ac-clang-async-autocomplete-manualtrigger))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_callers_honor_runtime_rebinding_of_defsubst_function_cells() {
    let elisp_form = r##"(let ((clang-server--process
                    'fake-process)
                   (clang-server--packet-encoder
                    nil)
                   (clang-server-debug-log-buffer-p
                    nil)
                   events)
               (cl-letf
                   (((symbol-function
                      'clang-server-live-p)
                     (lambda ()
                       (push 'rebound-live events)
                       t))
                    ((symbol-function
                      'process-live-p)
                     (lambda (_)
                       (push 'original-live events)
                       nil))
                    ((symbol-function
                      'clang-server--send-shutdown-command)
                     (lambda ()
                       (push 'shutdown events)
                       'sent))
                    ((symbol-function
                      'clang-server--send-command)
                     (lambda (&rest command)
                       (push
                        (cons
                         'override-command command)
                        events)
                       'override-command))
                    ((symbol-function
                      'clang-server--process-send-string)
                     (lambda (packet)
                       (push
                        (list
                         'override-send packet)
                        events)
                       'override-send))
                    ((symbol-function
                      'process-send-string)
                     (lambda (_process packet)
                       (push
                        (list
                         'original-send packet)
                        events)
                       'original-send)))
                 (list
                  (condition-case error-data
                      (clang-server-shutdown)
                    (error
                     (cons :error error-data)))
                  (progn
                    (setq clang-server--process
                          'fake-process)
                    (condition-case error-data
                        (clang-server--send-specification-command)
                      (error
                       (cons :error error-data))))
                  (let ((clang-server--packet-encoder
                         (lambda (_) "x")))
                    (condition-case error-data
                        (clang-server--send-command-packet
                         '(:fixture t))
                      (error
                       (cons :error error-data))))
                  (nreverse events))))"##;
    let expect = expect![
        r#"OK (t override-command override-send (rebound-live shutdown (override-command :CommandType "Server" :CommandName "GET_SPECIFICATION") (override-send "PacketSize:1\nx")))"#
    ];

    assert_ac_clang_parity(elisp_form, expect);
}
