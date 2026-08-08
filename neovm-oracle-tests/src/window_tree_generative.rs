//! Generative window-tree parity: random operation sequences replayed in GNU
//! Emacs and neomacs, comparing the resulting `(window-tree)` after **every**
//! step.
//!
//! # Why generative
//!
//! Hand-written window-tree cases only cover the bugs someone already thought
//! of. Two concrete misses from the session that produced this file:
//!
//! - A 29-scenario hand-written battery reached byte-identical parity with GNU
//!   while `recombine_windows` was still entirely absent from the delete path.
//! - The first attempt to hand-write a reproducer for that gap *appeared to
//!   pass*, because the nesting was built with `window-combination-limit t` —
//!   which seals the new parent, and GNU skips sealed nodes. Only an
//!   *orthogonal* split produces the unsealed combination that triggers the
//!   merge.
//!
//! Neither is something you write a targeted test for before you understand the
//! bug; both fall out of random op sequences immediately.
//!
//! # What is compared
//!
//! For each step: whether the operation succeeded or signalled, and the full
//! window tree (combination direction, buffer names, window edges). Error
//! *messages* are deliberately not compared — the fact of an error is a
//! behavioural contract, the wording is not (neomacs embeds frame pointers in
//! some window errors, which would be pure noise here).
//!
//! Comparing after every step, rather than only at the end, means a failure
//! reports the first diverging operation instead of a final tree that has to be
//! reverse-engineered.

use proptest::prelude::*;

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;
use crate::common::{ORACLE_PROP_CASES, eval_oracle_and_neovm};

/// How many random sequences to run.
///
/// Defaults to the suite-wide [`ORACLE_PROP_CASES`], which is deliberately low
/// so the whole oracle corpus stays fast. Raise it with
/// `NEOVM_WINDOW_PROP_CASES` to hunt: this generator explores a far larger
/// space than a single scalar form does, so it earns a deeper run than a
/// typical parity property.
fn window_prop_cases() -> u32 {
    std::env::var("NEOVM_WINDOW_PROP_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(ORACLE_PROP_CASES)
}

/// The side argument of a split, and the edge of a side window.
#[derive(Debug, Clone, Copy)]
enum Side {
    Left,
    Right,
    Above,
    Below,
}

impl Side {
    fn split_arg(self) -> &'static str {
        match self {
            Side::Left => "'left",
            Side::Right => "'right",
            Side::Above => "'above",
            Side::Below => "'below",
        }
    }

    /// `window-side` parameter value; `above`/`below` are spelled
    /// `top`/`bottom` for side windows.
    fn side_window_arg(self) -> &'static str {
        match self {
            Side::Left => "'left",
            Side::Right => "'right",
            Side::Above => "'top",
            Side::Below => "'bottom",
        }
    }
}

fn side_strategy() -> impl Strategy<Value = Side> {
    prop_oneof![
        Just(Side::Left),
        Just(Side::Right),
        Just(Side::Above),
        Just(Side::Below),
    ]
}

/// The `window-combination-resize` binding in force for one operation.
///
/// This is a *policy* variable read by `window.el`, which stages the resulting
/// sizes for the primitive to commit. `t` makes a split take its space from
/// every sibling proportionally rather than only from the split target, and
/// makes a delete give the freed space back the same way.
#[derive(Debug, Clone, Copy)]
enum Resize {
    Nil,
    T,
    Side,
}

impl Resize {
    fn elisp(self) -> &'static str {
        match self {
            Resize::Nil => "nil",
            Resize::T => "t",
            Resize::Side => "'side",
        }
    }
}

fn resize_strategy() -> impl Strategy<Value = Resize> {
    // `nil` is the default and must stay the common case; `side` is what
    // `window--make-major-side-window` binds.
    prop_oneof![4 => Just(Resize::Nil), 2 => Just(Resize::T), 1 => Just(Resize::Side)]
}

/// One window-tree mutation. `window` indexes into
/// `(window-list nil 'no-minibuf nil)` modulo its length, so every op applies
/// to *some* live window regardless of how the tree has evolved.
#[derive(Debug, Clone, Copy)]
enum WindowOp {
    /// `split-window`, with `window-combination-limit` and
    /// `window-combination-resize` bound around it.
    Split {
        window: usize,
        side: Side,
        limit: bool,
        resize: Resize,
    },
    /// `delete-window`, with `window-combination-resize` bound around it.
    Delete { window: usize, resize: Resize },
    /// `display-buffer-in-side-window` on the given edge.
    SideWindow { side: Side, slot: i8 },
    /// `set-window-combination-limit` on a window's parent — the slot that
    /// decides whether a delete may recombine it.
    SealParent { window: usize, value: bool },
    /// `delete-other-windows`.
    DeleteOtherWindows { window: usize },
    /// `balance-windows`.
    Balance,
}

