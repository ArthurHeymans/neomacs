use std::time::Duration;

use crate::{AHG_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod commit;
mod history;
mod mq;
mod status;

const AHG_TEST_TIMEOUT: Duration = Duration::from_secs(60);

const AHG_TEST_PRELUDE: &str = r##"
(defun neomacs-ahg-fixture ()
  (let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
         (root (file-name-as-directory
                (expand-file-name "release-repo" sandbox)))
         (source (expand-file-name "src/main.el" root))
         (guide (expand-file-name "docs/guide.md" root))
         (todo (expand-file-name "notes.todo" root))
         (patches (expand-file-name ".hg/patches" root))
         (fake-hg (expand-file-name "bin/hg" sandbox)))
    (make-directory (expand-file-name ".hg" root) t)
    (make-directory (file-name-directory source) t)
    (make-directory (file-name-directory guide) t)
    (make-directory patches t)
    (make-directory (file-name-directory fake-hg) t)
    (with-temp-file source
      (insert
       "(defun deploy-release ()\n"
       "  (message \"release ready\"))\n"
       "\n"
       "(defun rollback-release ()\n"
       "  (message \"rollback ready\"))\n"))
    (with-temp-file guide
      (insert
       "# Release guide\n\n"
       "Deploy the release after review.\n"
       "Rollback the release if monitoring fails.\n"))
    (with-temp-file todo
      (insert "verify release checks\n"))
    (with-temp-file
        (expand-file-name "release-candidate" patches)
      (insert
       "# HG changeset patch\n"
       "# User Ada Lovelace <ada@example.test>\n"
       "# Date 1700000000 0\n"
       "#      Wed Nov 13 08:48:06 2024 +0000\n"
       "# Node ID c0ffee\n"
       "# Parent  bead123\n"
       "Prepare the release candidate\n\n"
       "diff --git a/src/main.el b/src/main.el\n"
       "--- a/src/main.el\n"
       "+++ b/src/main.el\n"
       "@@ -1,1 +1,1 @@\n"
       "-(defun deploy-candidate ()\n"
       "+(defun deploy-release ()\n"))
    (with-temp-file fake-hg
      (insert
       "#!/bin/sh\n"
       "while [ \"$1\" = \"--config\" ]; do shift 2; done\n"
       "command=$1\n"
       "shift\n"
       "repo=$PWD\n"
       "while [ ! -d \"$repo/.hg\" ] && [ \"$repo\" != / ]; do\n"
       "  repo=${repo%/*}\n"
       "  if [ -z \"$repo\" ]; then repo=/; fi\n"
       "done\n"
       "printf '%s|%s\\n' \"$command\" \"$*\" >> \"$repo/.hg/commands.log\"\n"
       "case \"$command\" in\n"
       "  identify)\n"
       "    printf '2+ feature release\\n'\n"
       "    ;;\n"
       "  summary)\n"
       "    printf 'branch: feature\\nparent: 2:c0ffee\\ncommit: 2 modified, 1 unknown\\n'\n"
       "    ;;\n"
       "  id)\n"
       "    printf '2+\\n'\n"
       "    ;;\n"
       "  status)\n"
       "    printf 'M src/main.el\\n? notes.todo\\n! removed.txt\\n'\n"
       "    ;;\n"
       "  showconfig)\n"
       "    printf 'Ada Lovelace <ada@example.test>\\n'\n"
       "    ;;\n"
       "  branch)\n"
       "    printf 'feature\\n'\n"
       "    ;;\n"
       "  debugcomplete)\n"
       "    printf 'status\\nsummary\\n'\n"
       "    ;;\n"
       "  help)\n"
       "    printf 'hg status [OPTION]... [FILE]...\\n\\nshow changed files in the working directory\\n\\noptions:\\n -A --all  show all files\\n'\n"
       "    ;;\n"
       "  files)\n"
       "    printf 'src/main.el\\ndocs/guide.md\\n'\n"
       "    ;;\n"
       "  qseries)\n"
       "    printf 'base\\nrelease-candidate\\ncleanup\\n'\n"
       "    ;;\n"
       "  qapplied)\n"
       "    printf 'base\\nrelease-candidate\\n'\n"
       "    ;;\n"
       "  qguard)\n"
       "    printf 'base: unguarded\\nrelease-candidate: +linux -windows\\ncleanup: +cleanup\\n'\n"
       "    ;;\n"
       "  qgoto)\n"
       "    printf 'now at: %s\\n' \"$*\"\n"
       "    ;;\n"
       "  diff)\n"
       "    printf 'diff --git a/src/main.el b/src/main.el\\n'\n"
       "    printf '%s\\n' '--- a/src/main.el' '+++ b/src/main.el'\n"
       "    printf '@@ -1,2 +1,2 @@\\n'\n"
       "    printf -- '-(defun deploy-candidate ()\\n'\n"
       "    printf -- '+(defun deploy-release ()\\n'\n"
       "    printf '   (message \"release ready\"))\\n'\n"
       "    ;;\n"
       "  annotate)\n"
       "    printf 'Ada 2 2024-11-13: 1: (defun deploy-release ()\\n'\n"
       "    printf 'Ada 2 2024-11-13: 2:   (message \"release ready\"))\\n'\n"
       "    printf 'Grace 1 2024-11-12: 4: (defun rollback-release ()\\n'\n"
       "    printf 'Grace 1 2024-11-12: 5:   (message \"rollback ready\"))\\n'\n"
       "    ;;\n"
       "  log)\n"
       "    case \"$*\" in\n"
       "      *'{rev} {date|shortdate}'*)\n"
       "        printf '2 2024-11-13 ada Ship release safely\\n'\n"
       "        printf '1 2024-11-12 grace Add rollback procedure\\n'\n"
       "        printf '0 2024-11-11 ada Bootstrap repository\\n'\n"
       "        ;;\n"
       "      *'{rev} {desc|firstline}'*)\n"
       "        printf '2 Ship release safely\\n'\n"
       "        printf '1 Add rollback procedure\\n'\n"
       "        ;;\n"
       "      *'--style'*)\n"
       "        printf '2:c0ffee\\n\\n\\ndraft\\nfeature\\nv2.0\\nrelease\\n1:bead123\\nAda Lovelace <ada@example.test>\\n1700000000 0\\nsrc/main.el\\ndocs/guide.md\\n\\n\\tShip release safely\\n\\tDocument deployment\\n\\n'\n"
       "        printf '1:bead123\\n\\n\\npublic\\n\\n\\n\\n0:0000000\\nGrace Hopper <grace@example.test>\\n1699900000 0\\nsrc/main.el\\n\\n\\tAdd rollback procedure\\n\\n'\n"
       "        ;;\n"
       "      *'{bookmarks}'*)\n"
       "        printf 'release\\n'\n"
       "        ;;\n"
       "      *'{rev}'*)\n"
       "        printf '2 '\n"
       "        ;;\n"
       "      *'{node|short}'*)\n"
       "        case \"$*\" in\n"
       "          *'-r 1 '*) printf 'bead123 ' ;;\n"
       "          *'-r 2 '*) printf 'c0ffee ' ;;\n"
       "          *) printf 'c0ffee ' ;;\n"
       "        esac\n"
       "        ;;\n"
       "      *)\n"
       "        printf 'c0ffee '\n"
       "        ;;\n"
       "    esac\n"
       "    ;;\n"
       "  commit)\n"
       "    : > \"$repo/.hg/last-commit-files\"\n"
       "    while [ \"$#\" -gt 0 ]; do\n"
       "      if [ \"$1\" = \"-m\" ]; then\n"
       "        shift\n"
       "        printf '%s' \"$1\" > \"$repo/.hg/last-commit-message\"\n"
       "      else\n"
       "        printf '%s\\n' \"$1\" >> \"$repo/.hg/last-commit-files\"\n"
       "      fi\n"
       "      shift\n"
       "    done\n"
       "    ;;\n"
       "  *)\n"
       "    printf 'unsupported fixture command: %s\\n' \"$command\" >&2\n"
       "    exit 9\n"
       "    ;;\n"
       "esac\n"))
    (set-file-modes fake-hg #o755)
    (list root fake-hg source guide todo patches)))

(defun neomacs-ahg-wait-until (predicate)
  (let ((deadline (+ (float-time) 12.0)))
    (while (and (not (funcall predicate))
                (< (float-time) deadline))
      (accept-process-output nil 0.02))
    (funcall predicate)))

(defun neomacs-ahg-file-string (file)
  (when (file-exists-p file)
    (with-temp-buffer
      (insert-file-contents file)
      (buffer-string))))
"##;

fn ahg_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AHG_MELPA_PIN, "ahg.el")
        .expect("prepare pinned ahg source below ./tmp")
        .with_prelude(AHG_TEST_PRELUDE)
        .with_timeout(AHG_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ahg parity test").into()
}

pub(crate) fn assert_ahg_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ahg_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ahg parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
