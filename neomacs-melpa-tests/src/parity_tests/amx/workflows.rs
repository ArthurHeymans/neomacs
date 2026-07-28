use expect_test::expect;

use super::assert_amx_parity;

/// Turning on `amx-mode' is the whole installation: it initializes the ranking
/// cache from the (empty) save file, remaps `execute-extended-command' so `M-x'
/// runs amx, and arranges for the ranking to be written out on auto-save and at
/// exit.  A freshly built cache has no usage data yet, so the five probe
/// commands are ordered by the two remaining rules -- shortest name first, ties
/// alphabetically.  Turning the mode off gives `M-x' back and removes the
/// auto-save hook.
#[test]
fn enabling_amx_mode_takes_over_m_x_and_ranks_the_commands_it_finds() {
    let elisp_form = r##"(unwind-protect
    (progn
      (amx-test-setup)
      (let ((before (list :m-x (key-binding (kbd "M-x"))
                          :remap (global-key-binding
                                  [remap execute-extended-command])
                          :initialized amx-initialized
                          :auto-save (and (memq 'amx-save-to-file auto-save-hook) t))))
        (amx-mode 1)
        (let ((enabled (list :m-x (key-binding (kbd "M-x"))
                             :remap (global-key-binding
                                     [remap execute-extended-command])
                             :initialized amx-initialized
                             :auto-save (and (memq 'amx-save-to-file auto-save-hook) t)
                             :kill-emacs (and (memq 'amx-save-to-file kill-emacs-hook) t)
                             :order (amx-test-order)
                             :data (copy-tree amx-data)
                             :history (copy-sequence amx-history)
                             :save-file-written (amx-test-read-save-file))))
          (amx-mode 0)
          (list :before before
                :enabled enabled
                :disabled (list :m-x (key-binding (kbd "M-x"))
                                :remap (global-key-binding
                                        [remap execute-extended-command])
                                :auto-save (and (memq 'amx-save-to-file auto-save-hook) t)
                                :still-initialized amx-initialized)))))
  (amx-test-cleanup))"##;

    let expect = expect![
        "OK (:before (:m-x execute-extended-command :remap 1 :initialized nil :auto-save nil) :enabled (:m-x amx :remap amx :initialized t :auto-save t :kill-emacs t :order ((amx-probe-open) (amx-probe-quit) (amx-probe-zoom) (amx-probe-close) (amx-probe-refresh)) :data nil :history nil :save-file-written no-save-file) :disabled (:m-x execute-extended-command :remap nil :auto-save nil :still-initialized t))"
    ];

    assert_amx_parity(elisp_form, expect);
}

/// The point of the package: commands the user runs often rise to the top.
/// After a session of six invocations the cache is ordered by run count, the
/// counts are recorded in `amx-data', and the most recent three commands are
/// remembered in `amx-history'.  `amx-sort-according-to-cache' -- what the
/// major-mode command list is filtered through -- reports any subset in the
/// same ranked order, and a command run once more moves ahead of one with the
/// same count.
#[test]
fn a_session_of_command_invocations_reorders_the_ranked_list() {
    let elisp_form = r##"(unwind-protect
    (progn
      (amx-test-setup)
      (amx-mode 1)
      (let ((initial (amx-test-order)))
        (amx-test-run 'amx-probe-zoom 'amx-probe-zoom 'amx-probe-close
                      'amx-probe-zoom 'amx-probe-close 'amx-probe-open)
        (amx-save-history)
        (let ((ranked (list :order (amx-test-order)
                            :data (copy-tree amx-data)
                            :history (copy-sequence amx-history)
                            :subset (amx-sort-according-to-cache
                                     (list 'amx-probe-refresh 'amx-probe-zoom
                                           'amx-probe-open 'amx-probe-quit))
                            :default (amx-get-default amx-cache))))
          (amx-test-run 'amx-probe-open 'amx-probe-open)
          (amx-save-history)
          (list :initial initial
                :ranked ranked
                :after-more-use (list :order (amx-test-order)
                                      :data (copy-tree amx-data)
                                      :history (copy-sequence amx-history))))))
  (amx-test-cleanup))"##;

    let expect = expect![[
        r#"OK (:initial ((amx-probe-open) (amx-probe-quit) (amx-probe-zoom) (amx-probe-close) (amx-probe-refresh)) :ranked (:order ((amx-probe-open . 1) (amx-probe-close . 2) (amx-probe-zoom . 3) (amx-probe-quit) (amx-probe-refresh)) :data ((amx-probe-zoom . 3) (amx-probe-close . 2) (amx-probe-open . 1)) :history (amx-probe-open amx-probe-close amx-probe-zoom) :subset (amx-probe-open amx-probe-zoom amx-probe-quit amx-probe-refresh) :default "amx-probe-open") :after-more-use (:order ((amx-probe-open . 3) (amx-probe-close . 2) (amx-probe-zoom . 3) (amx-probe-quit) (amx-probe-refresh)) :data ((amx-probe-zoom . 3) (amx-probe-close . 2) (amx-probe-open . 3)) :history (amx-probe-open amx-probe-close amx-probe-zoom)))"#
    ]];

    assert_amx_parity(elisp_form, expect);
}

/// The ranking is meant to survive a restart.  Under `emacs -Q' amx refuses to
/// write anything and says so in a warning -- which is exactly the state a
/// batch editor is in -- so the workflow first pins that refusal, then saves as
/// a configured editor would, pins the file's exact contents, throws away
/// everything amx knows, and re-initializes: the counts, the history and the
/// resulting order all come back from disk.
#[test]
fn the_save_file_round_trips_the_ranking_into_a_fresh_session() {
    let elisp_form = r##"(unwind-protect
    (progn
      (amx-test-setup)
      (amx-mode 1)
      (amx-test-run 'amx-probe-zoom 'amx-probe-zoom 'amx-probe-close
                    'amx-probe-zoom 'amx-probe-close 'amx-probe-open)
      (amx-save-to-file)
      (let ((refused (list :init-file-user init-file-user
                           :file (amx-test-read-save-file)
                           :warnings (amx-test-warnings))))
        (let ((init-file-user "melpa-test"))
          (amx-save-to-file))
        (let ((saved (list :file (amx-test-read-save-file)
                           :order (amx-test-order)
                           :data (copy-tree amx-data)
                           :history (copy-sequence amx-history))))
          (amx-test-fresh-session)
          (let ((forgotten (list :cache amx-cache
                                 :data (copy-tree amx-data)
                                 :history (copy-sequence amx-history))))
            (amx-initialize)
            (list :refused refused
                  :saved saved
                  :forgotten forgotten
                  :restored (list :order (amx-test-order)
                                  :data (copy-tree amx-data)
                                  :history (copy-sequence amx-history)
                                  :initialized amx-initialized))))))
  (amx-test-cleanup))"##;

    let expect = expect![[
        r#"OK (:refused (:init-file-user nil :file no-save-file :warnings ("Warning (amx): Not saving amx state from \"emacs -Q\".")) :saved (:file "\n;; ----- amx-history -----\n(\n amx-probe-open\n amx-probe-close\n amx-probe-zoom\n)\n\n;; ----- amx-data -----\n(\n (amx-probe-zoom . 3)\n (amx-probe-close . 2)\n (amx-probe-open . 1)\n)\n" :order ((amx-probe-open . 1) (amx-probe-close . 2) (amx-probe-zoom . 3) (amx-probe-quit) (amx-probe-refresh)) :data ((amx-probe-zoom . 3) (amx-probe-close . 2) (amx-probe-open . 1)) :history (amx-probe-open amx-probe-close amx-probe-zoom)) :forgotten (:cache nil :data nil :history nil) :restored (:order ((amx-probe-open . 1) (amx-probe-close . 2) (amx-probe-zoom . 3) (amx-probe-quit) (amx-probe-refresh)) :data ((amx-probe-zoom . 3) (amx-probe-close . 2) (amx-probe-open . 1)) :history (amx-probe-open amx-probe-close amx-probe-zoom) :initialized t))"#
    ]];

    assert_amx_parity(elisp_form, expect);
}

/// Commands a user never types at `M-x' are kept out of the completion list:
/// `self-insert-command' and the `menu-bar' commands by the default regexps,
/// mouse-only commands by their `interactive' spec, and anything the user
/// hides with `amx-ignore-command'.  Ignoring a command also moves the
/// default -- what `RET' would accept -- past it, and `amx-unignore-command'
/// undoes all of it.
#[test]
fn ignored_commands_are_hidden_from_completion_and_can_be_unignored() {
    let elisp_form = r##"(unwind-protect
    (progn
      (amx-test-setup)
      (amx-mode 1)
      (amx-test-run 'amx-probe-zoom 'amx-probe-close 'amx-probe-open)
      (let ((defaults (list :matchers (copy-tree amx-ignored-command-matchers)
                            :ignored (mapcar
                                      (lambda (command)
                                        (cons command
                                              (and (amx-command-ignored-p command) t)))
                                      '(self-insert-command menu-bar-open
                                        kill-emacs amx-probe-mouse
                                        amx-probe-open amx-probe-helper))
                            :order (amx-test-order)
                            :default (amx-get-default amx-cache))))
        (amx-ignore-command 'amx-probe-open)
        (let ((ignored (list :ignored-p (and (amx-command-ignored-p 'amx-probe-open) t)
                             :property (get 'amx-probe-open 'amx-ignored)
                             :marked-p (and (amx-command-marked-ignored-p
                                             'amx-probe-open)
                                            t)
                             :still-in-cache (and (assq 'amx-probe-open amx-cache) t)
                             :default (amx-get-default amx-cache))))
          (amx-unignore-command 'amx-probe-open)
          (list :defaults defaults
                :ignored ignored
                :unignored (list :ignored-p (and (amx-command-ignored-p 'amx-probe-open)
                                                 t)
                                 :property (get 'amx-probe-open 'amx-ignored)
                                 :default (amx-get-default amx-cache))))))
  (amx-test-cleanup))"##;

    let expect = expect![[
        r#"OK (:defaults (:matchers ("\\`self-insert-command\\'" "\\`self-insert-and-exit\\'" "\\`ad-Orig-" "\\`menu-bar" "\\`kill-emacs\\'" amx-command-marked-ignored-p amx-command-obsolete-p amx-command-mouse-interactive-p) :ignored ((self-insert-command . t) (menu-bar-open . t) (kill-emacs . t) (amx-probe-mouse . t) (amx-probe-open) (amx-probe-helper)) :order ((amx-probe-open . 1) (amx-probe-close . 1) (amx-probe-zoom . 1) (amx-probe-quit) (amx-probe-refresh)) :default "amx-probe-open") :ignored (:ignored-p t :property t :marked-p t :still-in-cache t :default "amx-probe-close") :unignored (:ignored-p nil :property nil :default "amx-probe-open"))"#
    ]];

    assert_amx_parity(elisp_form, expect);
}

/// A command defined after amx started -- by loading a package, say -- has to
/// appear in the list.  `amx-detect-new-commands' notices the obarray grew,
/// `amx-update-if-needed' rebuilds only when asked to count, and the rebuilt
/// cache places the newcomer by the sorting rules while every previously
/// recorded count survives the rebuild.  Command counts are editor-specific, so
/// only the change in count is pinned, never the count itself.
#[test]
fn commands_defined_during_the_session_are_detected_and_folded_into_the_ranking() {
    let elisp_form = r##"(unwind-protect
    (progn
      (amx-test-setup)
      (amx-mode 1)
      (amx-test-run 'amx-probe-zoom 'amx-probe-zoom 'amx-probe-close
                    'amx-probe-open)
      (amx-save-history)
      (let ((steady (list :detected-again (amx-detect-new-commands)
                          :order (amx-test-order))))
        (let ((count-before amx-command-count))
          (defun amx-probe-newcomer () (interactive) 'newcomer)
          (let ((detection (list :detected (and (amx-detect-new-commands) t)
                                 :delta (- amx-command-count count-before)
                                 :in-cache (and (assq 'amx-probe-newcomer amx-cache) t))))
            (setq amx-last-update-time (current-time))
            (amx-update-if-needed)
            (let ((without-counting (and (assq 'amx-probe-newcomer amx-cache) t)))
              (defun amx-probe-latecomer () (interactive) 'latecomer)
              (amx-update-if-needed t)
              (list :steady steady
                    :detection detection
                    :without-counting without-counting
                    :after-counting
                    (list :newcomer (and (assq 'amx-probe-newcomer amx-cache) t)
                          :latecomer (and (assq 'amx-probe-latecomer amx-cache) t)
                          :order (amx-test-order)
                          :data (copy-tree amx-data)
                          :history (copy-sequence amx-history))))))))
  (amx-test-cleanup))"##;

    let expect = expect![
        "OK (:steady (:detected-again nil :order ((amx-probe-open . 1) (amx-probe-close . 1) (amx-probe-zoom . 2) (amx-probe-quit) (amx-probe-refresh))) :detection (:detected t :delta 1 :in-cache nil) :without-counting nil :after-counting (:newcomer t :latecomer t :order ((amx-probe-open . 1) (amx-probe-close . 1) (amx-probe-zoom . 2) (amx-probe-quit) (amx-probe-refresh)) :data ((amx-probe-zoom . 2) (amx-probe-close . 1) (amx-probe-open . 1)) :history (amx-probe-open amx-probe-close amx-probe-zoom)))"
    ];

    assert_amx_parity(elisp_form, expect);
}
