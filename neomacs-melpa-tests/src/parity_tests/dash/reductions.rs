use expect_test::expect;

use super::{assert_dash_parity, assert_dash_signal_parity};

#[test]
fn dash_reduce_from_variants_fold_left_from_an_explicit_seed() {
    let elisp_form = r##"(list
              (-reduce-from #'+ 10 '(1 2 3))
              (--reduce-from (+ acc it) 10 '(1 2 3))
              (-reduce-from #'cons 'z '(a b))
              (-reduce-from #'+ 10 nil)
              (--reduce-from (list acc it) 'seed '(a b)))"##;
    let expect = expect!["OK (16 16 ((z . a) . b) 10 ((seed a) b))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_reduce_variants_use_the_first_item_as_the_left_seed() {
    let elisp_form = r##"(list
              (-reduce #'- '(10 2 1))
              (--reduce (- acc it) '(10 2 1))
              (-reduce #'+ '(7))
              (--reduce (+ acc it) '(1 2 3 4))
              (-reduce #'+ nil))"##;
    let expect = expect!["OK (7 7 7 10 0)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_reduce_right_variants_associate_from_the_end() {
    let elisp_form = r##"(list
              (-reduce-r #'- '(10 2 1))
              (--reduce-r (- it acc) '(10 2 1))
              (-reduce-r #'+ '(7))
              (-reduce-r #'+ nil)
              (--reduce-r (list it acc) '(a b c)))"##;
    let expect = expect!["OK (9 9 7 0 (a (b c)))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_reduce_right_from_variants_use_an_explicit_terminal_seed() {
    let elisp_form = r##"(list
              (-reduce-r-from #'- 5 '(10 2 1))
              (--reduce-r-from (- it acc) 5 '(10 2 1))
              (-reduce-r-from #'+ 10 nil)
              (--reduce-r-from (list it acc) 'seed '(a b)))"##;
    let expect = expect!["OK (4 4 10 (a (b seed)))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_reductions_variants_return_each_left_intermediate_value() {
    let elisp_form = r##"(list
              (-reductions-from #'+ 0 '(1 2 3))
              (--reductions-from (+ acc it) 0 '(1 2 3))
              (-reductions #'+ '(1 2 3))
              (--reductions (+ acc it) '(1 2 3))
              (-reductions-from #'+ 10 nil)
              (-reductions #'+ nil))"##;
    let expect = expect!["OK ((0 1 3 6) (0 1 3 6) (1 3 6) (1 3 6) (10) (0))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_reductions_right_variants_return_each_right_intermediate_value() {
    let elisp_form = r##"(list
              (-reductions-r #'- '(10 2 1))
              (--reductions-r (- it acc) '(10 2 1))
              (-reductions-r-from #'- 5 '(10 2 1))
              (--reductions-r-from (- it acc) 5 '(10 2 1))
              (-reductions-r-from #'+ 10 nil)
              (-reductions-r #'+ nil))"##;
    let expect = expect!["OK ((9 1 1) (9 1 1) (4 6 -4 5) (4 6 -4 5) (10) (0))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_filter_select_remove_and_reject_aliases_partition_by_truth() {
    let elisp_form = r##"(list
              (-filter #'cl-evenp '(1 2 3 4))
              (--filter (> it 2) '(1 2 3 4))
              (-select #'symbolp '(a 1 b 2))
              (--select (numberp it) '(a 1 b 2))
              (-remove #'cl-evenp '(1 2 3 4))
              (--remove (> it 2) '(1 2 3 4))
              (-reject #'cl-oddp '(1 2 3 4))
              (--reject (symbolp it) '(a 1 b 2))
              (-filter #'identity nil)
              (--remove t nil))"##;
    let expect = expect!["OK ((2 4) (3 4) (a b) (1 2) (1 3) (1 2) (2 4) (1 2) nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_remove_first_and_reject_first_drop_only_the_first_match() {
    let elisp_form = r##"(list
              (-remove-first #'cl-evenp '(1 2 4 3))
              (--remove-first (cl-evenp it) '(1 2 4 3))
              (-reject-first #'cl-oddp '(1 2 3 4))
              (--reject-first (cl-oddp it) '(1 2 3 4))
              (--remove-first (eq it 'x) '(a b))
              (--remove-first t nil))"##;
    let expect = expect!["OK ((1 4 3) (1 4 3) (2 3 4) (2 3 4) (a b) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_remove_last_and_reject_last_drop_only_the_last_match() {
    let elisp_form = r##"(list
              (-remove-last #'cl-evenp '(1 2 4 3))
              (--remove-last (cl-evenp it) '(1 2 4 3))
              (-reject-last #'cl-oddp '(1 2 3 4))
              (--reject-last (cl-oddp it) '(1 2 3 4))
              (--remove-last (eq it 'x) '(a b))
              (--remove-last t nil))"##;
    let expect = expect!["OK ((1 2 3) (1 2 3) (1 2 4) (1 2 4) (a b) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_keep_non_nil_remove_item_and_count_cover_sparse_results() {
    let elisp_form = r##"(list
              (-remove-item 'x '(a x b x))
              (-keep
               (lambda (item) (and (numberp item) (* item 2)))
               '(a 1 b 2))
              (--keep (and (numberp it) (* it 3)) '(a 1 b 2))
              (-non-nil '(nil a nil b))
              (-count #'cl-evenp '(1 2 3 4))
              (--count (> it 2) '(1 2 3 4))
              (-keep #'identity nil)
              (-count #'identity nil))"##;
    let expect = expect!["OK ((a b) (2 4) (3 6) (a b) 2 2 nil 0)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_first_find_some_and_any_aliases_return_the_first_matching_result() {
    let elisp_form = r##"(list
              (-first #'cl-evenp '(1 2 4))
              (--first (> it 2) '(1 2 3 4))
              (-find #'symbolp '(1 a 2 b))
              (--find (stringp it) '(a "x" "y"))
              (-some
               (lambda (item) (and (numberp item) (* item 2)))
               '(a nil 3 4))
              (--some (and (numberp it) (* it 3)) '(a nil 3 4))
              (-any #'cl-evenp '(1 3 4 5))
              (--any (> it 3) '(1 2 4 5))
              (--first t nil)
              (--some t nil))"##;
    let expect = expect![[r#"OK (2 3 a "x" 6 9 t t nil nil)"#]];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_last_and_positional_item_accessors_cover_short_and_empty_lists() {
    let elisp_form = r##"(list
              (-last #'cl-evenp '(1 2 3 4 5))
              (--last (< it 4) '(1 2 3 4 5))
              (-first-item '(a b c d e f))
              (-second-item '(a b c d e f))
              (-third-item '(a b c d e f))
              (-fourth-item '(a b c d e f))
              (-fifth-item '(a b c d e f))
              (-last-item '(a b c d e f))
              (-butlast '(a b c d e f))
              (-second-item '(a))
              (-last-item nil)
              (-butlast nil))"##;
    let expect = expect!["OK (4 3 a b c d e f (a b c d e) nil nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_any_alias_family_normalizes_truthy_values_to_booleans() {
    let elisp_form = r##"(list
              (-any? #'cl-evenp '(1 3 4))
              (--any? (cl-evenp it) '(1 3 4))
              (-some? #'symbolp '(1 a 2))
              (--some? (symbolp it) '(1 a 2))
              (-any-p #'stringp '(a "x"))
              (--any-p (stringp it) '(a "x"))
              (-some-p #'null '(a nil b))
              (--some-p (null it) '(a nil b))
              (-any? #'identity nil)
              (-any? #'identity '(nil value)))"##;
    let expect = expect!["OK (t t t t t t t t nil t)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_all_alias_family_treats_empty_lists_as_vacuously_true() {
    let elisp_form = r##"(list
              (-all? #'numberp '(1 2 3))
              (--all? (numberp it) '(1 2 3))
              (-every? #'cl-evenp '(2 4 6))
              (--every? (cl-evenp it) '(2 4 6))
              (-all-p #'symbolp '(a b c))
              (--all-p (symbolp it) '(a b c))
              (-every-p #'stringp '("a" "b"))
              (--every-p (stringp it) '("a" "b"))
              (-all? #'identity nil)
              (-all? #'identity '(t nil)))"##;
    let expect = expect!["OK (t t t t t t t t t nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_every_variants_return_the_last_truthy_result_and_short_circuit() {
    let elisp_form = r##"(list
              (-every
               (lambda (item) (and (numberp item) (* item 10)))
               '(1 2 3))
              (--every (and (numberp it) (cons it it)) '(1 2 3))
              (-every #'numberp '(1 symbol 3))
              (let (seen)
                (list
                 (-every
                  (lambda (item)
                    (push item seen)
                    (< item 2))
                  '(1 2 3))
                 (nreverse seen)))
              (-every #'identity nil)
              (--every it nil))"##;
    let expect = expect!["OK (30 (3 . 3) nil (nil (1 2)) t t)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_none_and_only_some_aliases_cover_zero_some_and_all_matches() {
    let elisp_form = r##"(list
              (-none? #'null '(a b c))
              (--none? (null it) '(a b c))
              (-none-p #'numberp '(a b c))
              (--none-p (numberp it) '(a b c))
              (-only-some? #'numberp '(a 1 b))
              (--only-some? (numberp it) '(a 1 b))
              (-only-some-p #'symbolp '(a 1 b))
              (--only-some-p (symbolp it) '(a 1 b))
              (-none? #'identity nil)
              (-only-some? #'identity nil)
              (-only-some? #'identity '(t t))
              (-only-some? #'identity '(nil t)))"##;
    let expect = expect!["OK (t t t t t t t t t nil nil t)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
fn dash_reduce_empty_with_a_binary_function_signals_exact_arity_data() {
    let elisp_form = r##"(-reduce #'cons nil)"##;
    let expect = expect!["ERR (wrong-number-of-arguments #<subr cons> 0)"];

    assert_dash_signal_parity(elisp_form, expect);
}
