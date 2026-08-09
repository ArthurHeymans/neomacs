use std::ops::Range;
use std::time::Duration;

use expect_test::expect;
use neomacs_tui_tests::{RawTerminalSnapshot, assert_raw_terminal_snapshots_eq};

use crate::{COMPAT_GNU_ELPA_PIN, CachedMelpaOracle, VERTICO_MELPA_PIN};

use super::support::PackageTuiPair;

const VERTICO_TUI_PRELUDE: &str = r#"
(require 'vertico)
(setq vertico-count 5
      vertico-cycle t)
(vertico-mode 1)
(dolist (fixture '(("project-alpha" . "ALPHA BUFFER\n")
                   ("project-beta" . "BETA BUFFER\n")
                   ("project-notes" . "NOTES BUFFER\n")))
  (with-current-buffer (get-buffer-create (car fixture))
    (erase-buffer)
    (insert (cdr fixture))))
"#;

fn candidate_rows(grid: &[String]) -> Vec<u16> {
    grid.iter()
        .enumerate()
        .filter_map(|(row, contents)| {
            contents
                .trim_start()
                .starts_with("project-")
                .then_some(row as u16)
        })
        .collect()
}

fn covered_rows(rows: &[u16]) -> Range<u16> {
    let first = *rows.first().expect("Vertico rendered package candidates");
    let last = *rows.last().expect("Vertico rendered package candidates");
    first..last + 1
}

#[test]
fn vertico_real_minibuffer_candidates_and_selection_match_gnu_grid() {
    let oracle = CachedMelpaOracle::new(VERTICO_MELPA_PIN, "vertico.el")
        .expect("prepare revision-pinned Vertico source")
        .with_gnu_elpa_dependency(COMPAT_GNU_ELPA_PIN)
        .expect("prepare exact Compat dependency")
        .with_prelude(VERTICO_TUI_PRELUDE);
    let mut pair = PackageTuiPair::spawn("vertico-minibuffer", oracle.prepared_packages())
        .expect("spawn package TUI pair");

    let ready = |grid: &[String]| grid.iter().any(|row| row.contains("*scratch*"));
    pair.gnu.read_until(Duration::from_secs(15), ready);
    pair.neo.read_until(Duration::from_secs(20), ready);

    for session in [&mut pair.gnu, &mut pair.neo] {
        session.send_keys("C-x b");
        session.read_until(Duration::from_secs(8), |grid| {
            grid.iter().any(|row| row.contains("Switch to buffer"))
        });
        session.send(b"project-");
        session.read_until(Duration::from_secs(8), |grid| {
            candidate_rows(grid).len() >= 3
        });
    }

    let gnu_rows = candidate_rows(&pair.gnu.text_grid());
    let neo_rows = candidate_rows(&pair.neo.text_grid());
    assert_eq!(neo_rows, gnu_rows, "Vertico candidate rows differ from GNU");
    let candidate_rows = covered_rows(&gnu_rows);
    let gnu_snapshot = RawTerminalSnapshot::capture_rows(pair.gnu.screen(), candidate_rows.clone());
    let neo_snapshot = RawTerminalSnapshot::capture_rows(pair.neo.screen(), candidate_rows);

    let expected_ansi_grid = expect![[r#"
        [0;38;2;173;216;230;48;2;85;107;47mproject-[0;1;48;2;85;107;47mb[0;48;2;85;107;47meta                                                                                                                                                    [0m
        [0;38;2;173;216;230mproject-[0;1ma[0mlpha                                                                                                                                                   [0m
        [0;38;2;173;216;230mproject-[0;1mn[0motes                                                                                                                                                   [0m
    "#]];
    expected_ansi_grid.assert_eq(&gnu_snapshot.ansi_grid());
    let expected_plain_grid = expect![[r#"
        45 |project-beta␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠␠|
        46 |project-alpha∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅|
        47 |project-notes∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅∅|
    "#]];
    expected_plain_grid.assert_eq(&gnu_snapshot.plain_grid());

    assert_raw_terminal_snapshots_eq(
        "Vertico candidate terminal state",
        &gnu_snapshot,
        &neo_snapshot,
    );

    for session in [&mut pair.gnu, &mut pair.neo] {
        session.send(b"beta");
        session.send_key("RET");
        session.read_until(Duration::from_secs(8), |grid| {
            grid.iter().any(|row| row.contains("BETA BUFFER"))
        });
        assert!(
            session
                .text_grid()
                .iter()
                .any(|row| row.contains("BETA BUFFER")),
            "{} did not select the real project-beta buffer",
            session.name
        );
    }
}
