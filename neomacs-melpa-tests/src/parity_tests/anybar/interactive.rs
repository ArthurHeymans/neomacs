use expect_test::expect;

use super::assert_anybar_parity;

#[test]
fn style_and_port_readers_delegate_exact_prompts_defaults_and_candidate_order() {
    let elisp_form = r##"(let ((anybar-images
                         '("custom"
                           "green"))
                        (events nil))
                     (cl-letf
                         (((symbol-function
                            'completing-read)
                           (lambda
                             (prompt collection
                                     &rest arguments)
                             (push
                              (list
                               'style
                               prompt
                               collection
                               arguments)
                              events)
                             "custom"))
                          ((symbol-function
                            'read-number)
                           (lambda
                             (prompt default
                                     &rest arguments)
                             (push
                              (list
                               'port
                               prompt
                               default
                               arguments)
                              events)
                             4242)))
                       (list
                        (anybar--read-style)
                        (anybar--read-port)
                        (nreverse events)
                        anybar-styles
                        anybar-images)))"##;
    let expect = expect![[
        r#"OK ("custom" 4242 ((style "Style: " ("white" "red" "orange" "yellow" "green" "cyan" "blue" "purple" "black" "question" "exclamation" . #1=("custom" "green")) nil) (port "Port: " 1738 nil)) ("white" "red" "orange" "yellow" "green" "cyan" "blue" "purple" "black" "question" "exclamation") #1#)"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn interactive_send_reads_command_then_port_before_running_udp_lifecycle() {
    let elisp_form = r##"(let ((events nil))
                     (cl-letf
                         (((symbol-function
                            'read-string)
                           (lambda
                             (prompt
                              &rest arguments)
                             (push
                              (list
                               'read-command
                               prompt
                               arguments)
                              events)
                             "orange"))
                          ((symbol-function
                            'anybar--read-port)
                           (lambda ()
                             (push
                              '(read-port)
                              events)
                             5151))
                          ((symbol-function
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
                              events)))
                          ((symbol-function
                            'delete-process)
                           (lambda (process)
                             (push
                              (list
                               'delete
                               process)
                              events))))
                       (call-interactively
                        #'anybar-send)
                       (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((read-command "Command: " nil) (read-port) (connect :name "anybar" :type datagram :host local :service 5151) (send connection "orange") (delete connection))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn interactive_set_reads_style_then_port_and_routes_selected_custom_image() {
    let elisp_form = r##"(let ((events nil)
                         (anybar-images
                          '("deploying")))
                     (cl-letf
                         (((symbol-function
                            'anybar--read-style)
                           (lambda ()
                             (push
                              '(read-style)
                              events)
                             "deploying"))
                          ((symbol-function
                            'anybar--read-port)
                           (lambda ()
                             (push
                              '(read-port)
                              events)
                             6161))
                          ((symbol-function
                            'anybar-send)
                           (lambda
                             (style
                              &optional port)
                             (push
                              (list
                               'send
                               style
                               port)
                              events)
                             'sent)))
                       (list
                        (call-interactively
                         #'anybar-set)
                        (nreverse events))))"##;
    let expect = expect![[r#"OK (sent ((read-style) (read-port) (send "deploying" 6161)))"#]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn interactive_quit_and_start_read_ports_then_route_network_and_shell_commands() {
    let elisp_form = r##"(let ((events nil)
                         (ports
                          '(7171 8181))
                         (anybar-executable-location
                          "/Applications/AnyBar.app"))
                     (cl-letf
                         (((symbol-function
                            'anybar--read-port)
                           (lambda ()
                             (let ((port
                                    (pop ports)))
                               (push
                                (list
                                 'read-port
                                 port)
                                events)
                               port)))
                          ((symbol-function
                            'anybar-send)
                           (lambda
                             (command
                              &optional port)
                             (push
                              (list
                               'send
                               command
                               port)
                              events)
                             'sent))
                          ((symbol-function
                            'shell-command)
                           (lambda
                             (command
                              &optional output error)
                             (push
                              (list
                               'shell
                               command
                               output
                               error)
                              events)
                             'launched)))
                       (list
                        (call-interactively
                         #'anybar-quit)
                        (call-interactively
                         #'anybar-start)
                        (nreverse events)
                        ports)))"##;
    let expect = expect![[
        r#"OK (sent launched ((read-port 7171) (send "quit" 7171) (read-port 8181) (shell "ANYBAR_PORT=8181 open -n /Applications/AnyBar.app" nil nil)) nil)"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}
