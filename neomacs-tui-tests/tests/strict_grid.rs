//! Prototype: strict, contract-level grid comparison of Neomacs vs GNU Emacs.
//!
//! Unlike the fuzzy/text-only helpers, this compares the *exact* character grid
//! plus face *identity* (a palette-independent colour-class partition — see
//! `compare_grids_strict`), over the text area, with chrome rows masked and an
//! explicit allow-list for known parity gaps.

use neomacs_tui_tests::*;
use std::time::Duration;

/// A fresh `-Q` buffer with deterministic typed text must render an *identical*
/// character grid in the text area (logical layout), and the same trivial
/// face-class partition (all default — plain text), as GNU Emacs.
///
/// This is the strongest, least-flaky form of strictness: the buffer content
/// is fully deterministic, chrome (mode-line + echo) is masked, and the only
/// thing under test is whether the two editors lay out the same characters at
/// the same cells with the same logical faces.
#[test]
fn strict_text_area_matches_gnu_for_typed_buffer() {
    let mut gnu = TuiSession::gnu_emacs("");
    let mut neo = TuiSession::neomacs("");
    gnu.read(Duration::from_secs(2));
    neo.read(Duration::from_secs(2));

    // Move to a fresh empty buffer (avoids the divergent *scratch* message),
    // type deterministic content, then return to the top of the buffer.
    for s in [&mut gnu, &mut neo] {
        s.send(b"\x18bstrict-grid\r"); // C-x b strict-grid RET
        s.read(Duration::from_millis(800));
        s.send(b"alpha\rbeta gamma\rdelta epsilon zeta\r"); // three lines
        s.read(Duration::from_millis(800));
        s.send(b"\x1b<"); // M-< : beginning of buffer
        s.read(Duration::from_millis(800));
    }

    // Mask the bottom two rows (mode-line + echo area): they legitimately
    // diverge (version string, buffer/position indicators) and are not part of
    // the logical-display contract for this fixture.
    let opts = StrictGridOptions {
        masked_rows: ((ROWS - 2)..ROWS).collect(),
        row_range: Some(0..(ROWS - 2)),
        compare_faces: true,
        // Calibrated below from the first real run, if needed.
        allow: Vec::new(),
    };

    assert_grids_strict("typed buffer text area", gnu.screen(), neo.screen(), &opts);
}
