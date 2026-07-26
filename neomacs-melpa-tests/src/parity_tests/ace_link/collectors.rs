use super::assert_ace_link_parity;
use expect_test::expect;

#[test]
fn ace_link_help_collect_returns_visible_button_labels_and_start_positions() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (make-text-button
          2 5
          'label "First")
         (make-text-button
          7 10
          'label "Second")
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (list
            (ace-link--help-collect)
            (point))))"##;
    let expect = expect![[r#"OK ((("123" . 2) ("678" . 7)) 11)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_woman_collect_returns_buttons_in_forward_order_without_moving_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (make-text-button
          2 4
          'label "First")
         (make-text-button
          7 9
          'label "Second")
         (goto-char 5)
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (list
            (ace-link--woman-collect)
            (point))))"##;
    let expect = expect![[r#"OK ((("12" . 2) ("67" . 7)) 5)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_eww_collect_handles_leading_newlines_adjacent_links_and_custom_property() {
    let elisp_form = r##"(with-temp-buffer
         (insert "aa\n\nFirst bb Second\n\ncc")
         (let ((first-beg 3)
               (first-end 10)
               (second-beg 13)
               (second-end 21))
           (add-text-properties
            first-beg first-end
            '(shr-url "first"))
           (add-text-properties
            second-beg second-end
            '(shr-url "second"))
           (add-text-properties
            22 24
            '(fixture-link "newlines"))
           (cl-letf (((symbol-function 'window-start)
                      (lambda (&rest _)
                        (point-min)))
                     ((symbol-function 'window-end)
                      (lambda (&rest _)
                        (point-max))))
             (list
              (ace-link--eww-collect)
              (ace-link--eww-collect
               'fixture-link)
              (point-min)
              (point-max)))))"##;
    let expect = expect![[r#"OK ((("First" . 5) (" Second\n" . 13)) (("cc" . 22)) 1 24)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_eww_collect_handles_a_window_start_inside_a_link_and_link_at_buffer_end() {
    let elisp_form = r##"(with-temp-buffer
         (insert "prefix LINK suffix FINAL")
         (add-text-properties
          8 12
          '(shr-url first))
         (add-text-properties
          20 25
          '(shr-url final))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      9))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (list
            (ace-link--eww-collect)
            (point)
            (point-min)
            (point-max))))"##;
    let expect = expect![[r#"OK ((("INK" . 9) ("FINAL" . 20)) 25 1 25)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_eww_collect_omits_an_all_newline_link_and_continues_to_later_candidates() {
    let elisp_form = r##"(with-temp-buffer
         (insert "First\n\n\n xx Second tail")
         (add-text-properties
          1 6
          '(shr-url first))
         (add-text-properties
          6 9
          '(shr-url all-newlines))
         (add-text-properties
          13 19
          '(shr-url second))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (list
            (ace-link--eww-collect)
            (point))))"##;
    let expect = expect![[r#"OK ((("First" . 1) ("Second" . 13)) 24)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_w3m_collect_groups_each_anchor_sequence_run_in_visible_order() {
    let elisp_form = r##"(with-temp-buffer
         (insert "aaFirst bb Second cc")
         (add-text-properties
          3 8
          '(w3m-anchor-sequence 1))
         (add-text-properties
          12 18
          '(w3m-anchor-sequence 2))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (list
            (ace-link--w3m-collect)
            (point))))"##;
    let expect = expect![[r#"OK ((("First" . 3) ("Second" . 12)) 21)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_info_collect_keeps_targets_and_positions_in_visible_forward_order() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (goto-char 7)
         (let ((references '(2 5 10)))
           (cl-letf (((symbol-function 'window-start)
                      (lambda (&rest _)
                        1))
                     ((symbol-function 'window-end)
                      (lambda (&rest _)
                        10))
                     ((symbol-function 'Info-next-reference)
                      (lambda ()
                        (goto-char
                         (pop references))))
                     ((symbol-function 'ace-link--info-current)
                      (lambda ()
                        (cons
                         (intern
                          (format
                           "target-%s"
                           (point)))
                         (point)))))
             (list
              (ace-link--info-collect)
              (point)
              references))))"##;
    let expect = expect!["OK (((target-2 . 2) (target-5 . 5)) 7 nil)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_man_collect_includes_buttons_and_manpage_shaped_properties_only() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (add-text-properties
          2 4
          '(fixture first))
         (add-text-properties
          6 8
          '(fixture second))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      1))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      10))
                   ((symbol-function 'Man-default-man-entry)
                    (lambda (position)
                      (cond
                       ((= position 2)
                        "printf(3)")
                       ((= position 6)
                        "plain")
                       (t
                        "ignored"))))
                   ((symbol-function 'button-at)
                    (lambda (position)
                      (and
                       (= position 6)
                       'fixture-button))))
           (ace-link--man-collect)))"##;
    let expect = expect![[r#"OK (("printf(3)" . 2) ("plain" . 6))"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_org_collect_returns_visible_matches_and_excludes_hidden_links() {
    let elisp_form = r##"(with-temp-buffer
         (insert "[[one]] xx [[two]] yy [[three]]")
         (setq org-link-any-re
               "\\[\\[[^]]+\\]\\]")
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max)))
                   ((symbol-function 'outline-invisible-p)
                    (lambda (position)
                      (and
                       (>= position 13)
                       (< position 20)))))
           (list
            (ace-link--org-collect)
            (point))))"##;
    let expect = expect![[r#"OK ((("[[one]]" . 1) ("[[three]]" . 23)) 32)"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_org_agenda_collect_returns_txt_labels_at_each_marker_run() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (add-text-properties
          2 4
          '(org-marker first-marker
            txt "First"))
         (add-text-properties
          7 10
          '(org-marker second-marker
            txt "Second"))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (ace-link--org-agenda-collect)))"##;
    let expect = expect![[r#"OK (("First" . 2) ("Second" . 7))"#]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_xref_collect_returns_one_position_per_visible_property_run() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (add-text-properties
          2 5
          '(xref-item first))
         (add-text-properties
          7 9
          '(xref-item second))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (ace-link--xref-collect)))"##;
    let expect = expect!["OK (2 7)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_address_collect_returns_only_matching_overlay_starts_in_overlay_order() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (let ((first
                (make-overlay 2 4))
               (ignored
                (make-overlay 5 6))
               (second
                (make-overlay 7 9)))
           (overlay-put
            first
            'goto-address
            t)
           (overlay-put
            second
            'goto-address
            'url)
           (cl-letf (((symbol-function 'window-start)
                      (lambda (&rest _)
                        (point-min)))
                     ((symbol-function 'window-end)
                      (lambda (&rest _)
                        (point-max))))
             (list
              (ace-link--addr-collect)
              (overlay-start ignored)))))"##;
    let expect = expect!["OK ((2 7) 5)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_slime_xref_collect_returns_each_property_run_start() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (add-text-properties
          2 4
          '(slime-location first))
         (add-text-properties
          7 9
          '(slime-location second))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (ace-link--slime-xref-collect)))"##;
    let expect = expect!["OK (2 7)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_slime_inspector_collect_unifies_part_range_and_action_positions() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (add-text-properties
          2 3
          '(slime-part-number 1))
         (add-text-properties
          5 6
          '(slime-range-button t))
         (add-text-properties
          8 9
          '(slime-action-number 2))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (ace-link--slime-inspector-collect)))"##;
    let expect = expect!["OK (2 5 8)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_cider_inspector_collect_returns_each_value_property_run_start() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (add-text-properties
          2 4
          '(cider-value-idx 1))
         (add-text-properties
          7 9
          '(cider-value-idx 2))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max))))
           (ace-link--cider-inspector-collect)))"##;
    let expect = expect!["OK (2 7)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_widget_and_custom_collectors_preserve_widget_walk_order_and_filter_buttons() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (add-text-properties
          5 6
          '(button fixture-button))
         (let ((positions '(2 5 8 8))
               widget-result
               custom-result)
           (cl-letf (((symbol-function 'window-start)
                      (lambda (&rest _)
                        (point-min)))
                     ((symbol-function 'window-end)
                      (lambda (&rest _)
                        (point-max)))
                     ((symbol-function 'widget-forward)
                      (lambda (&rest _)
                        (goto-char
                         (pop positions)))))
             (setq widget-result
                   (ace-link--widget-collect)))
           (setq positions
                 '(2 5 8 8))
           (cl-letf (((symbol-function 'window-start)
                      (lambda (&rest _)
                        (point-min)))
                     ((symbol-function 'window-end)
                      (lambda (&rest _)
                        (point-max)))
                     ((symbol-function 'widget-forward)
                      (lambda (&rest _)
                        (goto-char
                         (pop positions)))))
             (setq custom-result
                   (ace-link--custom-collect)))
           (list
            widget-result
            custom-result
            (point))))"##;
    let expect = expect!["OK ((2 5 8) (5) 11)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_indium_collectors_stop_on_repeated_or_minimum_positions() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (let ((inspector-positions
                '(8 5 5))
               (frame-positions
                '(8 4 1))
               inspector-result
               frame-result)
           (cl-letf (((symbol-function 'indium-inspector-previous-reference)
                      (lambda ()
                        (goto-char
                         (pop inspector-positions)))))
             (setq inspector-result
                   (ace-link--indium-inspector-collect)))
           (cl-letf (((symbol-function 'indium-debugger-frames-previous-frame)
                      (lambda ()
                        (goto-char
                         (pop frame-positions)))))
             (setq frame-result
                   (ace-link--indium-debugger-frames-collect)))
           (list
            inspector-result
            frame-result
            (point))))"##;
    let expect = expect!["OK ((5 8) (4 8) 11)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_gnus_collect_switches_between_modern_buttons_and_legacy_widget_properties() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (add-text-properties
          3 4
          '(gnus-string first))
         (add-text-properties
          7 8
          '(shr-url second))
         (require 'wid-edit)
         (let ((widget-positions
                '(3 7 7))
               modern
               legacy)
           (cl-letf (((symbol-function 'ace-link--woman-collect)
                      (lambda ()
                        '(("One" . 3)
                          ("Two" . 7)))))
             (cl-progv
                 '(emacs-major-version)
                 '(30)
               (setq modern
                     (ace-link--gnus-collect))))
           (cl-letf (((symbol-function 'window-start)
                      (lambda (&rest _)
                        (point-min)))
                     ((symbol-function 'window-end)
                      (lambda (&rest _)
                        (point-max)))
                     ((symbol-function 'widget-forward)
                      (lambda (&rest _)
                        (goto-char
                         (pop widget-positions)))))
             (cl-progv
                 '(emacs-major-version)
                 '(26)
               (setq legacy
                     (ace-link--gnus-collect))))
           (list
            modern
            legacy
            (point))))"##;
    let expect = expect!["OK ((3 7) (3 7) 11)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_sldb_collect_places_local_values_before_frame_positions() {
    let elisp_form = r##"(with-temp-buffer
         (insert "frame one\nlocal value\nframe two\n")
         (add-text-properties
          1 10
          '(frame first-frame))
         (add-text-properties
          11 22
          '(frame first-frame
            var local-var))
         (add-text-properties
          17 22
          '(face sldb-local-value-face))
         (add-text-properties
          23 32
          '(frame second-frame))
         (cl-letf (((symbol-function 'window-start)
                    (lambda (&rest _)
                      (point-min)))
                   ((symbol-function 'window-end)
                    (lambda (&rest _)
                      (point-max)))
                   ((symbol-function 'forward-visible-line)
                    (lambda (&optional count)
                      (forward-line
                       (or count 1)))))
           (ace-link--sldb-collect)))"##;
    let expect = expect!["OK (17 1 23)"];
    assert_ace_link_parity(elisp_form, expect);
}
