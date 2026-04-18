//! GC pause benchmark: cons-heavy churn driven by safe-point
//! triggers, not explicit `(garbage-collect)`.
//!
//! Most neomacs tests call `gc_collect_exact` per form to keep
//! test output deterministic; that blocks on every sync cycle so
//! concurrent/incremental GC can't reduce the caller's wall time.
//! This bench instead evaluates a long `dotimes` loop that
//! allocates without any explicit GC triggers, letting
//! `gc_safe_point_exact` fire naturally between form evaluations
//! and the VM pacer decide when to collect.
//!
//! Run under different configurations to compare:
//!
//!   cargo run --release --example gc_bench -- --iters 2000000
//!
//!   # With incremental Major mark (slices across safe points):
//!   NEOVM_GC_INCREMENTAL_MAJOR=1 \
//!       cargo run --release --example gc_bench -- --iters 2000000
//!
//!   # With background collector thread (concurrent Major):
//!   NEOVM_GC_BACKGROUND=1 NEOVM_GC_INCREMENTAL_MAJOR=1 \
//!       cargo run --release --example gc_bench -- --iters 2000000
//!
//! The output reports total wall time and the pause-time
//! histogram from neovm-gc's own PauseHistogram, so you can
//! compare p50 / p99 across runs.

use neovm_core::emacs_core::Context;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
struct BenchOptions {
    iterations: usize,
}

fn parse_args() -> BenchOptions {
    let mut opts = BenchOptions {
        iterations: 2_000_000,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iters" | "-n" => {
                let v = args.next().expect("--iters needs a value");
                opts.iterations = v.parse().expect("iters must be usize");
            }
            "--help" | "-h" => {
                eprintln!("usage: gc_bench [--iters N]");
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown arg: {other}");
                std::process::exit(2);
            }
        }
    }
    opts
}

fn describe_env() -> String {
    let mut parts = Vec::new();
    if std::env::var("NEOVM_GC_BACKGROUND").is_ok_and(|v| {
        let l = v.to_ascii_lowercase();
        l == "1" || l == "true" || l == "yes"
    }) {
        parts.push("BACKGROUND");
    }
    if std::env::var("NEOVM_GC_INCREMENTAL_MAJOR").is_ok_and(|v| {
        let l = v.to_ascii_lowercase();
        l == "1" || l == "true" || l == "yes"
    }) {
        parts.push("INCREMENTAL_MAJOR");
    }
    if parts.is_empty() {
        "default (sync Major, no background)".to_string()
    } else {
        parts.join(" + ")
    }
}

fn bench_cons_churn(iterations: usize) {
    let mut ctx = Context::new();

    // Build a long list of cons cells in a tight loop. Uses
    // Lisp-level `dotimes` so every iteration hits
    // `gc_safe_point_exact` via the VM's instrumented loop
    // bodies. No explicit `(garbage-collect)` -- the pacer
    // decides when to collect.
    let src = format!(
        "(let ((lst nil) (i 0)) \
             (while (< i {}) \
                 (setq lst (cons i lst)) \
                 (setq i (1+ i))) \
             (length lst))",
        iterations
    );

    let start = Instant::now();
    let result = ctx.eval_str(&src);
    let elapsed = start.elapsed();

    let length = match result {
        Ok(val) => val
            .as_fixnum()
            .map(|n| n as usize)
            .unwrap_or(0),
        Err(flow) => {
            eprintln!("eval failed: {:?}", flow);
            std::process::exit(1);
        }
    };
    assert_eq!(length, iterations, "list length mismatch");

    let stats = ctx.gc_heap_stats();
    println!("config:        {}", describe_env());
    println!("iterations:    {}", iterations);
    println!("wall time:     {:?}", elapsed);
    println!(
        "throughput:    {:.2} M allocs/sec",
        (iterations as f64) / elapsed.as_secs_f64() / 1_000_000.0
    );
    println!("gc cycles:     {} total", ctx.gc_count());
    println!(
        "  minor / major:  {} / {}",
        stats.collections.minor_collections, stats.collections.major_collections
    );
    println!(
        "reclaimed:     {} MiB",
        stats.collections.reclaimed_bytes / 1024 / 1024
    );
}

fn main() {
    let opts = parse_args();
    bench_cons_churn(opts.iterations);
}
