use expect_test::expect;

use super::assert_ahg_parity;

/// Opening the status view of a repository with real working-tree changes.
///
/// `hg status' reports one modified file and one untracked file, and aHg also
/// asks for `hg summary' to build the header.  Both answers are replayed from
/// real Mercurial 7.1, which matters most for `summary': its real output leads
/// with `parent:' and carries `update:' and `phases:' lines, and a header
/// parser keyed on line order would happily pass against a differently ordered
/// invention.
///
/// The exact argument vectors are asserted beside the rendered buffer, because
/// the buffer alone cannot say whether aHg asked Mercurial the right question.
#[test]
fn the_status_view_renders_mercurials_own_status_and_summary_for_a_dirty_tree() {
    let elisp_form = r##"(progn
  (ahg-test-install-hg)
  (let* ((root (ahg-test-repo "repoA"))
         (default-directory root))
    (ahg-status)
    (ahg-test-settle 15)
    (list :status (ahg-test-buffer "*hg status:")
          :mode (ahg-test-buffer-mode "*hg status:")
          :calls (ahg-test-calls)
          :unrecorded (ahg-test-unrecorded))))"##;

    let expect = expect![[
        r#"OK (:status "hg status for [ORACLE-SANDBOX]/repoA/\n\n M docs/guide.md\n ? notes.todo\n\n-------------------------------------------------------------------------------\nparent: 2:60eb783c89a0 tip\n Ship release safely\ncommit: 1 modified, 1 unknown\nupdate: (current)\nphases: 3 draft\n" :mode ahg-status-mode :calls ("repoA: --config ui.report_untrusted=0 status" "repoA: --config ui.report_untrusted=0 summary") :unrecorded nil)"#
    ]];

    assert_ahg_parity(elisp_form, expect);
}

/// The two log views a user moves between: the one-line-per-revision summary
/// from `ahg-short-log', and the detailed view from `ahg-log'.
///
/// The detailed view is the interesting half.  aHg writes its own template into
/// `.hg/ahg-log-style-map' and asks Mercurial to render through it, so the
/// reply is a fixed-order block per revision with *blank lines standing in for
/// empty fields* -- no tags, no bookmarks, no branch.  That layout is exactly
/// what a hand-written fixture gets wrong, and getting it wrong shifts every
/// field aHg reads by a line.  Here it is Mercurial's own rendering, so the
/// parsed result is a real test of the parser.
///
/// Both revision hashes are pinned: the recorded repository is reproducible, so
/// `60eb783c89a0' is a fact about the fixture rather than a captured accident.
#[test]
fn both_log_views_are_parsed_from_mercurials_own_template_and_style_output() {
    let elisp_form = r##"(progn
  (ahg-test-install-hg)
  (let* ((root (ahg-test-repo "repoA"))
         (default-directory root)
         (observed nil))
    (ahg-short-log "0" "2")
    (ahg-test-settle 15)
    (push (list :short-log (ahg-test-buffer "*hg log (summary):")
                :mode (ahg-test-buffer-mode "*hg log (summary):"))
          observed)
    (ahg-log "0" "2")
    (ahg-test-settle 15)
    (push (list :detailed-log (ahg-test-buffer "*hg log (details):")
                :mode (ahg-test-buffer-mode "*hg log (details):"))
          observed)
    (push (list :calls (ahg-test-calls)
                :unrecorded (ahg-test-unrecorded))
          observed)
    (nreverse observed)))"##;

    let expect = expect![[
        r#"OK ((:short-log "hg log (summary) for [ORACLE-SANDBOX]/repoA/\n\n--------------------------------------------------------------------------------\n    Rev |    Date    |  Author  | Summary\n--------------------------------------------------------------------------------\n      0 | 2023-11-14 |    grace | Bootstrap repository                          \n      1 | 2023-11-15 |    grace | Add rollback procedure                        \n      2*| 2023-11-16 |      ada | Ship release safely                           \n--------------------------------------------------------------------------------\n" :mode ahg-short-log-mode) (:detailed-log "hg log for [ORACLE-SANDBOX]/repoA/\n\nchangeset:   0:84d4a1540886\nphase:       draft\nuser:        Grace Hopper <grace@example.test>\ndate:        Tue Nov 14 22:13:20 2023 +0000\nfiles:       docs/guide.md\n             src/main.el\ndescription:\nBootstrap repository\n\n\nchangeset:   1:9eb7836204d1\nphase:       draft\nuser:        Grace Hopper <grace@example.test>\ndate:        Wed Nov 15 22:13:20 2023 +0000\nfiles:       src/main.el\ndescription:\nAdd rollback procedure\n\n\nchangeset:   2:60eb783c89a0\nphase:       draft\ntag:         tip\nuser:        Ada Lovelace <ada@example.test>\ndate:        Thu Nov 16 22:13:20 2023 +0000\nfiles:       src/main.el\ndescription:\nShip release safely\n\n\n" :mode ahg-log-mode) (:calls ("repoA: --config ui.report_untrusted=0 log -r 0:2 --template {rev} {date|shortdate} {author|user} {desc|firstline}\\n" "repoA: --config ui.report_untrusted=0 log -r . --template {rev} " "repoA: --config ui.report_untrusted=0 log -r 0:2 --style .hg/ahg-log-style-map") :unrecorded nil))"#
    ]];

    assert_ahg_parity(elisp_form, expect);
}

