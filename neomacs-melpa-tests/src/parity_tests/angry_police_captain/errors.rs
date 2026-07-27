use expect_test::expect;

use super::assert_angry_police_captain_parity;

#[test]
fn angry_police_captain_real_hidden_callback_rejects_kill_this_buffer() {
    let elisp_form = r##"(let (callback retrieval-buffer
               message-call outcome)
         (cl-letf
             (((symbol-function
                'url-retrieve)
               (lambda (_url function
                        &rest _arguments)
                 (setq callback function)))
              ((symbol-function 'message)
               (lambda (&rest arguments)
                 (setq message-call arguments)
                 (apply #'format arguments))))
           (angry-police-captain)
           (setq retrieval-buffer
                 (generate-new-buffer
                  " *angry-real-hidden*"))
           (with-current-buffer retrieval-buffer
             (insert
              "<a href=\"http://theangrypolicecaptain.com\">"
              "HIDDEN QUOTE!</a>")
             (goto-char (point-min))
             (setq outcome
                   (condition-case error
                       (list
                        'value
                        (funcall callback nil))
                     (error
                      (list
                       'error
                       (car error)
                       (cdr error)))))))
         (prog1
             (list
              outcome message-call
              (buffer-live-p retrieval-buffer))
           (when
               (buffer-live-p retrieval-buffer)
             (kill-buffer retrieval-buffer))))"##;
    let expect = expect![[
        r#"OK ((error error ("This command must be called from a menu or a tool bar")) nil t)"#
    ]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_missing_opening_link_signals_and_keeps_buffer() {
    let elisp_form = r##"(let (callback retrieval-buffer outcome)
         (cl-letf
             (((symbol-function
                'url-retrieve)
               (lambda (_url function
                        &rest _arguments)
                 (setq callback function))))
           (angry-police-captain)
           (setq retrieval-buffer
                 (generate-new-buffer
                  " *angry-no-link*"))
           (with-current-buffer retrieval-buffer
             (insert
              "HTTP/1.1 200 OK\n\n"
              "<html>No captain today.</html>")
             (goto-char (point-min))
             (setq outcome
                   (condition-case error
                       (list
                        'value
                        (funcall callback nil))
                     (error
                      (list
                       'error
                       (car error)
                       (cdr error)))))))
         (prog1
             (list
              outcome
              (buffer-live-p retrieval-buffer))
           (when
               (buffer-live-p retrieval-buffer)
             (kill-buffer retrieval-buffer))))"##;
    let expect =
        expect![[r#"OK ((error search-failed ("http://theangrypolicecaptain.com\">")) t)"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_missing_closing_anchor_signals_and_keeps_buffer() {
    let elisp_form = r##"(let (callback retrieval-buffer outcome)
         (cl-letf
             (((symbol-function
                'url-retrieve)
               (lambda (_url function
                        &rest _arguments)
                 (setq callback function))))
           (angry-police-captain)
           (setq retrieval-buffer
                 (generate-new-buffer
                  " *angry-no-close*"))
           (with-current-buffer retrieval-buffer
             (insert
              "<a href=\"http://theangrypolicecaptain.com\">"
              "UNFINISHED QUOTE")
             (goto-char (point-min))
             (setq outcome
                   (condition-case error
                       (list
                        'value
                        (funcall callback nil))
                     (error
                      (list
                       'error
                       (car error)
                       (cdr error)))))))
         (prog1
             (list
              outcome
              (buffer-live-p retrieval-buffer))
           (when
               (buffer-live-p retrieval-buffer)
             (kill-buffer retrieval-buffer))))"##;
    let expect = expect![[r#"OK ((error search-failed ("</a>")) t)"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_empty_anchor_signals_range_error_and_keeps_buffer() {
    let elisp_form = r##"(let (callback retrieval-buffer outcome)
         (cl-letf
             (((symbol-function
                'url-retrieve)
               (lambda (_url function
                        &rest _arguments)
                 (setq callback function))))
           (angry-police-captain)
           (setq retrieval-buffer
                 (generate-new-buffer
                  " *angry-empty*"))
           (with-current-buffer retrieval-buffer
             (insert
              "<a href=\"http://theangrypolicecaptain.com\"></a>")
             (goto-char (point-min))
             (setq outcome
                   (condition-case error
                       (list
                        'value
                        (funcall callback nil))
                     (error
                      (list
                       'error
                       (car error)
                       (cdr error)))))))
         (prog1
             (list
              outcome
              (buffer-live-p retrieval-buffer))
           (when
               (buffer-live-p retrieval-buffer)
             (kill-buffer retrieval-buffer))))"##;
    let expect = expect![[
        r#"OK ((error error ("This command must be called from a menu or a tool bar")) t)"#
    ]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_callback_primitive_call_order_is_exact() {
    let elisp_form = r##"(let* (callback
                (calls nil)
                retrieval-buffer
                message-call
                (search-advice
                 (lambda (&rest arguments)
                   (push
                    (list
                     're-search-forward
                     arguments)
                    calls)))
                (backward-advice
                 (lambda (&rest arguments)
                   (push
                    (list
                     'backward-char
                     arguments)
                    calls)))
                (substring-advice
                 (lambda (&rest arguments)
                   (push
                    (list
                     'buffer-substring-no-properties
                     arguments)
                    calls)))
                (kill-advice
                 (lambda (&rest arguments)
                   (push
                    (list
                     'kill-this-buffer
                     arguments)
                    calls))))
         (cl-letf
             (((symbol-function
                'url-retrieve)
               (lambda (_url function
                        &rest _arguments)
                 (setq callback function)))
              ((symbol-function 'message)
               (lambda (&rest arguments)
                 (setq message-call arguments)
                 (apply #'format arguments)))
              ((symbol-function
                'kill-this-buffer)
               (lambda ()
                 (kill-buffer
                  (current-buffer)))))
           (angry-police-captain)
           (setq retrieval-buffer
                 (generate-new-buffer
                  " *angry-trace*"))
           (with-current-buffer retrieval-buffer
             (insert
              "<a href=\"http://theangrypolicecaptain.com\">"
             "TRACE ME!</a>")
             (goto-char (point-min))
             (unwind-protect
                 (progn
                   (advice-add
                    're-search-forward
                    :before search-advice)
                   (advice-add
                    'backward-char
                    :before backward-advice)
                   (advice-add
                    'buffer-substring-no-properties
                    :before substring-advice)
                   (advice-add
                    'kill-this-buffer
                    :before kill-advice)
                   (funcall callback
                            'ignored-status))
               (advice-remove
                're-search-forward
                search-advice)
               (advice-remove
                'backward-char
                backward-advice)
               (advice-remove
                'buffer-substring-no-properties
                substring-advice)
               (advice-remove
                'kill-this-buffer
                kill-advice))))
         (list
          (nreverse calls)
          message-call
          (buffer-live-p retrieval-buffer)))"##;
    let expect = expect![[
        r#"OK (((re-search-forward ("http://theangrypolicecaptain.com\">")) (re-search-forward ("</a>")) (backward-char (5)) (buffer-substring-no-properties (44 52)) (kill-this-buffer nil)) ("%s" "TRACE ME") nil)"#
    ]];
    assert_angry_police_captain_parity(elisp_form, expect);
}
