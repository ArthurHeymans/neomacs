use super::{assert_ack_menu_parity, assert_ack_menu_signal_parity};
use expect_test::expect;

#[test]
fn ack_menu_property_helpers_cover_runs_boundaries_gaps_and_previous_values() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "abcdefghi")
         (add-text-properties
          1 4
          '(fixture first))
         (add-text-properties
          6 9
          '(fixture second))
         (list
          (mapcar
           (lambda (position)
             (ack-previous-property-value
              'fixture
              position))
           '(1 3 4 5 6 9))
          (mapcar
           (lambda (position)
             (ack-property-beg
              position
              'fixture))
           '(1 2 3 4 6 7 8 9))
          (mapcar
           (lambda (position)
             (ack-property-end
              position
              'fixture))
           '(1 2 3 4 6 7 8 9))))"##;
    let expect = expect![
        "OK ((first first nil first second nil) (1 nil nil nil 6 6 6 nil) (4 4 3 nil 9 9 8 nil))"
    ];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_next_and_previous_marker_traverse_property_runs_exactly() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "aabbccdd")
         (add-text-properties
          1 3
          '(fixture t))
         (add-text-properties
          5 7
          '(fixture t))
         (list
          (progn
            (goto-char 1)
            (ack-next-marker
             1 1
             'fixture
             "fixture"))
          (progn
            (goto-char 3)
            (ack-next-marker
             3 1
             'fixture
             "fixture"))
          (progn
            (goto-char 7)
            (ack-previous-marker
             7 1
             'fixture
             "fixture"))
          (point)))"##;
    let expect = expect!["OK (5 5 3 3)"];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_next_marker_signals_past_last_property_run() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "abcd")
         (add-text-properties
          1 3
          '(fixture t))
         (goto-char 1)
         (ack-next-marker
          1 3
          'fixture
          "fixture"))"##;
    let expect = expect![[r#"ERR (error "Moved past last fixture")"#]];
    assert_ack_menu_signal_parity(elisp_form, expect);
}

#[test]
fn ack_menu_previous_marker_signals_before_first_property_run() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "abcd")
         (add-text-properties
          2 4
          '(fixture t))
         (goto-char 4)
         (ack-previous-marker
          4 3
          'fixture
          "fixture"))"##;
    let expect = expect![[r#"ERR (error "Moved back before first fixture")"#]];
    assert_ack_menu_signal_parity(elisp_form, expect);
}

#[test]
fn ack_menu_next_marker_rejects_zero_count() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "abcd")
         (ack-next-marker
          1 0
          'fixture
          "fixture"))"##;
    let expect = expect!["ERR (cl-assertion-failed (> arg 0))"];
    assert_ack_menu_signal_parity(elisp_form, expect);
}

#[test]
fn ack_menu_match_and_file_navigation_wrappers_forward_exact_markers_and_counts() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'ack-next-marker)
               (lambda (&rest arguments)
                 (push
                  (cons 'next arguments)
                  calls)
                 'next-result))
              ((symbol-function
                'ack-previous-marker)
               (lambda (&rest arguments)
                 (push
                  (cons 'previous arguments)
                  calls)
                 'previous-result)))
           (with-temp-buffer
             (insert
              "fixture")
             (list
              (progn
                (goto-char 3)
                (ack-next-match
                 (point)
                 2))
              (ack-previous-match
               5 3)
              (progn
                (goto-char
                 (point-min))
                (ack-next-file
                 (point)
                 1))
              (ack-previous-file
               8 4)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (next-result previous-result next-result previous-result ((next 3 2 ack-match "match") (previous 5 3 ack-match "match") (next 1 2 ack-file "file") (previous 8 4 ack-file "file")))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_next_error_tracks_reset_forward_and_backward_positions() {
    let elisp_form = r##"(let ((ack-error-pos
                7)
               calls)
         (cl-letf
             (((symbol-function
                'ack-next-match)
               (lambda (position count)
                 (push
                  (list
                   'next
                   position
                   count)
                  calls)
                 20))
              ((symbol-function
                'ack-previous-match)
               (lambda (position count)
                 (push
                  (list
                   'previous
                   position
                   count)
                  calls)
                 3))
              ((symbol-function
                'ack-find-match)
               (lambda (position)
                 (push
                  (list
                   'find
                   position)
                  calls)
                 (setq ack-error-pos
                       position)
                 'found)))
           (with-temp-buffer
             (insert
              "0123456789")
             (list
              (ack-next-error-function
               2 nil)
              ack-error-pos
              (ack-next-error-function
               -3 t)
              ack-error-pos
              (nreverse calls)))))"##;
    let expect = expect!["OK (found 20 found 3 ((next 7 2) (find 20) (previous 1 3) (find 3)))"];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_move_to_line_widens_and_handles_first_middle_and_past_end() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "one\ntwo\nthree\n")
         (let (positions)
           (dolist (line
                    '(1 2 3 99))
             (narrow-to-region
              5 9)
             (ack--move-to-line
              line)
             (push
              (list
               line
               (point)
               (line-number-at-pos))
              positions)
             (widen))
           (nreverse positions)))"##;
    let expect = expect!["OK ((1 5 1) (2 5 1) (3 9 2) (99 9 2))"];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_create_marker_resolves_file_line_offset_and_force_semantics() {
    let elisp_form = r##"(let ((message-buffer
                (generate-new-buffer
                 " *ack-marker-message*"))
               (target-buffer
                (generate-new-buffer
                 " *ack-marker-target*"))
               calls)
         (unwind-protect
             (progn
               (with-current-buffer
                   target-buffer
                 (insert
                  "line-one\nline-two\nline-three\n"))
               (with-current-buffer
                   message-buffer
                 (insert
                  "file.el\n12: match")
                 (add-text-properties
                  1 8
                  '(ack-file
                    "/fixture/file.el"))
                 (add-text-properties
                  9 12
                  '(ack-line
                    "12"))
                 (cl-letf
                     (((symbol-function
                        'file-exists-p)
                       (lambda (path)
                         (push
                          (list
                           'exists
                           path)
                          calls)
                         t))
                      ((symbol-function
                        'find-file-noselect)
                       (lambda (path)
                         (push
                          (list
                           'noselect
                           path)
                          calls)
                         target-buffer))
                      ((symbol-function
                        'find-buffer-visiting)
                       (lambda (path)
                         (push
                          (list
                           'visiting
                           path)
                          calls)
                         target-buffer))
                      ((symbol-function
                        'ack--move-to-line)
                       (lambda (line)
                         (push
                          (list
                           'move
                           line)
                          calls)
                         (goto-char 10))))
                   (let ((forced
                          (ack-create-marker
                           14 t))
                         (existing
                          (ack-create-marker
                           15 nil)))
                     (list
                      (list
                       (marker-position
                        forced)
                       (buffer-name
                        (marker-buffer
                         forced)))
                      (list
                       (marker-position
                        existing)
                       (buffer-name
                        (marker-buffer
                         existing)))
                      (nreverse calls))))))
           (when
               (buffer-live-p
                message-buffer)
             (kill-buffer
              message-buffer))
           (when
               (buffer-live-p
                target-buffer)
             (kill-buffer
              target-buffer))))"##;
    let expect = expect![[
        r#"OK ((11 " *ack-marker-target*") (12 " *ack-marker-target*") ((exists "/fixture/file.el") (noselect "/fixture/file.el") (move 12) (visiting "/fixture/file.el") (move 12)))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_create_marker_returns_nil_without_force_or_a_visiting_buffer() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "file.el\n12: match")
         (add-text-properties
          1 8
          '(ack-file
            "/fixture/not-visited.el"))
         (add-text-properties
          9 12
          '(ack-line
            "12"))
         (let (calls)
           (cl-letf
               (((symbol-function
                  'find-buffer-visiting)
                 (lambda (path)
                   (push path calls)
                   nil)))
             (list
              (ack-create-marker
               14 nil)
              (nreverse calls)))))"##;
    let expect = expect![[r#"OK (nil ("/fixture/not-visited.el"))"#]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_create_marker_signals_exact_missing_file_when_forced() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "file.el\n12: match")
         (add-text-properties
          1 8
          '(ack-file
            "/fixture/missing.el"))
         (add-text-properties
          9 12
          '(ack-line
            "12"))
         (cl-letf
             (((symbol-function
                'file-exists-p)
               (lambda (path)
                 path
                 nil)))
           (ack-create-marker
            14 t)))"##;
    let expect = expect![[r#"ERR (error "File </fixture/missing.el> not found")"#]];
    assert_ack_menu_signal_parity(elisp_form, expect);
}

#[test]
fn ack_menu_find_match_is_a_no_op_outside_match_properties() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "plain output")
         (let ((ack-error-pos
                'unchanged)
               (overlay-arrow-position
                'unchanged)
               calls)
           (cl-letf
               (((symbol-function
                  'ack-create-marker)
                 (lambda (&rest arguments)
                   (push
                    (cons
                     'create
                     arguments)
                    calls)))
                ((symbol-function
                  'compilation-goto-locus)
                 (lambda (&rest arguments)
                   (push
                    (cons
                     'goto
                     arguments)
                    calls))))
             (list
              (ack-find-match
               3)
              ack-error-pos
              overlay-arrow-position
              calls))))"##;
    let expect = expect!["OK (nil unchanged unchanged nil)"];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_find_match_caches_marker_sets_overlay_and_calls_compilation_locus() {
    let elisp_form = r##"(let ((message-buffer
                (generate-new-buffer
                 " *ack-find-message*"))
               (target-buffer
                (generate-new-buffer
                 " *ack-find-target*"))
               calls
               overlay-arrow-position)
         (unwind-protect
             (progn
               (with-current-buffer
                   target-buffer
                 (insert
                  "target text"))
               (with-current-buffer
                   message-buffer
                 (insert
                  "before MATCH after")
                 (add-text-properties
                  8 13
                  '(ack-match t))
                 (cl-letf
                     (((symbol-function
                        'ack-create-marker)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'create
                           arguments)
                          calls)
                         (with-current-buffer
                             target-buffer
                           (copy-marker
                            3))))
                      ((symbol-function
                        'compilation-goto-locus)
                       (lambda (message marker end)
                         (push
                          (list
                           'goto
                           (marker-position
                            message)
                           (marker-position
                            marker)
                           (marker-position
                            end))
                          calls))))
                   (list
                    (ack-find-match
                     9)
                    (ack-find-match
                     10)
                    (and
                     (markerp
                      overlay-arrow-position)
                     (marker-position
                      overlay-arrow-position))
                    ack-error-pos
                    (let ((cached
                           (get-text-property
                            8
                            'ack-marker)))
                      (and
                       (markerp cached)
                       (list
                        (marker-position
                         cached)
                        (buffer-name
                         (marker-buffer
                          cached)))))
                    (nreverse calls)))))
           (when
               (buffer-live-p
                message-buffer)
             (kill-buffer
              message-buffer))
           (when
               (buffer-live-p
                target-buffer)
             (kill-buffer
              target-buffer))))"##;
    let expect = expect![[
        r#"OK ((:marker nil nil) (:marker nil nil) 1 8 (3 " *ack-find-target*") ((create (:marker nil nil) t) (goto 8 3 8) (goto 8 3 8)))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}
