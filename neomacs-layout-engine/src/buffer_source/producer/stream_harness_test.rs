//! Phase-4 stream-equivalence harness: the shadow spine of the producer
//! inversion.
//!
//! For each corpus case the harness drives TWO legs over the same buffer and
//! asserts they produce the identical element stream on the assertion surface
//! from tmp/p4-test-inventory.md section 1 (char, provenance, scan before and
//! after, face-ref, class):
//!
//! * the PRODUCER leg — [`BufferElementProducer`] consumption straight through,
//!   runs yielded whole (as rungs land this becomes `next_element`);
//! * the PIPELINE-REPLAY leg — the same producer with the renderer's split
//!   feeders applied, using the REAL splitters
//!   (`DisplaySourceStepItem::split_text_run_items`, the per-char feeder at
//!   char_render.rs:111-117, and `split_text_run_at_charpos`, the fit split at
//!   item_render.rs:371-389) pushed back through the pending queue exactly as
//!   the renderer does.
//!
//! Runs are expanded to per-character observations before comparison, so the
//! legs agree only if splitting is STREAM-TRANSPARENT: same characters, same
//! provenance, same scan positions, same faces, same order. That is precisely
//! the invariant P4.3-P4.5 must preserve while they delete the feeders, which
//! is why this file lands before them.
//!
//! WHAT THE PRODUCER OWNS AT THIS RUNG — measured with a probe, not assumed:
//! it yields whole text runs, row breaks and replacement items with buffer
//! provenance, and it DOES terminate runs at a resolvable face boundary. It
//! does NOT elide invisible text, expand tabs, or emit truncation marks; those
//! are renderer-owned today (the invisible checkpoint at row_lifecycle.rs:694+,
//! `DisplayTabPolicy::advance_from`, special_glyphs.rs). Corpus cases whose
//! inventory expectation depends on
//! producer-owned stop state are landed `#[ignore]` naming the rung that will
//! un-ignore them, so the checklist lives in the code rather than a document.

use super::vocabulary::{BufferScanPos, GlyphProvenance, ProducedElement};
use super::*;
use crate::buffer_source::consumption::BufferSourceConsumedItem;
use crate::display_item::RenderFaceRef;
use crate::display_row::metrics::DisplayRowFallbackMetrics;
use crate::display_source::DisplaySourceTextPosition;
use crate::display_source_resolver::DisplaySourceFaceBasis;
use crate::frame_face_arena::FrameFaceAttempt;
use crate::neovm_bridge::{FaceResolver, LayoutBufferSnapshot, ResolvedFace};
use neomacs_display_protocol::types::FaceId;
use neovm_core::buffer::{BufferId, CharPos0, EmacsByteRange};
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::FaceTable;

const BASE_FACE: FaceId = FaceId::new(1);

/// Enough elements to drain every corpus case to end of text.
const DRAIN_LIMIT: usize = 128;

// ---------------------------------------------------------------------------
// Corpus rows
// ---------------------------------------------------------------------------

/// One text property applied to a half-open character range of a corpus
/// buffer. The Lisp value is built inside the fixture's `Context`, not here:
/// constructing a list allocates on the thread's Lisp heap, which only exists
/// while a `Context` does.
struct CaseProperty {
    start_char: usize,
    end_char: usize,
    kind: CasePropertyKind,
}

enum CasePropertyKind {
    /// An ANONYMOUS face (an attribute plist), not a named one: a named face
    /// only resolves when the face table happens to define it, which made this
    /// corpus depend on which Lisp state the build had dumped. A plist always
    /// resolves, so the seam is deterministic.
    Face {
        attribute: &'static str,
        value: &'static str,
    },
    Invisible,
}

impl CaseProperty {
    fn face(
        start_char: usize,
        end_char: usize,
        attribute: &'static str,
        value: &'static str,
    ) -> Self {
        Self {
            start_char,
            end_char,
            kind: CasePropertyKind::Face { attribute, value },
        }
    }

    fn invisible(start_char: usize, end_char: usize) -> Self {
        Self {
            start_char,
            end_char,
            kind: CasePropertyKind::Invisible,
        }
    }

    fn name(&self) -> &'static str {
        match self.kind {
            CasePropertyKind::Face { .. } => "face",
            CasePropertyKind::Invisible => "invisible",
        }
    }

    fn value(&self) -> Value {
        match self.kind {
            CasePropertyKind::Face { attribute, value } => {
                Value::list(vec![Value::symbol(attribute), Value::symbol(value)])
            }
            CasePropertyKind::Invisible => Value::t(),
        }
    }
}

/// A corpus row: buffer content plus the properties that create its seam.
struct StreamCase {
    name: &'static str,
    text: &'static str,
    properties: Vec<CaseProperty>,
}

