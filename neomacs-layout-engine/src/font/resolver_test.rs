use super::*;
use crate::font_backend::{PlatformFontDesignMetrics, PlatformFontMetadata};
use neomacs_display_protocol::font::ResolvedFontIdentity;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct CandidateBackend {
    candidates: Vec<FontCandidate>,
}

impl FontBackend for CandidateBackend {
    fn kind(&self) -> FontBackendKind {
        FontBackendKind::Fontconfig
    }

    fn resolve_family(&self, family: &str) -> String {
        family.to_string()
    }

    fn family_prefers_monospace(&self, _family: &str) -> bool {
        true
    }

    fn list_candidates(&self, _query: &FontCandidateQuery) -> Vec<FontCandidate> {
        self.candidates.clone()
    }
}

fn candidate(family: &str, weight: u16, slant: FontSlant, spacing: i32) -> FontCandidate {
    FontCandidate {
        matched: PlatformFontMatch {
            identity: ResolvedFontIdentity::from_file(
                &format!("/fixture/{family}-{weight}.ttf"),
                0,
                None,
            ),
            metadata: PlatformFontMetadata {
                family: family.to_string(),
                weight: Some(weight),
                slant,
                design_metrics: Some(PlatformFontDesignMetrics::default()),
            },
        },
        width: Some(FontWidth::Normal),
        spacing: Some(spacing),
    }
}

#[test]
fn shared_primary_scoring_prefers_requested_style() {
    let resolver = FontResolver::new(Box::new(CandidateBackend {
        candidates: vec![
            candidate("Fixture", 400, FontSlant::Normal, 100),
            candidate("Fixture", 700, FontSlant::Italic, 100),
        ],
    }));
    let selected = resolver
        .resolve_primary("Fixture", 700, FontSlant::Italic, FontWidth::Normal)
        .expect("candidate");
    assert_eq!(selected.weight(), Some(700));
    assert_eq!(selected.slant(), FontSlant::Italic);
}

#[test]
fn equal_score_entities_keep_their_own_discovery_order() {
    let mut variable_seed = candidate("Fixture", 800, FontSlant::Italic, 100);
    variable_seed.matched.identity = ResolvedFontIdentity::from_file(
        "/fixture/Fixture[wdth,wght].ttf",
        0x0008_0000,
        Some("Fixture-ExtraBoldItalic".to_string()),
    );
    let mut static_bold = candidate("Fixture", 700, FontSlant::Italic, 100);
    static_bold.matched.identity = ResolvedFontIdentity::from_file(
        "/fixture/Fixture-BoldItalic.ttf",
        0,
        Some("Fixture-BoldItalic".to_string()),
    );
    let mut variable_bold = candidate("Fixture", 700, FontSlant::Italic, 100);
    variable_bold.matched.identity = ResolvedFontIdentity::from_file(
        "/fixture/Fixture[wdth,wght].ttf",
        0x0007_0000,
        Some("Fixture-BoldItalic".to_string()),
    );

    let selected = select_best_candidate(
        vec![variable_seed, static_bold, variable_bold],
        &SelectionRequest {
            weight: 700,
            slant: FontSlant::Italic,
            width: Some(FontWidth::Normal),
            spacing: None,
            prefer_monospace: false,
            queried_family: Some("Fixture"),
        },
    )
    .expect("equal-score entity");

    assert_eq!(
        selected.file_path(),
        Some("/fixture/Fixture-BoldItalic.ttf"),
        "GNU scores each entity independently; a variable file's earlier, non-matching instance must not donate its ordinal to a later instance"
    );
}

struct MetricBackend {
    candidates: Vec<FontCandidate>,
    probes: Arc<AtomicUsize>,
}

impl FontBackend for MetricBackend {
    fn kind(&self) -> FontBackendKind {
        FontBackendKind::CoreText
    }

    fn resolve_family(&self, family: &str) -> String {
        family.to_string()
    }

    fn family_prefers_monospace(&self, _family: &str) -> bool {
        true
    }

    fn list_candidates(&self, _query: &FontCandidateQuery) -> Vec<FontCandidate> {
        self.candidates.clone()
    }

    fn design_metrics(&self, _matched: &PlatformFontMatch) -> Option<PlatformFontDesignMetrics> {
        self.probes.fetch_add(1, Ordering::Relaxed);
        Some(PlatformFontDesignMetrics {
            units_per_em: 1_000,
            ascent: 800,
            descent: 200,
            line_gap: 0,
            max_advance: 700,
            space_advance: 500,
            average_advance: 600,
        })
    }
}

#[test]
fn native_metrics_are_probed_only_for_the_cached_winner() {
    let probes = Arc::new(AtomicUsize::new(0));
    let mut regular = candidate("Fixture", 400, FontSlant::Normal, 100);
    regular.matched.metadata.design_metrics = None;
    let mut bold = candidate("Fixture", 700, FontSlant::Normal, 100);
    bold.matched.metadata.design_metrics = None;
    let resolver = FontResolver::new(Box::new(MetricBackend {
        candidates: vec![regular, bold],
        probes: Arc::clone(&probes),
    }));

    let first = resolver
        .resolve_primary("Fixture", 700, FontSlant::Normal, FontWidth::Normal)
        .expect("selected winner");
    let second = resolver
        .resolve_primary("Fixture", 700, FontSlant::Normal, FontWidth::Normal)
        .expect("cached winner");

    assert_eq!(first.identity, second.identity);
    assert_eq!(
        first.pixel_metrics(20.0).expect("native metrics").ascent,
        16
    );
    assert_eq!(probes.load(Ordering::Relaxed), 1);
}