/// `ahg-diff' on a dirty working tree.  Mercurial is asked for `diff --git' and
/// its reply is rendered into a diff buffer, so this pins both the argument
/// vector aHg chose -- git-style rather than plain unified -- and the hunk as
/// Mercurial actually emits it for the recorded edit to `docs/guide.md'.
#[test]
fn the_diff_view_renders_mercurials_git_style_diff_of_the_working_tree() {
    let elisp_form = r##"(progn
  (ahg-test-install-hg)
  (let* ((root (ahg-test-repo "repoA"))
         (default-directory root))
    (ahg-diff)
    (ahg-test-settle 15)
    (list :diff (ahg-test-buffer "*aHg-diff*")
          :mode (ahg-test-buffer-mode "*aHg-diff*")
          :calls (ahg-test-calls)
          :unrecorded (ahg-test-unrecorded))))"##;

    let expect = expect![[
        r#"OK (:diff "diff --git a/docs/guide.md b/docs/guide.md\n--- a/docs/guide.md\n+++ b/docs/guide.md\n@@ -1,3 +1,4 @@\n # Release guide\n \n Deploy after review.\n+Rollback if monitoring fails.\n" :mode ahg-diff-mode :calls ("repoA: --config ui.report_untrusted=0 diff --git" "repoA: --config ui.report_untrusted=0 log -r . --template {node|short} ") :unrecorded nil)"#
    ]];

    assert_ahg_parity(elisp_form, expect);
}