impl StreamCase {
    fn new(name: &'static str, text: &'static str) -> Self {
        Self {
            name,
            text,
            properties: Vec::new(),
        }
    }

    fn with(mut self, property: CaseProperty) -> Self {
        self.properties.push(property);
        self
    }

    /// A face property per character: the C3 shape that forces the pipeline to
    /// render a line character by character.
    fn per_char_faces(mut self, faces: &[(&'static str, &'static str)]) -> Self {
        for (index, (attribute, value)) in faces.iter().enumerate() {
            self.properties
                .push(CaseProperty::face(index, index + 1, attribute, value));
        }
        self
    }
}

// ---------------------------------------------------------------------------
// The assertion surface
// ---------------------------------------------------------------------------

/// The element classes the assertion surface distinguishes. Classes the
/// producer cannot yet compute (Wide, ComposedExtender) arrive with the rung
/// that gives `ProducedChar` its char class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ElementClass {
    PlainChar,
    Tab,
    RowBreak,
    Stretch,
    Replacement,
}

/// One character's worth of the element stream. A run contributes one of these
/// per character, so a split run and a whole run are directly comparable — the
/// property the whole harness rests on.
#[derive(Clone, Debug, PartialEq)]
struct CharObservation {
    class: ElementClass,
    ch: Option<char>,
    provenance: GlyphProvenance,
    scan_before: BufferScanPos,
    scan_after: BufferScanPos,
    face: RenderFaceRef,
}

impl CharObservation {
    fn chars(stream: &[Self]) -> String {
        stream
            .iter()
            .filter_map(|observation| observation.ch)
            .collect()
    }
}

fn char_class(ch: char) -> ElementClass {
    if ch == '\t' {
        ElementClass::Tab
    } else {
        ElementClass::PlainChar
    }
}

/// Expand one consumed item into per-character observations.
fn observe(item: &BufferSourceConsumedItem, scan_before: BufferScanPos) -> Vec<CharObservation> {
    let BufferSourceConsumedItem::Renderable(step) = item else {
        // A display-property replacement is one opaque element at this rung;
        // decomposing its covered range is P4.7's business.
        return vec![CharObservation {
            class: ElementClass::Replacement,
            ch: None,
            provenance: GlyphProvenance::buffer(CharPos0::new(
                scan_before.charpos().max(0) as usize
            )),
            scan_before,
            scan_after: scan_before,
            face: RenderFaceRef::Inherit,
        }];
    };
    let Some(element) = ProducedElement::from_step_item(step) else {
        // Kinds the P4.1 vocabulary does not model yet (glyphless, media).
        return Vec::new();
    };
    match &element {
        ProducedElement::Run(run) => {
            let mut observations = Vec::new();
            let mut scan = scan_before;
            for (offset, ch) in run.text().chars().enumerate() {
                let scan_after = BufferScanPos::new(
                    scan.byte_idx() + ch.len_utf8(),
                    scan.charpos().saturating_add(1),
                );
                observations.push(CharObservation {
                    class: char_class(ch),
                    ch: Some(ch),
                    provenance: run.glyph_provenance(offset),
                    scan_before: scan,
                    scan_after,
                    face: run.face(),
                });
                scan = scan_after;
            }
            observations
        }
        ProducedElement::Char(produced) => vec![CharObservation {
            class: char_class(produced.ch()),
            ch: Some(produced.ch()),
            provenance: produced.position().stamp(),
            scan_before,
            scan_after: BufferScanPos::new(
                scan_before.byte_idx() + produced.ch().len_utf8(),
                scan_before.charpos().saturating_add(1),
            ),
            face: produced.face(),
        }],
        ProducedElement::RowBreak(row_break) => vec![CharObservation {
            class: ElementClass::RowBreak,
            ch: Some('\n'),
            provenance: row_break.position().stamp(),
            scan_before,
            scan_after: BufferScanPos::new(
                scan_before.byte_idx() + 1,
                scan_before.charpos().saturating_add(1),
            ),
            face: RenderFaceRef::Inherit,
        }],
        ProducedElement::Stretch(stretch) => vec![CharObservation {
            class: ElementClass::Stretch,
            ch: None,
            provenance: stretch.position().stamp(),
            scan_before,
            scan_after: scan_before,
            face: stretch.face(),
        }],
        ProducedElement::EndOfText => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// The two legs
// ---------------------------------------------------------------------------

/// How a leg splits what the producer hands it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SplitPolicy {
    /// Producer leg: whole runs, no feeders.
    None,
    /// The per-char feeder (char_render.rs:111-117): every multi-char run is
    /// split, the first character is consumed now and the remainder goes back
    /// on the pending queue.
    PerChar,
    /// The fit split as it was before P4.3: a run crossing `charpos` is cut
    /// there and the tail is queued for a later iteration to pop.
    FitAt(i64),
    /// P4.3's replacement: cut the run at `charpos`, consume only the prefix,
    /// and reseat the producer there instead of queueing the tail.
    PrefixAt(i64),
}

/// Buffer plus face machinery for one corpus case. Owns its `FaceResolver`
/// (which copies the face table), so a driver can borrow it for a whole run.
struct Fixture {
    buffer_id: BufferId,
    snapshot: LayoutBufferSnapshot,
    resolver: FaceResolver,
    base_face: ResolvedFace,
}

impl Fixture {
    fn new(case: &StreamCase) -> Self {
        let mut eval = Context::new();
        let buffer_id = eval
            .buffer_manager()
            .current_buffer()
            .expect("current buffer")
            .id();
        {
            let buffer = eval
                .buffer_manager_mut()
                .get_mut(buffer_id)
                .expect("buffer");
            buffer.insert(case.text);
            for property in &case.properties {
                let start =
                    buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(property.start_char));
                let end =
                    buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(property.end_char));
                buffer.text_props_put_property_in_emacs_byte_range(
                    EmacsByteRange::new(start, end),
                    Value::symbol(property.name()),
                    property.value(),
                );
            }
        }
        let buffer = eval.buffer_manager().get(buffer_id).expect("buffer");
        let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
        let table = FaceTable::new();
        let resolver = FaceResolver::new(&table, 0x00ff_ffff, 0x0000_0000, 14.0, None);
        let base_face = resolver.default_face().clone();
        Self {
            buffer_id,
            snapshot,
            resolver,
            base_face,
        }
    }

    fn driver(&self, split: SplitPolicy) -> StreamDriver<'_> {
        StreamDriver {
            fixture: self,
            producer: BufferElementProducer::new(self.buffer_id, &self.snapshot, 0, 0),
            position: DisplaySourceTextPosition::new(0, 0),
            face_ids: FrameFaceAttempt::for_test_with_next_id(BASE_FACE.get() + 1),
            split,
        }
    }
}

