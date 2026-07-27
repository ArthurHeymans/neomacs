use expect_test::expect;

use super::assert_angry_police_captain_parity;

#[test]
fn angry_police_captain_dispatches_exact_url_and_returns_retrieval_handle() {
    let elisp_form = r##"(let (calls callback)
         (cl-letf
             (((symbol-function
                'url-retrieve)
               (lambda (&rest arguments)
                 (setq calls arguments
                       callback
                       (cadr arguments))
                 'retrieval-handle)))
           (list
            (angry-police-captain)
            (car calls)
            (length calls)
            (functionp callback)
            (help-function-arglist
             callback t))))"##;
    let expect = expect![[r#"OK (retrieval-handle "http://theangrypolicecaptain.com" 2 t (x))"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_interactive_invocation_uses_same_async_dispatch() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'url-retrieve)
               (lambda (&rest arguments)
                 (push arguments calls)
                 'interactive-request)))
           (list
            (call-interactively
             'angry-police-captain)
            (mapcar
             (lambda (arguments)
               (list
                (car arguments)
                (functionp
                 (cadr arguments))
                (cddr arguments)))
             (nreverse calls)))))"##;
    let expect =
        expect![[r#"OK (interactive-request (("http://theangrypolicecaptain.com" t nil)))"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_callback_extracts_quote_kills_buffer_and_messages() {
    let elisp_form = r##"(let (callback message-call
               retrieval-buffer callback-result)
         (cl-letf
             (((symbol-function
                'url-retrieve)
               (lambda (_url function
                        &rest _arguments)
                 (setq callback function)
                 'scheduled))
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
                  " *angry-response*"))
           (with-current-buffer retrieval-buffer
             (insert
              "HTTP/1.1 200 OK\n\n"
              "<html><a href=\"http://theangrypolicecaptain.com\">"
              "YOU'RE OFF THE CASE!</a></html>")
             (goto-char (point-min))
             (setq callback-result
                   (funcall callback
                            '(:redirect nil)))))
         (list
          callback-result
          message-call
          (buffer-live-p retrieval-buffer)))"##;
    let expect = expect![[r#"OK ("YOU'RE OFF THE CASE" ("%s" "YOU'RE OFF THE CASE") nil)"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_callback_handles_unicode_entities_and_punctuation() {
    let elisp_form = r##"(let (callback message-call
               retrieval-buffer)
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
                  " *angry-unicode*"))
           (with-current-buffer retrieval-buffer
             (insert
              "<a href=\"http://theangrypolicecaptain.com\">"
              "¿Dónde está García? — &amp; Ω!</a>")
             (goto-char (point-min))
             (funcall callback nil)))
         (list
          message-call
          (buffer-live-p retrieval-buffer)))"##;
    let expect = expect![[r#"OK (("%s" "¿Dónde está García? — &amp; Ω") nil)"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_callback_preserves_multiline_text_but_strips_properties() {
    let elisp_form = r##"(let (callback message-call
               retrieval-buffer)
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
                  " *angry-properties*"))
           (with-current-buffer retrieval-buffer
             (insert
              "<a href=\"http://theangrypolicecaptain.com\">"
              "FIRST LINE\nSECOND LINE.</a>")
             (add-text-properties
              (point-min) (point-max)
              '(face bold source "fixture"))
             (goto-char (point-min))
             (funcall callback
                      '(:status 200))))
         (list
          message-call
          (text-properties-at
           0 (cadr message-call))
          (buffer-live-p retrieval-buffer)))"##;
    let expect = expect![[r#"OK (("%s" "FIRST LINE\nSECOND LINE") nil nil)"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_callback_uses_first_matching_link_and_closing_anchor() {
    let elisp_form = r##"(let (callback messages
               retrieval-buffer)
         (cl-letf
             (((symbol-function
                'url-retrieve)
               (lambda (_url function
                        &rest _arguments)
                 (setq callback function)))
              ((symbol-function 'message)
               (lambda (&rest arguments)
                 (push arguments messages)
                 (apply #'format arguments)))
              ((symbol-function
                'kill-this-buffer)
               (lambda ()
                 (kill-buffer
                  (current-buffer)))))
           (angry-police-captain)
           (setq retrieval-buffer
                 (generate-new-buffer
                  " *angry-links*"))
           (with-current-buffer retrieval-buffer
             (insert
              "<a href=\"http://theangrypolicecaptain.com\">"
              "FIRST QUOTE!</a>"
              "<a href=\"http://theangrypolicecaptain.com\">"
              "SECOND QUOTE!</a>")
             (goto-char (point-min))
             (funcall callback nil)))
         (list
          (nreverse messages)
          (buffer-live-p retrieval-buffer)))"##;
    let expect = expect![[r#"OK ((("%s" "FIRST QUOTE")) nil)"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_callback_searches_from_retrieval_buffer_point() {
    let elisp_form = r##"(let (callback message-call
               retrieval-buffer)
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
                  " *angry-point*"))
           (with-current-buffer retrieval-buffer
             (insert
              "<a href=\"http://theangrypolicecaptain.com\">"
              "DECOY!</a>\n"
              "response starts here\n"
              "<a href=\"http://theangrypolicecaptain.com\">"
              "ACTIVE QUOTE!</a>")
             (search-backward
              "response starts here")
             (funcall callback nil)))
         (list
          message-call
          (buffer-live-p retrieval-buffer)))"##;
    let expect = expect![[r#"OK (("%s" "ACTIVE QUOTE") nil)"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}

#[test]
fn angry_police_captain_callback_exposes_single_character_truncation_behavior() {
    let elisp_form = r##"(let (callback message-call
               retrieval-buffer)
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
                  " *angry-one-character*"))
           (with-current-buffer retrieval-buffer
             (insert
              "<a href=\"http://theangrypolicecaptain.com\">"
              "X</a>")
             (goto-char (point-min))
             (funcall callback nil)))
         (list
          message-call
          (buffer-live-p retrieval-buffer)))"##;
    let expect = expect![[r#"OK (("%s" "") nil)"#]];
    assert_angry_police_captain_parity(elisp_form, expect);
}