/// Annotating a real source file.  aHg invokes `hg annotate -undql', whose
/// output is column-aligned in a way no invented fixture reproduces: the user
/// name is right-aligned in a field wide enough for the longest name in the
/// file, so `ada' arrives padded to match `grace', and the line number is
/// packed against the date with no space.  Both of those are load-bearing for
/// a parser that has to split the annotation from the source text.
///
/// The workflow opens with an upstream defect, asserted as its precondition.
/// aHg calls `word-at-point' at six sites and never requires `thingatpt'.
/// `thing-at-point' is autoloaded but `word-at-point' is NOT, so in a session
/// where nothing else has pulled the library in, annotate dies with
/// `Symbol\'s function definition is void: word-at-point' -- and it dies inside
/// a process sentinel, which in GNU batch is fatal, so the failure itself
/// cannot be asserted through this harness (see DIVERGENCES.md entry 18 for the
/// same shape).  What is assertable is that the function is missing before the
/// library is loaded, which is the whole of the defect; the workflow then
/// requires `thingatpt', as any real session has, and annotates for real.
///
/// This is also the "a probe is the more forgiving environment" trap paying
/// out: a scratch driver that called `package-initialize' annotated happily,
/// and only the harness's bare session failed.
#[test]
fn annotate_renders_mercurials_column_aligned_blame_for_a_real_source_file() {
    let elisp_form = r##"(progn
  (ahg-test-install-hg)
  (let* ((root (ahg-test-repo "repoA"))
         (default-directory root)
         (source (expand-file-name "src/main.el" root))
         (buffer (let ((enable-dir-local-variables nil))
                   (find-file-noselect source)))
         (bare (list :word-at-point (fboundp 'word-at-point)
                     :thing-at-point (fboundp 'thing-at-point)
                     :ahg-requires-thingatpt (featurep 'thingatpt))))
    (require 'thingatpt)
    (set-window-buffer (selected-window) buffer)
    (set-buffer buffer)
    (ahg-annotate)
    (ahg-test-settle 15)
    (list :in-a-bare-session bare
          :annotate (ahg-test-buffer "*hg annotate:")
          :source-still-current (buffer-name buffer)
          :calls (ahg-test-calls)
          :unrecorded (ahg-test-unrecorded))))"##;

    let expect = expect![[
        r#"OK (:in-a-bare-session (:word-at-point nil :thing-at-point t :ahg-requires-thingatpt nil) :annotate "  ada 2 2023-11-16:1: (defun deploy-release ()\n  ada 2 2023-11-16:2:   (message \"release ready\"))\ngrace 1 2023-11-15:3: \ngrace 1 2023-11-15:4: (defun rollback-release ()\ngrace 1 2023-11-15:5:   (message \"rollback ready\"))\n" :source-still-current "main.el" :calls ("repoA: --config ui.report_untrusted=0 log --template {rev} {desc|firstline}\\n src/main.el" "repoA: --config ui.report_untrusted=0 annotate -undql src/main.el") :unrecorded nil)"#
    ]];

    assert_ahg_parity(elisp_form, expect);
}

/// The MQ patch queue, against the second recorded repository -- the one whose
/// queue carries a guard that was really set with `hg qguard'.
///
/// This is where an invented fixture is most tempting and most misleading.
/// `hg qguard -l' reports `release-candidate: +linux -windows' and
/// `cleanup: unguarded', and because the `+linux' guard is not selected the
/// patch is *not applied* -- `hg summary' says `mq: 1 unapplied' and the parent
/// is still the base revision.  A queue where the guard has no consequence
/// would render the same list and prove nothing about guards.
///
/// Note also the real argument vector: aHg asks for `qguard -l', not bare
/// `qguard'.
#[test]
fn the_patch_queue_view_shows_the_real_guard_that_kept_a_patch_unapplied() {
    let elisp_form = r##"(progn
  (ahg-test-install-hg)
  (let* ((root (ahg-test-repo "repoB" t))
         (default-directory root))
    (ahg-mq-list-patches)
    (ahg-test-settle 15)
    (list :patches (ahg-test-buffer "*aHg mq patches for:")
          :mode (ahg-test-buffer-mode "*aHg mq patches for:")
          :calls (ahg-test-calls)
          :unrecorded (ahg-test-unrecorded))))"##;

    let expect = expect![[
        r#"OK (:patches "mq patch queue for [ORACLE-SANDBOX]/repoB/\n\n--------------------------------------------------------------------------------\n Index | App | Patch (Guards)\n--------------------------------------------------------------------------------\n     0 |     | release-candidate (+linux -windows)                              \n     1 |     | cleanup                                                          \n--------------------------------------------------------------------------------\n" :mode ahg-mq-patches-mode :calls ("repoB: --config ui.report_untrusted=0 qseries" "repoB: --config ui.report_untrusted=0 qapplied" "repoB: --config ui.report_untrusted=0 qguard -l") :unrecorded nil)"#
    ]];

    assert_ahg_parity(elisp_form, expect);
}