/// A producer seated where it was when the snapshot was taken, plus the walk
/// position of that moment — GNU SAVE_IT for the harness.
struct DriverSnapshot {
    producer: ProducerSnapshot,
    position: DisplaySourceTextPosition,
}

struct StreamDriver<'a> {
    fixture: &'a Fixture,
    producer: BufferElementProducer<'a, LayoutBufferSnapshot>,
    position: DisplaySourceTextPosition,
    face_ids: FrameFaceAttempt,
    split: SplitPolicy,
}

impl<'a> StreamDriver<'a> {
    /// Consume one element, applying this leg's split feeder, and return the
    /// per-character observations it contributes.
    fn next_observations(&mut self) -> Option<Vec<CharObservation>> {
        let scan_before = self.position;
        let basis = DisplaySourceFaceBasis::new(
            &self.fixture.resolver,
            BASE_FACE,
            &self.fixture.base_face,
            DisplayRowFallbackMetrics::from_default_face_extents(8.0, 16.0, 12.0),
        );
        let item = self.producer.next_consumed_item_with_face_basis(
            &self.fixture.snapshot,
            basis,
            &mut self.face_ids,
            &mut self.position,
        )?;
        let item = self.apply_split(item, scan_before);
        if let BufferSourceConsumedItem::Renderable(step) = &item {
            // Publish the position of the item actually consumed. A split leg
            // consumed only the prefix, so both tracks come from THAT item --
            // the renderer does the same with `into_render_parts` rather than
            // keeping the position the whole run advanced to.
            self.position = BufferScanPos::new(
                step.source_end_byte_idx()
                    .unwrap_or_else(|| self.position.byte_idx()),
                step.end_charpos(),
            );
        }
        Some(observe(&item, scan_before))
    }

    fn apply_split(
        &mut self,
        item: BufferSourceConsumedItem,
        scan_before: BufferScanPos,
    ) -> BufferSourceConsumedItem {
        let BufferSourceConsumedItem::Renderable(step) = item else {
            return item;
        };
        match self.split {
            SplitPolicy::None => BufferSourceConsumedItem::Renderable(step),
            SplitPolicy::PerChar => {
                let Some((first, pending)) = step
                    .is_multi_char_text_run()
                    .then(|| step.clone().split_text_run_items(0))
                    .flatten()
                else {
                    return BufferSourceConsumedItem::Renderable(step);
                };
                self.producer.prepend_pending_render_items(pending);
                BufferSourceConsumedItem::Renderable(first)
            }
            SplitPolicy::FitAt(at_charpos) => {
                let crosses = scan_before.charpos() < at_charpos
                    && step.end_charpos() > at_charpos
                    && step.is_multi_char_text_run();
                let Some((prefix, suffix)) = crosses
                    .then(|| step.clone().split_text_run_at_charpos(at_charpos, 0))
                    .flatten()
                else {
                    return BufferSourceConsumedItem::Renderable(step);
                };
                self.producer.prepend_pending_render_items(vec![suffix]);
                BufferSourceConsumedItem::Renderable(prefix)
            }
            SplitPolicy::PrefixAt(at_charpos) => {
                let crosses = scan_before.charpos() < at_charpos
                    && step.end_charpos() > at_charpos
                    && step.is_multi_char_text_run();
                let Some((prefix, _tail)) = crosses
                    .then(|| step.clone().split_text_run_at_charpos(at_charpos, 0))
                    .flatten()
                else {
                    return BufferSourceConsumedItem::Renderable(step);
                };
                self.producer.consume_prefix_to(at_charpos);
                BufferSourceConsumedItem::Renderable(prefix)
            }
        }
    }

