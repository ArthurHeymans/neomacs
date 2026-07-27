use expect_test::expect;

use super::assert_anybar_parity;

#[test]
fn send_builds_local_datagram_transmits_command_and_deletes_connection_in_order() {
    let elisp_form = r##"(let ((events nil)
                         (connection
                          'deterministic-connection))
                     (cl-letf
                         (((symbol-function
                            'make-network-process)
                           (lambda
                             (&rest arguments)
                             (push
                              (cons
                               'connect
                               arguments)
                              events)
                             connection))
                          ((symbol-function
                            'process-send-string)
                           (lambda
                             (process command)
                             (push
                              (list
                               'send
                               process
                               command)
                              events)
                             'sent))
                          ((symbol-function
                            'delete-process)
                           (lambda (process)
                             (push
                              (list
                               'delete
                               process)
                              events)
                             'deleted)))
                       (list
                        (anybar-send
                         "green"
                         4242)
                        (nreverse events))))"##;
    let expect = expect![[
        r#"OK (deleted ((connect :name "anybar" :type datagram :host local :service 4242) (send deterministic-connection "green") (delete deterministic-connection)))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn send_uses_default_port_for_nil_but_preserves_zero_and_string_services() {
    let elisp_form = r##"(let ((events nil))
                     (cl-letf
                         (((symbol-function
                            'make-network-process)
                           (lambda
                             (&rest arguments)
                             (push arguments events)
                             (list
                              'connection
                              (plist-get
                               arguments
                               :service))))
                          ((symbol-function
                            'process-send-string)
                           (lambda
                             (&rest _arguments)
                             nil))
                          ((symbol-function
                            'delete-process)
                           (lambda
                             (&rest _arguments)
                             nil)))
                       (anybar-send "default" nil)
                       (anybar-send "zero" 0)
                       (anybar-send "service" "1738")
                       (mapcar
                        (lambda (arguments)
                          (list
                           (plist-get
                            arguments
                            :name)
                           (plist-get
                            arguments
                            :type)
                           (plist-get
                            arguments
                            :host)
                           (plist-get
                            arguments
                            :service)))
                        (nreverse events))))"##;
    let expect = expect![[
        r#"OK (("anybar" datagram local 1738) ("anybar" datagram local 0) ("anybar" datagram local "1738"))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn connection_failure_propagates_without_attempting_send_or_delete() {
    let elisp_form = r##"(let ((events nil))
                     (list
                      (condition-case error
                          (cl-letf
                              (((symbol-function
                                 'make-network-process)
                                (lambda
                                  (&rest arguments)
                                  (push
                                   (cons
                                    'connect
                                    arguments)
                                   events)
                                  (error
                                   "No local AnyBar listener")))
                               ((symbol-function
                                 'process-send-string)
                                (lambda
                                  (&rest arguments)
                                  (push
                                   (cons
                                    'unexpected-send
                                    arguments)
                                   events)))
                               ((symbol-function
                                 'delete-process)
                                (lambda
                                  (&rest arguments)
                                  (push
                                   (cons
                                    'unexpected-delete
                                    arguments)
                                   events))))
                            (anybar-send
                             "purple"
                             31337))
                        (error
                         (list
                          (car error)
                          (cadr error))))
                      (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((error "No local AnyBar listener") ((connect :name "anybar" :type datagram :host local :service 31337)))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn send_failure_propagates_and_documents_non_unwind_protected_connection_lifecycle() {
    let elisp_form = r##"(let ((events nil))
                     (list
                      (condition-case error
                          (cl-letf
                              (((symbol-function
                                 'make-network-process)
                                (lambda
                                  (&rest arguments)
                                  (push
                                   (cons
                                    'connect
                                    arguments)
                                   events)
                                  'connection))
                               ((symbol-function
                                 'process-send-string)
                                (lambda
                                  (process command)
                                  (push
                                   (list
                                    'send
                                    process
                                    command)
                                   events)
                                  (error
                                   "Datagram send failed")))
                               ((symbol-function
                                 'delete-process)
                                (lambda (process)
                                  (push
                                   (list
                                    'unexpected-delete
                                    process)
                                   events))))
                            (anybar-send
                             "cyan"))
                        (error
                         (list
                          (car error)
                          (cadr error))))
                      (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((error "Datagram send failed") ((connect :name "anybar" :type datagram :host local :service 1738) (send connection "cyan")))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}
