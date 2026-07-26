use super::assert_ace_link_parity;
use expect_test::expect;

#[test]
fn ace_link_guarded_actions_ignore_nil_and_non_position_values_without_side_effects() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdef")
         (goto-char 4)
         (let (events)
           (cl-letf (((symbol-function 'push-button)
                      (lambda (&rest arguments)
                        (push
                         (cons 'push-button arguments)
                         events)))
                     ((symbol-function 'eww-follow-link)
                      (lambda (&rest arguments)
                        (push
                         (cons 'eww arguments)
                         events)))
                     ((symbol-function 'w3m-view-this-url)
                      (lambda (&rest arguments)
                        (push
                         (cons 'w3m arguments)
                         events)))
                     ((symbol-function 'compile-goto-error)
                      (lambda (&rest arguments)
                        (push
                         (cons 'compile arguments)
                         events)))
                     ((symbol-function 'org-open-at-point)
                      (lambda (&rest arguments)
                        (push
                         (cons 'org arguments)
                         events)))
                     ((symbol-function 'org-agenda-goto)
                      (lambda (&rest arguments)
                        (push
                         (cons 'agenda arguments)
                         events)))
                     ((symbol-function 'xref-goto-xref)
                      (lambda (&rest arguments)
                        (push
                         (cons 'xref arguments)
                         events)))
                     ((symbol-function 'Custom-newline)
                      (lambda (&rest arguments)
                        (push
                         (cons 'custom arguments)
                         events)))
                     ((symbol-function 'goto-address-at-point)
                      (lambda (&rest arguments)
                        (push
                         (cons 'addr arguments)
                         events)))
                     ((symbol-function 'slime-goto-xref)
                      (lambda (&rest arguments)
                        (push
                         (cons 'slime arguments)
                         events)))
                     ((symbol-function 'indium-follow-link)
                      (lambda (&rest arguments)
                        (push
                         (cons 'indium arguments)
                         events)))
                     ((symbol-function 'cider-inspector-operate-on-point)
                      (lambda (&rest arguments)
                        (push
                         (cons 'cider arguments)
                         events))))
             (ace-link--info-action nil)
             (ace-link--help-action nil)
             (ace-link--man-action nil)
             (ace-link--woman-action nil)
             (ace-link--eww-action nil 'external)
             (ace-link--w3m-action nil)
             (ace-link--compilation-action nil)
             (ace-link--gnus-action nil)
             (ace-link--mu4e-action nil)
             (ace-link--notmuch-plain-action nil)
             (ace-link--widget-action nil)
             (ace-link--org-action nil)
             (ace-link--org-agenda-action nil)
             (ace-link--xref-action nil)
             (ace-link--custom-action nil)
             (ace-link--addr-action nil)
             (ace-link--sldb-action nil)
             (ace-link--slime-xref-action nil)
             (ace-link--slime-inspector-action nil)
             (ace-link--indium-inspector-action nil)
             (ace-link--indium-debugger-frames-action nil)
             (ace-link--cider-inspector-action nil)
             (list
              (point)
              events))))"##;
    let expect = expect!["OK (4 nil)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_info_current_returns_resolved_target_and_original_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdef")
         (goto-char 4)
         (cl-letf (((symbol-function 'Info-try-follow-nearest-node)
                    (lambda ()
                      (list
                       (Info-goto-node
                        'fixture-node
                        'no-going-back)
                       (funcall
                        browse-url-browser-function
                        "https://example.test"
                        'ignored)))))
           (ace-link--info-current)))"##;
    let expect = expect![[r#"OK ((fixture-node "https://example.test") . 4)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_info_action_pushes_mark_moves_and_retries_until_follow_succeeds() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 2)
         (let ((outcomes '(nil nil followed))
               events)
           (cl-letf (((symbol-function 'window-end)
                      (lambda (&rest _)
                        10))
                     ((symbol-function 'Info-follow-nearest-node)
                      (lambda ()
                        (let ((outcome
                               (pop outcomes)))
                          (push
                           (list
                            'follow
                            (point)
                            outcome)
                           events)
                          outcome))))
             (ace-link--info-action 5)
             (list
              (point)
              (mark t)
              mark-active
              (nreverse events)
              outcomes))))"##;
    let expect = expect!["OK (7 2 t ((follow 5 nil) (follow 6 nil) (follow 7 followed)) nil)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_info_action_signals_after_retrying_beyond_window_end() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 2)
         (let ((calls 0))
           (cl-letf (((symbol-function 'window-end)
                      (lambda (&rest _)
                        6))
                     ((symbol-function 'Info-follow-nearest-node)
                      (lambda ()
                        (setq calls
                              (1+ calls))
                        nil)))
             (condition-case error-data
                 (list
                  'unexpected-success
                  (ace-link--info-action 5))
               (error
                (list
                 error-data
                 calls
                 (point)
                 (mark t)
                 mark-active))))))"##;
    let expect = expect![[r#"OK ((error "Could not follow link") 2 7 2 t)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_help_woman_eww_w3m_and_compilation_actions_route_exact_positions() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let (events)
           (cl-letf (((symbol-function 'push-button)
                      (lambda (&rest arguments)
                        (push
                         (cons
                          'push-button
                          (cons
                           (point)
                           arguments))
                         events)))
                     ((symbol-function 'eww-follow-link)
                      (lambda (&rest arguments)
                        (push
                         (cons
                          'eww
                          (cons
                           (point)
                           arguments))
                         events)))
                     ((symbol-function 'w3m-view-this-url)
                      (lambda ()
                        (push
                         (list
                          'w3m
                          (point))
                         events)))
                     ((symbol-function 'compile-goto-error)
                      (lambda ()
                        (push
                         (list
                          'compile
                          (point))
                         events))))
             (ace-link--help-action 2)
             (ace-link--woman-action 4)
             (ace-link--eww-action 6 'external)
             (ace-link--w3m-action 8)
             (ace-link--compilation-action 9)
             (nreverse events))))"##;
    let expect =
        expect!["OK ((push-button 3) (push-button 5) (eww 6 external) (w3m 8) (compile 10))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_man_action_uses_buttons_or_interactive_man_follow() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let ((button-position 3)
               events)
           (cl-letf (((symbol-function 'button-at)
                      (lambda (position)
                        (and
                         (= position
                            button-position)
                         'fixture-button)))
                     ((symbol-function 'push-button)
                      (lambda (&rest arguments)
                        (push
                         (cons
                          'push
                          (cons
                           (point)
                           arguments))
                         events)))
                     ((symbol-function 'call-interactively)
                      (lambda (function &optional record keys)
                        (push
                         (list
                          'interactive
                          function
                          record
                          keys
                          (point))
                         events)
                        'interactive-result)))
             (ace-link--man-action 3)
             (ace-link--man-action 6)
             (nreverse events))))"##;
    let expect = expect!["OK ((push 4 3) (interactive man-follow nil nil 7))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_gnus_action_routes_legacy_shr_callback_and_button_branches() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (add-text-properties
          4 5
          '(shr-url "https://example.test"))
         (add-text-properties
          6 7
          '(gnus-callback fixture-callback))
         (let (events)
           (cl-letf (((symbol-function 'widget-button-press)
                      (lambda (position)
                        (push
                         (list
                          'widget
                          position
                          (point))
                         events)))
                     ((symbol-function 'shr-browse-url)
                      (lambda ()
                        (push
                         (list
                          'shr
                          (point))
                         events)))
                     ((symbol-function 'gnus-article-press-button)
                      (lambda ()
                        (push
                         (list
                          'gnus
                          (point))
                         events)))
                     ((symbol-function 'push-button)
                      (lambda (&rest arguments)
                        (push
                         (cons
                          'button
                          (cons
                           (point)
                           arguments))
                         events))))
             (cl-progv
                 '(emacs-major-version
                   mm-text-html-renderer)
                 '(26 other)
               (ace-link--gnus-action 1))
             (cl-progv
                 '(emacs-major-version
                   mm-text-html-renderer)
                 '(30 shr)
               (ace-link--gnus-action 3))
             (cl-progv
                 '(emacs-major-version
                   mm-text-html-renderer)
                 '(30 other)
               (ace-link--gnus-action 5))
             (cl-progv
                 '(emacs-major-version
                   mm-text-html-renderer)
                 '(30 other)
               (ace-link--gnus-action 7))
             (nreverse events))))"##;
    let expect = expect!["OK ((widget 2 2) (shr 4) (gnus 6) (button 8))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_mu4e_action_routes_shr_url_mu4e_url_and_attachment_properties() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (add-text-properties
          2 3
          '(shr-url "https://example.test"))
         (add-text-properties
          4 5
          '(mu4e-url "https://mu4e.test"))
         (add-text-properties
          6 7
          '(mu4e-attnum 7))
         (let (events)
           (cl-letf (((symbol-function 'shr-browse-url)
                      (lambda ()
                        (push
                         (list
                          'shr
                          (point))
                         events)))
                     ((symbol-function 'mu4e~view-browse-url-from-binding)
                      (lambda ()
                        (push
                         (list
                          'mu4e-url
                          (point))
                         events)))
                     ((symbol-function 'mu4e~view-open-attach-from-binding)
                      (lambda ()
                        (push
                         (list
                          'attachment
                          (point))
                         events))))
             (ace-link--mu4e-action 1)
             (ace-link--mu4e-action 3)
             (ace-link--mu4e-action 5)
             (ace-link--mu4e-action 7)
             (nreverse events))))"##;
    let expect = expect!["OK ((shr 2) (mu4e-url 4) (attachment 6))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_notmuch_actions_preserve_their_distinct_point_semantics() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (add-text-properties
          7 8
          '(shr-url "https://example.test"))
         (let (events)
           (cl-letf (((symbol-function 'browse-url-at-point)
                      (lambda ()
                        (push
                         (list
                          'plain
                          (point))
                         events)))
                     ((symbol-function 'shr-browse-url)
                      (lambda ()
                        (push
                         (list
                          'html
                          (point))
                         events))))
             (ace-link--notmuch-plain-action 3)
             (goto-char 7)
             (ace-link--notmuch-html-action 2)
             (goto-char 2)
             (ace-link--notmuch-html-action 7)
             (nreverse events))))"##;
    let expect = expect!["OK ((plain 3) (html 7))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_widget_action_applies_only_a_button_at_the_selected_position() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (add-text-properties
          5 6
          '(button fixture-widget))
         (let (events)
           (cl-letf (((symbol-function 'widget-apply-action)
                      (lambda (button)
                        (push
                         (list
                          button
                          (point))
                         events)
                        'applied)))
             (list
              (ace-link--widget-action 5)
              (ace-link--widget-action 3)
              (nreverse events)
              (point)))))"##;
    let expect = expect!["OK (applied nil ((fixture-widget 5)) 3)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_org_agenda_xref_custom_and_address_actions_route_exact_positions() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let (events)
           (cl-letf (((symbol-function 'org-open-at-point)
                      (lambda ()
                        (push
                         (list
                          'org
                          (point))
                         events)))
                     ((symbol-function 'org-agenda-goto)
                      (lambda ()
                        (push
                         (list
                          'agenda
                          (point))
                         events)))
                     ((symbol-function 'xref-goto-xref)
                      (lambda ()
                        (push
                         (list
                          'xref
                          (point))
                         events)))
                     ((symbol-function 'Custom-newline)
                      (lambda (position)
                        (push
                         (list
                          'custom
                          position
                          (point))
                         events)))
                     ((symbol-function 'goto-address-at-point)
                      (lambda ()
                        (push
                         (list
                          'addr
                          (point))
                         events))))
             (ace-link--org-action 2)
             (ace-link--org-agenda-action 4)
             (ace-link--xref-action 6)
             (ace-link--custom-action 8)
             (ace-link--addr-action 9)
             (nreverse events))))"##;
    let expect = expect!["OK ((org 2) (agenda 4) (xref 6) (custom 8 8) (addr 10))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_sldb_and_slime_xref_actions_use_configured_function_and_exact_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let (events)
           (let ((ace-link--sldb-action-fn
                  (lambda ()
                    (push
                     (list
                      'sldb
                      (point))
                     events))))
             (cl-letf (((symbol-function 'slime-goto-xref)
                        (lambda ()
                          (push
                           (list
                            'xref
                            (point))
                           events))))
               (ace-link--sldb-action 3)
               (ace-link--slime-xref-action 7)
               (nreverse events)))))"##;
    let expect = expect!["OK ((sldb 3) (xref 7))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_slime_inspector_action_distinguishes_copy_and_operate_paths() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let (events)
           (cl-letf (((symbol-function 'call-interactively)
                      (lambda (function &optional record keys)
                        (push
                         (list
                          'interactive
                          function
                          record
                          keys
                          (point))
                         events)))
                     ((symbol-function 'slime-inspector-operate-on-point)
                      (lambda ()
                        (push
                         (list
                          'operate
                          (point))
                         events))))
             (ace-link--slime-inspector-action 1)
             (ace-link--slime-inspector-action 6)
             (nreverse events))))"##;
    let expect =
        expect!["OK ((interactive slime-inspector-copy-down-to-repl nil nil 1) (operate 6))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_indium_and_cider_actions_move_to_selected_positions_before_following() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let (events)
           (cl-letf (((symbol-function 'indium-follow-link)
                      (lambda ()
                        (push
                         (list
                          'indium
                          (point))
                         events)))
                     ((symbol-function 'cider-inspector-operate-on-point)
                      (lambda ()
                        (push
                         (list
                          'cider
                          (point))
                         events))))
             (ace-link--indium-inspector-action 2)
             (ace-link--indium-debugger-frames-action 5)
             (ace-link--cider-inspector-action 8)
             (nreverse events))))"##;
    let expect = expect!["OK ((indium 2) (indium 5) (cider 8))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_number_or_marker_actions_accept_a_live_marker_and_use_exact_offsets() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let ((marker
                (copy-marker 5))
               events)
           (cl-letf (((symbol-function 'button-at)
                      (lambda (_position)
                        'fixture-button))
                     ((symbol-function 'push-button)
                      (lambda (&rest arguments)
                        (push
                         (cons
                          'button
                          (cons
                           (point)
                           arguments))
                         events)))
                     ((symbol-function 'eww-follow-link)
                      (lambda (external)
                        (push
                         (list
                          'eww
                          (point)
                          external)
                         events)))
                     ((symbol-function 'compile-goto-error)
                      (lambda ()
                        (push
                         (list
                          'compile
                          (point))
                         events)))
                     ((symbol-function 'Custom-newline)
                      (lambda (position)
                        (push
                         (list
                          'custom
                          position
                          (point))
                         events)))
                     ((symbol-function 'goto-address-at-point)
                      (lambda ()
                        (push
                         (list
                          'addr
                          (point))
                         events)))
                     ((symbol-function 'slime-goto-xref)
                      (lambda ()
                        (push
                         (list
                          'slime
                          (point))
                         events)))
                     ((symbol-function 'cider-inspector-operate-on-point)
                      (lambda ()
                        (push
                         (list
                          'cider
                          (point))
                         events))))
             (ace-link--woman-action marker)
             (ace-link--eww-action marker
                                    'external)
             (ace-link--compilation-action marker)
             (ace-link--custom-action marker)
             (ace-link--addr-action marker)
             (ace-link--slime-xref-action marker)
             (ace-link--cider-inspector-action marker)
             (list
              (nreverse events)
              (marker-position marker)
              (marker-buffer marker)))))"##;
    let expect = expect![
        "OK (((button 6) (eww 5 external) (compile 6) (custom 5 5) (addr 6) (slime 5) (cider 5)) 5 (:buffer nil))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}