fn op_strategy() -> impl Strategy<Value = WindowOp> {
    prop_oneof![
        // Splits dominate: they are what builds interesting trees.
        4 => (0usize..8, side_strategy(), any::<bool>(), resize_strategy())
            .prop_map(|(window, side, limit, resize)| WindowOp::Split { window, side, limit, resize }),
        3 => (0usize..8, resize_strategy())
            .prop_map(|(window, resize)| WindowOp::Delete { window, resize }),
        3 => (side_strategy(), -1i8..2)
            .prop_map(|(side, slot)| WindowOp::SideWindow { side, slot }),
        1 => (0usize..8, any::<bool>())
            .prop_map(|(window, value)| WindowOp::SealParent { window, value }),
        1 => (0usize..8).prop_map(|window| WindowOp::DeleteOtherWindows { window }),
        1 => Just(WindowOp::Balance),
    ]
}

/// The elisp body for one op. `step` disambiguates side-window buffer names so
/// a later side window does not silently reuse an earlier one's slot.
fn op_elisp(op: WindowOp, step: usize) -> String {
    match op {
        WindowOp::Split {
            window,
            side,
            limit,
            resize,
        } => format!(
            "(let ((window-combination-limit {}) (window-combination-resize {})) \
               (split-window (oracle--nth-window {window}) nil {}))",
            if limit { "t" } else { "nil" },
            resize.elisp(),
            side.split_arg(),
        ),
        WindowOp::Delete { window, resize } => format!(
            "(let ((window-combination-resize {})) \
               (delete-window (oracle--nth-window {window})))",
            resize.elisp(),
        ),
        WindowOp::SideWindow { side, slot } => format!(
            "(display-buffer (get-buffer-create \"*side-{step}*\") \
               (list 'display-buffer-in-side-window \
                     (cons 'side {}) (cons 'slot {slot})))",
            side.side_window_arg(),
        ),
        WindowOp::SealParent { window, value } => format!(
            "(set-window-combination-limit \
               (window-parent (oracle--nth-window {window})) {})",
            if value { "t" } else { "nil" },
        ),
        WindowOp::DeleteOtherWindows { window } => {
            format!("(delete-other-windows (oracle--nth-window {window}))")
        }
        WindowOp::Balance => "(balance-windows)".to_string(),
    }
}

/// Build the full elisp program: run each op inside `condition-case`, and after
/// each one record `ok`/`err` plus the whole tree.
fn program(ops: &[WindowOp]) -> String {
    let mut src = String::from(
        r#"
(defun oracle--wt (node)
  (cond
   ((windowp node)
    (cons (buffer-name (window-buffer node)) (window-edges node)))
   ((consp node)
    (cons (if (car node) 'v 'h) (mapcar #'oracle--wt (cddr node))))))
(defun oracle--nth-window (n)
  (let ((ws (window-list nil 'no-minibuf nil)))
    (nth (mod n (length ws)) ws)))
(defvar oracle--log nil)
(defmacro oracle--step (&rest body)
  `(setq oracle--log
         (cons (list (condition-case nil (progn ,@body 'ok) (error 'err))
                     (oracle--wt (car (window-tree))))
               oracle--log)))
"#,
    );
    for (step, op) in ops.iter().enumerate() {
        src.push_str(&format!("(oracle--step {})\n", op_elisp(*op, step)));
    }
    src.push_str("(nreverse oracle--log)\n");
    src
}

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(window_prop_cases()))]

    /// Random split/delete/side-window/seal/balance sequences must leave GNU and
    /// neomacs with identical window trees at every step.
    ///
    /// # Ignored by default — this is a hunting tool, not a gate
    ///
    /// It still finds KNOWN-OPEN divergences, so it cannot gate CI yet. And at
    /// the default [`ORACLE_PROP_CASES`] it would be *flaky* rather than
    /// cleanly red: the shallow run usually misses them. A flaky test is worse
    /// than an ignored one.
    ///
    /// Run it explicitly, deep:
    ///
    /// ```text
    /// NEOVM_FORCE_ORACLE_PATH=/path/to/emacs NEOVM_ORACLE_MODE=live \
    /// NEOVM_WINDOW_PROP_CASES=400 \
    /// cargo nextest run --release -p neovm-oracle-tests --run-ignored all \
    ///   -E 'test(oracle_prop_window_tree_survives_random_operation_sequences)' --no-capture
    /// ```
    ///
    /// Known-open finding (5 ops, surfaces around a few hundred cases):
    /// deleting a window on a frame that has `top` and `left` side windows
    /// leaves a different tree than GNU — the top side window disappears.
    ///
    /// Un-ignore once the corpus runs clean at a few hundred cases, and
    /// consider wiring the deep run into a nightly job rather than per-commit
    /// CI.
    #[test]
    #[ignore = "hunting tool: finds known-open divergences; run explicitly with NEOVM_WINDOW_PROP_CASES"]
    fn oracle_prop_window_tree_survives_random_operation_sequences(
        ops in prop::collection::vec(op_strategy(), 2..9),
    ) {
        return_if_neovm_enable_oracle_proptest_not_set!(Ok(()));

        let form = program(&ops);
        let (oracle, neovm) = eval_oracle_and_neovm(&form);
        prop_assert_eq!(
            &oracle,
            &neovm,
            "window tree diverged from GNU for op sequence {:?}\nprogram:\n{}",
            ops,
            form
        );
    }
}
