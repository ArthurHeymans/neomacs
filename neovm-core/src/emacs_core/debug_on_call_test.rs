//! Ledger 172: the `debug-on-next-call` / `debug-on-exit` arming handshake.
//!
//! Every expectation in this file was measured under GNU Emacs 31.0.90
//! (`emacs -Q --batch`) before it was written; the recording probe is
//! reproduced in each test's doc comment so a future reader can re-run it
//! rather than trust the constant.
//!
//! The shared shape is: bind `debugger` to a recorder, run a case, and read
//! back `(RESULT LOG debug-on-next-call)`.  `LOG` is the list of argument
//! lists the debugger was called with, oldest first -- so `((t) (exit 1))`
//! means "entry debugger with `Qt`, then exit debugger with the value 1".

use super::super::eval::Context;
use super::super::print::print_value;

/// `debugger` is a recorder; every case reports
/// `(RESULT LOG debug-on-next-call)`.
fn recorder_context() -> Context {
    let mut eval = Context::new();
    eval.eval_str(
        r#"(progn
             (defvar l172-log nil)
             (setq debugger (lambda (&rest args) (setq l172-log (cons args l172-log)) nil))
             nil)"#,
    )
    .expect("recorder setup should evaluate");
    eval
}

fn case(eval: &mut Context, body: &str) -> String {
    let form = format!(
        "(progn (setq l172-log nil)
                (let ((v (progn {body})))
                  (prog1 (list v (reverse l172-log) debug-on-next-call)
                    (setq debug-on-next-call nil))))"
    );
    let value = eval
        .eval_str(&form)
        .unwrap_or_else(|err| panic!("case should evaluate: {err:?}\n{form}"));
    print_value(&value)
}

