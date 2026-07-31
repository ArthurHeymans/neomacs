use std::time::Duration;

use crate::{ABS_MODE_MELPA_PIN, CachedMelpaOracle};

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod workflows;

const ABS_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

/// Real Abs models plus sandbox helpers shared by the workflows.
///
/// abs-mode is a front end for the `absc` compiler and for the Erlang, Maude
/// and Java runtimes, so the tests install a recording `absc` stand-in on PATH
/// and in `exec-path`.  It parses the same option vector the package builds,
/// produces the generated files each backend is expected to leave behind (the
/// files `abs--needs-compilation' looks for), and can report GNU-format
/// diagnostics; abs-mode itself keeps running its real command construction,
/// `compile', flymake and inferior-process path.
const ABS_MODE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defconst abs-test-bank-model
  (concat
   "module Bank;\n"
   "\n"
   "export Account, SavingsAccount;\n"
   "import * from Util;\n"
   "\n"
   "// Zinsen: 3 % pro Jahr — für Sparkonten\n"
   "data AccountId = AccountId(String label);\n"
   "type Balance = Rat;\n"
   "exception InsufficientFunds(Rat requested);\n"
   "\n"
   "interface Account {\n"
   "    Rat deposit(Rat amount);\n"
   "    Rat withdraw(Rat amount);\n"
   "}\n"
   "\n"
   "class SavingsAccount(Rat initial) implements Account {\n"
   "    Rat balance = initial;\n"
   "\n"
   "    Rat deposit(Rat amount) {\n"
   "        balance = balance + amount;\n"
   "        println(\"Überweisung €50 ✓\");\n"
   "        return balance;\n"
   "    }\n"
   "\n"
   "    Rat withdraw(Rat amount) {\n"
   "        if (amount > balance) { throw InsufficientFunds(amount); }\n"
   "        balance = balance - amount;\n"
   "        return balance;\n"
   "    }\n"
   "}\n"
   "\n"
   "def Rat total(List<Rat> amounts) =\n"
   "    case amounts {\n"
   "        Nil => 0;\n"
   "        Cons(x, rest) => x + total(rest);\n"
   "    };\n"
   "\n"
   "delta DFee;\n"
   "uses Bank;\n"
   "\n"
   "{\n"
   "    Account a = new SavingsAccount(100);\n"
   "    Fut<Rat> f = a!deposit(50);\n"
   "    Rat r = f.get;\n"
   "    println(toString(total(list[r])));\n"
   "}\n"))

(defconst abs-test-util-model
  (concat
   "module Util;\n"
   "\n"
   "export toBalance;\n"
   "\n"
   "def Rat toBalance(Int cents) = cents / 100;\n"))

(defconst abs-test-counter-model
  (concat
   "module Counter;\n"
   "[HTTPName: \"counter\"]\n"
   "class Counter(Int start) implements Countable {\n"
   "Int count = start;\n"
   "Unit inc() {\n"
   "count = count + 1;\n"
   "if (count > 10) {\n"
   "count = 0;\n"
   "}\n"
   "}\n"
   "Int classify(Int n) {\n"
   "case n {\n"
   "0 => return 0;\n"
   "_ => return 1;\n"
   "}\n"
   "}\n"
   "}\n"))

(defconst abs-test-ledger-model
  (concat
   "module Nav;\n"
   "\n"
   "// class LegacyLedger was removed in delta DFee\n"
   "interface Ledger {\n"
   "    Unit record(String note);\n"
   "}\n"
   "\n"
   "class FileLedger implements Ledger {\n"
   "    Unit record(String note) {\n"
   "        println(\"Reading class data from \" + note);\n"
   "    }\n"
   "}\n"
   "\n"
   "def Int one() = 1;\n"))

(defconst abs-test-timed-model
  (concat
   "module Timed;\n"
   "\n"
   "import * from Util;\n"
   "\n"
   "{\n"
   "    println(\"tick\");\n"
   "}\n"
   "\n"
   "// Local Variables:\n"
   "// abs-backend: maude\n"
   "// abs-clock-limit: 42\n"
   "// abs-default-resourcecost: 7\n"
   "// abs-input-files: (\"timed.abs\" \"helper.abs\")\n"
   "// abs-maude-output-file: \"timed.maude\"\n"
   "// abs-product-name: \"Deluxe\"\n"
   "// abs-compiler-program: \"/bin/false\"\n"
   "// abs-output-directory: \"../outside\"\n"
   "// End:\n"))

