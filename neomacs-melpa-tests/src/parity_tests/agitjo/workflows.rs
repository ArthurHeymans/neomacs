use expect_test::expect;

use super::assert_agitjo_parity;

/// The block the Commentary documents for installation:
///
///     (use-package agitjo
///       :config (agitjo-setup "#"))
///
/// `agitjo-setup' has to do two separate things for that one line to work --
/// bind the key in `magit-status-mode-map', which is step 2 of the documented
/// workflow ("or by inputting the `#' key inside a Magit status buffer"), and
/// append the same key to `magit-dispatch' so it is reachable from Magit's
/// global menu.  Both are asserted before and after, against a Magit dispatch
/// menu that has to be otherwise untouched: exactly one key is added, and no
/// existing entry moves.
///
/// The `agitjo-push' menu's own layout is pinned whole beside it, because that
/// is the surface steps 3 and 4 of the documented workflow are performed on.
#[test]
fn the_documented_setup_block_adds_one_key_to_magit_status_and_magit_dispatch() {
    let elisp_form = r##"(let* ((before (agitjo-test-transient-keys 'magit-dispatch))
       (observed nil))
  (push (list :before
              (list :status-key (keymap-lookup magit-status-mode-map "#")
                    :dispatch-entry (copy-tree (assoc "#" before))
                    :dispatch-keys (length before)))
        observed)
  (agitjo-setup "#")
  (let* ((after (agitjo-test-transient-keys 'magit-dispatch))
         (added (seq-remove (lambda (entry) (member entry before)) after)))
    (push (list :after
                (list :status-key (keymap-lookup magit-status-mode-map "#")
                      :dispatch-entry (copy-tree (assoc "#" after))
                      :dispatch-keys (length after)
                      :added (copy-tree added)
                      :nothing-else-changed
                      (equal (seq-remove (lambda (entry) (member entry added))
                                         after)
                             before)))
          observed))
  (push (list :agitjo-push-menu (agitjo-test-transient-keys 'agitjo-push))
        observed)
  (nreverse observed))"##;

    let expect = expect![[
        r##"OK ((:before (:status-key nil :dispatch-entry nil :dispatch-keys 51)) (:after (:status-key agitjo-push :dispatch-entry ("#" . agitjo-push) :dispatch-keys 52 :added (("#" . agitjo-push)) :nothing-else-changed t)) (:agitjo-push-menu (("-f" . agitjo-force-push-switch) ("-s" . agitjo-topic-variable) ("-t" . agitjo-title-option) ("+" . agitjo--pullreq-type-switches) ("u" . agitjo-push-pullreq-current-to-upstream) ("e" . agitjo-push-pullreq-current) ("l" . agitjo-push-pullreq-local-branch) ("r" . agitjo-push-pullreq-local-branch-or-ref) ("C" . magit-branch-configure) ("V" . agitjo-visit-last-pushed-pullreq))))"##
    ]];

    assert_agitjo_parity(elisp_form, expect);
}

/// The default experience for a user who never touches the `-s' session option.
///
/// `agitjo--pullreq-refspec' falls back to the *source branch name* as the
/// AGit session when no topic is set, so pushing `feature/parser-recovery' at
/// `origin/main' produces `refs/for/main/feature/parser-recovery' -- the branch
/// name appears twice in one refspec, which is what makes a wrong fallback
/// obvious.  The session is also held per project root, so setting a topic in
/// one repository must not follow the user into another: the second repository
/// still falls back to its own branch name while the first uses the topic.
///
/// Both refspecs are read out of the argument vector that reached `git push',
/// not out of the object, so this is the refspec the forge would actually see.
#[test]
fn without_a_session_topic_the_refspec_falls_back_to_each_projects_source_branch() {
    let elisp_form = r##"(let* ((agitjo--current-topics nil)
       (agitjo-test-push-requests nil)
       (agitjo-test-sentinel-events nil)
       (observed nil))
  (dolist (case (list (list :untopiced "parser-repo" "feature/parser-recovery" nil)
                      (list :topiced "docs-repo" "feature/handbook" "team/handbook-42")))
    (let* ((root (agitjo-test-repo (nth 1 case)
                                   '(("README.md" . "# Project\n"))))
           (default-directory root)
           (branch (agitjo-test-branch root (nth 2 case)
                                       '(("src/change.el" . "(provide 'change)\n"))
                                       "Change the thing"))
           (config nil))
      (when (nth 3 case) (agitjo--set-current-topic (nth 3 case)))
      (setq config (agitjo--pullreq-configuration
                    :type "for" :source branch :target "origin/main"
                    :args '("normal")))
      (let ((buffer (agitjo-post--buffer)))
        (set-window-buffer (selected-window) buffer)
        (set-buffer buffer)
        (agitjo-post-mode)
        (setq-local agitjo-post--pullreq-config config)
        (erase-buffer)
        (insert "Describe the change.\n")
        (cl-letf (((symbol-function 'magit-run-git-async)
                   (agitjo-test-push-stand-in 0))
                  ((symbol-function 'magit-process-sentinel)
                   #'agitjo-test-record-sentinel))
          (execute-kbd-macro (kbd "C-c C-c"))
          (agitjo-test-await agitjo-test-last-process)))
      (push (list (car case)
                  (list :topic (agitjo--get-current-topic)
                        :refspec (nth 3 (car (last (agitjo-test-requests))))))
            observed)))
  (nreverse observed))"##;

    let expect = expect![[
        r#"OK ((:untopiced (:topic nil :refspec "feature/parser-recovery:refs/for/main/feature/parser-recovery")) (:topiced (:topic "team/handbook-42" :refspec "feature/handbook:refs/for/main/team/handbook-42")))"#
    ]];

    assert_agitjo_parity(elisp_form, expect);
}

/// The `WIP:' HACK in `agitjo--push-args' is how agitjo tells Forgejo a pull
/// request is a draft, and it is the title the reviewer ends up reading, so its
/// three branches are worth separating.  With the `draft' switch on and an
/// explicit `-t' title the prefix is spliced into that title in place; with
/// `draft' on and no title at all the package falls back to the *subject line
/// of the source branch's head commit* and prefixes that; with the type left at
/// `normal' the title must be passed through untouched.
///
/// The commit subject and the explicit title are deliberately different
/// sentences, so a branch that reached the wrong one cannot produce the right
/// answer.  Each case is driven through the real `C-c C-c' binding in the post
/// buffer and asserted on the argument vector that reached `git push'.
#[test]
fn the_draft_switch_titles_the_request_wip_from_the_commit_subject_when_none_is_given() {
    let elisp_form = r##"(let* ((agitjo--current-topics nil)
       (agitjo-test-push-requests nil)
       (agitjo-test-sentinel-events nil)
       (root (agitjo-test-repo "titles-repo" '(("README.md" . "# Project\n"))))
       (default-directory root)
       (branch (agitjo-test-branch
                root "feature/lookahead"
                '(("src/parser.el" . "(defun parser-state () 'recovered)\n"))
                "Recover parser transitions after lookahead reset"))
       (observed nil))
  (dolist (case (list (list :draft-with-an-explicit-title
                            '("draft" "--push-option=title=Parser recovery"))
                      (list :draft-without-a-title '("draft"))
                      (list :normal-with-an-explicit-title
                            '("normal" "--push-option=title=Parser recovery"))))
    (let ((config (agitjo--pullreq-configuration
                   :type "for" :source branch :target "origin/main"
                   :args (copy-sequence (nth 1 case))))
          (buffer (agitjo-post--buffer)))
      (set-window-buffer (selected-window) buffer)
      (set-buffer buffer)
      (agitjo-post-mode)
      (setq-local agitjo-post--pullreq-config config)
      (erase-buffer)
      (insert "Reset lookahead after recovery.\n")
      (cl-letf (((symbol-function 'magit-run-git-async)
                 (agitjo-test-push-stand-in 0))
                ((symbol-function 'magit-process-sentinel)
                 #'agitjo-test-record-sentinel))
        (execute-kbd-macro (kbd "C-c C-c"))
        (agitjo-test-await agitjo-test-last-process))
      (push (list (car case)
                  (seq-filter (lambda (argument)
                                (and (stringp argument)
                                     (string-prefix-p "--push-option=title="
                                                      argument)))
                              (car (last (agitjo-test-requests)))))
            observed)))
  (push (list :commit-subject (magit-rev-format "%s" branch)) observed)
  (nreverse observed))"##;

    let expect = expect![[
        r#"OK ((:draft-with-an-explicit-title ("--push-option=title=WIP: Parser recovery")) (:draft-without-a-title ("--push-option=title=WIP: Recover parser transitions after lookahead reset")) (:normal-with-an-explicit-title ("--push-option=title=Parser recovery")) (:commit-subject "Recover parser transitions after lookahead reset"))"#
    ]];

    assert_agitjo_parity(elisp_form, expect);
}

/// Step 6 of the documented workflow: "Visit the created pull request's link
/// (=V=) in a browser".  `agitjo-visit-last-pushed-pullreq' finds that link by
/// searching the Magit process buffer *backwards* for a `remote:' line naming a
/// `/pulls/N' URL, so the fixture pushes twice and the command must return the
/// second pull request, not the first.
///
/// The surrounding lines are what make that a real discrimination rather than
/// a regexp matching the only thing present: the buffer also holds the
/// repository's own `remote:' URL with no `/pulls/' segment, a `/pulls/'
/// URL that is not on a `remote:' line, and ordinary git transport output.
/// Only one line in each push is a legitimate answer.
///
/// `browse-url' is stood in for -- it launches an external browser -- and is
/// asserted to receive the exact URL.  The empty case needs no fixture at all:
/// with nothing pushed the command must refuse with its documented `user-error'
/// and open nothing.
#[test]
fn visiting_the_last_pushed_pull_request_takes_the_most_recent_link_from_git_output() {
    let elisp_form = r##"(let* ((agitjo--current-topics nil)
       (agitjo-test-push-requests nil)
       (agitjo-test-sentinel-events nil)
       (root (agitjo-test-repo "visit-repo" '(("README.md" . "# Project\n"))))
       (default-directory root)
       (branch (agitjo-test-branch root "feature/visit"
                                   '(("src/visit.el" . "(provide 'visit)\n"))
                                   "Add the visit path"))
       (visited nil)
       (observed nil))
  (cl-letf (((symbol-function 'browse-url)
             (lambda (url &rest _) (push (copy-sequence url) visited) 'opened)))
    (push (list :before-any-push
                (condition-case error (agitjo-visit-last-pushed-pullreq)
                  (error (list (car error) (cadr error))))
                :opened (reverse visited))
          observed)
    (dolist (push-case (list (list 41 "feature/visit")
                             (list 42 "feature/visit")))
      (let ((config (agitjo--pullreq-configuration
                     :type "for" :source branch :target "origin/main"
                     :args '("normal")))
            (buffer (agitjo-post--buffer)))
        (set-window-buffer (selected-window) buffer)
        (set-buffer buffer)
        (agitjo-post-mode)
        (setq-local agitjo-post--pullreq-config config)
        (erase-buffer)
        (insert "Describe the change.\n")
        (cl-letf (((symbol-function 'magit-run-git-async)
                   (agitjo-test-push-stand-in
                    0
                    (format (concat "remote: Resolving deltas: 100%% (1/1)\n"
                                    "remote: Repository at https://forge.invalid/halvin/agitjo\n"
                                    "remote: See https://forge.invalid/halvin/agitjo/pulls/%d for details\n"
                                    "remote:   https://forge.invalid/halvin/agitjo/pulls/%d\n"
                                    "To ssh://forge.invalid/halvin/agitjo.git\n"
                                    " * [new reference] %s -> refs/for/main/%s\n")
                            (- (car push-case) 10) (car push-case)
                            (nth 1 push-case) (nth 1 push-case))))
                  ((symbol-function 'magit-process-sentinel)
                   #'agitjo-test-record-sentinel))
          (execute-kbd-macro (kbd "C-c C-c"))
          (agitjo-test-await agitjo-test-last-process))))
    (push (list :after-two-pushes
                (agitjo-visit-last-pushed-pullreq)
                :opened (reverse visited))
          observed))
  (nreverse observed))"##;

    let expect = expect![[
        r#"OK ((:before-any-push (user-error "No pull request link could be found") :opened nil) (:after-two-pushes opened :opened ("https://forge.invalid/halvin/agitjo/pulls/42")))"#
    ]];

    assert_agitjo_parity(elisp_form, expect);
}

/// Where the pull request template comes from, which is not where a user would
/// guess.  `agitjo-post--find-pullreq-template-object' reads the template out
/// of `<primary-remote>/<main-branch>' with `git ls-tree' -- so a template that
/// exists in the working tree, and even one committed on the feature branch, is
/// *not* the one the draft is seeded with.  The fixture puts a different
/// template in each of those three places and the draft must show the one from
/// `origin/main'.
///
/// The second repository covers precedence, since the package looks for two
/// file names across three directories: with a `.forgejo' and a `.github'
/// template both committed on main, `.forgejo' must win.
#[test]
fn the_draft_template_comes_from_the_remote_main_branch_and_prefers_forgejo() {
    let elisp_form = r####"(let* ((agitjo--current-topics nil)
       (observed nil))
  ;; A template on origin/main, a different one committed on the feature
  ;; branch, and a third one only in the working tree.
  (let* ((root (agitjo-test-repo
                "template-origin"
                '((".github/pull_request_template.md"
                   . "## From origin main\n\nDescribe the change.\n"))))
         (default-directory root))
    (agitjo-test-branch root "feature/templates"
                        '((".forgejo/PULL_REQUEST_TEMPLATE.md"
                           . "## From the feature branch\n")
                          ("src/thing.el" . "(provide 'thing)\n"))
                        "Add the thing")
    (agitjo-test-write (expand-file-name ".gitea/PULL_REQUEST_TEMPLATE.md" root)
                       "## Only in the working tree\n")
    (let ((config (agitjo--pullreq-configuration
                   :type "for" :source "feature/templates" :target "origin/main"
                   :args '("normal")))
          (buffer nil))
      (agitjo-post--setup-buffer config)
      (setq buffer (agitjo-post--buffer))
      (push (list :three-candidate-templates
                  (list :draft (with-current-buffer buffer (buffer-string))
                        :draft-file (agitjo-test-relative
                                     (buffer-file-name buffer))))
            observed)
      (with-current-buffer buffer (set-buffer-modified-p nil))
      (kill-buffer buffer)))
  ;; Both a .forgejo and a .github template committed on main.
  (let* ((root (agitjo-test-repo
                "template-precedence"
                '((".forgejo/PULL_REQUEST_TEMPLATE.md" . "## Forgejo template\n")
                  (".github/pull_request_template.md" . "## GitHub template\n"))))
         (default-directory root))
    (agitjo-test-branch root "feature/precedence"
                        '(("src/thing.el" . "(provide 'thing)\n"))
                        "Add the thing")
    (let ((config (agitjo--pullreq-configuration
                   :type "for" :source "feature/precedence" :target "origin/main"
                   :args '("normal")))
          (buffer nil))
      (agitjo-post--setup-buffer config)
      (setq buffer (agitjo-post--buffer))
      (push (list :forgejo-and-github-templates
                  (list :draft (with-current-buffer buffer (buffer-string))))
            observed)
      (with-current-buffer buffer (set-buffer-modified-p nil))
      (kill-buffer buffer)))
  (nreverse observed))"####;

    let expect = expect![[
        r###"OK ((:three-candidate-templates (:draft "## From origin main\n\nDescribe the change.\n" :draft-file "template-origin/.git/agitjo/pullreq-draft")) (:forgejo-and-github-templates (:draft "## Forgejo template\n")))"###
    ]];

    assert_agitjo_parity(elisp_form, expect);
}

/// `agitjo-post-confirm' is bound to `C-c C-c' only inside the post buffer, but
/// it is an ordinary command and `M-x' reaches it from anywhere.  Invoked from
/// an unrelated buffer it must refuse with its documented `user-error' and push
/// nothing.
///
/// It is worth pinning what the refusal costs, because the guard runs *after*
/// the buffer has been located: `agitjo-post--buffer' unconditionally creates
/// the draft *directory* below `.git' and visits the draft file before the name
/// comparison rejects the call.  The directory is left behind; the draft file
/// itself is not, because visiting a missing file creates only a buffer.  Both
/// halves are recorded, since "refuses cleanly" and "refuses after touching the
/// repository" are different promises.
#[test]
fn confirming_from_an_unrelated_buffer_refuses_and_pushes_nothing() {
    let elisp_form = r##"(let* ((agitjo--current-topics nil)
       (agitjo-test-push-requests nil)
       (root (agitjo-test-repo "guard-repo" '(("README.md" . "# Project\n"))))
       (default-directory root)
       (draft-file (expand-file-name ".git/agitjo/pullreq-draft" root))
       (scratch (get-buffer-create "*agitjo-unrelated*"))
       (observed nil))
  (agitjo-test-branch root "feature/guard"
                      '(("src/guard.el" . "(provide 'guard)\n"))
                      "Add the guard")
  (push (list :before (list :draft-directory-exists
                            (file-directory-p (file-name-directory draft-file))
                            :draft (agitjo-test-draft-contents draft-file)))
        observed)
  (with-current-buffer scratch
    (setq default-directory root)
    (insert "not a pull request draft\n")
    (set-window-buffer (selected-window) scratch)
    (push (list :refusal
                (condition-case error (call-interactively #'agitjo-post-confirm)
                  (error (list (car error) (cadr error)))))
          observed))
  (push (list :after (list :draft-directory-exists
                           (file-directory-p (file-name-directory draft-file))
                           :draft (agitjo-test-draft-contents draft-file)
                           :pushes (agitjo-test-requests)
                           :unrelated-buffer-text
                           (with-current-buffer scratch (buffer-string))))
        observed)
  (nreverse observed))"##;

    let expect = expect![[
        r#"OK ((:before (:draft-directory-exists nil :draft no-draft-file)) (:refusal (user-error "Function called outside AGitjo post buffer")) (:after (:draft-directory-exists t :draft no-draft-file :pushes nil :unrelated-buffer-text "not a pull request draft\n")))"#
    ]];

    assert_agitjo_parity(elisp_form, expect);
}
