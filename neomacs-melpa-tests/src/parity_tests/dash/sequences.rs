use expect_test::expect;

use super::{assert_dash_parity, assert_dash_signal_parity};

#[test]
fn dash_slice_handles_positive_negative_open_and_reverse_ranges() {
    let elisp_form = r##"(list
              (-slice '(a b c d e f) 1 5 2)
              (-slice '(a b c d e f) -4 nil 1)
              (-slice '(a b c d e f) 0 nil 1)
              (-slice '(a b c d e f) 5 nil -2)
              (-slice nil 0 nil 1)
              (-slice '(å ß 中) 1 nil 1))"##;
    let expect = expect!["OK ((b d) (c d e f) (a b c d e f) (f) nil (ß 中))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_take_and_drop_while_stop_at_the_first_false_item() {
    let elisp_form = r##"(list
              (-take-while #'numberp '(1 2 a 3))
              (--take-while (< it 3) '(1 2 3 1))
              (-drop-while #'numberp '(1 2 a 3))
              (--drop-while (< it 3) '(1 2 3 1))
              (--take-while t nil)
              (--drop-while t nil)
              (--take-while nil '(a b))
              (--drop-while nil '(a b)))"##;
    let expect = expect!["OK ((1 2) (1 2) (a 3) (3 1) nil nil nil (a b))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_take_and_drop_clamp_counts_to_sequence_bounds() {
    let elisp_form = r##"(list
              (-take 3 '(a b c d e))
              (-take-last 3 '(a b c d e))
              (-drop 2 '(a b c d e))
              (-drop-last 2 '(a b c d e))
              (-take 20 '(a b))
              (-take-last 20 '(a b))
              (-drop 20 '(a b))
              (-drop-last 20 '(a b))
              (-take 0 '(a b))
              (-drop 0 '(a b))
              (-take 3 nil)
              (-drop 3 nil))"##;
    let expect =
        expect!["OK ((a b c) (c d e) (c d e) (a b c) (a b) (a b) nil nil nil (a b) nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_split_at_and_rotate_handle_boundaries_and_direction() {
    let elisp_form = r##"(list
              (-split-at 2 '(a b c d))
              (-split-at 0 '(a b))
              (-split-at 20 '(a b))
              (-rotate 2 '(a b c d e))
              (-rotate -1 '(a b c d e))
              (-rotate 0 '(a b c))
              (-rotate 10 '(a b c))
              (-rotate 2 nil))"##;
    let expect = expect![
        "OK (((a b) (c d)) (nil (a b)) ((a b) nil) (d e a b c) (b c d e a) (a b c) (c a b) nil)"
    ];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_insert_replace_and_update_at_target_one_position() {
    let elisp_form = r##"(list
              (-insert-at 2 'x '(a b c d))
              (-insert-at 0 'x '(a b))
              (-insert-at 2 'x '(a b))
              (-replace-at 2 'x '(a b c d))
              (-update-at
               2
               (lambda (item)
                 (intern (upcase (symbol-name item))))
               '(a b c d))
              (--update-at 2 (list it 'seen) '(a b c d))
              (--update-at 0 it nil))"##;
    let expect =
        expect!["OK ((a b x c d) (x a b) (a b x) (a b x d) (a b C d) (a b (c seen) d) (nil))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_remove_at_and_remove_at_indices_preserve_remaining_order() {
    let elisp_form = r##"(list
              (-remove-at 2 '(a b c d))
              (-remove-at 0 '(a b c d))
              (-remove-at 20 '(a b c d))
              (-remove-at-indices '(1 3) '(a b c d e))
              (-remove-at-indices '(3 1 1) '(a b c d e))
              (-remove-at-indices nil '(a b))
              (-remove-at-indices '(0) nil))"##;
    let expect = expect!["OK ((a b d) (b c d) (a b c d) (a c e) (a c e) (a b) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_replace_variants_change_all_first_or_last_equal_items() {
    let elisp_form = r##"(list
              (-replace 'x 'z '(a x b x))
              (-replace-first 'x 'z '(a x b x))
              (-replace-last 'x 'z '(a x b x))
              (-replace 'missing 'z '(a b))
              (-replace-first 'x 'z nil)
              (-replace-last 'x 'z nil)
              (let ((-compare-fn #'string-equal))
                (-replace "A" "x" '("A" "B" "A"))))"##;
    let expect = expect![[r#"OK ((a z b z) (a z b x) (a x b z) (a b) nil nil ("x" "B" "x"))"#]];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_split_with_variants_partition_one_prefix() {
    let elisp_form = r##"(list
              (-split-with #'numberp '(1 2 a 3))
              (--split-with (< it 3) '(1 2 3 1))
              (--split-with t '(a b))
              (--split-with nil '(a b))
              (--split-with t nil))"##;
    let expect = expect!["OK (((1 2) (a 3)) ((1 2) (3 1)) ((a b) nil) (nil (a b)) (nil nil))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_split_on_and_split_when_handle_repeated_and_boundary_separators() {
    let elisp_form = r##"(list
              (-split-on 'x '(a x b c x d))
              (-split-on 'x '(x a x))
              (-split-on 'x '(a b))
              (-split-when #'numberp '(a b 1 c 2 d))
              (--split-when (numberp it) '(a b 1 c 2 d))
              (--split-when t '(a b))
              (--split-when t nil))"##;
    let expect =
        expect!["OK (((a) (b c) (d)) ((a)) ((a b)) ((a b) (c) (d)) ((a b) (c) (d)) nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_separate_variants_return_false_items_then_true_items() {
    let elisp_form = r##"(list
              (-separate #'numberp '(a 1 b 2))
              (--separate (symbolp it) '(a 1 b 2))
              (--separate t '(a b))
              (--separate nil '(a b))
              (--separate t nil))"##;
    let expect = expect!["OK (((1 2) (a b)) ((a b) (1 2)) ((a b) nil) (nil (a b)) (nil nil))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_fixed_partition_variants_control_partial_and_overlapping_groups() {
    let elisp_form = r##"(list
              (-partition 2 '(1 2 3 4 5))
              (-partition-all 2 '(1 2 3 4 5))
              (-partition-in-steps 2 1 '(1 2 3 4))
              (-partition-all-in-steps 3 2 '(1 2 3 4 5))
              (-partition 2 nil)
              (-partition-all 2 '(1))
              (-partition-in-steps 3 4 '(1 2 3 4 5 6))
              (-partition-all-in-steps 3 4 '(1 2 3 4 5 6)))"##;
    let expect = expect![
        "OK (((1 2) (3 4)) ((1 2) (3 4) (5)) ((1 2) (2 3) (3 4)) ((1 2 3) (3 4 5) (5)) nil ((1)) ((1 2 3)) ((1 2 3) (5 6)))"
    ];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_partition_by_variants_group_adjacent_equal_keys() {
    let elisp_form = r##"(list
              (-partition-by #'cl-evenp '(1 3 2 4 5))
              (--partition-by (cl-evenp it) '(1 3 2 4 5))
              (--partition-by it '(a a b b a))
              (--partition-by it nil))"##;
    let expect = expect!["OK (((1 3) (2 4) (5)) ((1 3) (2 4) (5)) ((a a) (b b) (a)) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_partition_by_header_starts_groups_at_header_items() {
    let elisp_form = r##"(list
              (-partition-by-header #'numberp '(a 1 2 b 3))
              (--partition-by-header (numberp it) '(a 1 2 b 3))
              (--partition-by-header (eq it 'h) '(h a h b c))
              (--partition-by-header t '(a b))
              (--partition-by-header t nil))"##;
    let expect = expect!["OK (((a 1 2) (b 3)) ((a 1 2) (b 3)) ((h a) (h b c)) ((a b)) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_partition_around_predicates_and_items_places_boundaries_exactly() {
    let elisp_form = r##"(list
              (-partition-after-pred #'cl-evenp '(1 2 3 4 5))
              (--partition-after-pred (cl-evenp it) '(1 2 3 4 5))
              (-partition-before-pred #'cl-evenp '(1 2 3 4 5))
              (-partition-after-item 'x '(a x b c x d))
              (-partition-before-item 'x '(a x b c x d))
              (-partition-after-item 'x '(a b))
              (-partition-before-item 'x '(a b))
              (-partition-after-item 'x nil))"##;
    let expect = expect![
        "OK (((1 2) (3 4) (5)) ((1 2) (3 4) (5)) ((1) (2 3) (4 5)) ((a x) (b c x) (d)) ((a) (x b c) (x d)) ((a b)) ((a b)) nil)"
    ];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_group_by_variants_collect_equal_keys_without_reordering_values() {
    let elisp_form = r##"(list
              (-group-by #'cl-evenp '(1 2 3 4))
              (--group-by
               (if (numberp it) 'number 'other)
               '(a 1 b 2))
              (--group-by it '(a b a c))
              (--group-by it nil))"##;
    let expect =
        expect!["OK (((nil 1 3) (t 2 4)) ((other a b) (number 1 2)) ((a a a) (b b) (c c)) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_interpose_and_interleave_preserve_source_order() {
    let elisp_form = r##"(list
              (-interpose 'x '(a b c))
              (-interpose 'x '(a))
              (-interpose 'x nil)
              (-interleave '(1 2 3) '(a b) '(x y z))
              (-interleave '(1 2) '(a b))
              (-interleave '(1 2) nil)
              (-interleave))"##;
    let expect = expect!["OK ((a x b x c) (a) nil (1 a x 2 b y) (1 a 2 b) nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_zip_with_and_zip_lists_stop_at_the_shortest_input() {
    let elisp_form = r##"(list
              (-zip-with #'cons '(1 2 3) '(a b))
              (--zip-with (list it other) '(1 2 3) '(a b))
              (-zip-lists '(1 2 3) '(a b))
              (-zip-lists '(1 2) '(a b) '(x y))
              (-zip-lists nil '(a b))
              (-zip-with #'list nil '(a b)))"##;
    let expect =
        expect!["OK (((1 . a) (2 . b)) ((1 a) (2 b)) ((1 a) (2 b)) ((1 a x) (2 b y)) nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_zip_fill_and_pad_extend_shorter_inputs_with_the_fill_value() {
    let elisp_form = r##"(list
              (-zip-lists-fill 'missing '(1 2 3) '(a b))
              (-zip-fill 'missing '(1 2 3) '(a b))
              (-pad 'missing '(1 2 3) '(a b))
              (-zip-lists-fill 'missing nil '(a b))
              (-zip-fill 'missing nil '(a b))
              (-pad 'missing nil '(a b)))"##;
    let expect = expect![
        "OK (((1 a) (2 b) (3 missing)) ((1 . a) (2 . b) (3 . missing)) ((1 2 3) (a b missing)) ((missing a) (missing b)) ((missing . a) (missing . b)) ((missing missing) (a b)))"
    ];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_legacy_zip_and_unzip_variants_preserve_pair_shapes() {
    let elisp_form = r##"(list
              (-unzip-lists '((1 a) (2 b)))
              (-zip '(1 2 3) '(a b))
              (-zip-pair '(1 2 3) '(a b))
              (-unzip '((1 a) (2 b)))
              (-zip '(1 2) '(a b) '(x y))
              (-unzip-lists nil)
              (-unzip nil))"##;
    let expect = expect![
        "OK (((1 2) (a b)) ((1 . a) (2 . b)) ((1 . a) (2 . b)) ((1 . 2) (a . b)) ((1 a x) (2 b y)) nil nil)"
    ];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_cycle_produces_a_circular_list_and_preserves_empty_input() {
    let elisp_form = r##"(list
              (-cycle '(a b c))
              (-cycle '(x))
              (-cycle nil))"##;
    let expect = expect!["OK (#1=(a b c . #1#) #2=(x . #2#) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_annotate_table_and_table_flat_pair_computed_values_with_inputs() {
    let elisp_form = r##"(list
              (-annotate #'length '("a" "bbb" "cc"))
              (--annotate (* it it) '(1 2 3))
              (-annotate #'identity nil)
              (-table #'* '(1 2) '(10 20 30))
              (-table-flat
               (lambda (left right) (list left right))
               '(1 2) '(a b))
              (-table-flat #'list '(1 2) nil))"##;
    let expect = expect![[
        r#"OK (((1 . "a") (3 . "bbb") (2 . "cc")) ((1 . 1) (4 . 2) (9 . 3)) nil ((10 20) (20 40) (30 60)) ((1 a) (2 a) (1 b) (2 b)) nil)"#
    ]];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_table_rejects_an_empty_input_axis() {
    let elisp_form = r##"(-table #'* nil '(1 2))"##;
    let expect = expect!["ERR (wrong-type-argument number-or-marker-p nil)"];

    assert_dash_signal_parity(elisp_form, expect);
}

#[test]
fn dash_grade_up_and_down_return_stable_source_indices() {
    let elisp_form = r##"(list
              (-grade-up #'< '(30 10 20))
              (-grade-down #'< '(30 10 20))
              (-grade-up #'< '(2 1 1))
              (-grade-down #'< '(2 1 1))
              (-grade-up #'< nil)
              (-grade-down #'< '(1)))"##;
    let expect = expect!["OK ((1 2 0) (0 2 1) (1 2 0) (0 1 2) nil (0))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_find_index_variants_cover_first_last_and_all_matches() {
    let elisp_form = r##"(list
              (-find-index #'cl-evenp '(1 3 4 6))
              (--find-index (> it 3) '(1 3 4 6))
              (-elem-index 'x '(a x b x))
              (-find-indices #'cl-evenp '(1 2 3 4))
              (--find-indices (> it 2) '(1 2 3 4))
              (-elem-indices 'x '(a x b x))
              (-find-last-index #'cl-evenp '(1 2 3 4 5))
              (--find-last-index (< it 4) '(1 2 3 4 5))
              (--find-index t nil)
              (-elem-indices 'x nil))"##;
    let expect = expect!["OK (2 2 1 (1 3) (2 3) (1 3) 3 2 nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_select_by_indices_columns_and_column_preserve_requested_order() {
    let elisp_form = r##"(list
              (-select-by-indices '(0 2 4) '(a b c d e))
              (-select-by-indices '(4 0 0) '(a b c d e))
              (-select-by-indices nil '(a b))
              (-select-columns '(0 2) '((a b c) (d e f)))
              (-select-columns '(2 0) '((a b c) (d e f)))
              (-select-column 1 '((a b c) (d e f)))
              (-select-column 1 nil))"##;
    let expect = expect!["OK ((a c e) (e a a) nil ((a c) (d f)) ((c a) (f d)) (b e) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_slice_rejects_a_zero_step() {
    let elisp_form = r##"(-slice '(a b c) 0 nil 0)"##;
    let expect = expect!["ERR (arith-error)"];

    assert_dash_signal_parity(elisp_form, expect);
}
