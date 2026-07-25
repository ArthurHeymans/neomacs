use std::time::Duration;

use neomacs_melpa_tests::{CachedMelpaOracle, DASH_MELPA_PIN};

const DASH_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn dash_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(DASH_MELPA_PIN, "dash.el")
        .expect("prepare pinned Dash source below ./tmp")
        .with_prelude(r##"(require 'cl-lib)"##)
        .with_timeout(DASH_TEST_TIMEOUT)
}

fn assert_dash_parity(name: &str, form: &str) {
    dash_oracle()
        .run_value(name, form)
        .unwrap_or_else(|error| panic!("Dash parity case `{name}` failed:\n{error}"));
}

fn assert_dash_signal_parity(name: &str, form: &str) {
    dash_oracle()
        .run_signal(name, form)
        .unwrap_or_else(|error| panic!("Dash signal parity case `{name}` failed:\n{error}"));
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_destructive_and_iterative_traversal() {
    assert_dash_parity(
        "dash_destructive_and_iterative_traversal",
        r##"(list
              (let ((items '(b c))) (!cons 'a items) items)
              (let ((items '(a b c))) (!cdr items) items)
              (let (out)
                (-each '(1 2 3) (lambda (item) (push item out)))
                (nreverse out))
              (let (out)
                (--each '(a b) (push (list it-index it) out))
                (nreverse out))
              (let (out)
                (--each-indexed '(x y) (push (cons it-index it) out))
                (nreverse out))
              (let (out)
                (-each-indexed
                 '(x y)
                 (lambda (index item) (push (cons index item) out)))
                (nreverse out))
              (let (out)
                (-each-while
                 '(1 2 0 3) #'identity
                 (lambda (item) (push item out)))
                (nreverse out))
              (let (out)
                (--each-while '(1 2 0 3) (> it 0) (push it out))
                (nreverse out))
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
                (nreverse out))
              (let (out)
                (-dotimes 4 (lambda (index) (push index out)))
                (nreverse out))
              (let (out)
                (--dotimes 4 (push (* it it) out))
                (nreverse out)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_mapping_and_generation() {
    assert_dash_parity(
        "dash_mapping_and_generation",
        r##"(list
              (-map (lambda (number) (1+ number)) '(1 2 3))
              (--map (* it it) '(1 2 3))
              (-map-indexed
               (lambda (index item) (list index item))
               '(a b c))
              (--map-indexed (cons it-index it) '(a b c))
              (-map-when #'cl-evenp #'1+ '(1 2 3 4))
              (--map-when (cl-evenp it) (* it 10) '(1 2 3 4))
              (-replace-where
               #'cl-oddp
               (lambda (_item) 'odd)
               '(1 2 3))
              (--replace-where (cl-evenp it) 'even '(1 2 3))
              (-map-first #'cl-evenp #'1+ '(1 2 4))
              (--map-first (cl-evenp it) (* it 10) '(1 2 4))
              (-map-last #'cl-evenp #'1+ '(2 3 4))
              (--map-last (cl-evenp it) (* it 10) '(2 3 4))
              (-mapcat (lambda (item) (list item (- item))) '(1 2))
              (--mapcat (list it it) '(a b))
              (-iterate #'1+ 3 4)
              (--iterate (* it 2) 1 5))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_left_and_right_reductions() {
    assert_dash_parity(
        "dash_left_and_right_reductions",
        r##"(list
              (-reduce-from #'+ 10 '(1 2 3))
              (--reduce-from (+ acc it) 10 '(1 2 3))
              (-reduce #'- '(10 2 1))
              (--reduce (- acc it) '(10 2 1))
              (-reduce-r #'- '(10 2 1))
              (--reduce-r (- it acc) '(10 2 1))
              (-reduce-r-from #'- 5 '(10 2 1))
              (--reduce-r-from (- it acc) 5 '(10 2 1))
              (-reductions-from #'+ 0 '(1 2 3))
              (--reductions-from (+ acc it) 0 '(1 2 3))
              (-reductions #'+ '(1 2 3))
              (--reductions (+ acc it) '(1 2 3))
              (-reductions-r #'- '(10 2 1))
              (--reductions-r (- it acc) '(10 2 1))
              (-reductions-r-from #'- 5 '(10 2 1))
              (--reductions-r-from (- it acc) 5 '(10 2 1)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_filtering_and_removal() {
    assert_dash_parity(
        "dash_filtering_and_removal",
        r##"(list
              (-filter #'cl-evenp '(1 2 3 4))
              (--filter (> it 2) '(1 2 3 4))
              (-select #'symbolp '(a 1 b 2))
              (--select (numberp it) '(a 1 b 2))
              (-remove #'cl-evenp '(1 2 3 4))
              (--remove (> it 2) '(1 2 3 4))
              (-reject #'cl-oddp '(1 2 3 4))
              (--reject (symbolp it) '(a 1 b 2))
              (-remove-first #'cl-evenp '(1 2 4 3))
              (--remove-first (cl-evenp it) '(1 2 4 3))
              (-reject-first #'cl-oddp '(1 2 3 4))
              (--reject-first (cl-oddp it) '(1 2 3 4))
              (-remove-last #'cl-evenp '(1 2 4 3))
              (--remove-last (cl-evenp it) '(1 2 4 3))
              (-reject-last #'cl-oddp '(1 2 3 4))
              (--reject-last (cl-oddp it) '(1 2 3 4))
              (-remove-item 'x '(a x b x))
              (-keep
               (lambda (item) (and (numberp item) (* item 2)))
               '(a 1 b 2))
              (--keep (and (numberp it) (* it 3)) '(a 1 b 2))
              (-non-nil '(nil a nil b))
              (-count #'cl-evenp '(1 2 3 4))
              (--count (> it 2) '(1 2 3 4)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_flattening_splicing_and_construction() {
    assert_dash_parity(
        "dash_flattening_splicing_and_construction",
        r##"(list
              (-flatten '(1 (2 (3)) nil 4))
              (-flatten-n 1 '(1 (2 (3)) 4))
              (-concat '(1 2) [3 4] "ab")
              (let* ((source '((a) (b)))
                     (copy (-copy source)))
                (setcar (car copy) 'changed)
                (list source copy))
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
              (-cons* 1 2 3 '(4 5))
              (-snoc '(1 2) 3 4 5))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_search_and_item_access() {
    assert_dash_parity(
        "dash_search_and_item_access",
        r##"(list
              (-first #'cl-evenp '(1 2 4))
              (--first (> it 2) '(1 2 3 4))
              (-find #'symbolp '(1 a 2 b))
              (--find (stringp it) '(a "x" "y"))
              (-some (lambda (item) (and (numberp item) (* item 2)))
                     '(a nil 3 4))
              (--some (and (numberp it) (* it 3)) '(a nil 3 4))
              (-any #'cl-evenp '(1 3 4 5))
              (--any (> it 3) '(1 2 4 5))
              (-last #'cl-evenp '(1 2 3 4 5))
              (--last (< it 4) '(1 2 3 4 5))
              (-first-item '(a b c d e f))
              (-second-item '(a b c d e f))
              (-third-item '(a b c d e f))
              (-fourth-item '(a b c d e f))
              (-fifth-item '(a b c d e f))
              (-last-item '(a b c d e f))
              (-butlast '(a b c d e f)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_boolean_predicates_and_aliases() {
    assert_dash_parity(
        "dash_boolean_predicates_and_aliases",
        r##"(list
              (-every #'numberp '(1 2 3))
              (--every (> it 0) '(1 2 3))
              (-any? #'cl-evenp '(1 3 4))
              (--any? (cl-evenp it) '(1 3 4))
              (-some? #'symbolp '(1 a 2))
              (--some? (symbolp it) '(1 a 2))
              (-any-p #'stringp '(a "x"))
              (--any-p (stringp it) '(a "x"))
              (-some-p #'null '(a nil b))
              (--some-p (null it) '(a nil b))
              (-all? #'numberp '(1 2 3))
              (--all? (numberp it) '(1 2 3))
              (-every? #'cl-evenp '(2 4 6))
              (--every? (cl-evenp it) '(2 4 6))
              (-all-p #'symbolp '(a b c))
              (--all-p (symbolp it) '(a b c))
              (-every-p #'stringp '("a" "b"))
              (--every-p (stringp it) '("a" "b"))
              (-none? #'null '(a b c))
              (--none? (null it) '(a b c))
              (-none-p #'numberp '(a b c))
              (--none-p (numberp it) '(a b c))
              (-only-some? #'numberp '(a 1 b))
              (--only-some? (numberp it) '(a 1 b))
              (-only-some-p #'symbolp '(a 1 b))
              (--only-some-p (symbolp it) '(a 1 b)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_slicing_taking_and_dropping() {
    assert_dash_parity(
        "dash_slicing_taking_and_dropping",
        r##"(list
              (-slice '(a b c d e f) 1 5 2)
              (-slice '(a b c d e f) -4 nil 1)
              (-take-while #'numberp '(1 2 a 3))
              (--take-while (< it 3) '(1 2 3 1))
              (-drop-while #'numberp '(1 2 a 3))
              (--drop-while (< it 3) '(1 2 3 1))
              (-take 3 '(a b c d e))
              (-take-last 3 '(a b c d e))
              (-drop 2 '(a b c d e))
              (-drop-last 2 '(a b c d e)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_positional_updates() {
    assert_dash_parity(
        "dash_positional_updates",
        r##"(list
              (-split-at 2 '(a b c d))
              (-rotate 2 '(a b c d e))
              (-rotate -1 '(a b c d e))
              (-insert-at 2 'x '(a b c d))
              (-replace-at 2 'x '(a b c d))
              (-update-at
               2
               (lambda (item)
                 (intern (upcase (symbol-name item))))
               '(a b c d))
              (--update-at 2 (list it 'seen) '(a b c d))
              (-remove-at 2 '(a b c d))
              (-remove-at-indices '(1 3) '(a b c d e)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_replacement_variants() {
    assert_dash_parity(
        "dash_replacement_variants",
        r##"(list
              (-replace 'x 'z '(a x b x))
              (-replace-first 'x 'z '(a x b x))
              (-replace-last 'x 'z '(a x b x)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_splitting_and_separation() {
    assert_dash_parity(
        "dash_splitting_and_separation",
        r##"(list
              (-split-with #'numberp '(1 2 a 3))
              (--split-with (< it 3) '(1 2 3 1))
              (-split-on 'x '(a x b c x d))
              (-split-when #'numberp '(a b 1 c 2 d))
              (--split-when (numberp it) '(a b 1 c 2 d))
              (-separate #'numberp '(a 1 b 2))
              (--separate (symbolp it) '(a 1 b 2)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_partitioning_and_grouping() {
    assert_dash_parity(
        "dash_partitioning_and_grouping",
        r##"(list
              (-partition 2 '(1 2 3 4 5))
              (-partition-all 2 '(1 2 3 4 5))
              (-partition-in-steps 2 1 '(1 2 3 4))
              (-partition-all-in-steps 3 2 '(1 2 3 4 5))
              (-partition-by #'cl-evenp '(1 3 2 4 5))
              (--partition-by (cl-evenp it) '(1 3 2 4 5))
              (-partition-by-header #'numberp '(a 1 2 b 3))
              (--partition-by-header (numberp it) '(a 1 2 b 3))
              (-partition-after-pred #'cl-evenp '(1 2 3 4 5))
              (--partition-after-pred (cl-evenp it) '(1 2 3 4 5))
              (-partition-before-pred #'cl-evenp '(1 2 3 4 5))
              (-partition-after-item 'x '(a x b c x d))
              (-partition-before-item 'x '(a x b c x d))
              (-group-by #'cl-evenp '(1 2 3 4))
              (--group-by (if (numberp it) 'number 'other) '(a 1 b 2)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_zipping_interleaving_and_padding() {
    assert_dash_parity(
        "dash_zipping_interleaving_and_padding",
        r##"(list
              (-interpose 'x '(a b c))
              (-interleave '(1 2 3) '(a b) '(x y z))
              (-zip-with #'cons '(1 2 3) '(a b))
              (--zip-with (list it other) '(1 2 3) '(a b))
              (-zip-lists '(1 2 3) '(a b))
              (-zip-lists-fill 'missing '(1 2 3) '(a b))
              (-unzip-lists '((1 a) (2 b)))
              (-zip '(1 2 3) '(a b))
              (-zip-pair '(1 2 3) '(a b))
              (-zip-fill 'missing '(1 2 3) '(a b))
              (-unzip '((1 a) (2 b)))
              (-cycle '(a b c))
              (-pad 'missing '(1 2 3) '(a b)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_annotation_tables_and_grades() {
    assert_dash_parity(
        "dash_annotation_tables_and_grades",
        r##"(list
              (-annotate #'length '("a" "bbb" "cc"))
              (--annotate (* it it) '(1 2 3))
              (-table #'* '(1 2) '(10 20 30))
              (-table-flat (lambda (left right) (list left right))
                           '(1 2) '(a b))
              (-grade-up #'< '(30 10 20))
              (-grade-down #'< '(30 10 20)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_index_search_and_selection() {
    assert_dash_parity(
        "dash_index_search_and_selection",
        r##"(list
              (-find-index #'cl-evenp '(1 3 4 6))
              (--find-index (> it 3) '(1 3 4 6))
              (-elem-index 'x '(a x b x))
              (-find-indices #'cl-evenp '(1 2 3 4))
              (--find-indices (> it 2) '(1 2 3 4))
              (-elem-indices 'x '(a x b x))
              (-find-last-index #'cl-evenp '(1 2 3 4 5))
              (--find-last-index (< it 4) '(1 2 3 4 5))
              (-select-by-indices '(0 2 4) '(a b c d e))
              (-select-columns '(0 2) '((a b c) (d e f)))
              (-select-column 1 '((a b c) (d e f))))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_threading_and_side_effect_macros() {
    assert_dash_parity(
        "dash_threading_and_side_effect_macros",
        r##"(list
              (-> 5 1+ (* 2))
              (->> '(1 2 3) (mapcar #'1+) (apply #'+))
              (--> 5 (+ it 1) (* it 2))
              (-as-> 5 value (+ value 1) (* value value))
              (-some-> 5 1+ (* 2))
              (-some-> nil 1+ (* 2))
              (-some->> '(1 2) (mapcar #'1+) (apply #'+))
              (-some--> 5 (+ it 1) (* it 2))
              (let ((value (list 1)))
                (-doto value
                  (setcdr '(2 3))
                  (nreverse))
                value)
              (let ((value (list 1)))
                (--doto value
                  (setcdr it '(2 3))
                  (nreverse it))
                value))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_destructuring_bindings() {
    assert_dash_parity(
        "dash_destructuring_bindings",
        r##"(list
              (-let (((a b . rest) '(1 2 3 4)))
                (list a b rest))
              (-let* (((a b) '(1 2))
                       ((c d) (list b a)))
                (list a b c d))
              (-let [[a b &rest rest] [1 2 3 4]]
                (list a b rest))
              (-let (((&plist :name name :age age)
                      '(:name ada :age 36)))
                (list name age))
              (-let (((&alist 'name name 'age age)
                      '((name . ada) (age . 36))))
                (list name age))
              (let ((table (make-hash-table :test 'eq)))
                (puthash 'name 'ada table)
                (-let (((&hash 'name name) table)) name))
              (mapcar (-lambda ((key . value)) (list value key))
                      '((a . 1) (b . 2)))
              (let (a b)
                (-setq (a b) '(1 2))
                (list a b)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_conditional_bindings() {
    assert_dash_parity(
        "dash_conditional_bindings",
        r##"(list
              (-if-let (value 3) (* value 2) 'missing)
              (-if-let ((a b) '(1 2)) (+ a b) 'missing)
              (-if-let* ((a 2) (b (+ a 3))) (* a b) 'missing)
              (--if-let 4 (* it 2) 'missing)
              (-when-let (value 3) (* value 2))
              (-when-let* ((a 2) (b (+ a 3))) (* a b))
              (--when-let 4 (* it 2))
              (-if-let (value nil) value 'else)
              (-if-let* ((a 1) (b nil)) (+ a b) 'else))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_set_operations_and_custom_comparison() {
    assert_dash_parity(
        "dash_set_operations_and_custom_comparison",
        r##"(list
              (-distinct '(a b a c b))
              (-uniq '(a b a c b))
              (-union '(a b c) '(b c d))
              (-intersection '(a b c) '(b c d))
              (-difference '(a b c) '(b d))
              (-frequencies '(a b a c b a))
              (let ((-compare-fn #'string-equal))
                (list
                 (-distinct '("A" "A" "B"))
                 (-union '("A") '("A" "B"))
                 (-intersection '("A" "B") '("B")))))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_combinatorics_and_sequence_edges() {
    assert_dash_parity(
        "dash_combinatorics_and_sequence_edges",
        r##"(list
              (-powerset '(a b c))
              (-permutations '(a b c))
              (-permutations '(a a b))
              (-inits '(a b c))
              (-tails '(a b c))
              (-common-prefix '(a b c) '(a b d) '(a b))
              (-common-suffix '(a b c) '(x b c) '(b c)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_membership_relations_and_sorting() {
    assert_dash_parity(
        "dash_membership_relations_and_sorting",
        r##"(list
              (-contains? '(a b c) 'b)
              (-contains-p '(a b c) 'z)
              (-same-items? '(a b a) '(b a a))
              (-same-items-p '(a b) '(b a))
              (-is-prefix? '(a b) '(a b c))
              (-is-prefix-p '(a b) '(a b c))
              (-is-suffix? '(b c) '(a b c))
              (-is-suffix-p '(b c) '(a b c))
              (-is-infix? '(b c) '(a b c d))
              (-is-infix-p '(b c) '(a b c d))
              (-sort #'< '(3 1 2))
              (--sort (< it other) '(3 1 2))
              (-list nil)
              (-list 1 2 3)
              (-repeat 3 'x))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_numeric_aggregates_and_ranges() {
    assert_dash_parity(
        "dash_numeric_aggregates_and_ranges",
        r##"(list
              (-sum '(1 2 3 4))
              (-running-sum '(1 2 3 4))
              (-product '(1 2 3 4))
              (-running-product '(1 2 3 4))
              (-max '(3 1 4 2))
              (-min '(3 1 4 2))
              (-max-by (lambda (left right)
                         (< (length left) (length right)))
                       '("a" "bbbb" "cc"))
              (-min-by (lambda (left right)
                         (< (length left) (length right)))
                       '("a" "bbbb" "cc"))
              (--max-by (< (length it) (length other))
                        '("a" "bbbb" "cc"))
              (--min-by (< (length it) (length other))
                        '("a" "bbbb" "cc"))
              (-iota 5)
              (-iota 4 10 3))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_tree_mapping_and_reduction() {
    assert_dash_parity(
        "dash_tree_mapping_and_reduction",
        r##"(list
              (-cons-pair? '(a . b))
              (-cons-pair-p '(a b))
              (-cons-to-list '(a . b))
              (-value-to-list '(a . b))
              (-value-to-list '(a b))
              (-tree-map #'1+ '(1 (2 3) 4))
              (--tree-map (* it 2) '(1 (2 3) 4))
              (-tree-mapreduce #'1+ #'+ '(1 (2 3) 4))
              (--tree-mapreduce (1+ it) (+ it acc) '(1 (2 3) 4))
              (-tree-mapreduce-from #'1+ #'+ 0 '(1 (2 3) 4))
              (--tree-mapreduce-from (1+ it) (+ it acc) 0
                                     '(1 (2 3) 4))
              (-tree-reduce #'+ '(1 (2 3) 4))
              (--tree-reduce (+ it acc) '(1 (2 3) 4))
              (-tree-reduce-from #'+ 0 '(1 (2 3) 4))
              (--tree-reduce-from (+ it acc) 0 '(1 (2 3) 4))
              (-tree-map-nodes #'numberp #'1+ '(1 (2 3) 4))
              (--tree-map-nodes (numberp it) (1+ it) '(1 (2 3) 4))
              (-tree-seq #'listp #'identity '(1 (2 3)))
              (--tree-seq (listp it) it '(1 (2 3)))
              (let* ((source '((a) (b c)))
                     (clone (-clone source)))
                (setcar (car clone) 'changed)
                (list source clone)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_fixpoint_unfold_and_closure_generators() {
    assert_dash_parity(
        "dash_fixpoint_unfold_and_closure_generators",
        r##"(list
              (-fix (lambda (items) (-distinct items)) '(a b a b))
              (--fix (-distinct it) '(a b a b))
              (-unfold
               (lambda (seed)
                 (and (< seed 5) (cons seed (1+ seed))))
               1)
              (--unfold (and (< it 5) (cons it (1+ it))) 1)
              (funcall (-iteratefn #'1+ 4) 10)
              (let ((counter (-counter 2 8 2)))
                (list (funcall counter)
                      (funcall counter)
                      (funcall counter)
                      (funcall counter)))
              (funcall
               (-fixfn (lambda (number) (/ (+ number 10) 2)))
               0))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_function_combinators() {
    assert_dash_parity(
        "dash_function_combinators",
        r##"(list
              (funcall (-partial #'+ 10) 1 2)
              (funcall (-rpartial #'- 3) 10)
              (funcall (-juxt #'1+ #'1- (lambda (x) (* x x))) 5)
              (funcall (-compose #'1+ (lambda (x) (* x 2))) 5)
              (funcall (-applify #'+) '(1 2 3))
              (funcall (-on #'+ #'1+) 1 2 3)
              (funcall (-flip #'-) 3 10)
              (funcall (-rotate-args 1 #'list) 'a 'b 'c)
              (funcall (-const 'fixed) 1 2 3)
              (funcall (-cut list 1 <> 3 <>) 2 4)
              (funcall (-not #'numberp) 'a)
              (funcall (-orfn #'numberp #'symbolp) 'a)
              (funcall (-andfn #'numberp #'cl-evenp) 4)
              (funcall
               (-prodfn
                #'1+
                (lambda (item)
                  (intern (upcase (symbol-name item)))))
               '(1 a)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_package_integration_commands() {
    assert_dash_parity(
        "dash_package_integration_commands",
        r##"(list
              (with-temp-buffer
                (dash-fontify-mode 1)
                (prog1 dash-fontify-mode
                  (dash-fontify-mode -1)))
              (with-temp-buffer
                (dash-enable-font-lock 1)
                (prog1 dash-fontify-mode
                  (dash-enable-font-lock -1)))
              (progn
                (global-dash-fontify-mode 1)
                (prog1 global-dash-fontify-mode
                  (global-dash-fontify-mode -1)))
              (progn (dash-register-info-lookup) t)
              (dash-unload-function))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_empty_collection_semantics() {
    assert_dash_parity(
        "dash_empty_collection_semantics",
        r##"(list
              (-map #'identity nil)
              (-filter #'identity nil)
              (-flatten nil)
              (-take 3 nil)
              (-drop 3 nil)
              (-partition-all 2 nil)
              (-powerset nil)
              (-tails nil)
              (-zip-lists nil nil))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_nil_short_circuit_semantics() {
    assert_dash_parity(
        "dash_nil_short_circuit_semantics",
        r##"(list
              (-some-> nil 1+ (* 2))
              (-some->> nil (mapcar #'1+) (apply #'+))
              (-some--> nil (1+ it) (* it 2)))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_reduce_empty_arity_signal() {
    assert_dash_signal_parity(
        "dash_reduce_empty_arity_signal",
        r##"(-reduce (lambda (left right) (+ left right)) nil)"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_iota_negative_count_signal() {
    assert_dash_signal_parity("dash_iota_negative_count_signal", r##"(-iota -1)"##);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_slice_zero_step_signal() {
    assert_dash_signal_parity(
        "dash_slice_zero_step_signal",
        r##"(-slice '(a b c) 0 nil 0)"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_destructuring_short_vector_signal() {
    assert_dash_signal_parity(
        "dash_destructuring_short_vector_signal",
        r##"(-let ([a b c] [1 2]) (list a b c))"##,
    );
}
