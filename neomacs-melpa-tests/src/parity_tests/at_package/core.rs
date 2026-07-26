use expect_test::expect;

use super::{assert_at_parity, assert_at_signal_parity};

#[test]
fn at_root_object_core_methods_features_and_help_binding_match_the_pin() {
    let elisp_form = r##"(list
              (@p @)
              (aref @ 0)
              (plist-get
               (aref @ 1)
               :proto)
              (@! @ :keys)
              (mapcar
               (lambda (property)
                 (functionp
                  (@ @ property)))
               '(:set :get :init
                 :new :is :keys))
              (featurep '@)
              (featurep '@-mixins)
              (lookup-key
               global-map
               (kbd "C-h @")))"##;
    let expect = expect![[
        r#"OK (t @ nil (:proto :set :get :init :new :is :keys) (t t t t t t) t t describe-@)"#
    ]];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_predicate_and_extend_cover_root_default_multiple_prototypes_and_properties() {
    let elisp_form = r##"(let* ((left
                      (@extend :side 'left))
                     (right
                      (@extend :side 'right))
                     (child
                      (@extend
                       left right
                       :name "child"
                       :nil-value nil)))
               (list
                (mapcar
                 #'@p
                 (list @ left child
                       [not-an-at-object]
                       [@ nil]
                       '(list) nil))
                (eq
                 (car
                  (plist-get
                   (aref left 1)
                   :proto))
                 @)
                (mapcar
                 (lambda (object)
                   (cond
                    ((eq object left)
                     'left)
                    ((eq object right)
                     'right)
                    (t 'other)))
                 (plist-get
                  (aref child 1)
                  :proto))
                (@ child :name)
                (@ child :nil-value)
                (@ child :side)))"##;
    let expect = expect!["OK ((t t t nil t nil nil) t (left right) \"child\" nil left)"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_predicate_on_an_empty_vector_signals_the_exact_slot_error() {
    let elisp_form = r##"(@p [])"##;
    let expect = expect!["ERR (args-out-of-range [] 0)"];

    assert_at_signal_parity(elisp_form, expect);
}

#[test]
fn at_precedence_flattens_diamond_inheritance_and_removes_first_duplicate() {
    let elisp_form = r##"(let* ((root
                      (@extend :id 'root))
                     (left
                      (@extend root :id 'left))
                     (right
                      (@extend root :id 'right))
                     (top
                      (@extend left right)))
               (mapcar
                (lambda (object)
                  (cond
                   ((eq object left)
                    'left)
                   ((eq object right)
                    'right)
                   ((eq object root)
                    'root)
                   ((eq object @) '@)
                   (t 'unknown)))
                (@precedence top)))"##;
    let expect = expect!["OK (left right root @)"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_instance_checks_cover_identity_ancestors_unrelated_and_non_objects() {
    let elisp_form = r##"(let* ((parent (@extend))
                     (child (@extend parent))
                     (unrelated (@extend)))
               (list
                (@is child child)
                (@is child parent)
                (@is child @)
                (@is parent child)
                (@is child unrelated)
                (@is t @)
                (@is @ t)
                (@! child :is parent)
                (@! parent :is child)))"##;
    let expect = expect!["OK (t t t nil nil nil nil t nil)"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_internal_queue_preserves_fifo_head_and_empty_reset_contract() {
    let elisp_form = r##"(let ((queue
                    (@--queue-create)))
               (list
                (@--queue-head queue)
                (@--queue-enqueue
                 queue 'first)
                (copy-sequence
                 (@--queue-head queue))
                (@--queue-enqueue
                 queue 'second)
                (copy-sequence
                 (@--queue-head queue))
                (@--queue-dequeue queue)
                (copy-sequence
                 (@--queue-head queue))
                (@--queue-dequeue queue)
                (@--queue-head queue)
                queue))"##;
    let expect = expect![[
        r#"OK (nil first (first) second (first second) first (second) second nil (nil))"#
    ]];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_lookup_uses_breadth_first_inheritance_and_counts_super_matches() {
    let elisp_form = r##"(let* ((root
                      (@extend :name 'root))
                     (left
                      (@extend root :name 'left))
                     (right
                      (@extend root :name 'right))
                     (top
                      (@extend left right
                               :name 'top))
                     (right-only
                      (@extend
                       (@extend root)
                       right)))
               (list
                (@ top :name)
                (@ top :name :super 1)
                (@ top :name :super 2)
                (@ top :name :super 3)
                (@ right-only :name)))"##;
    let expect = expect!["OK (top left right root right)"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_lookup_distinguishes_implicit_error_explicit_nil_and_non_nil_defaults() {
    let elisp_form = r##"(let ((object (@extend)))
               (list
                (@ object :missing
                   :default nil)
                (@ object :missing
                   :default 'fallback)
                (@ object :missing
                   :super 10
                   :default 'past-end)))"##;
    let expect = expect!["OK (nil fallback past-end)"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_lookup_without_property_or_default_signals_exact_dynamic_getter_error() {
    let elisp_form = r##"(@ (@extend) :missing)"##;
    let expect = expect![[r#"ERR (error "Property unbound: :missing")"#]];

    assert_at_signal_parity(elisp_form, expect);
}

#[test]
fn at_setf_assigns_only_the_immediate_object_and_returns_the_new_value() {
    let elisp_form = r##"(let* ((parent
                      (@extend :value 'parent))
                     (child (@extend parent)))
               (list
                (@ child :value)
                (setf
                 (@ child :value)
                 'child)
                (@ child :value)
                (@ parent :value)
                (@! child :keys)
                (@! parent :keys)))"##;
    let expect = expect![[r#"OK (parent child child parent (:proto :value) (:proto :value))"#]];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_method_calls_and_super_method_dsl_chain_through_each_matching_prototype() {
    let elisp_form = r##"(let* ((a (@extend))
                     (b (@extend a))
                     (c (@extend b)))
               (def@ a :chain (value)
                 (list 'a value))
               (def@ b :chain (value)
                 (cons 'b
                       (@^:chain value)))
               (def@ c :chain (value)
                 (cons 'c
                       (@^:chain value)))
               (list
                (@! c :chain 7)
                (@--super! c :chain 8)
                (with-@@ c
                  (@^:chain 9))))"##;
    let expect = expect!["OK ((c b a 7) (b a 8) (b a 9))"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_property_super_dsl_reads_each_next_matching_value() {
    let elisp_form = r##"(let* ((a
                      (@extend :value 'a))
                     (b
                      (@extend a :value 'b))
                     (c
                      (@extend b :value 'c)))
               (list
                (with-@@ c @:value)
                (with-@@ c @^:value)
                (@ c :value :super 2)))"##;
    let expect = expect!["OK (c b a)"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_new_calls_initializer_and_core_keys_and_is_methods_observe_the_child() {
    let elisp_form = r##"(let ((rectangle
                    (@extend
                     :width nil
                     :height nil)))
               (def@ rectangle
                   :init (width height)
                 (setf
                  @:width width
                  @:height height))
               (def@ rectangle :area ()
                 (* @:width @:height))
               (let ((instance
                      (@! rectangle
                          :new 6 7)))
                 (list
                  (@! instance :area)
                  (@ instance :width)
                  (@ instance :height)
                  (@! instance :is
                      rectangle)
                  (@! instance :is @)
                  (@! instance :keys)
                  (@! rectangle :keys))))"##;
    let expect =
        expect![[r#"OK (42 6 7 t t (:proto :width :height) (:proto :width :height :init :area))"#]];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_dynamic_getter_receives_missing_property_but_explicit_default_bypasses_it() {
    let elisp_form = r##"(let ((object
                    (@extend
                     :prefix "got")))
               (def@ object :get (property)
                 (list
                  @:prefix property))
               (list
                (@ object :missing)
                (@ object :other
                   :default 'explicit)))"##;
    let expect = expect!["OK ((\"got\" :missing) explicit)"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_walk_replace_and_with_object_preserve_quote_and_expand_property_positions() {
    let elisp_form = r##"(list
              (@--walk
               '(setf @:name 10)
               '(quote)
               #'@--replace)
              (@--walk
               '(setf '@:name 10)
               '(quote)
               #'@--replace)
              (@--walk
               '(@:method @:argument)
               '(quote)
               #'@--replace)
              (macroexpand-1
               '(with-@@ object
                  (list @:value
                        (@:method 1)
                        @^:parent)))
              (with-@@
                  (@extend :value 'ok)
                @:value))"##;
    let expect = expect![[
        r#"OK ((setf (@ @@ :name) 10) (setf '@:name 10) (@! @@ :method (@ @@ :argument)) (let ((@@ object)) (list (@ @@ :value) (@! @@ :method 1) (@--super @@ :parent))) ok)"#
    ]];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_definer_returns_property_preserves_docstring_and_binds_self_before_arguments() {
    let elisp_form = r##"(let ((object
                    (@extend :base 10)))
               (list
                (def@ object :sum (left
                                   &optional
                                   (right 2))
                  "Add values to the base."
                  (+ @:base left right))
                (@! object :sum 3)
                (@! object :sum 3 4)
                (documentation
                 (@ object :sum))
                (help-function-arglist
                 (@ object :sum)
                 t)))"##;
    let expect = expect![[
        r#"OK (:sum 15 17 "Add values to the base.\n\n(fn @@ LEFT &optional (RIGHT 2))" (@@ left &rest --cl-rest--))"#
    ]];

    assert_at_parity(elisp_form, expect);
}