    fn drain(&mut self, limit: usize) -> Vec<CharObservation> {
        let mut stream = Vec::new();
        for _ in 0..limit {
            let Some(observations) = self.next_observations() else {
                break;
            };
            stream.extend(observations);
        }
        stream
    }

    /// Drain until the next element would start at or past `charpos`.
    fn drain_until_charpos(&mut self, charpos: i64) -> Vec<CharObservation> {
        let mut stream = Vec::new();
        while self.position.charpos() < charpos {
            let Some(observations) = self.next_observations() else {
                break;
            };
            stream.extend(observations);
        }
        stream
    }

    fn snapshot(&self) -> DriverSnapshot {
        DriverSnapshot {
            producer: self.producer.snapshot(),
            position: self.position,
        }
    }

    fn restore(&mut self, snapshot: DriverSnapshot) {
        self.producer.restore(snapshot.producer);
        self.position = snapshot.position;
    }
}

// ---------------------------------------------------------------------------
// Harness entry points
// ---------------------------------------------------------------------------

fn producer_stream(case: &StreamCase) -> Vec<CharObservation> {
    Fixture::new(case)
        .driver(SplitPolicy::None)
        .drain(DRAIN_LIMIT)
}

/// The harness assertion: a split feeder is invisible on the assertion surface.
fn assert_split_transparent(case: &StreamCase, split: SplitPolicy) {
    let fixture = Fixture::new(case);
    let producer_leg = fixture.driver(SplitPolicy::None).drain(DRAIN_LIMIT);
    let replay_leg = fixture.driver(split).drain(DRAIN_LIMIT);

    assert!(
        !producer_leg.is_empty(),
        "{}: the producer leg produced nothing",
        case.name
    );
    assert_eq!(
        producer_leg, replay_leg,
        "{}: the {:?} split feeder changed the element stream",
        case.name, split
    );
}

fn assert_streams_agree(case: &StreamCase) {
    assert_split_transparent(case, SplitPolicy::PerChar);
}

/// The snapshot/restore contract (C8, C9, C13): consume through a failed
/// overflow attempt past `candidate`, restore the seating saved AT the
/// candidate, and require the remainder stream to be byte-identical to a leg
/// that simply ran to the candidate and continued — INCLUDING re-production of
/// the candidate character consumed during the attempt (the bug class the
/// walk.rs rewind comment documents).
fn assert_restore_resumes_at_candidate(case: &StreamCase, candidate: i64, split: SplitPolicy) {
    let fixture = Fixture::new(case);

    let mut reference = fixture.driver(split);
    reference.drain_until_charpos(candidate);
    let expected = reference.drain(DRAIN_LIMIT);

    let mut attempted = fixture.driver(split);
    attempted.drain_until_charpos(candidate);
    let saved = attempted.snapshot();
    // The failed overflow attempt: consume past the candidate, then give up.
    attempted.next_observations();
    attempted.next_observations();
    attempted.restore(saved);
    let resumed = attempted.drain(DRAIN_LIMIT);

    assert!(
        !expected.is_empty(),
        "{}: nothing to resume at charpos {candidate}",
        case.name
    );
    assert_eq!(
        expected[0].scan_before.charpos(),
        candidate,
        "{}: the corpus candidate must fall on an element boundary",
        case.name
    );
    assert_eq!(
        resumed, expected,
        "{}: restore did not re-produce the stream from charpos {candidate}",
        case.name
    );
}

// ---------------------------------------------------------------------------
// Negative control: prove the harness can fail
// ---------------------------------------------------------------------------

