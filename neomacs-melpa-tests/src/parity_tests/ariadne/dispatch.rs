use expect_test::expect;

use super::assert_ariadne_with_legacy_cl_parity;

#[test]
fn reply_no_name_is_a_quiet_noop() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'message)
                    (lambda (&rest args) (push args calls)))
                   ((symbol-function 'ariadne-goto)
                    (lambda (&rest args) (push args calls))))
           (list (ariadne-handle-reply
                  (vector 'no_name))
                 calls)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}

#[test]
fn known_location_reply_forwards_exact_filename_line_and_column() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ariadne-goto)
                    (lambda (filename line column)
                      (push (list filename line column)
                            calls)
                      'visited)))
           (list (ariadne-handle-reply
                  (vector 'loc_known
                          "/workspace/src/Lib.hs" 81 13))
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK (visited (("/workspace/src/Lib.hs" 81 13)))"#]];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}

#[test]
fn unknown_location_reply_reports_the_defining_module() {
    let elisp_form = r##"(let (messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (let ((text
                             (apply #'format
                                    format-string args)))
                        (push text messages)
                        text))))
           (list (ariadne-handle-reply
                  (vector 'loc_unknown
                          "Data.Map.Internal"))
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ("The name at point is defined in Data.Map.Internal" ("The name at point is defined in Data.Map.Internal"))"#
    ]];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}

#[test]
fn reply_error_reports_server_detail_without_losing_unicode() {
    let elisp_form = r##"(let (messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (let ((text
                             (apply #'format
                                    format-string args)))
                        (push text messages)
                        text))))
           (list (ariadne-handle-reply
                  (vector 'error
                          "unknown identifier λ"))
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ("ariadne error: unknown identifier λ" ("ariadne error: unknown identifier λ"))"#
    ]];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}

#[test]
fn reply_event_dispatches_its_payload_to_the_reply_handler() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ariadne-handle-reply)
                    (lambda (reply)
                      (push reply calls)
                      'handled)))
           (list (ariadne-dispatch-event
                  (vector 'reply
                          (vector 'loc_known
                                  "Main.hs" 2 9))
                  'socket)
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK (handled ([loc_known "Main.hs" 2 9]))"#]];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}

#[test]
fn bert_rpc_error_event_extracts_the_detail_field() {
    let elisp_form = r##"(let (messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (let ((text
                             (apply #'format
                                    format-string args)))
                        (push text messages)
                        text))))
           (list (ariadne-dispatch-event
                  (vector 'error
                          (vector 'type 500 'server
                                  "database unavailable"
                                  ["frame-a" "frame-b"]))
                  'socket)
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ("BERT-RPC error: database unavailable" ("BERT-RPC error: database unavailable"))"#
    ]];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}

#[test]
fn unknown_event_and_unknown_reply_variants_are_quiet_noops() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'message)
                    (lambda (&rest args)
                      (push args calls)))
                   ((symbol-function 'ariadne-goto)
                    (lambda (&rest args)
                      (push args calls))))
           (list
            (ariadne-dispatch-event
             (vector 'notification "changed")
             'socket)
            (ariadne-handle-reply
             (vector 'future_variant "payload"))
            calls)))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}

#[test]
fn dispatching_a_practical_reply_stream_preserves_event_order() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ariadne-goto)
                    (lambda (file line column)
                      (push (list :goto file line column)
                            calls)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (list :message
                                  (apply #'format
                                         format-string args))
                            calls))))
           (dolist
               (event
                (list
                 (vector 'reply (vector 'no_name))
                 (vector 'reply
                         (vector 'loc_unknown
                                 "Foreign.Module"))
                 (vector 'reply
                         (vector 'loc_known
                                 "src/Local.hs" 19 5))
                 (vector 'error
                         (vector 'type 400 'request
                                 "bad request" []))))
             (ariadne-dispatch-event event 'socket))
           (nreverse calls)))"##;
    let expect = expect![[
        r#"OK ((:message "The name at point is defined in Foreign.Module") (:goto "src/Local.hs" 19 5) (:message "BERT-RPC error: bad request"))"#
    ]];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}
