use super::assert_ace_jump_mode_parity;
use expect_test::expect;

#[test]
fn ace_jump_mode_tree_constructs_single_leaf_without_branching() {
    let elisp_form = r##"(ace-jump-tree-breadth-first-construct 1 3)"##;
    let expect = expect!["OK (leaf)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_tree_constructs_exact_small_branch_shapes() {
    let elisp_form = r##"(mapcar
         (lambda (count)
           (ace-jump-tree-breadth-first-construct count 3))
         '(2 3 4 5 6 7))"##;
    let expect = expect![
        "OK ((branch (leaf) (leaf)) (branch (leaf) (leaf) (leaf)) (branch (branch (leaf) (leaf)) (leaf) (leaf)) (branch (branch (leaf) (leaf) (leaf)) (leaf) (leaf)) (branch (branch (leaf) (leaf) (leaf)) (branch (leaf) (leaf)) (leaf)) (branch (branch (leaf) (leaf) (leaf)) (branch (leaf) (leaf) (leaf)) (leaf)))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_tree_shape_varies_with_branching_factor() {
    let elisp_form = r##"(list
         (ace-jump-tree-breadth-first-construct 5 2)
         (ace-jump-tree-breadth-first-construct 5 3)
         (ace-jump-tree-breadth-first-construct 5 4)
         (ace-jump-tree-breadth-first-construct 8 2))"##;
    let expect = expect![
        "OK ((branch (branch (branch (leaf) (leaf)) (leaf)) (branch (leaf) (leaf))) (branch (branch (leaf) (leaf) (leaf)) (leaf) (leaf)) (branch (branch (leaf) (leaf)) (leaf) (leaf) (leaf)) (branch (branch (branch (leaf) (leaf)) (branch (leaf) (leaf))) (branch (branch (leaf) (leaf)) (branch (leaf) (leaf)))))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_tree_always_has_requested_leaf_count_and_bounded_branches() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let ((leaves 0)
                 (branch-widths nil)
                 (tree
                  (ace-jump-tree-breadth-first-construct
                   (car spec)
                   (cdr spec))))
             (ace-jump-tree-preorder-traverse
              tree
              (lambda (_node)
                (setq leaves (1+ leaves)))
              (lambda (node)
                (setq branch-widths
                      (cons (length (cdr node))
                            branch-widths))))
             (list
              spec
              leaves
              (nreverse branch-widths))))
         '((1 . 2) (2 . 2) (3 . 2) (9 . 2)
           (4 . 3) (10 . 3) (17 . 4)))"##;
    let expect = expect![
        "OK (((1 . 2) 1 nil) ((2 . 2) 2 (2)) ((3 . 2) 3 (2 2)) ((9 . 2) 9 (2 2 2 2 2 2 2 2)) ((4 . 3) 4 (3 2)) ((10 . 3) 10 (3 3 2 3 3)) ((17 . 4) 17 (4 4 2 4 4 4)))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_tree_preorder_traversal_callback_order_matches() {
    let elisp_form = r##"(let ((tree
              '(branch
                (branch (leaf . a) (leaf . b))
                (leaf . c)
                (branch (leaf . d))))
             (events nil))
         (ace-jump-tree-preorder-traverse
          tree
          (lambda (node)
            (setq events
                  (cons
                   (list 'leaf (cdr node))
                   events)))
          (lambda (node)
            (setq events
                  (cons
                   (list 'branch (length (cdr node)))
                   events))))
         (nreverse events))"##;
    let expect =
        expect!["OK ((branch 3) (branch 2) (leaf a) (leaf b) (leaf c) (branch 1) (leaf d))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_tree_preorder_traversal_supports_each_optional_callback() {
    let elisp_form = r##"(let ((tree
              '(branch
                (leaf . a)
                (branch (leaf . b) (leaf . c))))
             (leaves nil)
             (branches nil))
         (ace-jump-tree-preorder-traverse
          tree
          (lambda (node)
            (setq leaves
                  (cons (cdr node) leaves))))
         (ace-jump-tree-preorder-traverse
          tree
          nil
          (lambda (node)
            (setq branches
                  (cons (length (cdr node))
                        branches))))
         (list
          (nreverse leaves)
          (nreverse branches)
          (ace-jump-tree-preorder-traverse tree)))"##;
    let expect = expect!["OK ((a b c) (2 2) nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_tree_invalid_node_reports_message_and_continues() {
    let elisp_form = r##"(let (messages leaves)
         (cl-letf (((symbol-function 'message)
                    (lambda (&rest arguments)
                      (setq messages
                            (cons arguments messages)))))
           (ace-jump-tree-preorder-traverse
            '(branch
              (mystery . x)
              (leaf . y))
            (lambda (node)
              (setq leaves
                    (cons (cdr node) leaves)))))
         (list
          (nreverse messages)
          (nreverse leaves)))"##;
    let expect = expect![[r#"OK ((("[AceJump] Internal Error: invalid tree node type")) (y))"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_tree_callbacks_may_mutate_leaf_payloads() {
    let elisp_form = r##"(let ((tree
              '(branch
                (leaf . 1)
                (branch
                 (leaf . 2)
                 (leaf . 3)))))
         (ace-jump-tree-preorder-traverse
          tree
          (lambda (node)
            (setf (cdr node)
                  (* 10 (cdr node)))))
         tree)"##;
    let expect = expect!["OK (branch (leaf . 10) (branch (leaf . 20) (leaf . 30)))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
