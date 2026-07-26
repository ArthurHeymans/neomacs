use expect_test::expect;

use super::assert_dash_parity;

#[test]
fn dash_destructive_cons_and_cdr_update_the_bound_list() {
    let elisp_form = r##"(list
              (let ((items '(b c)))
                (list (!cons 'a items) items))
              (let ((items '(a b c)))
                (list (!cdr items) items))
              (let ((items nil))
                (list (!cons 'a items) items)))"##;
    let expect = expect!["OK ((#1=(a b c) #1#) (#2=(b c) #2#) (#3=(a) #3#))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_each_function_and_macro_preserve_order_and_evaluate_the_list_once() {
    let elisp_form = r##"(list
              (let (out)
                (-each '(1 2 3) (lambda (item) (push item out)))
                (nreverse out))
              (let (out)
                (--each '(a b c) (push it out))
                (nreverse out))
              (let ((evaluations 0)
                    out)
                (--each
                    (progn (setq evaluations (1+ evaluations)) '(a b))
                  (push it out))
                (list (nreverse out) evaluations))
              (let (out)
                (--each nil (push it out))
                out))"##;
    let expect = expect!["OK ((1 2 3) (a b c) ((a b) 1) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_each_indexed_variants_expose_zero_based_indices() {
    let elisp_form = r##"(list
              (let (out)
                (-each-indexed
                 '(x y)
                 (lambda (index item) (push (cons index item) out)))
                (nreverse out))
              (let (out)
                (--each-indexed '(x y) (push (cons it-index it) out))
                (nreverse out))
              (let (out)
                (--each-indexed nil (push (cons it-index it) out))
                out))"##;
    let expect = expect!["OK (((0 . x) (1 . y)) ((0 . x) (1 . y)) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_each_while_stops_before_the_first_false_predicate() {
    let elisp_form = r##"(list
              (let (out)
                (-each-while
                 '(1 2 0 3) #'identity
                 (lambda (item) (push item out)))
                (nreverse out))
              (let (out)
                (--each-while '(1 2 0 3) (> it 0) (push it out))
                (nreverse out))
              (let (out)
                (--each-while '(0 1 2) (> it 0) (push it out))
                out)
              (let (out)
                (--each-while nil t (push it out))
                out))"##;
    let expect = expect!["OK ((1 2 0 3) (1 2) nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_reverse_each_variants_visit_from_right_to_left() {
    let elisp_form = r##"(list
              (let (out)
                (-each-r '(1 2 3) (lambda (item) (push item out)))
                (nreverse out))
              (let (out)
                (--each-r '(a b c) (push (cons it-index it) out))
                (nreverse out))
              (let (out)
                (-each-r-while
                 '(0 1 2 3) (lambda (item) (> item 0))
                 (lambda (item) (push item out)))
                (nreverse out))
              (let (out)
                (--each-r-while '(0 1 2 3) (> it 0) (push it out))
                (nreverse out)))"##;
    let expect = expect!["OK ((3 2 1) ((2 . c) (1 . b) (0 . a)) (3 2 1) (3 2 1))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_dotimes_variants_cover_zero_and_positive_counts() {
    let elisp_form = r##"(list
              (let (out)
                (-dotimes 4 (lambda (index) (push index out)))
                (nreverse out))
              (let (out)
                (--dotimes 4 (push (* it it) out))
                (nreverse out))
              (let (out)
                (--dotimes 0 (push it out))
                out))"##;
    let expect = expect!["OK ((0 1 2 3) (0 1 4 9) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_map_variants_transform_every_item_without_mutating_the_source() {
    let elisp_form = r##"(list
              (-map #'1+ '(1 2 3))
              (--map (* it it) '(1 2 3))
              (-map #'identity nil)
              (let ((source '(1 2 3)))
                (list (--map (* it 2) source) source))
              (let ((evaluations 0))
                (list
                 (--map
                     (progn (setq evaluations (1+ evaluations)) (* it 2))
                   '(1 2 3))
                 evaluations)))"##;
    let expect = expect!["OK ((2 3 4) (1 4 9) nil ((2 4 6) (1 2 3)) ((2 4 6) 3))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_map_indexed_variants_pair_items_with_zero_based_indices() {
    let elisp_form = r##"(list
              (-map-indexed
               (lambda (index item) (list index item))
               '(a b c))
              (--map-indexed (cons it-index it) '(a b c))
              (-map-indexed #'list nil)
              (--map-indexed (list it-index it) '(å 中)))"##;
    let expect = expect!["OK (((0 a) (1 b) (2 c)) ((0 . a) (1 . b) (2 . c)) nil ((0 å) (1 中)))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_map_when_and_replace_where_change_only_matching_items() {
    let elisp_form = r##"(list
              (-map-when #'cl-evenp #'1+ '(1 2 3 4))
              (--map-when (cl-evenp it) (* it 10) '(1 2 3 4))
              (-replace-where
               #'cl-oddp
               (lambda (_item) 'odd)
               '(1 2 3))
              (--replace-where (cl-evenp it) 'even '(1 2 3))
              (--map-when t it nil)
              (--map-when nil 'changed '(a b)))"##;
    let expect = expect!["OK ((1 3 3 5) (1 20 3 40) (odd 2 odd) (1 even 3) nil (a b))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_map_first_changes_only_the_first_match() {
    let elisp_form = r##"(list
              (-map-first #'cl-evenp #'1+ '(1 2 4))
              (--map-first (cl-evenp it) (* it 10) '(1 2 4))
              (--map-first (eq it 'missing) 'x '(a b))
              (--map-first t 'x nil)
              (let ((source '(1 2 4)))
                (list (--map-first (cl-evenp it) (* it 10) source) source)))"##;
    let expect = expect!["OK ((1 3 4) (1 20 4) (a b) nil ((1 20 . #1=(4)) (1 2 . #1#)))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_map_last_changes_only_the_last_match() {
    let elisp_form = r##"(list
              (-map-last #'cl-evenp #'1+ '(2 3 4))
              (--map-last (cl-evenp it) (* it 10) '(2 3 4))
              (--map-last (eq it 'missing) 'x '(a b))
              (--map-last t 'x nil)
              (let ((source '(2 3 4)))
                (list (--map-last (cl-evenp it) (* it 10) source) source)))"##;
    let expect = expect!["OK ((2 3 5) (2 3 40) (a b) nil ((2 3 40) (2 3 4)))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_mapcat_variants_flatten_one_mapping_level() {
    let elisp_form = r##"(list
              (-mapcat (lambda (item) (list item (- item))) '(1 2))
              (--mapcat (list it it) '(a b))
              (-mapcat #'list nil)
              (--mapcat (list it) '(å 中)))"##;
    let expect = expect!["OK ((1 -1 2 -2) (a a b b) nil (å 中))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_iterate_variants_include_the_initial_value() {
    let elisp_form = r##"(list
              (-iterate #'1+ 3 4)
              (--iterate (* it 2) 1 5)
              (-iterate #'1+ 3 0)
              (-iterate #'1+ 3 1))"##;
    let expect = expect!["OK ((3 4 5 6) (1 2 4 8 16) nil (3))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_flatten_concat_and_copy_preserve_documented_structure() {
    let elisp_form = r##"(list
              (-flatten '(1 (2 (3)) nil 4))
              (-flatten nil)
              (-flatten-n 1 '(1 (2 (3)) 4))
              (-flatten-n 0 '(1 (2 3)))
              (-concat '(1 2) [3 4] "ab")
              (let* ((source '((a) (b)))
                     (copy (-copy source)))
                (setcar (car copy) 'changed)
                (list source copy)))"##;
    let expect = expect![[
        r#"OK ((1 2 3 4) nil (1 2 (3) 4) (1 (2 3)) (1 2 3 4 . "ab") ((#1=(changed) #2=(b)) (#1# #2#)))"#
    ]];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_splice_variants_replace_matches_with_generated_sequences() {
    let elisp_form = r##"(list
              (-splice
               #'numberp
               (lambda (item) (list item (- item)))
               '(a 1 b 2))
              (--splice
               (numberp it)
               (list it (* it 10))
               '(a 1 b 2))
              (-splice-list #'numberp '(x y) '(a 1 b 2))
              (--splice-list (numberp it) '(x y) '(a 1 b 2))
              (--splice t '(x) nil)
              (--splice nil '(x) '(a b)))"##;
    let expect =
        expect!["OK ((a 1 -1 b 2 -2) (a 1 10 b 2 20) (a x y b x y) (a x y b x y) nil (a b))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_cons_star_and_snoc_construct_lists_in_argument_order() {
    let elisp_form = r##"(list
              (-cons* 1 2 3 '(4 5))
              (-cons* 'a 'b)
              (-snoc '(1 2) 3 4 5)
              (-snoc nil 'a)
              (-snoc '(å) '中))"##;
    let expect = expect!["OK ((1 2 3 4 5) (a . b) (1 2 3 4 5) (a) (å 中))"];

    assert_dash_parity(elisp_form, expect);
}
