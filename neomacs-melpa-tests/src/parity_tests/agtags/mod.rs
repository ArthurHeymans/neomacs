use std::time::Duration;

use crate::{AGTAGS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod database;
mod navigation;
mod xref;

const AGTAGS_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const AGTAGS_TEST_PRELUDE: &str = r####"
(defun neomacs-agtags-test-write-executable (file content)
  (make-directory (file-name-directory file) t)
  (with-temp-file file
    (insert content))
  (set-file-modes file #o755))

(defun neomacs-agtags-test-install-tools (root)
  (let* ((bin (expand-file-name "tools/bin" root))
         (trace (expand-file-name "tools/invocations.log" root)))
    (make-directory bin t)
    (neomacs-agtags-test-write-executable
     (expand-file-name "gtags" bin)
     (concat
      "#!/bin/sh\n"
      "printf 'gtags cwd=%s' \"$PWD\" >> \"$AGTAGS_TEST_TRACE\"\n"
      "for argument do printf ' <%s>' \"$argument\" >> \"$AGTAGS_TEST_TRACE\"; done\n"
      "printf '\\n' >> \"$AGTAGS_TEST_TRACE\"\n"
      "printf '%s\\n' 'paths=include/parser.h,src/main.c,src/parser.c' > GPATH\n"
      "printf '%s\\n' 'definitions=parser_init,parser_reset,parse_request' > GTAGS\n"
      "printf '%s\\n' 'references=src/main.c,src/parser.c' > GRTAGS\n"))
    (neomacs-agtags-test-write-executable
     (expand-file-name "global" bin)
     (concat
      "#!/bin/sh\n"
      "printf 'global cwd=%s' \"$PWD\" >> \"$AGTAGS_TEST_TRACE\"\n"
      "for argument do printf ' <%s>' \"$argument\" >> \"$AGTAGS_TEST_TRACE\"; done\n"
      "printf '\\n' >> \"$AGTAGS_TEST_TRACE\"\n"
      "case \" $* \" in\n"
      "  *' --single-update='*)\n"
      "    for argument do\n"
      "      case \"$argument\" in\n"
      "        --single-update=*) updated=${argument#--single-update=} ;;\n"
      "      esac\n"
      "    done\n"
      "    printf 'updated=%s\\n' \"$updated\" >> GTAGS\n"
      "    ;;\n"
      "  *' --result=path '*)\n"
      "    printf '%s\\n' 'include/parser.h' 'src/main.c' 'src/parser.c'\n"
      "    ;;\n"
      "  *' -d -x -a '*)\n"
      "    printf '%s\\n' \"parser_reset 2 $PWD/src/parser.c int parser_reset(int state) {\"\n"
      "    ;;\n"
      "  *' -r -x -a '*)\n"
      "    printf '%s\\n' \\\n"
      "      \"parser_reset 5 $PWD/src/main.c return parser_reset(input);\" \\\n"
      "      \"parser_reset 7 $PWD/src/parser.c return parser_reset(state - 1);\"\n"
      "    ;;\n"
      "  *' -c -P '*)\n"
      "    printf '%s\\n' 'include/parser.h' 'src/main.c' 'src/parser.c'\n"
      "    ;;\n"
      "  *' -c '*)\n"
      "    printf '%s\\n' 'parse_request' 'parser_init' 'parser_reset'\n"
      "    ;;\n"
      "  *' --result=grep '*)\n"
      "    case \" $* \" in\n"
      "      *' -r '*)\n"
      "        printf '%s\\n' \\\n"
      "          'src/main.c:5:  return parser_reset(input);' \\\n"
      "          'src/parser.c:7:  return parser_reset(state - 1);'\n"
      "        ;;\n"
      "      *' -g '*)\n"
      "        printf '%s\\n' \\\n"
      "          'src/main.c:5:  return parser_reset(input);' \\\n"
      "          'src/parser.c:2:int parser_reset(int state) {'\n"
      "        ;;\n"
      "      *)\n"
      "        printf '%s\\n' 'src/parser.c:2:int parser_reset(int state) {'\n"
      "        ;;\n"
      "    esac\n"
      "    ;;\n"
      "esac\n"))
    (list bin trace)))

(defun neomacs-agtags-test-use-tools (tools)
  (setq exec-path (cons (car tools) exec-path)
        process-environment (copy-sequence process-environment))
  (setenv
   "PATH"
   (concat
    (car tools)
    path-separator
    (or (getenv "PATH") "")))
  (setenv "AGTAGS_TEST_TRACE" (cadr tools)))

(defun neomacs-agtags-test-wait-for-buffer (buffer)
  (let ((attempts 0)
        (process (and buffer (get-buffer-process buffer))))
    (while
        (and process
             (process-live-p process)
             (< attempts 200))
      (accept-process-output process 0.05)
      (setq attempts (1+ attempts)))
    (when
        (and process
             (process-live-p process))
      (error "Timed out waiting for %s" (buffer-name buffer)))
    (when process
      (accept-process-output process 0.05)))
  buffer)

(defun neomacs-agtags-test-file-string (file)
  (with-temp-buffer
    (insert-file-contents file)
    (buffer-string)))

(defun neomacs-agtags-test-normalize-result-buffer (text)
  (replace-regexp-in-string
   "\\(Global [^\n]+\\) at [^\n]+"
   "\\1 at TIME"
   text
   t))

(defun neomacs-agtags-test-cleanup (root)
  (dolist (buffer (buffer-list))
    (when-let ((file (buffer-file-name buffer)))
      (when
          (string-prefix-p root file)
        (with-current-buffer buffer
          (set-buffer-modified-p nil))
        (kill-buffer buffer))))
  (dolist (name '("*agtags-grep*" "*agtags-path*"))
    (when-let ((buffer (get-buffer name)))
      (when-let ((process (get-buffer-process buffer)))
        (when (process-live-p process)
          (delete-process process)))
      (kill-buffer buffer)))
  (when
      (file-exists-p root)
    (delete-directory root t)))
"####;

fn agtags_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AGTAGS_MELPA_PIN, "agtags.el")
        .expect("prepare pinned agtags source below ./tmp")
        .with_prelude(AGTAGS_TEST_PRELUDE)
        .with_timeout(AGTAGS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed agtags parity test").into()
}

pub(crate) fn assert_agtags_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = agtags_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("agtags parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
