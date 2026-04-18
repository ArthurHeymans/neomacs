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
    gc_threshold_bytes: Option<usize>,
}

fn parse_args() -> BenchOptions {
    let mut opts = BenchOptions {
        iterations: 2_000_000,
        gc_threshold_bytes: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iters" | "-n" => {
                let v = args.next().expect("--iters needs a value");
                opts.iterations = v.parse().expect("iters must be usize");
            }
            "--gc-threshold" => {
                let v = args.next().expect("--gc-threshold needs a value (bytes)");
                opts.gc_threshold_bytes = Some(v.parse().expect("threshold must be usize"));
            }
            "--help" | "-h" => {
                eprintln!("usage: gc_bench [--iters N] [--gc-threshold BYTES]");
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

fn bench_cons_churn(opts: BenchOptions) {
    let iterations = opts.iterations;
    let mut ctx = Context::new();
    if let Some(threshold) = opts.gc_threshold_bytes {
        ctx.set_gc_threshold(threshold);
    }

    // Build a long list of cons cells in a tight loop. Uses
    // Lisp-level `dotimes` so every iteration hits
    // `gc_safe_point_exact` via the VM's instrumented loop
    // bodies. No explicit `(garbage-collect)` -- the pacer
    // decides when to collect.
    // Build a short-lived cons in each iteration and let it
    // become unreachable the next time through the loop, so the
    // workload generates maximum garbage for the collector to
    // reclaim. The outer `last` binding keeps the final cons
    // alive so we can sanity-check the run completed.
    let src = format!(
        "(let ((last nil) (i 0)) \
             (while (< i {}) \
                 (setq last (cons i nil)) \
                 (setq i (1+ i))) \
             (car last))",
        iterations
    );

    let start = Instant::now();
    let result = ctx.eval_str(&src);
    let elapsed = start.elapsed();

    let final_i = match result {
        Ok(val) => val.as_fixnum().map(|n| n as usize).unwrap_or(usize::MAX),
        Err(flow) => {
            eprintln!("eval failed: {:?}", flow);
            std::process::exit(1);
        }
    };
    assert_eq!(final_i, iterations - 1, "final cons payload mismatch");

    let stats = ctx.gc_heap_stats();
    let hist = ctx.gc_pause_histogram();
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
    println!("pause stats ({} samples):", hist.sample_count);
    println!("  min:   {:.2} ms", (hist.min_nanos as f64) / 1_000_000.0);
    println!("  p50:   {:.2} ms", (hist.p50_nanos as f64) / 1_000_000.0);
    println!("  p95:   {:.2} ms", (hist.p95_nanos as f64) / 1_000_000.0);
    println!("  p99:   {:.2} ms", (hist.p99_nanos as f64) / 1_000_000.0);
    println!("  max:   {:.2} ms", (hist.max_nanos as f64) / 1_000_000.0);
    println!("  mean:  {:.2} ms", (hist.mean_nanos as f64) / 1_000_000.0);
}

fn main() {
    let opts = parse_args();
    bench_cons_churn(opts);
}
