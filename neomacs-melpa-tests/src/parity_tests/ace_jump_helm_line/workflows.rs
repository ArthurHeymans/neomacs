use expect_test::expect;

use super::assert_ace_jump_helm_line_parity;

/// The package's headline story, end to end: a helm command opens a real
/// session, `C-'' in `helm-map' runs `ace-jump-helm-line-and-select', one label
/// key picks the third candidate, and helm exits having run that candidate's
/// action.
///
/// This is the only workflow that needs helm's minibuffer.  Neomacs reads stdin
/// instead of the executing keyboard macro for every minibuffer prompt, so the
/// session dies before the first key arrives; the workflow is left failing on
/// that divergence rather than weakened.
#[test]
fn a_complete_helm_session_jumps_to_a_labelled_line_and_runs_its_action() {
    let elisp_form = r##"(progn
  (ajhl-test-setup)
  (define-key helm-map (kbd "C-'") 'ace-jump-helm-line-and-select)
  (unwind-protect
      (progn
        (execute-kbd-macro (kbd "C-c h C-' d"))
        (list :result ajhl-test-result
              :actions ajhl-test-actions
              :alive helm-alive-p
              :action-type ace-jump-helm-line--action-type
              :default-action ace-jump-helm-line-default-action
              :helm-buffer (buffer-name (get-buffer "*helm ajhl*"))))
    (define-key helm-map (kbd "C-'") nil)))"##;
    let expect = expect![[
        r#"OK (:result "deployed charlie-cache" :actions ((deploy "charlie-cache")) :alive nil :action-type nil :default-action nil :helm-buffer "*helm ajhl*")"#
    ]];

    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

/// The default action is move-only: the label key moves helm's selection to
/// that candidate line and nothing else happens -- no action runs, the rendered
/// candidate list is untouched, and a second jump inside the same session moves
/// the selection again.  The selection overlay follows the line, which is what
/// the user sees.
#[test]
fn jumping_moves_the_helm_selection_to_the_labelled_candidate_and_runs_nothing() {
    let elisp_form = r##"(progn
  (ajhl-test-setup)
  (ajhl-test-with-helm-session
   (let ((start (ajhl-test-state)))
     (execute-kbd-macro (kbd "C-c j g"))
     (let ((jumped (ajhl-test-state)))
       (execute-kbd-macro (kbd "C-c j s"))
       (list :start start
             :jumped jumped
             :second (ajhl-test-state)
             :text (ajhl-test-candidate-text)
             :labels (ajhl-test-labels)
             :default-action ace-jump-helm-line-default-action)))))"##;
    let expect = expect![[
        r#"OK (:start (:selection "alpha-api" :point 16 :line 2 :selection-overlay (16 26) :alive t :actions nil) :jumped (:selection "echo-cdn" :point 62 :line 6 :selection-overlay (62 71) :alive t :actions nil) :second (:selection "bravo-worker" :point 26 :line 3 :selection-overlay (26 39) :alive t :actions nil) :text "Deploy targets\nalpha-api\nbravo-worker\ncharlie-cache\ndelta-db\necho-cdn\n" :labels nil :default-action nil)"#
    ]];

    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

/// `ace-jump-helm-line-default-action' set to `persistent' runs helm's
/// persistent action on the candidate the label picked -- the "preview without
/// leaving helm" workflow -- while the same keys with the default value only
/// move the selection.  Both halves land on the same candidate, so the
/// difference is exactly the action.
#[test]
fn the_persistent_default_action_previews_the_jumped_candidate_without_exiting() {
    let elisp_form = r##"(progn
  (ajhl-test-setup)
  (list
   :persistent
   (ajhl-test-with-helm-session
    (let ((ace-jump-helm-line-default-action 'persistent))
      (execute-kbd-macro (kbd "C-c j s"))
      (list (ajhl-test-state) (ajhl-test-labels))))
   :move-only
   (ajhl-test-with-helm-session
    (let ((ace-jump-helm-line-default-action 'move-only))
      (execute-kbd-macro (kbd "C-c j s"))
      (list (ajhl-test-state) (ajhl-test-labels))))))"##;
    let expect = expect![[
        r#"OK (:persistent ((:selection "bravo-worker" :point 26 :line 3 :selection-overlay (26 39) :alive t :actions ((persistent "bravo-worker"))) nil) :move-only ((:selection "bravo-worker" :point 26 :line 3 :selection-overlay (26 39) :alive t :actions nil) nil))"#
    ]];

    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

/// Changing your mind after the labels are on screen: with `persistent' as the
/// default action, `ace-jump-helm-line-move-only-key' typed before the label
/// switches that one jump to move-only, and the label alone still runs the
/// persistent action.  The dispatch key is consumed by avy, so both jumps land
/// on the same candidate.
#[test]
fn a_dispatch_key_switches_the_action_for_one_jump_without_moving_the_target() {
    let elisp_form = r##"(progn
  (ajhl-test-setup)
  (list
   :switched-to-move-only
   (ajhl-test-with-helm-session
    (let ((ace-jump-helm-line-default-action 'persistent)
          (ace-jump-helm-line-move-only-key ?o)
          (ace-jump-helm-line-select-key ?e))
      (execute-kbd-macro (kbd "C-c j o d"))
      (ajhl-test-state)))
   :default-persistent
   (ajhl-test-with-helm-session
    (let ((ace-jump-helm-line-default-action 'persistent)
          (ace-jump-helm-line-move-only-key ?o)
          (ace-jump-helm-line-select-key ?e))
      (execute-kbd-macro (kbd "C-c j d"))
      (ajhl-test-state)))))"##;
    let expect = expect![[
        r#"OK (:switched-to-move-only (:selection "charlie-cache" :point 39 :line 4 :selection-overlay (39 53) :alive t :actions nil) :default-persistent (:selection "charlie-cache" :point 39 :line 4 :selection-overlay (39 53) :alive t :actions ((persistent "charlie-cache"))))"#
    ]];

    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

/// `ace-jump-helm-line-autoshow-mode' is the "line hints preview": enabling it
/// makes the labels appear on the candidate lines through helm's own update
/// hooks, without invoking the jump command at all.  This pins the label
/// assigned to each line, the `avy-lead-face' they are drawn with, how
/// `ace-jump-helm-line-style' changes the rendering (`at' overwrites the first
/// character, `pre' keeps it after the label), the
/// `ace-jump-helm-line-autoshow-use-linum' variant that right-aligns the same
/// labels in the margin instead, and that the candidate text itself is never
/// modified.
#[test]
fn autoshow_mode_previews_a_label_on_every_candidate_line_in_the_configured_style() {
    let elisp_form = r##"(progn
  (ajhl-test-setup)
  (unwind-protect
      (progn
        (ace-jump-helm-line-autoshow-mode 1)
        (list
         :hooks (list (and (memq 'ace-jump-helm-line--update-line-overlays-maybe
                                 helm-after-preselection-hook)
                           t)
                      (and (memq 'ace-jump-helm-line--update-line-overlays-maybe
                                 helm-move-selection-after-hook)
                           t)
                      (and (memq 'ace-jump-helm-line--update-line-overlays-maybe
                                 helm-after-update-hook)
                           t)
                      (and (memq 'ace-jump-helm-line--add-scroll-function
                                 helm-after-initialize-hook)
                           t))
         :at (let ((ace-jump-helm-line-style 'at))
               (ajhl-test-with-helm-session
                (list (ajhl-test-labels)
                      (ajhl-test-candidate-text)
                      (ajhl-test-state))))
         :pre (let ((ace-jump-helm-line-style 'pre))
                (ajhl-test-with-helm-session (ajhl-test-labels)))
         :linum (let ((ace-jump-helm-line-autoshow-use-linum t))
                  (ajhl-test-with-helm-session
                   (list (with-helm-buffer (list linum-mode linum-format))
                         (ajhl-test-linum-labels)
                         (ajhl-test-labels)
                         (ajhl-test-candidate-text))))
         :off (progn
                (ace-jump-helm-line-autoshow-mode -1)
                (list ace-jump-helm-line-autoshow-mode
                      helm-after-preselection-hook
                      helm-move-selection-after-hook
                      helm-after-update-hook
                      helm-after-initialize-hook))))
    (ace-jump-helm-line-autoshow-mode -1)))"##;
    let expect = expect![[
        r#"OK (:hooks (t t t t) :at (((16 17 "a" avy-lead-face) (26 27 "s" avy-lead-face) (39 40 "d" avy-lead-face) (53 54 "f" avy-lead-face) (62 63 "g" avy-lead-face)) "Deploy targets\nalpha-api\nbravo-worker\ncharlie-cache\ndelta-db\necho-cdn\n" (:selection "alpha-api" :point 16 :line 2 :selection-overlay (16 26) :alive t :actions nil)) :pre ((16 17 "aa" avy-lead-face) (26 27 "sb" avy-lead-face) (39 40 "dc" avy-lead-face) (53 54 "fd" avy-lead-face) (62 63 "ge" avy-lead-face)) :linum ((t ace-jump-helm-line--linum) ((1 "") (16 "  a") (26 "  s") (39 "  d") (53 "  f") (62 "  g")) nil "Deploy targets\nalpha-api\nbravo-worker\ncharlie-cache\ndelta-db\necho-cdn\n") :off (nil nil nil (helm-revive-visible-mark helm-confirm-and-exit-hook) (helm-reset-yank-point)))"#
    ]];

    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

/// A small key set makes avy hand out multi-character labels, and the user has
/// to type both keys to land on a candidate.  With `ace-jump-helm-line-keys'
/// set to two keys the five candidates get `jj', `jk', `kj', `kkj' and `kkk';
/// typing `k j' selects the third one and typing the wrong first key would have
/// landed elsewhere.
#[test]
fn a_small_key_set_produces_multi_character_labels_that_need_every_key() {
    let elisp_form = r##"(progn
  (ajhl-test-setup)
  (let ((ace-jump-helm-line-keys '(?j ?k)))
    (list
     :labels
     (unwind-protect
         (progn
           (ace-jump-helm-line-autoshow-mode 1)
           (ajhl-test-with-helm-session (ajhl-test-labels)))
       (ace-jump-helm-line-autoshow-mode -1))
     :two-key-jump
     (ajhl-test-with-helm-session
      (execute-kbd-macro (kbd "C-c j k j"))
      (ajhl-test-state))
     :other-branch
     (ajhl-test-with-helm-session
      (execute-kbd-macro (kbd "C-c j j k"))
      (ajhl-test-state)))))"##;
    let expect = expect![[
        r#"OK (:labels ((16 18 "jj" avy-lead-face) (26 28 "jk" avy-lead-face) (39 41 "kj" avy-lead-face) (53 56 "kkj" avy-lead-face) (62 65 "kkk" avy-lead-face)) :two-key-jump (:selection "charlie-cache" :point 39 :line 4 :selection-overlay (39 53) :alive t :actions nil) :other-branch (:selection "bravo-worker" :point 26 :line 3 :selection-overlay (26 39) :alive t :actions nil))"#
    ]];

    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

/// The experimental idle execution: `ace-jump-helm-line-idle-exec-add' advises
/// a helm command so that the labels appear by themselves when the session
/// starts and the user types nothing.  This pins the advice on the real
/// `helm-mini', the setup hook the advice installs for exactly one session, the
/// timer it schedules within `ace-jump-helm-line-idle-delay', and that
/// `ace-jump-helm-line-idle-exec-remove' takes all of it back.
#[test]
fn idle_execution_advice_schedules_the_jump_when_a_helm_command_starts() {
    let elisp_form = r##"(progn
  (ajhl-test-setup)
  (defun ajhl-test-helm-command ()
    "Stand in for a helm command: helm runs `helm-minibuffer-set-up-hook'
when the session's minibuffer is set up."
    (list :hook-during (copy-sequence helm-minibuffer-set-up-hook)
          :ran (progn (run-hooks 'helm-minibuffer-set-up-hook) t)))
  (let ((ace-jump-helm-line-idle-delay 0.25)
        (helm-minibuffer-set-up-hook nil))
    (unwind-protect
        (progn
          (ace-jump-helm-line-idle-exec-add 'helm-mini)
          (ace-jump-helm-line-idle-exec-add 'ajhl-test-helm-command)
          (list :advised (list (and (advice-member-p #'ace-jump-helm-line--maybe
                                                     'helm-mini)
                                    t)
                               (and (advice-member-p #'ace-jump-helm-line--maybe
                                                     'ajhl-test-helm-command)
                                    t))
                :hook-before helm-minibuffer-set-up-hook
                :timers-before (ajhl-test-idle-timers)
                :call (let ((inside (ajhl-test-helm-command)))
                        (list (length (plist-get inside :hook-during))
                              (mapcar #'functionp (plist-get inside :hook-during))
                              (plist-get inside :ran)))
                :hook-after helm-minibuffer-set-up-hook
                :timers-after (ajhl-test-idle-timers)
                :idle-delay ace-jump-helm-line-idle-delay
                :removed (progn
                           (ajhl-test-cancel-idle-timers)
                           (ace-jump-helm-line-idle-exec-remove 'helm-mini)
                           (ace-jump-helm-line-idle-exec-remove 'ajhl-test-helm-command)
                           (list (advice-member-p #'ace-jump-helm-line--maybe 'helm-mini)
                                 (advice-member-p #'ace-jump-helm-line--maybe
                                                  'ajhl-test-helm-command)
                                 (ajhl-test-idle-timers)))))
      (ajhl-test-cancel-idle-timers)
      (ace-jump-helm-line-idle-exec-remove 'helm-mini)
      (ace-jump-helm-line-idle-exec-remove 'ajhl-test-helm-command))))"##;
    let expect = expect![
        "OK (:advised (t t) :hook-before nil :timers-before nil :call (1 (t) t) :hook-after nil :timers-after ((ace-jump-helm-line--do-if-empty nil nil t)) :idle-delay 0.25 :removed (nil nil nil))"
    ];

    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

/// The two ways out: `ESC' at the label prompt aborts the jump, leaving helm's
/// selection, the candidate text and the label overlays exactly as they were;
/// and invoking either command with no helm session running is a plain error,
/// not a crash inside avy.
#[test]
fn an_aborted_jump_changes_nothing_and_no_session_is_a_plain_error() {
    let elisp_form = r##"(progn
  (ajhl-test-setup)
  (list
   :aborted
   (ajhl-test-with-helm-session
    (let ((start (ajhl-test-state)))
      (execute-kbd-macro (kbd "C-c j ESC"))
      (list start
            (ajhl-test-state)
            (ajhl-test-labels)
            (ajhl-test-candidate-text))))
   :no-session
   (list (condition-case error (ace-jump-helm-line) (error error))
         (condition-case error (ace-jump-helm-line-and-select) (error error))
         helm-alive-p)))"##;
    let expect = expect![[
        r#"OK (:aborted ((:selection "alpha-api" :point 16 :line 2 :selection-overlay (16 26) :alive t :actions nil) (:selection "alpha-api" :point 16 :line 2 :selection-overlay (16 26) :alive t :actions nil) nil "Deploy targets\nalpha-api\nbravo-worker\ncharlie-cache\ndelta-db\necho-cdn\n") :no-session ((error "No helm session is running") (error "No helm session is running") nil))"#
    ]];

    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
