use expect_test::expect;

use super::{assert_all_ext_parity, assert_all_ext_signal};

#[test]
fn all_ext_real_conversion_surfaces_the_upstream_missing_kill_helper() {
    let elisp_form = r##"(let ((source
                           (generate-new-buffer
                            "all-ext-broken-source"))
                          (candidates
                           (generate-new-buffer
                            " *all-ext-broken-candidates*")))
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert "alpha\n"))
                            (with-current-buffer candidates
                              (insert
                               "Results\n"
                               "all-ext-broken-source:1:alpha\n"))
                            (all-from-anything-occur-internal
                             "helm-occur" candidates source))
                        (when (get-buffer "*All*")
                          (kill-buffer "*All*"))
                        (kill-buffer source)
                        (kill-buffer candidates)))"##;
    let expect = expect!["ERR (void-function kill-All-buffer-maybe)"];
    assert_all_ext_signal(elisp_form, expect);
}

#[test]
fn all_ext_helm_swoop_match_face_path_signals_on_removed_return_function() {
    let elisp_form = r##"(let ((source
                           (generate-new-buffer "orders.ex"))
                          (candidates
                           (generate-new-buffer
                            " *all-ext-helm-results*")))
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert
                               "defmodule Orders do\n"
                               "  def create(id), do: id\n"
                               "  def cancel(id), do: id\n"
                               "end\n"))
                            (with-current-buffer candidates
                              (insert
                               "Helm Swoop\n"
                               "orders.ex:2:  def create(id), do: id\n"
                               "orders.ex:3:  def cancel(id), do: id\n")
                              (let
                                  ((start
                                    (progn
                                      (goto-char (point-min))
                                      (forward-line 1)
                                      (search-forward "create")
                                      (match-beginning 0))))
                                (put-text-property
                                 start (+ start 6)
                                 'face
                                 'helm-swoop-target-word-face)))
                            (cl-letf
                                (((symbol-function
                                   'kill-All-buffer-maybe)
                                  (lambda (&rest _)
                                    (when (get-buffer "*All*")
                                      (kill-buffer "*All*")))))
                              (let
                                  ((all-from-occur-select-window-flag
                                    nil))
                                (all-from-anything-occur-internal
                                 "helm-swoop" candidates source)))
                            (with-current-buffer "*All*"
                              (list
                               (buffer-string)
                               major-mode
                               (eq all-buffer source)
                               (eq buffer-undo-list t)
                               (mapcar
                                (lambda (overlay)
                                  (let ((marker
                                         (overlay-get
                                          overlay 'all-marker)))
                                    (when marker
                                      (list
                                       (buffer-substring
                                        (overlay-start overlay)
                                        (overlay-end overlay))
                                       (marker-position marker)))))
                                (seq-filter
                                 (lambda (overlay)
                                   (overlay-get
                                    overlay 'all-marker))
                                 (overlays-in
                                  (point-min) (point-max))))
                               (let ((position
                                      (text-property-any
                                       (point-min) (point-max)
                                       'face 'match)))
                                 (and
                                  position
                                  (buffer-substring
                                   position (+ position 1)))))))
                        (when (get-buffer "*All*")
                          (kill-buffer "*All*"))
                        (kill-buffer source)
                        (kill-buffer candidates)))"##;
    let expect = expect!["ERR (void-function return)"];
    assert_all_ext_signal(elisp_form, expect);
}

#[test]
fn all_ext_conversion_accepts_space_and_colon_formats_but_rejects_unrelated_rows() {
    let elisp_form = r##"(let ((source
                           (generate-new-buffer "ledger.txt"))
                          (candidates
                           (generate-new-buffer
                            " *all-ext-formats*")))
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert
                               "zero\none\ntwo\nthree\nfour\n"))
                            (with-current-buffer candidates
                              (insert
                               "Results\n"
                               " 1 zero\n"
                               "  2:one\n"
                               "ledger.txt:3:two\n"
                               "ledger.txt:4 three\n"
                               "other.txt:5:must-not-match\n"
                               "not a candidate\n"))
                            (cl-letf
                                (((symbol-function
                                   'kill-All-buffer-maybe)
                                  (lambda (&rest _)
                                    (when (get-buffer "*All*")
                                      (kill-buffer "*All*")))))
                              (let
                                  ((all-from-occur-select-window-flag
                                    nil))
                                (all-from-anything-occur-internal
                                 "mixed" candidates source)))
                            (with-current-buffer "*All*"
                              (list
                               (buffer-string)
                               (mapcar
                                (lambda (overlay)
                                  (marker-position
                                   (overlay-get
                                    overlay 'all-marker)))
                                (seq-filter
                                 (lambda (overlay)
                                   (overlay-get
                                    overlay 'all-marker))
                                 (overlays-in
                                  (point-min) (point-max)))))))
                        (when (get-buffer "*All*")
                          (kill-buffer "*All*"))
                        (kill-buffer source)
                        (kill-buffer candidates)))"##;
    let expect = expect![[r#"OK ("From mixed\n--------\nzero\none\ntwo\nthree\n" (1 6 10 14))"#]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_marked_candidates_filter_helm_results_and_prefer_anything_marks() {
    let elisp_form = r##"(let ((source
                           (generate-new-buffer "notes.txt"))
                          (candidates
                           (generate-new-buffer
                            " *all-ext-marked*"))
                          helm-marks
                          anything-marks
                          helm-output
                          anything-output)
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert
                               "alpha\nbeta\ngamma\ndelta\n"))
                            (with-current-buffer candidates
                              (insert
                               "Results\n"
                               "notes.txt:1:alpha\n"
                               "notes.txt:2:beta\n"
                               "notes.txt:3:gamma\n"
                               "notes.txt:4:delta\n")
                              (dolist
                                  (needle
                                   '("notes.txt:2:beta\n"
                                     "notes.txt:4:delta\n"))
                                (goto-char (point-min))
                                (search-forward needle)
                                (let ((overlay
                                       (make-overlay
                                        (match-beginning 0)
                                        (match-end 0))))
                                  (overlay-put
                                   overlay 'string needle)
                                  (push
                                   overlay
                                   helm-marks)))
                              (goto-char (point-min))
                              (search-forward
                               "notes.txt:3:gamma\n")
                              (let ((overlay
                                     (make-overlay
                                      (match-beginning 0)
                                      (match-end 0))))
                                (overlay-put
                                 overlay 'string
                                 "notes.txt:3:gamma\n")
                                (push overlay anything-marks)))
                            (cl-letf
                                (((symbol-function
                                   'kill-All-buffer-maybe)
                                  (lambda (&rest _)
                                    (when (get-buffer "*All*")
                                      (kill-buffer "*All*")))))
                              (cl-progv
                                  '(helm-visible-mark-overlays)
                                  (list helm-marks)
                                (let
                                    ((all-from-occur-select-window-flag
                                      nil))
                                  (all-from-anything-occur-internal
                                   "helm-occur" candidates source)))
                              (setq
                               helm-output
                               (with-current-buffer "*All*"
                                 (list
                                  (buffer-string)
                                  (mapcar
                                   (lambda (overlay)
                                     (marker-position
                                      (overlay-get
                                       overlay 'all-marker)))
                                   (seq-filter
                                    (lambda (overlay)
                                      (overlay-get
                                       overlay 'all-marker))
                                    (overlays-in
                                     (point-min) (point-max)))))))
                              (cl-progv
                                  '(anything-visible-mark-overlays
                                    helm-visible-mark-overlays)
                                  (list anything-marks helm-marks)
                                (let
                                    ((all-from-occur-select-window-flag
                                      nil))
                                  (all-from-anything-occur-internal
                                   "anything-occur"
                                   candidates source)))
                              (setq
                               anything-output
                               (with-current-buffer "*All*"
                                 (list
                                  (buffer-string)
                                  (mapcar
                                   (lambda (overlay)
                                     (marker-position
                                      (overlay-get
                                       overlay 'all-marker)))
                                   (seq-filter
                                    (lambda (overlay)
                                      (overlay-get
                                       overlay 'all-marker))
                                    (overlays-in
                                     (point-min) (point-max))))))))
                            (list helm-output anything-output))
                        (when (get-buffer "*All*")
                          (kill-buffer "*All*"))
                        (kill-buffer source)
                        (kill-buffer candidates)))"##;
    let expect = expect![[
        r#"OK (("From helm-occur\n--------\nbeta\ndelta\n" (7 18)) ("From anything-occur\n--------\ngamma\n" (12)))"#
    ]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_low_level_insert_adds_match_face_line_number_and_live_source_marker() {
    let elisp_form = r##"(let ((source
                           (generate-new-buffer
                            "all-ext-insert-source"))
                          (output
                           (generate-new-buffer
                            " *all-ext-insert-output*")))
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert
                               "first source line\n"
                               "second source line\n")
                              (goto-char (point-min))
                              (with-current-buffer output
                                (setq all-buffer source))
                              (let ((standard-output output))
                                (all-from-anything-occur-insert
                                 (line-beginning-position 2)
                                 (point-max)
                                 2
                                 "second source line"
                                 7)))
                            (with-current-buffer output
                              (list
                               (buffer-string)
                               (let ((position
                                      (text-property-any
                                       (point-min) (point-max)
                                       'face 'match)))
                                 (and
                                  position
                                  (list
                                   position
                                   (buffer-substring
                                    position (+ position 1)))))
                               (mapcar
                                (lambda (overlay)
                                  (list
                                   (overlay-start overlay)
                                   (overlay-end overlay)
                                   (let
                                       ((marker
                                         (overlay-get
                                          overlay 'all-marker)))
                                     (and
                                      marker
                                      (list
                                       (buffer-name
                                        (marker-buffer marker))
                                       (marker-position marker))))
                                   (overlay-get
                                    overlay 'before-string)))
                                (sort
                                 (overlays-in
                                  (point-min) (point-max))
                                 (lambda (left right)
                                   (<
                                    (overlay-start left)
                                    (overlay-start right))))))))
                        (kill-buffer source)
                        (kill-buffer output)))"##;
    let expect = expect![[
        r#"OK (#("second source line\n" 7 8 (face match)) (8 #("s" 0 1 (face match))) ((1 20 ("all-ext-insert-source" 19) nil) (1 1 nil #("2 " 0 1 (face linum) 1 2 (display ((margin left-margin)))))))"#
    ]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_replacing_existing_all_buffer_starts_clean_and_remains_undoable() {
    let elisp_form = r##"(let ((source
                           (generate-new-buffer "replace.txt"))
                          (first
                           (generate-new-buffer
                            " *all-ext-first*"))
                          (second
                           (generate-new-buffer
                            " *all-ext-second*"))
                          old-all-buffer)
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert "alpha\nbeta\n"))
                            (with-current-buffer first
                              (insert "One\nreplace.txt:1:alpha\n"))
                            (with-current-buffer second
                              (insert "Two\nreplace.txt:2:beta\n"))
                            (cl-letf
                                (((symbol-function
                                   'kill-All-buffer-maybe)
                                  (lambda (&rest _)
                                    (when (get-buffer "*All*")
                                      (kill-buffer "*All*")))))
                              (let
                                  ((all-from-occur-select-window-flag
                                    nil))
                                (all-from-anything-occur-internal
                                 "first" first source))
                              (setq old-all-buffer
                                    (get-buffer "*All*"))
                              (let
                                  ((all-from-occur-select-window-flag
                                    nil))
                                (all-from-anything-occur-internal
                                 "second" second source)))
                            (with-current-buffer "*All*"
                              (list
                               (buffer-string)
                               (buffer-live-p old-all-buffer)
                               (eq
                                old-all-buffer
                                (current-buffer))
                               (not (eq buffer-undo-list t))
                               (eq all-buffer source))))
                        (when (get-buffer "*All*")
                          (kill-buffer "*All*"))
                        (kill-buffer source)
                        (kill-buffer first)
                        (kill-buffer second)))"##;
    let expect = expect![[r#"OK ("From second\n--------\nbeta\n" nil nil t t)"#]];
    assert_all_ext_parity(elisp_form, expect);
}