#[test]
fn negative_control_the_harness_detects_a_perturbed_stream() {
    // Kept permanently: a harness that cannot fail proves nothing, and every
    // later rung reads these comparisons as evidence.
    let case = StreamCase::new("negative control", "hello\n");
    let stream = producer_stream(&case);

    let mut wrong_provenance = stream.clone();
    wrong_provenance[2].provenance = GlyphProvenance::buffer(CharPos0::new(99));
    assert_ne!(stream, wrong_provenance, "provenance must be compared");

    let mut wrong_scan = stream.clone();
    wrong_scan[2].scan_after = BufferScanPos::new(0, 0);
    assert_ne!(stream, wrong_scan, "scan positions must be compared");

    let mut wrong_class = stream.clone();
    wrong_class[2].class = ElementClass::Tab;
    assert_ne!(stream, wrong_class, "element class must be compared");

    let mut wrong_char = stream.clone();
    wrong_char[2].ch = Some('z');
    assert_ne!(stream, wrong_char, "characters must be compared");

    let mut dropped = stream.clone();
    dropped.remove(2);
    assert_ne!(stream, dropped, "stream length and order must be compared");
}

// ---------------------------------------------------------------------------
// C1 - C3: runs, faces, and the per-char feeder
// ---------------------------------------------------------------------------

#[test]
fn c1_plain_ascii_single_face() {
    let case = StreamCase::new("C1 plain ascii", "hello world\n");
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(CharObservation::chars(&stream), "hello world\n");
    for (index, observation) in stream.iter().enumerate() {
        assert_eq!(
            observation.provenance,
            GlyphProvenance::buffer(CharPos0::new(index)),
            "every buffer char stamps its own charpos"
        );
        assert_eq!(observation.scan_before.charpos(), index as i64);
    }
    assert_eq!(
        stream.last().expect("row break").class,
        ElementClass::RowBreak
    );
}

#[test]
fn c2_multi_face_line_keeps_one_continuous_scan_track() {
    // A face property cuts the producer's runs (see the boundary test below),
    // but the CHARACTER stream underneath must be untouched: same chars, same
    // provenance, same scan positions as the unfaced baseline. Only the
    // face-ref differs.
    let case = StreamCase::new("C2 multi-face", "abcdef\n")
        .with(CaseProperty::face(2, 4, ":weight", "bold"));
    assert_streams_agree(&case);

    let plain = producer_stream(&StreamCase::new("C2 baseline", "abcdef\n"));
    let faced = producer_stream(&case);
    assert_eq!(plain.len(), faced.len());
    for (plain, faced) in plain.iter().zip(faced.iter()) {
        assert_eq!(plain.ch, faced.ch);
        assert_eq!(plain.provenance, faced.provenance);
        assert_eq!(plain.scan_before, faced.scan_before);
        assert_eq!(plain.scan_after, faced.scan_after);
        assert_eq!(plain.class, faced.class);
    }
    // Where the run BOUNDARIES fall is pinned char-exactly by the boundary
    // test below; this case owns the continuity half.
}

#[test]
fn c2_multi_face_line_ends_runs_at_face_boundaries() {
    let case = StreamCase::new("C2 multi-face", "abcdef\n")
        .with(CaseProperty::face(2, 4, ":weight", "bold"));
    let fixture = Fixture::new(&case);
    let mut driver = fixture.driver(SplitPolicy::None);

    let first = driver.next_observations().expect("first run");
    assert_eq!(CharObservation::chars(&first), "ab");
    let second = driver.next_observations().expect("second run");
    assert_eq!(CharObservation::chars(&second), "cd");
}

#[test]
fn c3_per_char_face_line_is_stream_identical_under_the_per_char_feeder() {
    // THE case that must turn red the moment P4.5 changes consumption order or
    // provenance: the pipeline leg splits every run per character and drains
    // N-1 queued echoes, and the resulting stream must be indistinguishable
    // from the producer's whole-run stream.
    let case = StreamCase::new("C3 per-char faces", "abcdefgh\n").per_char_faces(&[
        (":weight", "bold"),
        (":slant", "italic"),
        (":underline", "t"),
        (":overline", "t"),
        (":strike-through", "t"),
        (":inverse-video", "t"),
        (":extend", "t"),
        (":weight", "ultra-bold"),
    ]);
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(CharObservation::chars(&stream), "abcdefgh\n");
}

// ---------------------------------------------------------------------------
// C4: tabs
// ---------------------------------------------------------------------------

#[test]
fn c4a_tab_at_line_start() {
    let case = StreamCase::new("C4a tab at line start", "\tab\n");
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(stream[0].class, ElementClass::Tab);
    assert_eq!(stream[0].scan_after.charpos(), 1);
}

#[test]
fn c4b_tab_mid_line() {
    let case = StreamCase::new("C4b tab mid line", "ab\tcd\n");
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(stream[2].class, ElementClass::Tab);
    assert_eq!(stream[2].scan_before.charpos(), 2);
    assert_eq!(stream[3].scan_before.charpos(), 3);
}

#[test]
fn c4c_two_adjacent_tabs() {
    let case = StreamCase::new("C4c adjacent tabs", "a\t\tb\n");
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(stream[1].class, ElementClass::Tab);
    assert_eq!(stream[2].class, ElementClass::Tab);
}

