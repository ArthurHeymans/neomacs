use super::assert_ace_jump_mode_parity;
use expect_test::expect;

#[test]
fn ace_jump_mode_character_categories_cover_ascii_boundaries() {
    let elisp_form = r##"(mapcar
         (lambda (character)
           (list
            character
            (ace-jump-char-category character)))
         '(8 9 10 31 32 47 48 57 58 64 65 90
           91 96 97 122 123 126 127))"##;
    let expect = expect![
        "OK ((8 other) (9 punc) (10 other) (31 other) (32 punc) (47 punc) (48 digit) (57 digit) (58 punc) (64 punc) (65 alpha) (90 alpha) (91 punc) (96 punc) (97 alpha) (122 alpha) (123 punc) (126 punc) (127 other))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_character_categories_cover_unicode_and_out_of_range_integers() {
    let elisp_form = r##"(mapcar
         (lambda (character)
           (list
            character
            (ace-jump-char-category character)))
         '(-1 0 255 955 9731 1114111))"##;
    let expect =
        expect!["OK ((-1 other) (0 other) (255 other) (955 other) (9731 other) (1114111 other))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_candidate_search_respects_case_fold_setting() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-candidates*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "Alpha alpha ALPHA end"))
               (let ((area
                      (make-aj-visual-area
                       :buffer buffer
                       :window (selected-window)
                       :frame (selected-frame))))
                 (mapcar
                  (lambda (fold)
                    (let ((ace-jump-mode-case-fold fold))
                      (mapcar
                       #'aj-position-offset
                       (ace-jump-search-candidate
                        "alpha"
                        (list area)))))
                  '(nil t))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK ((7) (1 7 13))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_candidate_search_returns_visual_area_identity_and_offsets() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-candidates*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "ab xx ab yy ab z"))
               (let* ((area
                       (make-aj-visual-area
                        :buffer buffer
                        :window (selected-window)
                        :frame (selected-frame)))
                      (positions
                       (ace-jump-search-candidate
                        "ab"
                        (list area))))
                 (mapcar
                  (lambda (position)
                    (list
                     (aj-position-offset position)
                     (eq
                      (aj-position-visual-area position)
                      area)
                     (eq
                      (aj-position-buffer position)
                      buffer)
                     (eq
                      (aj-position-window position)
                      (selected-window))))
                  positions)))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK ((1 t t t) (7 t t t) (13 t t t))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_candidate_search_filter_is_point_dependent() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-filter*"))
             observed)
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "x-x-x-x"))
               (let* ((area
                       (make-aj-visual-area
                        :buffer buffer
                        :window (selected-window)
                        :frame (selected-frame)))
                      (ace-jump-search-filter
                       (lambda ()
                         (setq observed
                               (cons (point) observed))
                         (= 0 (% (point) 4))))
                      (positions
                       (ace-jump-search-candidate
                        "x"
                        (list area))))
                 (list
                  (nreverse observed)
                  (mapcar
                   #'aj-position-offset
                   positions))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK ((2 4 6) (3))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_candidate_search_suppresses_filter_errors() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-filter-error*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "x x x"))
               (let* ((area
                       (make-aj-visual-area
                        :buffer buffer
                        :window (selected-window)
                        :frame (selected-frame)))
                      (ace-jump-search-filter
                       (lambda ()
                         (error "filter exploded"))))
                 (ace-jump-search-candidate
                  "x"
                  (list area))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK nil"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_candidate_search_invisible_policy_matches() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-invisible*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "x x x")
                 (put-text-property
                  3 4 'invisible t))
               (let ((area
                      (make-aj-visual-area
                       :buffer buffer
                       :window (selected-window)
                       :frame (selected-frame))))
                 (mapcar
                  (lambda (allow)
                    (let ((ace-jump-allow-invisible allow))
                      (mapcar
                       #'aj-position-offset
                       (ace-jump-search-candidate
                        "x"
                        (list area)))))
                  '(nil t))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK ((1) (1 3))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_candidate_search_line_anchor_advances_across_lines() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-lines*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "one\n\nthree\nfour"))
               (let ((area
                      (make-aj-visual-area
                       :buffer buffer
                       :window (selected-window)
                       :frame (selected-frame))))
                 (mapcar
                  #'aj-position-offset
                  (ace-jump-search-candidate
                   "^"
                   (list area)))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK (1 5 6 12)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_candidate_search_terminal_match_boundary_matches() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-terminal*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "x--x"))
               (let ((area
                      (make-aj-visual-area
                       :buffer buffer
                       :window (selected-window)
                       :frame (selected-frame))))
                 (mapcar
                  #'aj-position-offset
                  (ace-jump-search-candidate
                   "x"
                   (list area)))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK (1)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_candidate_search_stops_at_each_visible_window_end_and_appends_areas() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-visible-end*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "x-x-x-x"))
               (let* ((first
                       (make-aj-visual-area
                        :buffer buffer
                        :window (selected-window)
                        :frame (selected-frame)
                        :recover-buffer 'first))
                      (second
                       (make-aj-visual-area
                        :buffer buffer
                        :window (selected-window)
                        :frame (selected-frame)
                        :recover-buffer 'second)))
                 (cl-letf (((symbol-function 'window-start)
                            (lambda (_window) 1))
                           ((symbol-function 'window-end)
                            (lambda (_window &optional _update)
                              4)))
                   (mapcar
                    (lambda (position)
                      (list
                       (aj-position-recover-buffer
                        position)
                       (aj-position-offset position)))
                    (ace-jump-search-candidate
                     "x"
                     (list first second))))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK ((first 1) (first 3) (second 1) (second 3))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_candidate_search_empty_visual_area_list_is_empty() {
    let elisp_form = r##"(ace-jump-search-candidate "x" nil)"##;
    let expect = expect!["OK nil"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
