use expect_test::expect;

use super::assert_game_2048_batch;

#[test]
fn moves_public_surface_batch() {
    assert_game_2048_batch(&[
        (
            "game_2048_move_slides_through_empty_cells_to_the_edge",
            r##"(let ((*2048-columns* 4)
                     (*2048-rows* 1)
                     (*2048-board*
                      (vector 0 0 0 2))
                     (*2048-hi-tile* 2)
                     (*2048-score* 0)
                     (*2048-combines-this-move*
                      (make-vector 4 nil)))
               (list
                (2048-move 0 3 0 -1)
                *2048-board*
                *2048-score*
                *2048-combines-this-move*))"##,
            true,
            expect!["OK (t [2 0 0 0] 0 [nil nil nil nil])"],
        ),
        (
            "game_2048_move_combines_once_updates_score_and_marks_destination",
            r##"(let ((*2048-columns* 4)
                     (*2048-rows* 1)
                     (*2048-board*
                      (vector 2 2 4 0))
                     (*2048-hi-tile* 4)
                     (*2048-score* 10)
                     (*2048-combines-this-move*
                      (make-vector 4 nil))
                     first-board
                     first-flags)
               (2048-init-tile 8)
               (list
                (2048-move 0 1 0 -1)
                (progn
                  (setq first-board
                        (copy-sequence
                         *2048-board*))
                  first-board)
                *2048-score*
                *2048-hi-tile*
                (progn
                  (setq first-flags
                        (copy-sequence
                         *2048-combines-this-move*))
                  first-flags)
                (2048-move 0 2 0 -1)
                (copy-sequence *2048-board*)
                *2048-score*
                (copy-sequence
                 *2048-combines-this-move*)))"##,
            true,
            expect!["OK (t [4 0 4 0] 14 4 [t nil nil nil] t [4 4 0 0] 14 [t nil nil nil])"],
        ),
        (
            "game_2048_move_rejects_blocked_and_out_of_bounds_destinations",
            r##"(let ((*2048-columns* 2)
                     (*2048-rows* 1)
                     (*2048-board*
                      (vector 2 4))
                     (*2048-hi-tile* 4)
                     (*2048-score* 0)
                     (*2048-combines-this-move*
                      (make-vector 2 nil)))
               (list
                (2048-move 0 0 0 1)
                (2048-move 0 1 0 1)
                *2048-board*
                *2048-score*))"##,
            true,
            expect!["OK (nil nil [2 4] 0)"],
        ),
        (
            "game_2048_left_and_right_merge_pairs_without_double_combining",
            r##"(let ((*2048-columns* 4)
                     (*2048-rows* 1)
                     events)
               (cl-letf (((symbol-function
                           '2048-insert-random-cell)
                          (lambda ()
                            (push 'random events)))
                         ((symbol-function
                           '2048-print-board)
                          (lambda ()
                            (push 'print events)))
                         ((symbol-function
                           '2048-check-game-end)
                          (lambda ()
                            (push 'check events))))
                 (let ((*2048-board*
                        (vector 2 2 2 2))
                       (*2048-hi-tile* 2)
                       (*2048-score* 0)
                       left-board
                       left-score)
                   (2048-left)
                   (setq left-board
                         (copy-sequence
                          *2048-board*)
                         left-score
                         *2048-score*
                         *2048-board*
                         (vector 2 2 2 2)
                         *2048-hi-tile* 2
                         *2048-score* 0)
                   (2048-right)
                   (list
                    left-board
                    left-score
                    *2048-board*
                    *2048-score*
                    (nreverse events)))))"##,
            true,
            expect!["OK ([4 4 0 0] 8 [0 0 4 4] 8 (random print check random print check))"],
        ),
        (
            "game_2048_up_and_down_process_columns_in_directional_order",
            r##"(let ((*2048-columns* 1)
                     (*2048-rows* 4))
               (cl-letf (((symbol-function
                           '2048-insert-random-cell)
                          #'ignore)
                         ((symbol-function
                           '2048-print-board)
                          #'ignore)
                         ((symbol-function
                           '2048-check-game-end)
                          #'ignore))
                 (let ((*2048-board*
                        (vector 2 2 2 2))
                       (*2048-hi-tile* 2)
                       (*2048-score* 0)
                       up-board
                       up-score)
                   (2048-up)
                   (setq up-board
                         (copy-sequence
                          *2048-board*)
                         up-score *2048-score*
                         *2048-board*
                         (vector 2 2 2 2)
                         *2048-hi-tile* 2
                         *2048-score* 0)
                   (2048-down)
                   (list
                    (append up-board nil)
                    up-score
                    (append *2048-board*
                            nil)
                    *2048-score*))))"##,
            true,
            expect!["OK ((4 4 0 0) 8 (0 0 4 4) 8)"],
        ),
        (
            "game_2048_directional_noop_does_not_insert_a_random_tile",
            r##"(let ((*2048-columns* 2)
                     (*2048-rows* 1)
                     (*2048-board*
                      (vector 2 4))
                     (*2048-hi-tile* 4)
                     (*2048-score* 0)
                     random-called)
               (cl-letf (((symbol-function
                           '2048-insert-random-cell)
                          (lambda ()
                            (setq random-called t)))
                         ((symbol-function
                           '2048-print-board)
                          #'ignore)
                         ((symbol-function
                           '2048-check-game-end)
                          #'ignore))
                 (2048-left)
                 (list
                  *2048-board*
                  random-called)))"##,
            true,
            expect!["OK ([2 4] nil)"],
        ),
        (
            "game_2048_move_macro_resets_flags_then_prints_and_checks",
            r##"(let ((*2048-columns* 2)
                     (*2048-rows* 2)
                     (*2048-combines-this-move*
                      (vector t t t t))
                     events)
               (cl-letf (((symbol-function
                           '2048-print-board)
                          (lambda ()
                            (push 'print events)))
                         ((symbol-function
                           '2048-check-game-end)
                          (lambda ()
                            (push 'check events)
                            'checked)))
                 (list
                  (2048-game-move
                   (push
                    (copy-sequence
                     *2048-combines-this-move*)
                    events)
                   (push 'body events))
                  *2048-combines-this-move*
                  (nreverse events))))"##,
            true,
            expect!["OK (checked [nil nil nil nil] ([nil nil nil nil] body print check))"],
        ),
        (
            "game_2048_random_move_maps_each_random_index_to_one_direction",
            r##"(let (events)
               (cl-letf (((symbol-function '2048-left)
                          (lambda ()
                            (push 'left events)))
                         ((symbol-function '2048-right)
                          (lambda ()
                            (push 'right events)))
                         ((symbol-function '2048-up)
                          (lambda ()
                            (push 'up events)))
                         ((symbol-function '2048-down)
                          (lambda ()
                            (push 'down events)))
                         ((symbol-function 'random)
                          (let ((values '(0 1 2 3)))
                            (lambda (limit)
                              (unless (= limit 4)
                                (error
                                 "unexpected limit"))
                              (prog1
                                  (car values)
                                (setq values
                                      (cdr values)))))))
                 (dotimes (_ 4)
                   (2048-random-move))
                 (nreverse events)))"##,
            true,
            expect!["OK (left right up down)"],
        ),
    ]);
}