#[test]
fn c4d_tab_after_wrap_resumes_from_the_candidate() {
    // The tab lands on the continuation row; the producer contract is that a
    // restore at the wrap candidate re-produces it. Its EXPANSION from the
    // continuation row's own pen is renderer-side and stays pinned by the
    // shipped 2j shadow continuation_resume_shadow_matches_tab_after_wrap_row.
    let case = StreamCase::new("C4d tab after wrap", "aaaaaaaaaaaaaaaaaa\tZ\n");
    assert_restore_resumes_at_candidate(&case, 16, SplitPolicy::FitAt(16));
}

// ---------------------------------------------------------------------------
// C5 / C6: multibyte and cluster seams
// ---------------------------------------------------------------------------

#[test]
fn c5_wide_chars_track_multibyte_byte_deltas() {
    let case = StreamCase::new("C5 wide chars", "a漢字b\n");
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(CharObservation::chars(&stream), "a漢字b\n");
    assert_eq!(stream[1].scan_before, BufferScanPos::new(1, 1));
    assert_eq!(
        stream[1].scan_after,
        BufferScanPos::new(4, 2),
        "a 3-byte char advances byte_idx by 3 and charpos by 1"
    );
    assert_eq!(stream[3].scan_before, BufferScanPos::new(7, 3));
}

#[test]
fn c6a_base_and_extender_stay_in_one_stream() {
    let case = StreamCase::new("C6a combining acute", "ae\u{301}z\n");
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(CharObservation::chars(&stream), "ae\u{301}z\n");
    assert_eq!(
        stream[2].scan_before,
        BufferScanPos::new(2, 2),
        "the extender is its own scan step"
    );
}

#[test]
fn c6b_extender_at_a_face_seam_stays_in_one_stream() {
    let case = StreamCase::new("C6b extender at face seam", "ae\u{301}z\n")
        .with(CaseProperty::face(2, 3, ":weight", "bold"));
    assert_streams_agree(&case);
}

#[test]
#[ignore = "un-ignored by the rung that gives ProducedChar its char class: Wide / ComposedExtender classification and the never-split-a-cluster run seam rule (design section 4.10)"]
fn c6_cluster_classes_are_carried_on_the_element() {
    let case = StreamCase::new("C6 classes", "ae\u{301}z\n");
    let stream = producer_stream(&case);
    // Expected once the class is carried: the extender is a zero-advance
    // ComposedExtender and no run ever ends between a base and its extender.
    assert_ne!(stream[2].class, ElementClass::PlainChar);
}

// ---------------------------------------------------------------------------
// C7: invisible elision
// ---------------------------------------------------------------------------

#[test]
fn c7a_invisible_text_is_not_elided_by_the_producer_today() {
    // The honest current contract: elision is the renderer's invisible
    // checkpoint, so the producer streams the hidden characters and the split
    // feeder stays transparent over them.
    let case =
        StreamCase::new("C7a invisible mid-line", "abXXcd\n").with(CaseProperty::invisible(2, 4));
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(CharObservation::chars(&stream), "abXXcd\n");
}

#[test]
#[ignore = "un-ignored by P4.8: the invisible checkpoint (row_lifecycle.rs) moves into producer stop state, and the scan track then jumps the elided span"]
fn c7a_invisible_span_jumps_the_scan_track() {
    let case =
        StreamCase::new("C7a invisible mid-line", "abXXcd\n").with(CaseProperty::invisible(2, 4));
    let stream = producer_stream(&case);
    assert_eq!(CharObservation::chars(&stream), "abcd\n");
    assert_eq!(stream[2].scan_before.charpos(), 4);
}

#[test]
fn c7b_trailing_elision_before_the_newline_keeps_the_row_break() {
    let case =
        StreamCase::new("C7b trailing elision", "abXX\n").with(CaseProperty::invisible(2, 4));
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(
        stream.last().expect("row break").class,
        ElementClass::RowBreak
    );
}

#[test]
fn c7c_adjacent_invisible_runs_keep_one_continuous_scan_track() {
    let case = StreamCase::new("C7c adjacent invisible", "aXYb\n")
        .with(CaseProperty::invisible(1, 2))
        .with(CaseProperty::invisible(2, 3));
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    for (index, observation) in stream.iter().enumerate() {
        assert_eq!(observation.scan_before.charpos(), index as i64);
    }
}

// ---------------------------------------------------------------------------
// C8 / C9 / C13: the snapshot-restore contract
// ---------------------------------------------------------------------------