/// GNU, `emacs -Q --batch`:
/// `(setq debug-on-next-call t) (car '(1 2 3))` => `(nil ((t) (exit 1)) nil)`.
///
/// Three separate facts in one line: the entry debugger fired with `Qt`
/// (`src/eval.c:2602`, `eval_sub`), the *same* frame fired again on the way out
/// (`src/eval.c:339` set `debug_on_exit`, `src/eval.c:2777` spends it), and the
/// flag reads `nil` afterwards because `do_debug_on_call` cleared it before
/// either of those happened (`src/eval.c:338`).  The result is `nil` and not
/// `1` because the exit debugger's return value REPLACES the call's value.
#[test]
fn arming_debug_on_next_call_enters_the_debugger_and_disarms_it() {
    let mut eval = recorder_context();
    assert_eq!(
        case(
            &mut eval,
            "(setq debug-on-next-call t)
             (car '(1 2 3))"
        ),
        "(nil ((t) (exit 1)) nil)"
    );
}

/// GNU: `(setq debug-on-next-call t) (car '(1 2)) (cdr '(3 4))`
/// => `((4) ((t) (exit 1)) nil)`.
///
/// One arm, one entry: the second call is not debugged.  This is the property
/// `do_debug_on_call`'s first line exists to provide.
#[test]
fn the_arm_is_one_shot() {
    let mut eval = recorder_context();
    assert_eq!(
        case(
            &mut eval,
            "(setq debug-on-next-call t)
             (car '(1 2))
             (cdr '(3 4))"
        ),
        "((4) ((t) (exit 1)) nil)"
    );
}

/// Ledger 135's and 168's probe, whole:
/// `(list (default-value 'debug-on-next-call)
///        (progn (set-default 'debug-on-next-call 5) (default-value 'debug-on-next-call))
///        (progn (setq debug-on-next-call t) debug-on-next-call))`
/// => `(nil nil t)` in GNU, `(nil t t)` here before this entry.
///
/// The middle `nil` is the mechanism: `set-default` coerces 5 to `t` through
/// `Lisp_Fwd_Bool` (`src/data.c:1485-1487`) and arms; the very next cons form
/// disarms before the read.  The third element is `t` in both editors because
/// `progn` and `setq` are special forms and a plain symbol never reaches
/// `eval_sub`'s cons arm (`src/eval.c:2560-2576`), so nothing intervenes.
#[test]
fn ledger_135_probe_answers_nil_in_the_middle() {
    let mut eval = recorder_context();
    let value = eval
        .eval_str(
            r#"(list (default-value 'debug-on-next-call)
                     (progn (set-default 'debug-on-next-call 5)
                            (default-value 'debug-on-next-call))
                     (progn (setq debug-on-next-call t) debug-on-next-call))"#,
        )
        .expect("probe should evaluate");
    assert_eq!(print_value(&value), "(nil nil t)");
}

/// GNU: `(setq debug-on-next-call t) (if t 'yes 'no)` => `(nil ((t) (exit yes)) nil)`.
///
/// `eval_sub` records its backtrace frame and tests the arm at `src/eval.c:2598-2602`,
/// which is *before* the `SUBRP`/`UNEVALLED` dispatch at `2621-2632`.  A special
/// form is therefore armed exactly like a function call.
#[test]
fn special_form_frames_are_armed_too() {
    let mut eval = recorder_context();
    assert_eq!(
        case(
            &mut eval,
            "(setq debug-on-next-call t)
             (if t 'yes 'no)"
        ),
        "(nil ((t) (exit yes)) nil)"
    );
}

/// GNU: `(setq debug-on-next-call t) (apply #'car '((5 6)))` => `(nil ((t) (exit 5)) nil)`.
///
/// `Fapply` has no arm check of its own; it reaches `Ffuncall`
/// (`src/eval.c:3192`).  Here the outer `apply` form is armed by `eval_sub`
/// first, so `Qt` is the code -- exactly as GNU answers.
#[test]
fn apply_is_armed_through_the_call_that_reaches_it() {
    let mut eval = recorder_context();
    assert_eq!(
        case(
            &mut eval,
            "(setq debug-on-next-call t)
             (apply #'car '((5 6)))"
        ),
        "(nil ((t) (exit 5)) nil)"
    );
}

/// GNU: arming, then signalling out of the armed call
/// => `(after ((t)) nil)`: the ENTRY debugger ran, the EXIT debugger did not.
///
/// `unbind_to` pops `SPECPDL_BACKTRACE` with a bare `break`
/// (`src/eval.c:3818-3820`), so a non-local exit never spends `debug_on_exit`.
/// The flag is still down afterwards, because the disarm happened on entry.
#[test]
fn a_signal_out_of_an_armed_call_runs_only_the_entry_debugger() {
    let mut eval = recorder_context();
    assert_eq!(
        case(
            &mut eval,
            "(condition-case nil
                 (progn (setq debug-on-next-call t) (error \"boom\"))
               (error nil))
             'after"
        ),
        "(after ((t)) nil)"
    );
}

/// GNU: `(let ((inhibit-debugger t)) (setq debug-on-next-call t) (car '(1)))`
/// => `(nil ((t) (exit 1)) nil)`.
///
/// `call_debugger` *binds* `inhibit-debugger` to `t` (`src/eval.c:309`) but
/// never tests it; only the signal path consults it (`src/eval.c` `maybe_call_debugger`).
/// So `inhibit-debugger` does not gate `debug-on-next-call`.
#[test]
fn inhibit_debugger_does_not_gate_the_entry_debugger() {
    let mut eval = recorder_context();
    assert_eq!(
        case(
            &mut eval,
            "(let ((inhibit-debugger t))
               (setq debug-on-next-call t)
               (car '(1)))"
        ),
        "(nil ((t) (exit 1)) nil)"
    );
}

/// GNU: a debugger returning `'REPLACED` makes the debugged call return
/// `REPLACED` => `(REPLACED ((t) (exit 1)) nil)`.
///
/// `val = call_debugger (list2 (Qexit, val))` (`src/eval.c:2778`) is an
/// assignment, so this is not incidental -- it is how `debug.el`'s
/// "return a value from this frame" works.
#[test]
fn the_debuggers_return_value_replaces_the_calls_value() {
    let mut eval = recorder_context();
    assert_eq!(
        case(
            &mut eval,
            "(let ((debugger (lambda (&rest args)
                               (setq l172-log (cons args l172-log))
                               'REPLACED)))
               (setq debug-on-next-call t)
               (car '(1 2 3)))"
        ),
        "(REPLACED ((t) (exit 1)) nil)"
    );
}

/// GNU: `(defun f (x) (backtrace-debug 1 t) (* x 3))` then `(f 7)`
/// => `(nil ((exit 21)) nil)`.
///
/// Level 1 is the caller of `backtrace-debug`, i.e. `f`'s own frame
/// (`Fbacktrace_debug`, `src/eval.c:4016-4029`).  `f` computes 21 and the exit
/// debugger replaces it with the recorder's `nil`.  No `debug-on-next-call` is
/// involved: this is the other half of the same mechanism.
#[test]
fn backtrace_debug_flags_the_named_frame_for_exit() {
    let mut eval = recorder_context();
    eval.eval_str("(defalias 'l172-exit-caller (lambda (x) (backtrace-debug 1 t) (* x 3)))")
        .expect("defun should evaluate");
    assert_eq!(
        case(&mut eval, "(l172-exit-caller 7)"),
        "(nil ((exit 21)) nil)"
    );
}

/// GNU: `(defun f (x) (backtrace-debug 0 t) (* x 3))` then `(f 7)`
/// => `(21 ((exit t)) nil)`.
///
/// Level 0 is `backtrace-debug`'s *own* frame -- `get_backtrace_starting_at
/// (Qnil)` is `backtrace_top ()` (`src/eval.c:3988`), which is the running
/// subr.  So the flagged frame exits immediately with `backtrace-debug`'s own
/// return value `t`, and `f` is untouched and answers 21.
#[test]
fn backtrace_debug_level_zero_flags_its_own_frame() {
    let mut eval = recorder_context();
    eval.eval_str("(defalias 'l172-exit-self (lambda (x) (backtrace-debug 0 t) (* x 3)))")
        .expect("defun should evaluate");
    assert_eq!(case(&mut eval, "(l172-exit-self 7)"), "(21 ((exit t)) nil)");
}

/// GNU: a flagged frame reports `(:debug-on-exit t)` in the walker's fourth
/// slot => `(nil ((exit (l172-flags (:debug-on-exit t)))) nil)`.
///
/// `backtrace_frame_apply` builds that list from the same bit
/// (`src/eval.c:4003-4005`), so the flag `backtrace-debug` sets and the flag
/// the walker reports must be one bit, not two.
#[test]
fn debug_on_exit_is_visible_to_backtrace_frames() {
    let mut eval = recorder_context();
    eval.eval_str(
        "(defalias 'l172-flags
           (lambda ()
             (backtrace-debug 1 t)
             (let (out)
               (backtrace-frame--internal
                (lambda (_evald func _args flags) (setq out (list func flags)) nil)
                0 'l172-flags)
               out)))",
    )
    .expect("defun should evaluate");
    assert_eq!(
        case(&mut eval, "(l172-flags)"),
        "(nil ((exit (l172-flags (:debug-on-exit t)))) nil)"
    );
}

/// GNU: `(catch 'x (f))` where `f` flags its caller and throws
/// => `(thrown nil nil)` -- the debugger is not called at all.
///
/// This is the one row of the whole probe table that already matched before
/// this entry, and it matched for the wrong reason (nothing was flagged).  It
/// has to keep matching now that flagging works.
#[test]
fn a_throw_out_of_a_flagged_frame_does_not_enter_the_debugger() {
    let mut eval = recorder_context();
    eval.eval_str("(defalias 'l172-throw (lambda () (backtrace-debug 1 t) (throw 'l172 'thrown)))")
        .expect("defun should evaluate");
    assert_eq!(
        case(&mut eval, "(catch 'l172 (l172-throw))"),
        "(thrown nil nil)"
    );
}

/// `backtrace-debug` with a nil FLAG clears the bit again, so the frame exits
/// silently.  GNU: `set_backtrace_debug_on_exit (pdl, !NILP (flag))`
/// (`src/eval.c:4026`) -- the setter is not "arm", it is "assign".
#[test]
fn backtrace_debug_with_a_nil_flag_clears_the_bit() {
    let mut eval = recorder_context();
    eval.eval_str(
        "(defalias 'l172-clear
           (lambda (x) (backtrace-debug 1 t) (backtrace-debug 1 nil) (* x 3)))",
    )
    .expect("defun should evaluate");
    assert_eq!(case(&mut eval, "(l172-clear 7)"), "(21 nil nil)");
}

/// A two-argument call is the one frame shape the specpdl stores without a
/// `debug_on_exit` field at all (`SpecBinding::Backtrace2`), so flagging it
/// has to promote the frame.  GNU has no such split -- every `bt` has the bit
/// -- which makes this a port-specific way to lose a debugger entry.
///
/// GNU: `(setq debug-on-next-call t) (cons 1 2)` => `(nil ((t) (exit (1 . 2))) nil)`.
#[test]
fn a_two_argument_frame_is_promoted_rather_than_losing_the_flag() {
    let mut eval = recorder_context();
    assert_eq!(
        case(
            &mut eval,
            "(setq debug-on-next-call t)
             (cons 1 2)"
        ),
        "(nil ((t) (exit (1 . 2))) nil)"
    );
}

/// GNU `call_debugger` records the input-event count it was entered at
/// (`src/eval.c:299`) in the `DEFVAR_INT` `internal-when-entered-debugger`
/// (`src/eval.c:4553-4554`).  Measured under `emacs -Q --batch`: `-1` at
/// startup (`init_eval`, `src/eval.c:251`) and `0` after one entry, because
/// batch never reads a non-macro input event.
///
/// Nothing here reads it back yet -- `maybe_call_debugger`'s
/// `when_entered_debugger < num_nonmacro_input_events` guard
/// (`src/eval.c:2212`) is a separate, behaviour-changing gap recorded in the
/// ledger -- but the slot belongs to `call_debugger`, so it is written where
/// GNU writes it.
#[test]
fn entering_the_debugger_stamps_internal_when_entered_debugger() {
    let mut eval = recorder_context();
    let before = eval
        .eval_str("internal-when-entered-debugger")
        .expect("startup value should read");
    assert_eq!(print_value(&before), "-1");
    let after = eval
        .eval_str(
            "(progn (setq debug-on-next-call t)
                    (car '(1))
                    (prog1 internal-when-entered-debugger
                      (setq debug-on-next-call nil)))",
        )
        .expect("probe should evaluate");
    assert_eq!(print_value(&after), "0");
}
