use super::assert_ace_jump_mode_parity;
use expect_test::expect;

#[test]
fn ace_jump_mode_do_rejects_empty_singleton_and_noncharacter_move_keys() {
    let elisp_form = r##"(mapcar
         (lambda (keys)
           (condition-case error-data
               (let ((ace-jump-mode-move-keys keys))
                 (ace-jump-do "x")
                 'no-error)
             (error
              (cons
               (car error-data)
               (cdr error-data)))))
         '(nil (?a) (?a symbol) "ab"))"##;
    let expect = expect![[
        r#"OK ((error "[AceJump] Invalid move keys: check ace-jump-mode-move-keys") (error "[AceJump] Invalid move keys: check ace-jump-mode-move-keys") (error "[AceJump] Invalid move keys: check ace-jump-mode-move-keys") (error "[AceJump] No one found"))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_do_no_candidate_resets_mode_then_signals() {
    let elisp_form = r##"(let ((ace-jump-mode-move-keys '(?a ?b))
             (ace-jump-current-mode 'active))
         (cl-letf (((symbol-function
                     'ace-jump-list-visual-area)
                    (lambda () '(area)))
                   ((symbol-function
                     'ace-jump-search-candidate)
                    (lambda (&rest _arguments)
                      nil)))
           (list
            (condition-case error-data
                (ace-jump-do "missing")
              (error error-data))
            ace-jump-current-mode)))"##;
    let expect = expect![[r#"OK ((error "[AceJump] No one found") nil)"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_do_one_candidate_runs_exact_direct_jump_sequence() {
    let elisp_form = r##"(let ((ace-jump-mode-move-keys '(?a ?b))
             (ace-jump-current-mode 'word)
             (ace-jump-mode-before-jump-hook '(before))
             (ace-jump-mode-end-hook '(end))
             events)
         (every #'characterp ace-jump-mode-move-keys)
         (setq events nil)
         (cl-letf (((symbol-function
                     'ace-jump-list-visual-area)
                    (lambda ()
                      (setq events
                            (cons 'areas events))
                      '(area)))
                   ((symbol-function
                     'ace-jump-search-candidate)
                    (lambda (regexp areas)
                      (setq events
                            (cons
                             (list 'search regexp areas)
                             events))
                      '(only)))
                   ((symbol-function 'ace-jump-push-mark)
                    (lambda ()
                      (setq events
                            (cons 'push events))))
                   ((symbol-function 'run-hooks)
                    (lambda (&rest hooks)
                      (setq events
                            (cons
                             (cons 'hooks hooks)
                             events))))
                   ((symbol-function 'ace-jump-jump-to)
                    (lambda (position)
                      (setq events
                            (cons
                             (list 'jump position)
                             events))))
                   ((symbol-function 'message)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'message arguments)
                             events)))))
           (ace-jump-do "query"))
         (list
          (nreverse events)
          ace-jump-current-mode))"##;
    let expect = expect![[
        r#"OK ((areas (search "query" (area)) push (hooks ace-jump-mode-before-jump-hook) (jump only) (message "[AceJump] One candidate, move to it directly") (hooks ace-jump-mode-end-hook)) word)"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_do_multiple_candidates_configures_tree_keymap_hooks_and_word_label() {
    let elisp_form = r##"(let ((ace-jump-mode-move-keys '(?a ?b ?c))
             (ace-jump-mode-gray-background nil)
             (ace-jump-current-mode
              'ace-jump-word-mode)
             (mouse-leave-buffer-hook nil)
             (kbd-macro-termination-hook nil)
             (overriding-local-map nil)
             events)
         (cl-letf (((symbol-function
                     'ace-jump-list-visual-area)
                    (lambda () '(area)))
                   ((symbol-function
                     'ace-jump-search-candidate)
                    (lambda (&rest _arguments)
                      '(p1 p2 p3 p4)))
                   ((symbol-function
                     'ace-jump-populate-overlay-to-search-tree)
                    (lambda (tree positions)
                      (setq events
                            (cons
                             (list
                              'populate
                              (copy-tree tree)
                              positions)
                             events))
                      tree))
                   ((symbol-function
                     'ace-jump-update-overlay-in-search-tree)
                    (lambda (tree keys)
                      (setq events
                            (cons
                             (list
                              'update
                              (copy-tree tree)
                              keys)
                             events))))
                   ((symbol-function 'force-mode-line-update)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'force arguments)
                             events)))))
           (ace-jump-do "query"))
         (list
          (nreverse events)
          ace-jump-mode
          ace-jump-search-tree
          (mapcar
           (lambda (key)
             (lookup-key
              overriding-local-map
              key))
           '("a" "b" "c" "\C-c\C-c" "z" [t]))
          mouse-leave-buffer-hook
          kbd-macro-termination-hook))"##;
    let expect = expect![[
        r#"OK (((force) (force) (force) (force) (populate (branch (branch (leaf) (leaf)) (leaf) (leaf)) (p1 p2 p3 p4)) (update (branch (branch (leaf) (leaf)) (leaf) (leaf)) (97 98 99)) (force)) " AceJump - Word" (branch (branch (leaf) (leaf)) (leaf) (leaf)) (ace-jump-move ace-jump-move ace-jump-move ace-jump-quick-exchange nil ace-jump-done) (ace-jump-done) (ace-jump-done))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_do_multiple_candidates_sets_each_mode_line_label() {
    let elisp_form = r##"(mapcar
         (lambda (mode)
           (let ((ace-jump-current-mode mode)
                 (ace-jump-mode-move-keys '(?a ?b))
                 (ace-jump-mode-gray-background nil)
                 (mouse-leave-buffer-hook nil)
                 (kbd-macro-termination-hook nil)
                 ace-jump-mode)
             (cl-letf (((symbol-function
                         'ace-jump-list-visual-area)
                        (lambda () '(area)))
                       ((symbol-function
                         'ace-jump-search-candidate)
                        (lambda (&rest _arguments)
                          '(p1 p2)))
                       ((symbol-function
                         'ace-jump-populate-overlay-to-search-tree)
                        (lambda (tree _positions)
                          tree))
                       ((symbol-function
                         'ace-jump-update-overlay-in-search-tree)
                        (lambda (&rest _arguments) nil))
                       ((symbol-function
                         'force-mode-line-update)
                        (lambda (&rest _arguments) nil)))
               (ace-jump-do "x"))
             (list mode ace-jump-mode)))
         '(ace-jump-char-mode
           ace-jump-word-mode
           ace-jump-line-mode
           other
           nil))"##;
    let expect = expect![[
        r#"OK ((ace-jump-char-mode " AceJump - Char") (ace-jump-word-mode " AceJump - Word") (ace-jump-line-mode " AceJump - Line") (other " AceJump") (nil " AceJump"))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_do_gray_background_creates_per_area_overlays() {
    let elisp_form = r##"(let* ((areas
                (list
                 (make-aj-visual-area
                  :buffer 'b1 :window 'w1 :frame 'f1)
                 (make-aj-visual-area
                  :buffer 'b2 :window 'w2 :frame 'f2)))
               (ace-jump-mode-move-keys '(?a ?b))
               (ace-jump-mode-gray-background t)
               (ace-jump-current-mode nil)
               (mouse-leave-buffer-hook nil)
               (kbd-macro-termination-hook nil)
               events)
         (cl-letf (((symbol-function
                     'ace-jump-list-visual-area)
                    (lambda () areas))
                   ((symbol-function
                     'ace-jump-search-candidate)
                    (lambda (&rest _arguments)
                      '(p1 p2)))
                   ((symbol-function 'window-start)
                    (lambda (window)
                      (if (eq window 'w1) 10 20)))
                   ((symbol-function 'window-end)
                    (lambda (window &optional _update)
                      (if (eq window 'w1) 19 29)))
                   ((symbol-function 'make-overlay)
                    (lambda (&rest arguments)
                      (let ((overlay
                             (cons 'overlay arguments)))
                        (setq events
                              (cons
                               (list 'make arguments)
                               events))
                        overlay)))
                   ((symbol-function 'overlay-put)
                    (lambda (overlay property value)
                      (setq events
                            (cons
                             (list
                              'put
                              overlay
                              property
                              value)
                             events))))
                   ((symbol-function
                     'ace-jump-populate-overlay-to-search-tree)
                    (lambda (tree _positions)
                      tree))
                   ((symbol-function
                     'ace-jump-update-overlay-in-search-tree)
                    (lambda (&rest _arguments) nil))
                   ((symbol-function 'force-mode-line-update)
                    (lambda (&rest _arguments) nil)))
           (ace-jump-do "x"))
         (list
          (nreverse events)
          ace-jump-background-overlay-list))"##;
    let expect = expect![
        "OK (((make #1=(10 19 b1)) (put #3=(overlay . #1#) face ace-jump-face-background) (make #2=(20 29 b2)) (put #4=(overlay . #2#) face ace-jump-face-background)) (#3# #4#))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_done_resets_state_deletes_resources_and_removes_hooks() {
    let elisp_form = r##"(let ((ace-jump-query-char ?x)
             (ace-jump-current-mode 'active)
             (ace-jump-mode " label")
             (ace-jump-background-overlay-list
              '(background-one background-two))
             (ace-jump-search-tree
              '(branch (leaf . target)))
             (overriding-local-map 'map)
             (mouse-leave-buffer-hook
              '(other ace-jump-done))
             (kbd-macro-termination-hook
              '(ace-jump-done other))
             events)
         (cl-letf (((symbol-function 'force-mode-line-update)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'force arguments)
                             events))))
                   ((symbol-function 'delete-overlay)
                    (lambda (overlay)
                      (setq events
                            (cons
                             (list 'delete overlay)
                             events))))
                   ((symbol-function
                     'ace-jump-delete-overlay-in-search-tree)
                    (lambda (tree)
                      (setq events
                            (cons
                             (list 'tree tree)
                             events)))))
           (ace-jump-done))
         (list
          (nreverse events)
          ace-jump-query-char
          ace-jump-current-mode
          ace-jump-mode
          ace-jump-background-overlay-list
          ace-jump-search-tree
          overriding-local-map
          mouse-leave-buffer-hook
          kbd-macro-termination-hook))"##;
    let expect = expect![[
        r#"OK (((force) (delete background-one) (delete background-two) (tree (branch (leaf . target)))) nil nil nil nil nil nil (other) (other))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_jump_to_moves_within_current_buffer() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-target*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "abcdef")
                 (goto-char 2)
                 (let* ((area
                         (make-aj-visual-area
                          :buffer buffer
                          :window (selected-window)
                          :frame (selected-frame)))
                        (position
                         (make-aj-position
                          :offset 5
                          :visual-area area))
                        (ace-jump-current-mode
                         'ace-jump-char-mode))
                   (ace-jump-jump-to position)
                   (list
                    (point)
                    (eq
                     (current-buffer)
                     buffer)
                    (eq
                     (selected-window)
                     (aj-position-window position))))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK (5 t t)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_line_jump_preserves_original_column() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-line-target*")))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "abcdef\nuvwxyz\n")
                 (goto-char 5)
                 (let* ((area
                         (make-aj-visual-area
                          :buffer buffer
                          :window (selected-window)
                          :frame (selected-frame)))
                        (position
                         (make-aj-position
                          :offset 8
                          :visual-area area))
                        (ace-jump-current-mode
                         'ace-jump-line-mode))
                   (ace-jump-jump-to position)
                   (list
                    (line-number-at-pos)
                    (current-column)
                    (point)))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK (2 4 12)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_jump_to_focuses_frame_selects_window_switches_buffer_then_moves() {
    let elisp_form = r##"(let* ((area
                (make-aj-visual-area
                 :buffer 'target-buffer
                 :window 'target-window
                 :frame 'target-frame))
               (position
                (make-aj-position
                 :offset 9
                 :visual-area area))
               (ace-jump-current-mode
                'ace-jump-char-mode)
               (current-frame 'current-frame)
               (current-window 'current-window)
               (current-buffer-value 'current-buffer)
               events)
         (cl-letf (((symbol-function 'frame-live-p)
                    (lambda (frame)
                      (eq frame 'target-frame)))
                   ((symbol-function 'selected-frame)
                    (lambda () current-frame))
                   ((symbol-function
                     'select-frame-set-input-focus)
                    (lambda (frame)
                      (setq events
                            (cons
                             (list 'focus frame)
                             events))
                      (setq current-frame frame)))
                   ((symbol-function 'window-frame)
                    (lambda (_window)
                      'target-frame))
                   ((symbol-function 'window-live-p)
                    (lambda (window)
                      (eq window 'target-window)))
                   ((symbol-function 'selected-window)
                    (lambda () current-window))
                   ((symbol-function 'select-window)
                    (lambda (window &optional no-record)
                      (setq events
                            (cons
                             (list
                              'select-window
                              window
                              no-record)
                             events))
                      (setq current-window window)))
                   ((symbol-function 'buffer-live-p)
                    (lambda (buffer)
                      (eq buffer 'target-buffer)))
                   ((symbol-function 'window-buffer)
                    (lambda (_window)
                      'other-buffer))
                   ((symbol-function 'switch-to-buffer)
                    (lambda (buffer &rest arguments)
                      (setq events
                            (cons
                             (list
                              'switch
                              buffer
                              arguments)
                             events))
                      (setq current-buffer-value buffer)))
                   ((symbol-function 'current-buffer)
                    (lambda ()
                      current-buffer-value))
                   ((symbol-function 'goto-char)
                    (lambda (offset)
                      (setq events
                            (cons
                             (list 'goto offset)
                             events)))))
           (ace-jump-jump-to position))
         (list
          (nreverse events)
          current-frame
          current-window
          current-buffer-value))"##;
    let expect = expect![
        "OK (((focus target-frame) (select-window target-window nil) (switch target-buffer nil) (goto 9)) target-frame target-window target-buffer)"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_jump_to_skips_dead_frame_window_and_buffer_targets() {
    let elisp_form = r##"(let* ((area
                (make-aj-visual-area
                 :buffer 'dead-buffer
                 :window 'dead-window
                 :frame 'dead-frame))
               (position
                (make-aj-position
                 :offset 9
                 :visual-area area))
               (ace-jump-current-mode
                'ace-jump-char-mode)
               events)
         (cl-letf (((symbol-function 'frame-live-p)
                    (lambda (frame)
                      (setq events
                            (cons
                             (list 'frame-live frame)
                             events))
                      nil))
                   ((symbol-function 'selected-frame)
                    (lambda () 'current-frame))
                   ((symbol-function
                     'select-frame-set-input-focus)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'focus arguments)
                             events))))
                   ((symbol-function 'window-live-p)
                    (lambda (window)
                      (setq events
                            (cons
                             (list 'window-live window)
                             events))
                      nil))
                   ((symbol-function 'selected-window)
                    (lambda () 'current-window))
                   ((symbol-function 'select-window)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons
                              'select-window
                              arguments)
                             events))))
                   ((symbol-function 'buffer-live-p)
                    (lambda (buffer)
                      (setq events
                            (cons
                             (list 'buffer-live buffer)
                             events))
                      nil))
                   ((symbol-function 'switch-to-buffer)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'switch arguments)
                             events))))
                   ((symbol-function 'goto-char)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'goto arguments)
                             events)))))
           (ace-jump-jump-to position))
         (nreverse events))"##;
    let expect = expect![
        "OK ((frame-live dead-frame) (window-live dead-window) (buffer-live dead-buffer) (buffer-live dead-buffer))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
