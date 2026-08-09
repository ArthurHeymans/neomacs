use std::ops::Range;
use std::time::Duration;

use neomacs_tui_tests::{COLS, ExpectedDivergence, StrictGridOptions, assert_grids_strict};

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
    let options = StrictGridOptions {
        row_range: Some(covered_rows(&gnu_rows)),
        compare_faces: true,
        allow: vec![ExpectedDivergence {
            row: gnu_rows[0],
            col: COLS - 1,
            reason: "Neomacs does not extend vertico-current through the terminal edge cell",
        }],
        ..StrictGridOptions::default()
    };
    assert_grids_strict(
        "Vertico candidate grid",
        pair.gnu.screen(),
        pair.neo.screen(),
        &options,
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
