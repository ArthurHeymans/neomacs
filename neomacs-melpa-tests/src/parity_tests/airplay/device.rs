use expect_test::expect;

use super::assert_airplay_parity;

#[test]
fn airplay_available_interface_filter_matches_link_state_and_loopback_rules() {
    let elisp_form = r##"(cl-letf
         (((symbol-function 'network-interface-list)
           (lambda ()
             '(("lo-up" . [127 0 0 1 0])
               ("lo-down" . [127 0 0 1 0])
               ("wifi" . [10 0 0 8 0])
               ("eth-down" . [192 168 1 9 0]))))
          ((symbol-function 'format-network-address)
           (lambda (address _omit-port)
             (pcase (aref address 0)
               (127 "127.0.0.1")
               (10 "10.0.0.8")
               (_ "192.168.1.9"))))
          ((symbol-function 'network-interface-info)
           (lambda (name)
             (list nil nil nil nil
                   (if (member name '("lo-up" "wifi"))
                       '(up running)
                     '(broadcast))))))
         (airplay/device:--available-my-network-list))"##;
    let expect = expect![[
        r#"OK (("lo-down" . [127 0 0 1 0]) ("wifi" . [10 0 0 8 0]) ("eth-down" . [192 168 1 9 0]))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_client_ip_selects_from_available_interfaces_and_formats_address() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'airplay/device:--available-my-network-list)
               (lambda ()
                 '(("wifi" . [10 0 0 8 0])
                   ("ethernet" . [192 168 1 9 0]))))
              ((symbol-function 'shuffle-vector)
               (lambda (vector)
                 (push (append vector nil) calls)
                 (vector (aref vector 1) (aref vector 0))))
              ((symbol-function 'format-network-address)
               (lambda (address omit-port)
                 (push (list address omit-port) calls)
                 "192.168.1.9")))
           (list (airplay/device:client-ip) (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("192.168.1.9" ((("wifi" . [10 0 0 8 0]) ("ethernet" . #1=[192 168 1 9 0])) (#1# t)))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_device_browse_returns_nil_pair_after_empty_mdns_response() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'make-network-process)
               (lambda (&rest args)
                 (push (cons 'make args) calls)
                 'mdns-process))
              ((symbol-function 'dns-write)
               (lambda (query)
                 (push (list 'dns-write query) calls)
                 "wire-query"))
              ((symbol-function 'random)
               (lambda (&optional limit)
                 (push (list 'random limit) calls)
                 12345))
              ((symbol-function 'process-send-string)
               (lambda (process payload)
                 (push (list 'send process payload) calls)))
              ((symbol-function 'accept-process-output)
               (lambda (process seconds)
                 (push (list 'accept process seconds) calls)
                 nil))
              ((symbol-function 'delete-process)
               (lambda (process)
                 (push (list 'delete process) calls))))
           (list (airplay/device:browse)
                 (mapcar (lambda (call)
                           (if (eq (car-safe call) 'make)
                               (list 'make
                                     (plist-get (cdr call) :name)
                                     (plist-get (cdr call) :host)
                                     (plist-get (cdr call) :service)
                                     (plist-get (cdr call) :type))
                             call))
                         (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((nil) ((make "mdns" "224.0.0.251" 5353 datagram) (random 65000) (dns-write ((id 12345) (opcode query) (queries (("_airplay._tcp.local" (type PTR)))))) (send mdns-process "wire-query") (accept mdns-process 5) (delete mdns-process)))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_device_browse_decodes_mdns_address_and_service_port() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'make-network-process)
               (lambda (&rest _) 'mdns-process))
              ((symbol-function 'dns-write)
               (lambda (_) "wire-query"))
              ((symbol-function 'process-send-string)
               (lambda (_process _payload)
                 (insert "wire-response")))
              ((symbol-function 'delete-process)
               (lambda (process) (push (list 'delete process) calls)))
              ((symbol-function 'dns-read)
               (lambda (response)
                 (push (list 'read response) calls)
                 'parsed-response))
              ((symbol-function 'dns-get)
               (lambda (field object)
                 (push (list 'get field object) calls)
                 (cond
                  ((and (eq field 'additionals)
                        (eq object 'parsed-response))
                   '(address-record service-record))
                  ((and (eq field 'data)
                        (eq object 'address-record))
                   "10.0.0.42")
                  ((and (eq field 'data)
                        (eq object 'service-record))
                   'service-data)
                  ((and (eq field 'port)
                        (eq object 'service-data))
                   7000)))))
           (list (airplay/device:browse) (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("10.0.0.42" . 7000) ((delete mdns-process) (read "wire-response") (get additionals parsed-response) (get data address-record) (get data service-record) (get port service-data)))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}
