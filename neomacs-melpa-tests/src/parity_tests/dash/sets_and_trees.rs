use expect_test::expect;

use super::{assert_dash_parity, assert_dash_signal_parity};

#[test]
fn dash_distinct_and_uniq_preserve_first_occurrence_order() {
    let elisp_form = r##"(list
              (-distinct '(a b a c b))
              (-uniq '(a b a c b))
              (-distinct nil)
              (-distinct '(a))
              (let ((-compare-fn #'string-equal))
                (-distinct '("A" "A" "B"))))"##;
    let expect = expect![[r#"OK ((a b c) (a b c) nil (a) ("A" "B"))"#]];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_union_intersection_and_difference_honor_custom_comparison() {
    let elisp_form = r##"(list
              (-union '(a b c) '(b c d))
              (-intersection '(a b c) '(b c d))
              (-difference '(a b c) '(b d))
              (-union nil '(a b))
              (-intersection nil '(a b))
              (-difference '(a b) nil)
              (let ((-compare-fn #'string-equal))
                (list
                 (-union '("A") '("A" "B"))
                 (-intersection '("A" "B") '("B"))
                 (-difference '("A" "B") '("A")))))"##;
    let expect = expect![[r#"OK ((a b c d) (b c) (a c) (a b) nil (a b) (("A" "B") ("B") ("B")))"#]];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_frequencies_count_equal_values_in_first_seen_order() {
    let elisp_form = r##"(list
              (-frequencies '(a b a c b a))
              (-frequencies nil)
              (-frequencies '(a))
              (let ((-compare-fn #'string-equal))
                (-frequencies '("A" "B" "A"))))"##;
    let expect = expect![[r#"OK (((a . 3) (b . 2) (c . 1)) nil ((a . 1)) (("A" . 2) ("B" . 1)))"#]];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_powerset_inits_and_tails_include_empty_boundaries() {
    let elisp_form = r##"(list
              (-powerset '(a b c))
              (-powerset nil)
              (-inits '(a b c))
              (-inits nil)
              (-tails '(a b c))
              (-tails nil))"##;
    let expect = expect![
        "OK (((a . #2=(b . #1=(c))) (a . #3=(b)) (a . #1#) (a) #2# #3# #1# nil) (nil) (nil (a) (a b) (a b c)) (nil) ((a . #4=(b . #5=(c))) #4# #5# nil) (nil))"
    ];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_permutations_cover_unique_and_duplicate_items() {
    let elisp_form = r##"(list
              (-permutations '(a b c))
              (-permutations '(a a b))
              (-permutations '(a))
              (-permutations nil))"##;
    let expect = expect![
        "OK (((a b c) (a c b) (b a c) (b c a) (c a b) (c b a)) ((a a b) (a b a) (b a a)) ((a)) (nil))"
    ];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_common_prefix_and_suffix_compare_multiple_lists() {
    let elisp_form = r##"(list
              (-common-prefix '(a b c) '(a b d) '(a b))
              (-common-prefix '(a b) '(x b))
              (-common-prefix '(a b))
              (-common-prefix)
              (-common-suffix '(a b c) '(x b c) '(b c))
              (-common-suffix '(a b) '(a x))
              (-common-suffix '(a b))
              (-common-suffix))"##;
    let expect = expect!["OK ((a b) nil (a b) nil (b c) nil (a b) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_contains_and_same_items_predicates_honor_multiplicity_and_aliases() {
    let elisp_form = r##"(list
              (-contains? '(a b c) 'b)
              (-contains-p '(a b c) 'z)
              (-same-items? '(a b a) '(b a a))
              (-same-items? '(a b) '(a a b))
              (-same-items-p '(a b) '(b a))
              (-contains? nil 'a)
              (-same-items? nil nil)
              (-same-items? nil '(a)))"##;
    let expect = expect!["OK ((b c) nil t t t nil t nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_prefix_suffix_and_infix_predicates_cover_aliases_and_empty_patterns() {
    let elisp_form = r##"(list
              (-is-prefix? '(a b) '(a b c))
              (-is-prefix-p '(a b) '(a b c))
              (-is-prefix? nil '(a b))
              (-is-prefix? '(a c) '(a b c))
              (-is-suffix? '(b c) '(a b c))
              (-is-suffix-p '(b c) '(a b c))
              (-is-suffix? nil '(a b))
              (-is-suffix? '(a b) '(a b c))
              (-is-infix? '(b c) '(a b c d))
              (-is-infix-p '(b c) '(a b c d))
              (-is-infix? nil '(a b))
              (-is-infix? '(c a) '(a b c d)))"##;
    let expect = expect!["OK (t t t nil t t t nil t t t nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_sort_list_and_repeat_cover_order_empty_and_count_boundaries() {
    let elisp_form = r##"(list
              (-sort #'< '(3 1 2))
              (--sort (< it other) '(3 1 2))
              (-sort #'< nil)
              (-list nil)
              (-list 1 2 3)
              (-repeat 3 'x)
              (-repeat 0 'x)
              (-repeat 2 '(a b)))"##;
    let expect = expect!["OK ((1 2 3) (1 2 3) nil nil (1 2 3) (x x x) nil (#1=(a b) #1#))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_sum_product_and_running_aggregates_define_empty_identities() {
    let elisp_form = r##"(list
              (-sum '(1 2 3 4))
              (-running-sum '(1 2 3 4))
              (-product '(1 2 3 4))
              (-running-product '(1 2 3 4))
              (-sum nil)
              (-product nil)
              (-sum '(-2 5)))"##;
    let expect = expect!["OK (10 (1 3 6 10) 24 (1 2 6 24) 0 1 3)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_running_sum_rejects_an_empty_list() {
    let elisp_form = r##"(-running-sum nil)"##;
    let expect = expect!["ERR (wrong-type-argument consp nil)"];

    assert_dash_signal_parity(elisp_form, expect);
}

#[test]
fn dash_running_product_rejects_an_empty_list() {
    let elisp_form = r##"(-running-product nil)"##;
    let expect = expect!["ERR (wrong-type-argument consp nil)"];

    assert_dash_signal_parity(elisp_form, expect);
}

#[test]
fn dash_min_max_and_comparator_variants_select_extreme_items() {
    let elisp_form = r##"(list
              (-max '(3 1 4 2))
              (-min '(3 1 4 2))
              (-max-by
               (lambda (left right) (< (length left) (length right)))
               '("a" "bbbb" "cc"))
              (-min-by
               (lambda (left right) (< (length left) (length right)))
               '("a" "bbbb" "cc"))
              (--max-by
               (< (length it) (length other))
               '("a" "bbbb" "cc"))
              (--min-by
               (< (length it) (length other))
               '("a" "bbbb" "cc"))
              (-max '(7))
              (-min '(7)))"##;
    let expect = expect![[r#"OK (4 1 "a" "bbbb" "a" "bbbb" 7 7)"#]];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_iota_generates_requested_arithmetic_progressions() {
    let elisp_form = r##"(list
              (-iota 5)
              (-iota 4 10 3)
              (-iota 0)
              (-iota 4 3 -1)
              (-iota 3 0 0))"##;
    let expect = expect!["OK ((0 1 2 3 4) (10 13 16 19) nil (3 2 1 0) (0 0 0))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_cons_pair_and_value_conversion_distinguish_pairs_from_lists() {
    let elisp_form = r##"(list
              (-cons-pair? '(a . b))
              (-cons-pair-p '(a b))
              (-cons-pair? nil)
              (-cons-to-list '(a . b))
              (-value-to-list '(a . b))
              (-value-to-list '(a b))
              (-value-to-list 'a)
              (-value-to-list nil))"##;
    let expect = expect!["OK (t nil nil (a b) (a b) ((a b)) (a) (nil))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_tree_map_variants_transform_only_leaf_values() {
    let elisp_form = r##"(list
              (-tree-map #'1+ '(1 (2 3) 4))
              (--tree-map (* it 2) '(1 (2 3) 4))
              (-tree-map #'identity nil)
              (-tree-map #'1+ 1)
              (let ((source '(1 (2 3))))
                (list (--tree-map (* it 2) source) source)))"##;
    let expect = expect!["OK ((2 (3 4) 5) (2 (4 6) 8) nil 2 ((2 (4 6)) (1 (2 3))))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_tree_mapreduce_variants_fold_transformed_leaves() {
    let elisp_form = r##"(list
              (-tree-mapreduce #'1+ #'+ '(1 (2 3) 4))
              (--tree-mapreduce (1+ it) (+ it acc) '(1 (2 3) 4))
              (-tree-mapreduce-from #'1+ #'+ 0 '(1 (2 3) 4))
              (--tree-mapreduce-from
               (1+ it) (+ it acc) 0 '(1 (2 3) 4))
              (-tree-mapreduce-from #'1+ #'+ 10 nil)
              (-tree-mapreduce #'1+ #'+ 1))"##;
    let expect = expect!["OK (14 14 14 14 nil 2)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_tree_reduce_variants_fold_existing_leaf_values() {
    let elisp_form = r##"(list
              (-tree-reduce #'+ '(1 (2 3) 4))
              (--tree-reduce (+ it acc) '(1 (2 3) 4))
              (-tree-reduce-from #'+ 0 '(1 (2 3) 4))
              (--tree-reduce-from (+ it acc) 0 '(1 (2 3) 4))
              (-tree-reduce-from #'+ 10 nil)
              (-tree-reduce #'+ 7))"##;
    let expect = expect!["OK (10 10 10 10 nil 7)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_tree_map_nodes_changes_matching_subtrees_without_descending_into_them() {
    let elisp_form = r##"(list
              (-tree-map-nodes #'numberp #'1+ '(1 (2 3) 4))
              (--tree-map-nodes (numberp it) (1+ it) '(1 (2 3) 4))
              (--tree-map-nodes (listp it) 'branch '(1 (2 3) 4))
              (--tree-map-nodes t it nil))"##;
    let expect = expect!["OK ((2 (3 4) 5) (2 (3 4) 5) branch nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_tree_seq_and_clone_preserve_tree_order_and_deep_independence() {
    let elisp_form = r##"(list
              (-tree-seq #'listp #'identity '(1 (2 3)))
              (--tree-seq (listp it) it '(1 (2 3)))
              (-tree-seq #'listp #'identity nil)
              (let* ((source '((a) (b c)))
                     (clone (-clone source)))
                (setcar (car clone) 'changed)
                (list source clone)))"##;
    let expect = expect![
        "OK (((1 #1=(2 3)) 1 #1# 2 3) ((1 #2=(2 3)) 1 #2# 2 3) (nil) (((a) (b c)) ((changed) (b c))))"
    ];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_fix_and_unfold_generate_values_until_stable_or_finished() {
    let elisp_form = r##"(list
              (-fix (lambda (items) (-distinct items)) '(a b a b))
              (--fix (-distinct it) '(a b a b))
              (-fix #'identity nil)
              (-unfold
               (lambda (seed)
                 (and (< seed 5) (cons seed (1+ seed))))
               1)
              (--unfold (and (< it 5) (cons it (1+ it))) 1)
              (--unfold nil 1))"##;
    let expect = expect!["OK ((a b) (a b) nil (1 2 3 4) (1 2 3 4) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_partial_rpartial_juxt_and_compose_build_reusable_functions() {
    let elisp_form = r##"(list
              (funcall (-partial #'+ 10) 1 2)
              (funcall (-rpartial #'- 3) 10)
              (funcall (-juxt #'1+ #'1- (lambda (x) (* x x))) 5)
              (funcall (-juxt) 5)
              (funcall (-compose #'1+ (lambda (x) (* x 2))) 5)
              (funcall (-compose) 'value))"##;
    let expect = expect!["OK (13 7 (6 4 25) nil 11 value)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_applify_on_flip_and_rotate_args_adapt_calling_conventions() {
    let elisp_form = r##"(list
              (funcall (-applify #'+) '(1 2 3))
              (funcall (-on #'+ #'1+) 1 2 3)
              (funcall (-flip #'-) 3 10)
              (funcall (-rotate-args 1 #'list) 'a 'b 'c)
              (funcall (-rotate-args -1 #'list) 'a 'b 'c)
              (funcall (-rotate-args 0 #'list) 'a 'b 'c))"##;
    let expect = expect!["OK (6 9 7 (c a b) (b c a) (a b c))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_const_cut_not_orfn_and_andfn_compose_predicates_and_arguments() {
    let elisp_form = r##"(list
              (funcall (-const 'fixed) 1 2 3)
              (funcall (-cut list 1 <> 3 <>) 2 4)
              (funcall (-not #'numberp) 'a)
              (funcall (-not #'numberp) 1)
              (funcall (-orfn #'numberp #'symbolp) 'a)
              (funcall (-orfn #'numberp #'symbolp) "x")
              (funcall (-andfn #'numberp #'cl-evenp) 4)
              (funcall (-andfn #'numberp #'cl-evenp) 3)
              (funcall (-orfn) 'a)
              (funcall (-andfn) 'a))"##;
    let expect = expect!["OK (fixed (1 2 3 4) t nil t nil t nil nil t)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_iteratefn_counter_fixfn_and_prodfn_keep_independent_state() {
    let elisp_form = r##"(list
              (funcall (-iteratefn #'1+ 4) 10)
              (funcall (-iteratefn #'1+ 0) 10)
              (let ((counter (-counter 2 8 2)))
                (list
                 (funcall counter)
                 (funcall counter)
                 (funcall counter)
                 (funcall counter)))
              (let ((left (-counter))
                    (right (-counter)))
                (list (funcall left) (funcall left) (funcall right)))
              (funcall
               (-fixfn (lambda (number) (/ (+ number 10) 2)))
               0)
              (funcall
               (-prodfn
                #'1+
                (lambda (item)
                  (intern (upcase (symbol-name item)))))
               '(1 a)))"##;
    let expect = expect!["OK (14 10 (2 4 6 nil) (0 1 0) 9 (2 A))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_iota_rejects_a_negative_count() {
    let elisp_form = r##"(-iota -1)"##;
    let expect = expect!["ERR (wrong-type-argument natnump -1)"];

    assert_dash_signal_parity(elisp_form, expect);
}

#[test]
fn dash_max_rejects_an_empty_list() {
    let elisp_form = r##"(-max nil)"##;
    let expect = expect!["ERR (wrong-number-of-arguments #<subr max> 0)"];

    assert_dash_signal_parity(elisp_form, expect);
}