#[test]
fn c8a_single_wrap_restores_at_the_wrap_point() {
    let case = StreamCase::new("C8a single wrap", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n");
    assert_restore_resumes_at_candidate(&case, 20, SplitPolicy::FitAt(20));
}

#[test]
fn c8b_iterated_overflow_restores_at_every_row_edge() {
    let case = StreamCase::new(
        "C8b iterated overflow",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
    );
    for candidate in [20, 40, 60] {
        assert_restore_resumes_at_candidate(&case, candidate, SplitPolicy::FitAt(candidate));
    }
}

#[test]
fn c8c_wrap_at_a_wide_edge_char_restores_at_the_wide_char() {
    let case = StreamCase::new("C8c wide edge char", "aaaaaaaaaaaaaaaaaaa漢tail\n");
    assert_restore_resumes_at_candidate(&case, 19, SplitPolicy::FitAt(19));
}

#[test]
fn c9a_word_wrap_candidate_at_a_space() {
    // Break at the space; the continuation row resumes at `b` (charpos 5).
    let case = StreamCase::new("C9a candidate at space", "aaaa bbbbbb\n");
    assert_restore_resumes_at_candidate(&case, 5, SplitPolicy::FitAt(5));
}

#[test]
fn c9b_word_wrap_candidate_mid_run() {
    // The `b` word itself exceeds the row: after the space break the run
    // char-wraps with no candidate inside it.
    let case = StreamCase::new("C9b candidate mid run", "aaaa bbbbbbbbbbbb\n");
    assert_restore_resumes_at_candidate(&case, 5, SplitPolicy::FitAt(5));
    assert_restore_resumes_at_candidate(&case, 12, SplitPolicy::FitAt(12));
}

#[test]
fn c9c_word_wrap_with_no_candidate_falls_back_to_the_row_edge() {
    let case = StreamCase::new("C9c no candidate", "aaaaaaaaaaaa\n");
    assert_restore_resumes_at_candidate(&case, 10, SplitPolicy::FitAt(10));
}

#[test]
fn c9d_word_wrap_candidate_before_a_tab() {
    // Compound of C4d and C9a: the candidate space sits immediately before a
    // tab whose expansion overflows, so the tab is re-produced on the
    // continuation row.
    let case = StreamCase::new("C9d candidate before tab", "aaaa \tbb\n");
    assert_restore_resumes_at_candidate(&case, 5, SplitPolicy::FitAt(5));

    let fixture = Fixture::new(&case);
    let mut driver = fixture.driver(SplitPolicy::FitAt(5));
    driver.drain_until_charpos(5);
    let resumed = driver.drain(DRAIN_LIMIT);
    assert_eq!(resumed[0].class, ElementClass::Tab);
}

#[test]
fn c13_fit_split_isolation_resumes_at_the_split_point() {
    // The `consume_prefix(k)` contract in its P4.2 form: cut a 24-char run at
    // 20, and the next element starts at charpos 20 with the right byte_idx.
    let case = StreamCase::new("C13 fit split", "aaaaaaaaaaaaaaaaaaaaaaaa\n");
    let fixture = Fixture::new(&case);
    let mut driver = fixture.driver(SplitPolicy::FitAt(20));

    let prefix = driver.drain_until_charpos(20);
    assert_eq!(prefix.len(), 20);
    let remainder = driver.drain(DRAIN_LIMIT);
    assert_eq!(remainder[0].scan_before, BufferScanPos::new(20, 20));
    assert_eq!(CharObservation::chars(&remainder), "aaaa\n");

    assert_split_transparent(&case, SplitPolicy::FitAt(20));
}

// ---------------------------------------------------------------------------
// C10 / C11 / C14 / C15: renderer-owned glyphs are ABSENT from the stream
// ---------------------------------------------------------------------------

fn assert_no_redisplay_provenance(case: &StreamCase) {
    for observation in producer_stream(case) {
        assert!(
            matches!(observation.provenance, GlyphProvenance::Buffer { .. }),
            "{}: the producer stream must carry only buffer provenance; \
             redisplay's own glyphs are renderer-owned",
            case.name
        );
    }
}

#[test]
fn c10_truncation_marks_are_absent_from_the_producer_stream() {
    // Truncation marks are Redisplay(Mark) glyphs emitted by
    // special_glyphs.rs, never produced elements.
    let case = StreamCase::new("C10 truncation", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nZ\n");
    assert_no_redisplay_provenance(&case);
    assert_streams_agree(&case);
}

#[test]
fn c11_hscroll_left_markers_are_absent_and_the_stream_is_position_addressable() {
    // The hscroll skip is renderer-side; the producer is simply asked to start
    // at the post-skip position, and the left-truncation marker never appears.
    let case = StreamCase::new("C11 hscroll", "abcdefghij\n");
    assert_no_redisplay_provenance(&case);

    let fixture = Fixture::new(&case);
    let mut driver = fixture.driver(SplitPolicy::None);
    driver
        .producer
        .rewind_to(DisplaySourceTextPosition::new(5, 5));
    driver.position = DisplaySourceTextPosition::new(5, 5);
    let stream = driver.drain(DRAIN_LIMIT);

    assert_eq!(CharObservation::chars(&stream), "fghij\n");
    assert_eq!(stream[0].scan_before, BufferScanPos::new(5, 5));
}

#[test]
fn c14_eob_tail_without_a_trailing_newline_ends_without_a_row_break() {
    let case = StreamCase::new("C14 eob tail", "abc");
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(CharObservation::chars(&stream), "abc");
    assert!(
        stream
            .iter()
            .all(|observation| observation.class != ElementClass::RowBreak),
        "a newline-less last line produces no RowBreak"
    );
}

#[test]
fn c15_empty_lines_produce_row_breaks_and_no_empty_line_glyph() {
    let case = StreamCase::new("C15 empty lines", "\n\n");
    assert_streams_agree(&case);

    let stream = producer_stream(&case);
    assert_eq!(stream.len(), 2);
    assert!(
        stream
            .iter()
            .all(|observation| observation.class == ElementClass::RowBreak)
    );
    assert_eq!(
        stream[0].provenance,
        GlyphProvenance::buffer(CharPos0::new(0))
    );
    assert_eq!(
        stream[1].provenance,
        GlyphProvenance::buffer(CharPos0::new(1))
    );
    // The empty-line cursor glyph is Redisplay(EmptyLineNewline) and is
    // renderer-owned (row-level in this engine, e772f82ed) — never a produced
    // element.
    assert_no_redisplay_provenance(&case);
}

// ---------------------------------------------------------------------------
// C12: overlay seams
// ---------------------------------------------------------------------------

#[test]
#[ignore = "un-ignored by P4.4: the producer folds next_overlay_change into its stop state so a run never crosses an overlay seam (GNU compute_stop_pos, xdisp.c:4356-4365)"]
fn c12_overlay_face_seam_terminates_runs() {
    let case = StreamCase::new("C12 overlay seam", "abcdefgh\n");
    let fixture = Fixture::new(&case);
    let mut driver = fixture.driver(SplitPolicy::None);
    // With an overlay carrying only a face over [3,6), the producer must yield
    // runs "abc", "def", "gh" rather than one run.
    let first = driver.next_observations().expect("first run");
    assert_eq!(CharObservation::chars(&first), "abc");
}

// ---------------------------------------------------------------------------
// P4.3: the fit split is replaced by a prefix consume
// ---------------------------------------------------------------------------

#[test]
fn p43_prefix_consume_leaves_nothing_pending_and_resumes_at_the_prefix_end() {
    // The rung's contract: after the renderer takes a fitting prefix the
    // producer sits at the first unfitting character with an EMPTY pending
    // queue — no tail is pushed back for a later iteration to pop.
    let case = StreamCase::new("P4.3 fit consume", "aaaaaaaaaaaaaaaaaaaaaaaa\n");
    let fixture = Fixture::new(&case);
    let mut driver = fixture.driver(SplitPolicy::PrefixAt(20));

    let prefix = driver.drain_until_charpos(20);
    assert_eq!(prefix.len(), 20);
    assert_eq!(
        driver.producer.pending_render_items_len(),
        0,
        "the prefix consume must not queue a remainder"
    );

    let remainder = driver.drain(DRAIN_LIMIT);
    assert_eq!(remainder[0].scan_before, BufferScanPos::new(20, 20));
    assert_eq!(CharObservation::chars(&remainder), "aaaa\n");
}

#[test]
fn p43_prefix_consume_is_stream_identical_to_the_queueing_fit_split() {
    // The deletion proof, including a property seam inside the discarded tail
    // and a multibyte boundary: the old mechanism (queue the tail) and the new
    // one (reseat the producer) yield the same element stream.
    for case in [
        StreamCase::new("P4.3 plain", "aaaaaaaaaaaaaaaaaaaaaaaa\n"),
        StreamCase::new("P4.3 multibyte", "aaaaaaaaaaaaaaaaaaa漢tail\n"),
        StreamCase::new("P4.3 tab in tail", "aaaaaaaaaaaaaaaaaa\tZ\n"),
        StreamCase::new("P4.3 face seam in tail", "aaaaaaaaaaaaaaaaaaaaaaaa\n")
            .with(CaseProperty::face(18, 21, ":weight", "bold")),
    ] {
        let fixture = Fixture::new(&case);
        let queued = fixture.driver(SplitPolicy::FitAt(16)).drain(DRAIN_LIMIT);
        let consumed = fixture.driver(SplitPolicy::PrefixAt(16)).drain(DRAIN_LIMIT);
        assert!(!queued.is_empty(), "{}: nothing produced", case.name);
        assert_eq!(
            queued, consumed,
            "{}: the prefix consume changed the element stream",
            case.name
        );
    }
}
