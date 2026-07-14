use crate::report::cpu_report_from_folded;

// Fixture stacks:
//   a;b;c 10   a;b 5   a;d 3
// totals: a=18 (all), b=15 (first two), c=10, d=3
// self:   c=10, b=5, d=3, a=0
const FIXTURE: &str = "a;b;c 10\na;b 5\na;d 3";

#[test]
fn ranks_by_self_time() {
    let r = cpu_report_from_folded(FIXTURE, 10, true);
    assert_eq!(r.total_samples, 18);
    assert_eq!(r.distinct_stacks, 3);
    assert_eq!(r.top[0].function, "c");
    assert_eq!(r.top[0].self_samples, 10);
    assert_eq!(r.top[0].total_samples, 10);

    let a = r.top.iter().find(|h| h.function == "a").unwrap();
    assert_eq!(a.total_samples, 18);
    assert_eq!(a.self_samples, 0);
    assert!((a.total_pct - 100.0).abs() < 1e-9);
}

#[test]
fn ranks_by_total_time_with_top_n_cap() {
    let r = cpu_report_from_folded(FIXTURE, 2, false);
    assert_eq!(r.top.len(), 2);
    assert_eq!(r.top[0].function, "a"); // total 18
    assert_eq!(r.top[1].function, "b"); // total 15
}

#[test]
fn recursive_frame_not_double_counted_in_total() {
    // `a` appears twice in one stack; total for `a` must be 7 (the stack count),
    // not 14.
    let r = cpu_report_from_folded("a;a;leaf 7", 10, false);
    let a = r.top.iter().find(|h| h.function == "a").unwrap();
    assert_eq!(a.total_samples, 7);
    assert_eq!(a.self_samples, 0);
    let leaf = r.top.iter().find(|h| h.function == "leaf").unwrap();
    assert_eq!(leaf.self_samples, 7);
}

#[test]
fn empty_and_malformed_input() {
    let r = cpu_report_from_folded("", 10, true);
    assert_eq!(r.total_samples, 0);
    assert!(r.top.is_empty());

    // Lines without a trailing count are skipped.
    let r2 = cpu_report_from_folded("no-count-here\nvalid;stack 4", 10, true);
    assert_eq!(r2.total_samples, 4);
    assert_eq!(r2.distinct_stacks, 1);
}
