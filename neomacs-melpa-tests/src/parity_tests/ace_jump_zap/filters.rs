use super::{assert_ace_jump_zap_parity, assert_ace_jump_zap_signal_parity};
use expect_test::expect;

#[test]
fn ace_jump_zap_forward_query_covers_before_equal_and_after_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdef")
         (mapcar
          (lambda (spec)
            (let ((ajz/saved-point
                   (car spec)))
              (goto-char
               (cdr spec))
              (list
               spec
               (ajz/forward-query))))
          '((4 . 2)
            (4 . 4)
            (4 . 6))))"##;
    let expect = expect!["OK (((4 . 2) nil) ((4 . 4) nil) ((4 . 6) t))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_forward_query_without_saved_point_signals_exact_type_error() {
    let elisp_form = r##"(let ((ajz/saved-point nil))
         (ajz/forward-query))"##;
    let expect = expect!["ERR (wrong-type-argument number-or-marker-p nil)"];
    assert_ace_jump_zap_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_closeness_comparator_covers_left_right_and_ties() {
    let elisp_form = r##"(let ((ajz/saved-point 10))
         (mapcar
          (lambda (pair)
            (let ((left
                   (make-aj-position
                    :offset (car pair)))
                  (right
                   (make-aj-position
                    :offset (cdr pair))))
              (list
               pair
               (ajz/closeness-to-point
                left right)
               (ajz/closeness-to-point
                right left))))
          '((9 . 7)
            (13 . 9)
            (7 . 13)
            (10 . 10)
            (8 . 12))))"##;
    let expect = expect![
        "OK (((9 . 7) t nil) ((13 . 9) nil t) ((7 . 13) nil nil) ((10 . 10) nil nil) ((8 . 12) nil nil))"
    ];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_candidate_length_filter_covers_all_gate_combinations() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let* ((ajz/zapping
                   (nth 0 spec))
                  (ajz/52-character-limit
                   (nth 1 spec))
                  (args
                   (list
                    (nth 2 spec)
                    (nth 3 spec)))
                  (result
                   (ajz/maybe-limit-candidate-length
                    args)))
             (list
              spec
              result
              (eq result args))))
         '((nil t 100 8)
           (t nil 100 8)
           (t t 52 8)
           (t t 53 8)
           (t t 100 2)))"##;
    let expect = expect![
        "OK (((nil t 100 8) (100 8) t) ((t nil 100 8) (100 8) t) ((t t 52 8) (52 8) t) ((t t 53 8) (52 8) nil) ((t t 100 2) (52 2) nil))"
    ];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_candidate_length_filter_preserves_extra_args_when_inactive() {
    let elisp_form = r##"(let* ((ajz/zapping nil)
              (ajz/52-character-limit t)
              (args '(100 4 extra tail))
              (result
               (ajz/maybe-limit-candidate-length
                args)))
         (list
          result
          (eq result args)))"##;
    let expect = expect!["OK ((100 4 extra tail) t)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_sort_filter_is_identity_when_not_zapping() {
    let elisp_form = r##"(let* ((ajz/zapping nil)
              (ajz/saved-point 10)
              (args
               (list
                'tree
                (mapcar
                 (lambda (offset)
                   (make-aj-position
                    :offset offset))
                 '(20 9 4 11))))
              (result
               (ajz/maybe-sort-candidate-list
                args)))
         (list
          (eq result args)
          (mapcar
           #'aj-position-offset
           (nth 1 result))))"##;
    let expect = expect!["OK (t (20 9 4 11))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_sort_filter_orders_candidates_by_proximity() {
    let elisp_form = r##"(let* ((ajz/zapping t)
              (ajz/saved-point 10)
              (ajz/sort-by-closest t)
              (ajz/52-character-limit nil)
              (args
               (list
                'tree
                (mapcar
                 (lambda (offset)
                   (make-aj-position
                    :offset offset))
                 '(20 9 4 11 10 13 8))))
              (result
               (ajz/maybe-sort-candidate-list
                args)))
         (list
          (car result)
          (mapcar
           #'aj-position-offset
           (nth 1 result))
          (eq result args)))"##;
    let expect = expect!["OK (tree (10 9 11 8 13 4 20) nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_sort_filter_preserves_order_when_sorting_disabled() {
    let elisp_form = r##"(let* ((ajz/zapping t)
              (ajz/saved-point 10)
              (ajz/sort-by-closest nil)
              (ajz/52-character-limit nil)
              (candidates
               (mapcar
                (lambda (offset)
                  (make-aj-position
                   :offset offset))
                '(20 9 4 11)))
              (args
               (list 'tree candidates))
              (result
               (ajz/maybe-sort-candidate-list
                args)))
         (list
          (mapcar
           #'aj-position-offset
           (nth 1 result))
          (eq
           (nth 1 result)
           candidates)
          (eq result args)))"##;
    let expect = expect!["OK ((20 9 4 11) t nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_sort_filter_limits_after_sorting() {
    let elisp_form = r##"(let* ((ajz/zapping t)
              (ajz/saved-point 100)
              (ajz/sort-by-closest t)
              (ajz/52-character-limit t)
              (candidates
               (mapcar
                (lambda (offset)
                  (make-aj-position
                   :offset offset))
                (number-sequence 1 80)))
              (result
               (ajz/maybe-sort-candidate-list
                (list 'tree candidates))))
         (list
          (length
           (nth 1 result))
          (mapcar
           #'aj-position-offset
           (seq-take
            (nth 1 result)
            5))
          (mapcar
           #'aj-position-offset
           (last
            (nth 1 result)
            5))))"##;
    let expect = expect!["OK (52 (80 79 78 77 76) (33 32 31 30 29))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_sort_filter_limits_without_sorting() {
    let elisp_form = r##"(let* ((ajz/zapping t)
              (ajz/sort-by-closest nil)
              (ajz/52-character-limit t)
              (candidates
               (mapcar
                (lambda (offset)
                  (make-aj-position
                   :offset offset))
                (number-sequence 80 1 -1)))
              (result
               (ajz/maybe-sort-candidate-list
                (list 'tree candidates))))
         (list
          (length
           (nth 1 result))
          (aj-position-offset
           (car
            (nth 1 result)))
          (aj-position-offset
           (car
            (last
             (nth 1 result))))))"##;
    let expect = expect!["OK (52 80 29)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_sort_filter_handles_empty_candidates_and_discards_extra_args_while_active() {
    let elisp_form = r##"(let* ((ajz/zapping t)
              (ajz/sort-by-closest t)
              (ajz/52-character-limit t)
              (args '(tree nil extra tail))
              (result
               (ajz/maybe-sort-candidate-list
                args)))
         (list
          result
          (eq result args)))"##;
    let expect = expect!["OK ((tree nil) nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_tree_advice_limits_actual_leaf_count_only_while_zapping() {
    let elisp_form = r##"(mapcar
         (lambda (zapping)
           (let* ((ajz/zapping zapping)
                  (ajz/52-character-limit t)
                  (tree
                   (ace-jump-tree-breadth-first-construct
                    80 4))
                  (leaves 0))
             (ace-jump-tree-preorder-traverse
              tree
              (lambda (_node)
                (setq leaves
                      (1+ leaves))))
             (list zapping leaves)))
         '(nil t))"##;
    let expect = expect!["OK ((nil 80) (t 52))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_populate_advice_sorts_actual_overlay_assignment() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-zap-sort*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "abcdefghijklmnop"))
               (let* ((area
                       (make-aj-visual-area
                        :buffer buffer
                        :window (selected-window)
                        :frame (selected-frame)))
                      (tree
                       (ace-jump-tree-breadth-first-construct
                        4 4))
                      (positions
                       (mapcar
                        (lambda (offset)
                          (make-aj-position
                           :offset offset
                           :visual-area area))
                        '(2 12 7 5)))
                      (ajz/zapping t)
                      (ajz/saved-point 6)
                      (ajz/sort-by-closest t)
                      (ajz/52-character-limit nil))
                 (ace-jump-populate-overlay-to-search-tree
                  tree positions)
                 (mapcar
                  (lambda (node)
                    (list
                     (overlay-start
                      (cdr node))
                     (aj-position-offset
                      (overlay-get
                       (cdr node)
                       'aj-data))))
                  (cdr tree))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK ((7 7) (5 5) (2 2) (12 12))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}
