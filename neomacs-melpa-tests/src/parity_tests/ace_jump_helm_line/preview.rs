use super::{assert_ace_jump_helm_line_parity, assert_ace_jump_helm_line_signal_parity};
use expect_test::expect;

#[test]
fn ace_jump_helm_line_scroll_function_skips_an_unchanged_window_start() {
    let elisp_form = r##"(let ((ace-jump-helm-line--last-win-start 17)
                   events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--update-line-overlays-maybe)
                     (lambda (&rest args)
                       (push args events))))
                 (list
                  (ace-jump-helm-line--scroll-function 'window 17)
                  ace-jump-helm-line--last-win-start
                  events)))"##;
    let expect = expect!["OK (nil 17 nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_scroll_function_updates_changed_window_start_once() {
    let elisp_form = r##"(let ((ace-jump-helm-line--last-win-start 17)
                   events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--update-line-overlays-maybe)
                     (lambda (&rest args)
                       (setq events
                             (append events
                                     (list args)))
                       'update-result)))
                 (list
                  (ace-jump-helm-line--scroll-function 'window 23)
                  ace-jump-helm-line--last-win-start
                  events)))"##;
    let expect = expect!["OK (update-result 23 ((23)))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_add_scroll_function_installs_one_buffer_local_hook() {
    let elisp_form = r##"(let ((helm-buffer
                    (generate-new-buffer
                     " *ace-jump-helm-line-scroll*"))
                   (default-before
                    (default-value
                     'window-scroll-functions)))
               (unwind-protect
                   (progn
                     (ace-jump-helm-line--add-scroll-function)
                     (ace-jump-helm-line--add-scroll-function)
                     (with-current-buffer helm-buffer
                       (list
                        (local-variable-p
                         'window-scroll-functions)
                        (cl-count
                         'ace-jump-helm-line--scroll-function
                         window-scroll-functions)
                        (equal
                         (default-value
                          'window-scroll-functions)
                         default-before))))
                 (kill-buffer helm-buffer)))"##;
    let expect = expect!["OK (t 1 t)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_cleanup_overlays_runs_avy_done_in_helm_window() {
    let elisp_form = r##"(let* ((original-window
                      (selected-window))
                     (target-window
                      (split-window
                       original-window))
                     events)
               (unwind-protect
                   (cl-letf
                       (((symbol-function
                          'helm-window)
                         (lambda ()
                           target-window))
                        ((symbol-function
                          'avy--done)
                         (lambda ()
                           (push
                            (list
                             (eq
                              (selected-window)
                              target-window)
                             (eq
                              (selected-window)
                              original-window))
                            events)
                           'done-result)))
                     (list
                      (ace-jump-helm-line--cleanup-overlays)
                      events
                      (eq
                       (selected-window)
                       original-window)))
                 (delete-window target-window)))"##;
    let expect = expect!["OK (done-result ((t nil)) t)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_update_preview_is_a_noop_without_live_helm() {
    let elisp_form = r##"(let ((helm-alive-p nil)
                   (ace-jump-helm-line--tree-leafs 'outer)
                   events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (&rest _)
                       (push 'collect events))))
                 (list
                  (ace-jump-helm-line--update-line-overlays-maybe 9)
                  ace-jump-helm-line--tree-leafs
                  events)))"##;
    let expect = expect!["OK (nil outer nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_update_preview_without_start_uses_live_window_bounds() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-style 'post)
                   (ace-jump-helm-line-keys '(?x))
                   (ace-jump-helm-line-autoshow-use-linum nil)
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'window-start)
                     (lambda ()
                       (setq events
                             (append events
                                     '(window-start)))
                       4))
                    ((symbol-function
                      'window-end)
                     (lambda (window update)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'window-end
                                (eq
                                 window
                                 (selected-window))
                                update))))
                       12))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (start end)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'collect
                                start
                                end))))
                       nil))
                    ((symbol-function
                      'avy-tree)
                     (lambda (&rest _)
                       nil))
                    ((symbol-function
                      'avy-traverse)
                     (lambda (&rest _)
                       nil))
                    ((symbol-function
                      'avy--remove-leading-chars)
                     (lambda ()
                       (setq events
                             (append events
                                     '(remove))))))
                 (list
                  (ace-jump-helm-line--update-line-overlays-maybe)
                  events)))"##;
    let expect = expect!["OK (nil (window-start (window-end t t) (collect 4 12) remove))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_update_preview_builds_tree_removes_leading_chars_and_draws_each_leaf() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-style 'post)
                   (ace-jump-helm-line-keys '(?x ?y))
                   (ace-jump-helm-line-autoshow-use-linum nil)
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (start end)
                       (setq events
                             (append
                              events
                              (list
                               (list 'collect start end))))
                       '((10 . window-a)
                         (20 . window-b))))
                    ((symbol-function
                      'avy-tree)
                     (lambda (candidates keys)
                       (list 'tree candidates keys)))
                    ((symbol-function
                      'avy-traverse)
                     (lambda (tree callback)
                       (setq events
                             (append
                              events
                              (list
                               (list 'traverse tree))))
                       (funcall callback
                                '(?x)
                                '(10 . window-a))
                       (funcall callback
                                '(?y ?x)
                                '(20 . window-b))))
                    ((symbol-function
                      'avy--remove-leading-chars)
                     (lambda ()
                       (setq events
                             (append events
                                     '(remove)))))
                    ((symbol-function
                      'avy--style-fn)
                     (lambda (style)
                       (setq events
                             (append
                              events
                              (list
                               (list 'style style))))
                       (lambda (path leaf)
                         (setq events
                               (append
                                events
                                (list
                                 (list
                                  'draw
                                  path
                                  leaf))))))))
                 (list
                  (ace-jump-helm-line--update-line-overlays-maybe 7)
                  ace-jump-helm-line--tree-leafs
                  events)))"##;
    let expect = expect![
        "OK (nil ((#1=(121 120) . #2=(20 . window-b)) (#3=(120) . #4=(10 . window-a))) ((collect 7 nil) (traverse (tree ((10 . window-a) (20 . window-b)) (120 121))) remove (style post) (draw #1# #2#) (style post) (draw #3# #4#)))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_update_preview_builds_de_bruijn_leaf_paths() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-style 'de-bruijn)
                   (ace-jump-helm-line-keys '(?a ?b))
                   (ace-jump-helm-line-autoshow-use-linum t)
                   (helm-buffer
                    (current-buffer))
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (&rest _)
                       '((10 . window-a)
                         (20 . window-b)
                         (30 . window-c))))
                    ((symbol-function
                      'avy--path-alist-1)
                     (lambda (candidates length keys)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'paths
                                candidates
                                length
                                keys))))
                       '(((?a ?b) . (10 . window-a))
                         ((?b ?a) . (20 . window-b))
                         ((?a ?a) . (30 . window-c)))))
                    ((symbol-function
                      'linum-update)
                     (lambda (buffer)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'linum
                                (eq buffer helm-buffer))))))))
                 (list
                  (ace-jump-helm-line--update-line-overlays-maybe 1)
                  ace-jump-helm-line--tree-leafs
                  events)))"##;
    let expect = expect![
        "OK (#1=((paths ((10 . window-a) (20 . window-b) (30 . window-c)) 2 (97 98)) (linum t)) (((97 97) 30 . window-c) ((97 98) 20 . window-b) ((98 97) 10 . window-a)) #1#)"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_de_bruijn_retry_surfaces_upstream_unbound_lst_bug() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-style 'de-bruijn)
                   (ace-jump-helm-line-keys '(?a ?b))
                   (ace-jump-helm-line-autoshow-use-linum nil))
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (&rest _)
                       '((10 . window-a)
                         (20 . window-b))))
                    ((symbol-function
                      'avy--path-alist-1)
                     (lambda (&rest _)
                       nil)))
                 (ace-jump-helm-line--update-line-overlays-maybe 1)))"##;
    let expect = expect!["ERR (void-variable lst)"];
    assert_ace_jump_helm_line_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_linum_is_nil_without_live_helm() {
    let elisp_form = r##"(let ((helm-alive-p nil)
                   (ace-jump-helm-line--tree-leafs
                    '(((?a) 1 . window))))
               (ace-jump-helm-line--linum 1))"##;
    let expect = expect!["OK nil"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_linum_returns_empty_text_without_matching_leaf() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line--tree-leafs
                    '(((?a) 99 . window)))
                   (avy-highlight-first nil))
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window))))
                 (with-temp-buffer
                   (insert "one\ntwo\n")
                   (let ((value
                          (ace-jump-helm-line--linum 1)))
                     (list
                      (substring-no-properties value)
                      (get-text-property
                       0
                       'invisible
                       value)
                      (text-properties-at
                       0
                       value))))))"##;
    let expect = expect![[r#"OK ("" nil nil)"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_linum_reverses_matching_path_and_applies_lead_faces() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line--tree-leafs
                    '(((?a ?b) 1 . window)))
                   (avy-highlight-first nil))
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window))))
                 (with-temp-buffer
                   (insert "one\ntwo\n")
                   (let ((value
                          (ace-jump-helm-line--linum 1)))
                     (list
                      (substring-no-properties value)
                      (get-text-property
                       1
                       'face
                       value)
                      (get-text-property
                       2
                       'face
                       value))))))"##;
    let expect = expect![[r#"OK (" ba" avy-lead-face-0 avy-lead-face)"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_single_key_linum_keeps_lead_face_when_first_highlight_is_off() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line--tree-leafs
                    '(((?a) 1 . window)))
                   (avy-highlight-first nil))
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window))))
                 (with-temp-buffer
                   (insert "one\n")
                   (let ((value
                          (ace-jump-helm-line--linum 1)))
                     (list
                      (substring-no-properties value)
                      (get-text-property
                       2
                       'face
                       value))))))"##;
    let expect = expect![[r#"OK ("  a" avy-lead-face)"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_single_key_linum_uses_first_lead_face_when_highlight_is_on() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line--tree-leafs
                    '(((?a) 1 . window)))
                   (avy-highlight-first t))
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window))))
                 (with-temp-buffer
                   (insert "one\n")
                   (let ((value
                          (ace-jump-helm-line--linum 1)))
                     (list
                      (substring-no-properties value)
                      (get-text-property
                       2
                       'face
                       value))))))"##;
    let expect = expect![[r#"OK ("  a" avy-lead-face-0)"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_turn_on_linum_obeys_option_and_sets_buffer_local_format() {
    let elisp_form = r##"(let ((helm-buffer
                    (generate-new-buffer
                     " *ace-jump-helm-line-linum*"))
                   events)
               (unwind-protect
                   (cl-letf
                       (((symbol-function
                          'linum-mode)
                         (lambda (arg)
                           (push arg events)
                           'linum-result)))
                     (list
                      (let ((ace-jump-helm-line-autoshow-use-linum nil))
                        (turn-on-ace-jump-helm-line--linum))
                      (let ((ace-jump-helm-line-autoshow-use-linum t))
                        (turn-on-ace-jump-helm-line--linum))
                      (with-current-buffer helm-buffer
                        (list
                         (local-variable-p 'linum-format)
                         linum-format))
                      events))
                 (kill-buffer helm-buffer)))"##;
    let expect = expect!["OK (nil linum-result (t ace-jump-helm-line--linum) (1))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_autoshow_mode_adds_and_removes_exact_global_hooks() {
    let elisp_form = r##"(let (events)
               (let ((helm-after-preselection-hook
                      '(user-preselection))
                     (helm-move-selection-after-hook
                      '(user-move))
                     (helm-after-update-hook
                      '(user-update))
                     (helm-after-initialize-hook
                      '(user-initialize))
                     (ace-jump-helm-line-autoshow-mode-hook
                      (list
                       (lambda ()
                         (setq events
                               (append
                                events
                                (list
                                 ace-jump-helm-line-autoshow-mode)))))))
                 (setq-default
                  ace-jump-helm-line-autoshow-mode
                  nil)
                 (ace-jump-helm-line-autoshow-mode +1)
                 (let ((enabled
                        (list
                         ace-jump-helm-line-autoshow-mode
                         helm-after-preselection-hook
                         helm-move-selection-after-hook
                         helm-after-update-hook
                         helm-after-initialize-hook)))
                   (ace-jump-helm-line-autoshow-mode -1)
                   (list
                    enabled
                    ace-jump-helm-line-autoshow-mode
                    helm-after-preselection-hook
                    helm-move-selection-after-hook
                    helm-after-update-hook
                    helm-after-initialize-hook
                    events))))"##;
    let expect = expect![
        "OK ((t (ace-jump-helm-line--update-line-overlays-maybe . #1=(user-preselection)) (ace-jump-helm-line--update-line-overlays-maybe . #2=(user-move)) (user-update ace-jump-helm-line--update-line-overlays-maybe) (turn-on-ace-jump-helm-line--linum ace-jump-helm-line--add-scroll-function user-initialize)) nil #1# #2# (user-update) (user-initialize) (t nil))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
