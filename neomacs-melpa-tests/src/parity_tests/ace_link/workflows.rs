use super::assert_ace_link_parity;
use expect_test::expect;

#[test]
fn ace_link_info_help_man_and_woman_commands_pass_collected_positions_through_avy() {
    let elisp_form = r##"(let ((avy-style
              'fixture-style)
             events)
         (cl-letf (((symbol-function 'ace-link--info-collect)
                    (lambda ()
                      '(("info" . 2)
                        ("info-two" . 5))))
                   ((symbol-function 'ace-link--help-collect)
                    (lambda ()
                      '(("help" . 3))))
                   ((symbol-function 'ace-link--man-collect)
                    (lambda ()
                      '(("man" . 4))))
                   ((symbol-function 'ace-link--woman-collect)
                    (lambda ()
                      '(("woman" . 6))))
                   ((symbol-function 'avy--style-fn)
                    (lambda (style)
                      (list
                       'style
                       style)))
                   ((symbol-function 'avy-process)
                    (lambda (candidates &optional overlay)
                      (push
                       (list
                        'avy
                        candidates
                        overlay)
                       events)
                      (car
                       (last candidates))))
                   ((symbol-function 'ace-link--info-action)
                    (lambda (position)
                      (push
                       (list
                        'info-action
                        position)
                       events)
                      'info-result))
                   ((symbol-function 'ace-link--help-action)
                    (lambda (position)
                      (push
                       (list
                        'help-action
                        position)
                       events)
                      'help-result))
                   ((symbol-function 'ace-link--man-action)
                    (lambda (position)
                      (push
                       (list
                        'man-action
                        position)
                       events)
                      'man-result))
                   ((symbol-function 'ace-link--woman-action)
                    (lambda (position)
                      (push
                       (list
                        'woman-action
                        position)
                       events)
                      'woman-result)))
           (list
            (ace-link-info)
            (ace-link-help)
            (ace-link-man)
            (ace-link-woman)
            (nreverse events))))"##;
    let expect = expect![
        "OK (info-result help-result man-result woman-result ((avy (2 5) (style fixture-style)) (info-action 5) (avy (3) (style fixture-style)) (help-action 3) (avy (4) (style fixture-style)) (man-action 4) (avy (6) (style fixture-style)) (woman-action 6)))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_eww_w3m_and_compilation_commands_forward_prefix_and_selected_position() {
    let elisp_form = r##"(let ((avy-style
              'fixture-style)
             events)
         (provide 'w3m)
         (cl-letf (((symbol-function 'ace-link--eww-collect)
                    (lambda (&optional property)
                      (push
                       (list
                        'collect
                        property)
                       events)
                      '(("first" . 2)
                        ("second" . 7))))
                   ((symbol-function 'ace-link--w3m-collect)
                    (lambda ()
                      '(("w3m" . 4))))
                   ((symbol-function 'avy--style-fn)
                    (lambda (style)
                      (list
                       'style
                       style)))
                   ((symbol-function 'avy-process)
                    (lambda (candidates &optional overlay)
                      (push
                       (list
                        'avy
                        candidates
                        overlay)
                       events)
                      (car
                       (last candidates))))
                   ((symbol-function 'ace-link--eww-action)
                    (lambda (position external)
                      (push
                       (list
                        'eww-action
                        position
                        external)
                       events)
                      'eww-result))
                   ((symbol-function 'ace-link--w3m-action)
                    (lambda (position)
                      (push
                       (list
                        'w3m-action
                        position)
                       events)
                      'w3m-result))
                   ((symbol-function 'ace-link--compilation-action)
                    (lambda (position)
                      (push
                       (list
                        'compilation-action
                        position)
                       events)
                      'compilation-result)))
           (list
            (ace-link-eww '(16))
            (ace-link-w3m)
            (ace-link-compilation)
            (nreverse events))))"##;
    let expect = expect![
        "OK (eww-result w3m-result compilation-result ((collect nil) (avy (2 7) (style fixture-style)) (eww-action 7 (16)) (avy (4) (style fixture-style)) (w3m-action 4) (collect help-echo) (avy (2 7) (style fixture-style)) (compilation-action 7)))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_widget_org_agenda_xref_custom_and_address_commands_compose_collect_avy_action() {
    let elisp_form = r##"(let ((avy-style
              'fixture-style)
             events)
         (cl-letf (((symbol-function 'ace-link--widget-collect)
                    (lambda ()
                      '(2 3)))
                   ((symbol-function 'ace-link--org-collect)
                    (lambda ()
                      '(("org" . 4))))
                   ((symbol-function 'ace-link--org-agenda-collect)
                    (lambda ()
                      '(("agenda" . 5))))
                   ((symbol-function 'ace-link--xref-collect)
                    (lambda ()
                      '(6)))
                   ((symbol-function 'ace-link--custom-collect)
                    (lambda ()
                      '(7)))
                   ((symbol-function 'ace-link--addr-collect)
                    (lambda ()
                      '(8)))
                   ((symbol-function 'avy--style-fn)
                    (lambda (style)
                      (list
                       'style
                       style)))
                   ((symbol-function 'avy-process)
                    (lambda (candidates &optional overlay)
                      (push
                       (list
                        'avy
                        candidates
                        overlay)
                       events)
                      (car
                       (last candidates))))
                   ((symbol-function 'ace-link--widget-action)
                    (lambda (position)
                      (push
                       (list
                        'widget
                        position)
                       events)
                      'widget-result))
                   ((symbol-function 'ace-link--org-action)
                    (lambda (position)
                      (push
                       (list
                        'org
                        position)
                       events)
                      'org-result))
                   ((symbol-function 'ace-link--org-agenda-action)
                    (lambda (position)
                      (push
                       (list
                        'agenda
                        position)
                       events)
                      'agenda-result))
                   ((symbol-function 'ace-link--xref-action)
                    (lambda (position)
                      (push
                       (list
                        'xref
                        position)
                       events)
                      'xref-result))
                   ((symbol-function 'ace-link--custom-action)
                    (lambda (position)
                      (push
                       (list
                        'custom
                        position)
                       events)
                      'custom-result))
                   ((symbol-function 'ace-link--addr-action)
                    (lambda (position)
                      (push
                       (list
                        'addr
                        position)
                       events)
                      'addr-result)))
           (list
            (ace-link-widget)
            (ace-link-org)
            (ace-link-org-agenda)
            (ace-link-xref)
            (ace-link-custom)
            (ace-link-addr)
            (nreverse events))))"##;
    let expect = expect![
        "OK (widget-result org-result agenda-result xref-result custom-result addr-result ((avy (2 3) (style fixture-style)) (widget 3) (avy (4) (style fixture-style)) (org 4) (avy (5) (style fixture-style)) (agenda 5) (avy (6) (style fixture-style)) (xref 6) (avy (7) (style fixture-style)) (custom 7) (avy (8) (style fixture-style)) (addr 8)))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_sldb_slime_indium_and_cider_commands_compose_collect_avy_action() {
    let elisp_form = r##"(let ((avy-style
              'fixture-style)
             events)
         (cl-letf (((symbol-function 'ace-link--sldb-collect)
                    (lambda ()
                      '(2)))
                   ((symbol-function 'ace-link--slime-xref-collect)
                    (lambda ()
                      '(3)))
                   ((symbol-function 'ace-link--slime-inspector-collect)
                    (lambda ()
                      '(4)))
                   ((symbol-function 'ace-link--indium-inspector-collect)
                    (lambda ()
                      '(5)))
                   ((symbol-function 'ace-link--indium-debugger-frames-collect)
                    (lambda ()
                      '(6)))
                   ((symbol-function 'ace-link--cider-inspector-collect)
                    (lambda ()
                      '(7)))
                   ((symbol-function 'avy--style-fn)
                    (lambda (style)
                      (list
                       'style
                       style)))
                   ((symbol-function 'avy-process)
                    (lambda (candidates &optional overlay)
                      (push
                       (list
                        'avy
                        candidates
                        overlay)
                       events)
                      (car candidates)))
                   ((symbol-function 'ace-link--sldb-action)
                    (lambda (position)
                      (push
                       (list
                        'sldb
                        position)
                       events)
                      'sldb-result))
                   ((symbol-function 'ace-link--slime-xref-action)
                    (lambda (position)
                      (push
                       (list
                        'slime-xref
                        position)
                       events)
                      'slime-xref-result))
                   ((symbol-function 'ace-link--slime-inspector-action)
                    (lambda (position)
                      (push
                       (list
                        'slime-inspector
                        position)
                       events)
                      'slime-inspector-result))
                   ((symbol-function 'ace-link--indium-inspector-action)
                    (lambda (position)
                      (push
                       (list
                        'indium-inspector
                        position)
                       events)
                      'indium-inspector-result))
                   ((symbol-function 'ace-link--indium-debugger-frames-action)
                    (lambda (position)
                      (push
                       (list
                        'indium-frames
                        position)
                       events)
                      'indium-frames-result))
                   ((symbol-function 'ace-link--cider-inspector-action)
                    (lambda (position)
                      (push
                       (list
                        'cider
                        position)
                       events)
                      'cider-result)))
           (list
            (ace-link-sldb)
            (ace-link-slime-xref)
            (ace-link-slime-inspector)
            (ace-link-indium-inspector)
            (ace-link-indium-debugger-frames)
            (ace-link-cider-inspector)
            (nreverse events))))"##;
    let expect = expect![
        "OK (sldb-result slime-xref-result slime-inspector-result indium-inspector-result indium-frames-result cider-result ((avy (2) (style fixture-style)) (sldb 2) (avy (3) (style fixture-style)) (slime-xref 3) (avy (4) (style fixture-style)) (slime-inspector 4) (avy (5) (style fixture-style)) (indium-inspector 5) (avy (6) (style fixture-style)) (indium-frames 6) (avy (7) (style fixture-style)) (cider 7)))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_gnus_article_command_collects_then_acts_in_current_window() {
    let elisp_form = r##"(let ((major-mode
              'gnus-article-mode)
             (avy-style
              'fixture-style)
             events)
         (cl-letf (((symbol-function 'ace-link--gnus-collect)
                    (lambda ()
                      '(2 7)))
                   ((symbol-function 'avy--style-fn)
                    (lambda (style)
                      (list
                       'style
                       style)))
                   ((symbol-function 'avy-process)
                    (lambda (candidates &optional overlay)
                      (push
                       (list
                        'avy
                        candidates
                        overlay)
                       events)
                      7))
                   ((symbol-function 'ace-link--gnus-action)
                    (lambda (position)
                      (push
                       (list
                        'action
                        position)
                       events)
                      'gnus-result)))
           (list
            (ace-link-gnus)
            (nreverse events))))"##;
    let expect = expect!["OK (gnus-result ((avy (2 7) (style fixture-style)) (action 7)))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_gnus_summary_signals_when_no_visible_article_window_exists() {
    let elisp_form = r##"(cl-progv
         '(gnus-article-buffer)
         '(" *fixture-article*")
         (let ((major-mode
                'gnus-summary-mode))
           (cl-letf (((symbol-function 'gnus-get-buffer-window)
                      (lambda (&rest _)
                        nil)))
             (ace-link-gnus))))"##;
    let expect = expect![[r#"ERR (user-error "No article window found")"#]];
    super::assert_ace_link_signal_parity(elisp_form, expect);
}

#[test]
fn ace_link_mu4e_delegates_to_gnus_or_composes_html_collection_and_action() {
    let elisp_form = r##"(let ((avy-style
              'fixture-style)
             events)
         (cl-letf (((symbol-function 'ace-link-gnus)
                    (lambda ()
                      (push
                       '(gnus)
                       events)
                      'gnus-result))
                   ((symbol-function 'ace-link--email-view-html-collect)
                    (lambda (&optional mu4e)
                      (push
                       (list
                        'collect
                        mu4e)
                       events)
                      '(("first" . 2)
                        ("second" . 8))))
                   ((symbol-function 'avy--style-fn)
                    (lambda (style)
                      (list
                       'style
                       style)))
                   ((symbol-function 'avy-process)
                    (lambda (candidates &optional overlay)
                      (push
                       (list
                        'avy
                        candidates
                        overlay)
                       events)
                      8))
                   ((symbol-function 'ace-link--mu4e-action)
                    (lambda (position)
                      (push
                       (list
                        'action
                        position)
                       events)
                      'mu4e-result)))
           (let ((gnus-result
                  (cl-progv
                      '(mu4e-view-use-gnus)
                      '(t)
                    (ace-link-mu4e)))
                 (html-result
                  (cl-progv
                      '(mu4e-view-use-gnus)
                      '(nil)
                    (ace-link-mu4e))))
             (list
              gnus-result
              html-result
              (nreverse events)))))"##;
    let expect = expect![
        "OK (gnus-result mu4e-result ((gnus) (collect t) (avy (2 8) (style fixture-style)) (action 8)))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_notmuch_plain_and_html_commands_cover_selection_and_gnus_delegation() {
    let elisp_form = r##"(let ((avy-style
              'fixture-style)
             events)
         (cl-letf (((symbol-function 'ace-link-gnus)
                    (lambda ()
                      (push
                       '(gnus)
                       events)
                      'gnus-result))
                   ((symbol-function 'ace-link--email-view-plain-collect)
                    (lambda ()
                      '(2 5)))
                   ((symbol-function 'ace-link--email-view-html-collect)
                    (lambda (&optional mu4e)
                      (push
                       (list
                        'html-collect
                        mu4e)
                       events)
                      '(("html" . 7))))
                   ((symbol-function 'avy-process)
                    (lambda (candidates &optional overlay)
                      (push
                       (list
                        'avy
                        candidates
                        overlay)
                       events)
                      (car
                       (last candidates))))
                   ((symbol-function 'avy--style-fn)
                    (lambda (style)
                      (list
                       'style
                       style)))
                   ((symbol-function 'avy--overlay-pre)
                    (lambda (&rest arguments)
                      (cons
                       'overlay
                       arguments)))
                   ((symbol-function 'ace-link--notmuch-plain-action)
                    (lambda (position)
                      (push
                       (list
                        'plain-action
                        position)
                       events)
                      'plain-result))
                   ((symbol-function 'ace-link--mu4e-action)
                    (lambda (position)
                      (push
                       (list
                        'html-action
                        position)
                       events)
                      'html-result)))
           (let ((plain-result
                  (ace-link-notmuch-plain))
                 (html-result
                  (cl-progv
                      '(mu4e-view-use-gnus)
                      '(nil)
                    (ace-link-notmuch-html)))
                 (gnus-result
                  (cl-progv
                      '(mu4e-view-use-gnus)
                      '(t)
                    (ace-link-notmuch-html))))
             (list
              plain-result
              html-result
              gnus-result
              (nreverse events)))))"##;
    let expect = expect![
        "OK (plain-result html-result gnus-result ((avy (2 5) avy--overlay-pre) (plain-action 5) (html-collect nil) (avy (7) (style fixture-style)) (html-action 7) (gnus)))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_notmuch_combined_command_dispatches_the_function_paired_with_avy_match() {
    let elisp_form = r##"(let (events)
         (cl-letf (((symbol-function 'ace-link--notmuch-collect)
                    (lambda ()
                      '((3 . fixture-plain)
                        (8 . fixture-html))))
                   ((symbol-function 'avy-process)
                    (lambda (candidates &optional overlay)
                      (push
                       (list
                        'avy
                        candidates
                        overlay)
                       events)
                      (cadr candidates)))
                   ((symbol-function 'avy--overlay-pre)
                    (lambda (&rest arguments)
                      (cons
                       'overlay
                       arguments)))
                   ((symbol-function 'fixture-plain)
                    (lambda (position)
                      (push
                       (list
                        'plain
                        position)
                       events)))
                   ((symbol-function 'fixture-html)
                    (lambda (position)
                      (push
                       (list
                        'html
                        position)
                       events)
                      'html-result)))
           (list
            (ace-link-notmuch)
            (nreverse events))))"##;
    let expect = expect![
        "OK (html-result ((avy ((3 . fixture-plain) (8 . fixture-html)) avy--overlay-pre) (html 8)))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_commit_collects_issue_numbers_selects_one_resolves_url_and_fetches_it() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "Subject #12\nBody #345\nTail")
         (let (events)
           (provide 'counsel)
           (provide 'ffap)
           (cl-progv
               '(ivy-ffap-url-functions
                 ffap-url-fetcher)
               (list
                (list
                 (lambda ()
                   nil)
                 (lambda ()
                   (let ((url
                          (and
                           (looking-at
                            "#\\([0-9]+\\)")
                           (format
                            "https://example.test/issue/%s"
                            (match-string 1)))))
                     (push
                      (list
                       'resolve
                       (point)
                       url)
                      events)
                     url)))
                (lambda (url)
                  (push
                   (list
                    'fetch
                    url)
                   events)
                  'fetched))
             (let ((major-mode
                    'magit-commit-mode))
               (cl-letf (((symbol-function 'magit-goto-next-section)
                          (lambda ()
                            (push
                             '(next-section)
                             events)))
                         ((symbol-function 'magit-current-section)
                          (lambda ()
                            'fixture-section))
                         ((symbol-function 'magit-section-end)
                          (lambda (section)
                            (push
                             (list
                              'section-end
                              section)
                             events)
                            (point-max)))
                         ((symbol-function 'avy-process)
                          (lambda (positions &optional overlay)
                            (push
                             (list
                             'avy
                              positions
                              overlay)
                             events)
                            (let ((selected
                                   (car
                                    (last positions))))
                              (goto-char selected)
                              selected))))
                 (list
                  (ace-link-commit)
                  (nreverse events)))))))"##;
    let expect = expect![[
        r#"OK (fetched ((next-section) (section-end fixture-section) (avy (9 18) nil) (resolve 18 "https://example.test/issue/345") (fetch "https://example.test/issue/345")))"#
    ]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_help_real_button_workflow_collects_selects_and_invokes_button_action() {
    let elisp_form = r##"(with-temp-buffer
         (insert "aaFirst bbSecond ")
         (let (events)
           (make-text-button
            3 8
            'label "First"
            'action
            (lambda (button)
              (push
               (list
                'pressed
                (button-label button)
                (point))
               events)))
           (make-text-button
            11 17
            'label "Second"
            'action
            (lambda (button)
              (push
               (list
                'pressed
                (button-label button)
                (point))
               events)))
           (cl-letf (((symbol-function 'window-start)
                      (lambda (&rest _)
                        (point-min)))
                     ((symbol-function 'window-end)
                      (lambda (&rest _)
                        (point-max)))
                     ((symbol-function 'avy--style-fn)
                      (lambda (style)
                        (list
                         'style
                         style)))
                     ((symbol-function 'avy-process)
                      (lambda (positions &optional overlay)
                        (push
                         (list
                          'selected-from
                          positions
                          overlay)
                         events)
                        (car positions))))
             (list
              (ace-link-help)
              (nreverse events)
              (point)))))"##;
    let expect =
        expect![[r#"OK (t ((selected-from (3 11) (style at-full)) (pressed "First" 4)) 4)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_help_real_workflow_signals_when_the_final_button_reaches_buffer_end() {
    let elisp_form = r##"(with-temp-buffer
         (insert "aaFirst")
         (make-text-button
          3 8
          'label "First"
          'action
          (lambda (_button)
            'pressed))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (ace-link-help)))"##;
    let expect = expect!["ERR (wrong-type-argument integer-or-marker-p nil)"];
    super::assert_ace_link_signal_parity(elisp_form, expect);
}

#[test]
fn ace_link_eww_real_property_workflow_skips_newlines_selects_and_forwards_prefix() {
    let elisp_form = r##"(with-temp-buffer
         (insert "aa\n\nFirst bb Second")
         (add-text-properties
          3 10
          '(shr-url "first"))
         (add-text-properties
          13 19
          '(shr-url "second"))
         (let (events)
           (cl-letf (((symbol-function 'window-start)
                      (lambda (&rest _)
                        (point-min)))
                     ((symbol-function 'window-end)
                      (lambda (&rest _)
                        (point-max)))
                     ((symbol-function 'avy--style-fn)
                      (lambda (style)
                        (list
                         'style
                         style)))
                     ((symbol-function 'avy-process)
                      (lambda (positions &optional overlay)
                        (push
                         (list
                          'selected-from
                          positions
                          overlay)
                         events)
                        (car
                         (last positions))))
                     ((symbol-function 'eww-follow-link)
                      (lambda (external)
                        (push
                         (list
                          'follow
                          (point)
                          external
                          (get-text-property
                           (point)
                           'shr-url))
                         events)
                        'followed)))
             (list
              (ace-link-eww
               '(16))
              (nreverse events)
              (point)))))"##;
    let expect = expect![[
        r#"OK (followed ((selected-from (5 13) (style at-full)) (follow 13 (16) "second")) 13)"#
    ]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_gnus_summary_selects_visible_article_window_then_restores_original_selection() {
    let elisp_form = r##"(save-window-excursion
         (let* ((original
                 (selected-window))
                (article
                 (split-window
                  original))
                events)
           (cl-progv
               '(gnus-article-buffer)
               '(" *fixture-article*")
             (let ((major-mode
                    'gnus-summary-mode))
               (cl-letf (((symbol-function 'gnus-get-buffer-window)
                          (lambda (buffer visibility)
                            (push
                             (list
                              'find
                              buffer
                              visibility
                              (eq
                               (selected-window)
                               original))
                             events)
                            article))
                         ((symbol-function 'select-frame-set-input-focus)
                          (lambda (frame)
                            (push
                             (list
                              'focus
                              (eq frame
                                  (window-frame article))
                              (eq
                               (selected-window)
                               article))
                             events)))
                         ((symbol-function 'ace-link--gnus-collect)
                          (lambda ()
                            (push
                             (list
                              'collect
                              (eq
                               (selected-window)
                               article))
                             events)
                            '(4)))
                         ((symbol-function 'avy--style-fn)
                          (lambda (style)
                            (list
                             'style
                             style)))
                         ((symbol-function 'avy-process)
                          (lambda (positions &optional overlay)
                            (push
                             (list
                              'avy
                              positions
                              overlay
                              (eq
                               (selected-window)
                               article))
                             events)
                            4))
                         ((symbol-function 'ace-link--gnus-action)
                          (lambda (position)
                            (push
                             (list
                              'action
                              position
                              (eq
                               (selected-window)
                               article))
                             events)
                            'gnus-result)))
                 (list
                  (ace-link-gnus)
                  (eq
                   (selected-window)
                   original)
                  (window-live-p article)
                  (nreverse events)))))))"##;
    let expect = expect![[
        r#"OK (gnus-result t t ((find " *fixture-article*" visible t) (focus t t) (collect t) (avy (4) (style at-full) t) (action 4 t)))"#
    ]];
    assert_ace_link_parity(elisp_form, expect);
}
