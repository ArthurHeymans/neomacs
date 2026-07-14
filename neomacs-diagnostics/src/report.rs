//! Turn Brendan-Gregg folded stacks into a ranked, token-bounded JSON report.
//!
//! This is the AI-agent-facing projection of a Lisp CPU capture: instead of a
//! flamegraph image or a raw multi-thousand-line folded blob, an agent gets the
//! top-N functions with self/total sample counts and percentages — the same
//! `self` vs `total` (cumulative) vocabulary every Lisp profiler uses.

use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// One ranked function in a CPU report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hotspot {
    pub function: String,
    /// Samples where this function was the leaf (executing directly).
    pub self_samples: u64,
    /// Samples where this function appeared anywhere on the stack.
    pub total_samples: u64,
    pub self_pct: f64,
    pub total_pct: f64,
}

/// A ranked CPU report derived from folded stacks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CpuReport {
    /// Total samples across all stacks (the denominator for percentages).
    pub total_samples: u64,
    /// Number of distinct collapsed stacks in the capture.
    pub distinct_stacks: usize,
    /// Top-N functions, ranked (self or total per `sort_by_self`).
    pub top: Vec<Hotspot>,
}

/// Build a ranked report from folded stacks.
///
/// `top_n` caps the returned hotspots (keeps the response token-bounded).
/// `sort_by_self` ranks by self (leaf) time; otherwise by total (cumulative).
pub fn cpu_report_from_folded(folded: &str, top_n: usize, sort_by_self: bool) -> CpuReport {
    let mut self_counts: HashMap<&str, u64> = HashMap::new();
    let mut total_counts: HashMap<&str, u64> = HashMap::new();
    let mut grand_total: u64 = 0;
    let mut distinct_stacks: usize = 0;

    for line in folded.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Each folded line is `frameA;frameB;frameC <count>`.
        let Some((stack, count_str)) = line.rsplit_once(' ') else {
            continue;
        };
        let Ok(count) = count_str.trim().parse::<u64>() else {
            continue;
        };
        let frames: Vec<&str> = stack.split(';').filter(|f| !f.is_empty()).collect();
        if frames.is_empty() {
            continue;
        }
        distinct_stacks += 1;
        grand_total += count;

        // Total: each distinct frame in the stack accrues `count` once (a
        // recursive frame appearing twice must not be double-counted).
        let mut seen: HashSet<&str> = HashSet::new();
        for frame in &frames {
            if seen.insert(frame) {
                *total_counts.entry(frame).or_default() += count;
            }
        }
        // Self: only the leaf frame was executing directly.
        let leaf = frames[frames.len() - 1];
        *self_counts.entry(leaf).or_default() += count;
    }

    // Every self key is also a total key (a leaf is on its own stack), so
    // ranging over total_counts covers all functions.
    let mut hotspots: Vec<Hotspot> = total_counts
        .iter()
        .map(|(func, &total)| {
            let self_samples = self_counts.get(func).copied().unwrap_or(0);
            Hotspot {
                function: (*func).to_string(),
                self_samples,
                total_samples: total,
                self_pct: pct(self_samples, grand_total),
                total_pct: pct(total, grand_total),
            }
        })
        .collect();

    hotspots.sort_by(|a, b| {
        let (pa, sa) = if sort_by_self {
            (a.self_samples, a.total_samples)
        } else {
            (a.total_samples, a.self_samples)
        };
        let (pb, sb) = if sort_by_self {
            (b.self_samples, b.total_samples)
        } else {
            (b.total_samples, b.self_samples)
        };
        // Descending by primary then secondary; function name breaks ties for
        // a deterministic ordering.
        pb.cmp(&pa)
            .then(sb.cmp(&sa))
            .then(a.function.cmp(&b.function))
    });
    hotspots.truncate(top_n);

    CpuReport {
        total_samples: grand_total,
        distinct_stacks,
        top: hotspots,
    }
}

fn pct(n: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (n as f64 / total as f64) * 100.0
    }
}