(defun abs-test-path (name)
  (expand-file-name name (getenv "NEOMACS_TEST_SANDBOX_ROOT")))

(defun abs-test-write (name text)
  "Write TEXT to sandbox file NAME and return its absolute path."
  (let ((path (abs-test-path name)))
    (make-directory (file-name-directory path) t)
    (with-temp-buffer
      (insert text)
      (write-region (point-min) (point-max) path nil 'silent))
    path))

(defun abs-test-open (name text)
  "Visit a sandbox Abs file holding TEXT and return its buffer."
  (find-file-noselect (abs-test-write name text)))

(defun abs-test-write-executable (name body)
  (let ((path (abs-test-path (concat "bin/" name))))
    (make-directory (file-name-directory path) t)
    (with-temp-buffer
      (insert body)
      (write-region (point-min) (point-max) path nil 'silent))
    (set-file-modes path #o755)
    path))

(defconst abs-test-absc-script
  (concat
   "#!/bin/sh\n"
   "printf '%s\\n' \"absc $*\" >> \"$ABS_LOG\"\n"
   "backend=erlang\n"
   "out=\n"
   "source=\n"
   "while [ $# -gt 0 ]; do\n"
   "  case \"$1\" in\n"
   "    *.abs) if [ -z \"$source\" ]; then source=\"$1\"; fi ;;\n"
   "    --erlang) backend=erlang ;;\n"
   "    --java) backend=java ;;\n"
   "    --maude) backend=maude ;;\n"
   "    --prolog) backend=prolog ;;\n"
   "    -o) out=\"$2\"; shift ;;\n"
   "  esac\n"
   "  shift\n"
   "done\n"
   "if [ -n \"$ABS_COMPILER_ERROR\" ]; then\n"
   "  printf '%s\\n' \"$ABS_COMPILER_ERROR\" | sed \"s|@SOURCE@|$source|g\"\n"
   "  exit 1\n"
   "fi\n"
   "case \"$backend\" in\n"
   "  erlang)\n"
   "    mkdir -p gen/erl/absmodel\n"
   "    printf '{\"absmodel/src/*\", [debug_info]}.\\n' > gen/erl/absmodel/Emakefile\n"
   "    printf '#!/bin/sh\\nprintf \"%%s\\\\n\" \"run $*\" >> \"$ABS_LOG\"\\necho \"Bank.Main terminated.\"\\n' > gen/erl/run\n"
   "    chmod +x gen/erl/run\n"
   "    ;;\n"
   "  java)\n"
   "    mkdir -p gen/ABS/StdLib\n"
   "    printf 'package ABS.StdLib;\\n' > gen/ABS/StdLib/Bool.java\n"
   "    ;;\n"
   "  maude)\n"
   "    if [ -n \"$out\" ]; then printf 'load abs-interpreter .\\n' > \"$out\"; fi\n"
   "    ;;\n"
   "esac\n"
   "echo \"Compiled $backend model.\"\n"
   "exit 0\n"))

(defun abs-test-setup-compiler ()
  "Install the recording `absc' stand-in for this sandbox."
  (abs-test-write-executable "absc" abs-test-absc-script)
  (setenv "ABS_LOG" (abs-test-path "commands.log"))
  (setenv "PATH" (concat (abs-test-path "bin") path-separator (getenv "PATH")))
  (add-to-list 'exec-path (abs-test-path "bin")))

(defun abs-test-commands ()
  "Return the exact command lines the stand-in executables recorded."
  (let ((log (abs-test-path "commands.log")))
    (if (file-exists-p log)
        (with-temp-buffer
          (insert-file-contents log)
          (split-string (buffer-string) "\n" t))
      'no-command-ran)))

(defvar abs-test-compilation-outcome nil)

(defun abs-test-compile (thunk)
  "Call THUNK and wait for the compilation it starts.
Return the buffer name and sentinel message `compilation-finish-functions'
received, so the workflow never races the compiler subprocess."
  (setq abs-test-compilation-outcome nil)
  (let ((compilation-finish-functions
         (list (lambda (buffer message)
                 (setq abs-test-compilation-outcome
                       (list (buffer-name buffer)
                             (string-trim-right message)))))))
    (funcall thunk)
    (let ((deadline (+ (float-time) 60)))
      (while (and (null abs-test-compilation-outcome)
                  (< (float-time) deadline))
        (accept-process-output nil 0.05))))
  abs-test-compilation-outcome)

(defun abs-test-wait-for-process (buffer)
  "Wait until BUFFER has no live process, then return its text."
  (let ((deadline (+ (float-time) 60)))
    (while (and (get-buffer-process buffer)
                (process-live-p (get-buffer-process buffer))
                (< (float-time) deadline))
      (accept-process-output nil 0.05)))
  (while (accept-process-output nil 0.05))
  (with-current-buffer buffer
    (buffer-substring-no-properties (point-min) (point-max))))

(defun abs-test-drain-processes ()
  "Wait until no subprocess is alive, then drain pending output and sentinels."
  (let ((deadline (+ (float-time) 60)))
    (while (and (cl-some #'process-live-p (process-list))
                (< (float-time) deadline))
      (accept-process-output nil 0.05)))
  (while (accept-process-output nil 0.05)))

(defun abs-test-compilation-text ()
  "Return the compilation buffer with its wall-clock stamps replaced."
  (with-current-buffer "*compilation*"
    (replace-regexp-in-string
     "^\\(Compilation .*?\\) at .*$" "\\1 at <TIME>"
     (buffer-substring-no-properties (point-min) (point-max)))))

(defun abs-test-face-runs (&optional beginning end)
  "Return the (TEXT . FACE) runs font lock produced in the current buffer."
  (font-lock-ensure)
  (let ((position (or beginning (point-min)))
        (limit (or end (point-max)))
        (runs nil))
    (while (< position limit)
      (let ((next (next-single-property-change position 'face nil limit))
            (face (get-text-property position 'face)))
        (when face
          (push (cons (buffer-substring-no-properties position next) face) runs))
        (setq position next)))
    (nreverse runs)))

(defun abs-test-index-positions (index)
  "Return imenu INDEX with every marker replaced by its buffer position."
  (mapcar
   (lambda (entry)
     (cond
      ((not (consp entry)) entry)
      ((and (consp (cdr entry)) (listp (cdr entry)))
       (cons (car entry) (abs-test-index-positions (cdr entry))))
      ((markerp (cdr entry)) (cons (car entry) (marker-position (cdr entry))))
      (t entry)))
   index))

(defun abs-test-flymake-diagnostics ()
  "Run one flymake check in the current buffer and return its diagnostics."
  (flymake-start)
  (abs-test-drain-processes)
  (mapcar (lambda (diagnostic)
            (list (flymake-diagnostic-type diagnostic)
                  (flymake-diagnostic-beg diagnostic)
                  (flymake-diagnostic-end diagnostic)
                  (line-number-at-pos (flymake-diagnostic-beg diagnostic))
                  (buffer-substring-no-properties
                   (flymake-diagnostic-beg diagnostic)
                   (flymake-diagnostic-end diagnostic))
                  (flymake-diagnostic-text diagnostic)))
          (flymake-diagnostics)))

(defun abs-test-normalize-temp-names (text)
  "Replace the random part of flymake's temporary copy names in TEXT."
  (replace-regexp-in-string "_[0-9]+_flymake" "_<TEMP>_flymake" text))

(defun abs-test-relative-files (directory)
  "Return every file below sandbox DIRECTORY, relative and sorted."
  (let ((directory (file-name-as-directory (abs-test-path directory))))
    (sort (mapcar (lambda (path) (file-relative-name path directory))
                  (directory-files-recursively directory ".*"))
          #'string<)))
"##;

fn abs_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ABS_MODE_MELPA_PIN, "abs-mode.el")
        .expect("prepare pinned abs-mode source below ./tmp")
        .with_prelude(ABS_MODE_TEST_PRELUDE)
        .with_timeout(ABS_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed abs-mode parity test")
        .into()
}

/// Multi-probe batch for `assert_abs_mode_parity` cases (2a).
pub(crate) fn assert_abs_mode_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(abs_mode_oracle(), &name, "abs_mode_parity", cases);
}

// BEGIN generated package batch tests

#[test]
fn abs_mode_package_batch() {
    let cases: Vec<ParityBatchCase> = [workflows::workflows_public_surface_batch_cases()]
        .into_iter()
        .flatten()
        .collect();
    assert_abs_mode_batch(&cases);
}

// END generated package batch tests
