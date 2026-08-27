//! Shared GNU-compatible font selection policy.
//!
//! GNU Emacs keeps fontset lookup and entity scoring in `fontset.c`/`font.c`;
//! platform drivers only list/open entities and answer coverage questions.
//! [`FontResolver`] preserves that split.  A [`FontBackend`] may use
//! Fontconfig, CoreText, or DirectWrite to discover candidates, but it never
//! decides which fontset entry or style wins.

use crate::font::selection::{CandidateSelectionScore, candidate_selection_score};
use crate::font_backend::{
    FontBackend, FontCandidate, FontCandidateQuery, FontSelectionSize, PlatformFontMatch,
    PlatformFontSize, TextDirection,
};
use neomacs_display_protocol::font::FontBackendKind;
use neovm_core::emacs_core::font::alternative_font_families;
use neovm_core::emacs_core::fontset::{
    FontSpecEntry, StoredFontSpec, fontset_generation, matching_entries_for_char,
};
use neovm_core::emacs_core::intern::resolve_sym;
use neovm_core::face::{FontSlant, FontWidth};
use rustc_hash::FxHashMap as HashMap;
use std::sync::Mutex;

/// Platform-neutral owner of fontset policy and candidate scoring.
pub struct FontResolver {
    backend: Box<dyn FontBackend>,
    materializer: Option<neomacs_font_materializer::FontMaterializer>,
    capability_cache: Mutex<
        HashMap<
            neomacs_display_protocol::font::ResolvedFontIdentity,
            Result<
                neomacs_font_materializer::FontCapability,
                neomacs_font_materializer::FontMaterializationError,
            >,
        >,
    >,
    primary_cache: Mutex<HashMap<PrimaryCacheKey, Option<PlatformFontMatch>>>,
    char_cache: Mutex<HashMap<CharCacheKey, Option<PlatformFontMatch>>>,
}

impl FontResolver {
    pub fn new(backend: Box<dyn FontBackend>) -> Self {
        Self {
            backend,
            materializer: neomacs_font_materializer::FontMaterializer::new().ok(),
            capability_cache: Mutex::new(HashMap::default()),
            primary_cache: Mutex::new(HashMap::default()),
            char_cache: Mutex::new(HashMap::default()),
        }
    }

    pub fn platform_default() -> Self {
        Self::new(crate::font_backend::default_font_backend())
    }

    pub fn backend_kind(&self) -> FontBackendKind {
        self.backend.kind()
    }

    pub fn resolve_family(&self, family: &str) -> String {
        self.backend.resolve_family(family)
    }

    pub fn family_prefers_monospace(&self, family: &str) -> bool {
        self.backend.family_prefers_monospace(family)
    }

    /// Resolve a primary face from candidates in one concrete family.
    pub fn resolve_primary(
        &self,
        family: &str,
        requested_weight: u16,
        requested_slant: FontSlant,
        requested_width: FontWidth,
        size: FontSelectionSize,
    ) -> Option<PlatformFontMatch> {
        let key = PrimaryCacheKey {
            family: family.to_string(),
            weight: requested_weight,
            slant: requested_slant.gnu_numeric(),
            width: requested_width.gnu_numeric(),
            size,
        };
        if let Ok(cache) = self.primary_cache.lock()
            && let Some(cached) = cache.get(&key)
        {
            return cached.clone();
        }
        let family = self.resolve_family(family);
        let query = FontCandidateQuery {
            family: Some(family.clone()),
            fallback_family: family.clone(),
            required_char: None,
            charset_ranges: Vec::new(),
            languages: Vec::new(),
            requested_weight,
            requested_slant,
            requested_width,
            direction: TextDirection::LeftToRight,
            size,
        };
        let selected = select_best_candidate(
            self.classify_unknown_candidate_sizes(self.backend.list_candidates(&query)),
            &SelectionRequest {
                weight: requested_weight,
                slant: requested_slant,
                width: Some(requested_width),
                spacing: None,
                prefer_monospace: self.family_prefers_monospace(&family),
                queried_family: Some(&family),
                size,
            },
        )
        .map(|matched| self.backend.finalize_match(matched))
        .map(|matched| self.with_native_metrics(matched));
        if let Ok(mut cache) = self.primary_cache.lock() {
            cache.insert(key, selected.clone());
        }
        selected
    }

    /// Resolve the first usable fontset entry for a non-ASCII character.
    pub fn resolve_for_char(
        &self,
        family: &str,
        ch: char,
        requested_weight: u16,
        requested_slant: FontSlant,
        requested_width: FontWidth,
        size: FontSelectionSize,
    ) -> Option<PlatformFontMatch> {
        if ch.is_ascii() {
            return None;
        }
        let key = CharCacheKey {
            family: family.to_string(),
            ch,
            weight: requested_weight,
            slant: requested_slant.gnu_numeric(),
            width: requested_width.gnu_numeric(),
            fontset_generation: fontset_generation(),
            size,
        };
        if let Ok(cache) = self.char_cache.lock()
            && let Some(cached) = cache.get(&key)
        {
            return cached.clone();
        }

        let prefer_monospace = self.family_prefers_monospace(family);
        let mut selected = None;
        let mut allow_generic_fallback = true;
        for entry in matching_entries_for_char(ch) {
            match entry {
                FontSpecEntry::ExplicitNone => {
                    allow_generic_fallback = false;
                    break;
                }
                FontSpecEntry::Font(spec) => {
                    if let Some(matched) = self.resolve_from_spec(
                        family,
                        ch,
                        prefer_monospace,
                        requested_weight,
                        requested_slant,
                        requested_width,
                        size,
                        &spec,
                    ) {
                        selected = Some(matched);
                        break;
                    }
                }
            }
        }

        if selected.is_none() && allow_generic_fallback {
            selected = self.resolve_from_spec(
                family,
                ch,
                prefer_monospace,
                requested_weight,
                requested_slant,
                requested_width,
                size,
                &StoredFontSpec {
                    family: None,
                    registry: None,
                    lang: None,
                    weight: None,
                    slant: None,
                    width: None,
                    repertory: None,
                },
            );
        }
        selected = selected
            .map(|matched| self.backend.finalize_match(matched))
            .map(|matched| self.with_native_metrics(matched));
        if let Ok(mut cache) = self.char_cache.lock() {
            cache.insert(key, selected.clone());
        }
        selected
    }

    fn with_native_metrics(&self, mut matched: PlatformFontMatch) -> PlatformFontMatch {
        if matched.metadata.design_metrics.is_none() {
            matched.metadata.design_metrics = self.backend.design_metrics(&matched);
        }
        matched
    }

    /// Complete partial native size metadata before GNU entity scoring.
    /// `Unknown` is classified by opening the exact file face; it never
    /// silently receives the score of a scalable entity.
    fn classify_unknown_candidate_sizes(
        &self,
        candidates: Vec<FontCandidate>,
    ) -> Vec<FontCandidate> {
        let mut classified = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if candidate.matched.metadata.size != PlatformFontSize::Unknown {
                classified.push(candidate);
                continue;
            }
            let identity = &candidate.matched.identity;
            match self.inspect_capability(identity) {
                Err(neomacs_font_materializer::FontMaterializationError::NotBitmapFace) => {
                    let mut scalable = candidate;
                    scalable.matched.metadata.size = PlatformFontSize::Scalable;
                    classified.push(scalable);
                }
                Ok(neomacs_font_materializer::FontCapability::FreeTypeBitmap { strikes }) => {
                    classified.extend(strikes.into_iter().filter_map(|strike| {
                        let ppem = u32::try_from(strike.y_ppem_26_6).ok()?;
                        let mut fixed = candidate.clone();
                        fixed.matched.metadata.size = PlatformFontSize::Fixed {
                            device_ppem_26_6: ppem,
                        };
                        Some(fixed)
                    }));
                }
                Err(error) => {
                    tracing::warn!(
                        target: "font_boundary",
                        identity = %identity.stable_key,
                        %error,
                        "could not classify platform font size before GNU scoring"
                    );
                }
            }
        }
        classified
    }

    fn inspect_capability(
        &self,
        identity: &neomacs_display_protocol::font::ResolvedFontIdentity,
    ) -> Result<
        neomacs_font_materializer::FontCapability,
        neomacs_font_materializer::FontMaterializationError,
    > {
        if let Ok(cache) = self.capability_cache.lock()
            && let Some(cached) = cache.get(identity)
        {
            return cached.clone();
        }
        let result = self
            .materializer
            .as_ref()
            .ok_or(neomacs_font_materializer::FontMaterializationError::BackendUnavailable)
            .and_then(|materializer| materializer.inspect(identity));
        if let Ok(mut cache) = self.capability_cache.lock() {
            cache.insert(identity.clone(), result.clone());
        }
        result
    }

    pub(crate) fn clear_caches(&mut self) {
        self.primary_cache
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        self.char_cache
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        self.capability_cache
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    fn resolve_from_spec(
        &self,
        requested_family: &str,
        ch: char,
        prefer_monospace: bool,
        requested_weight: u16,
        requested_slant: FontSlant,
        requested_width: FontWidth,
        size: FontSelectionSize,
        spec: &StoredFontSpec,
    ) -> Option<PlatformFontMatch> {
        let effective_weight = spec
            .weight
            .map(|weight| weight.css_weight())
            .unwrap_or(requested_weight);
        let effective_slant = spec.slant.unwrap_or(requested_slant);
        let effective_width = spec.width.unwrap_or(requested_width);
        let charset_ranges = crate::font::fontconfig::query_charset_ranges(spec, ch);
        let registry_language = spec
            .registry
            .map(resolve_sym)
            .and_then(crate::font::fontconfig::registry_language);
        let languages = crate::font::fontconfig::combined_query_langs(
            registry_language,
            spec.lang.map(resolve_sym),
        );
        let search_order = family_search_order(self.backend.as_ref(), requested_family, spec);

        for family in search_order {
            let query = FontCandidateQuery {
                family: family.clone(),
                fallback_family: self.resolve_family(requested_family),
                required_char: Some(ch),
                charset_ranges: charset_ranges.clone(),
                languages: languages.clone(),
                requested_weight: effective_weight,
                requested_slant: effective_slant,
                requested_width: effective_width,
                direction: TextDirection::for_char(ch),
                size,
            };
            let request = SelectionRequest {
                weight: effective_weight,
                slant: effective_slant,
                width: spec.width,
                spacing: None,
                prefer_monospace,
                queried_family: family.as_deref(),
                size,
            };
            if let Some(matched) = select_best_candidate(
                self.classify_unknown_candidate_sizes(self.backend.list_candidates(&query)),
                &request,
            ) {
                return Some(matched);
            }
        }
        None
    }

    #[cfg(test)]
    pub(crate) fn replace_backend(&mut self, backend: Box<dyn FontBackend>) {
        self.backend = backend;
        self.primary_cache
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        self.char_cache
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        self.capability_cache
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct PrimaryCacheKey {
    family: String,
    weight: u16,
    slant: u16,
    width: u16,
    size: FontSelectionSize,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct CharCacheKey {
    family: String,
    ch: char,
    weight: u16,
    slant: u16,
    width: u16,
    fontset_generation: u64,
    size: FontSelectionSize,
}

struct SelectionRequest<'a> {
    weight: u16,
    slant: FontSlant,
    width: Option<FontWidth>,
    spacing: Option<i32>,
    prefer_monospace: bool,
    queried_family: Option<&'a str>,
    size: FontSelectionSize,
}

fn select_best_candidate(
    candidates: Vec<FontCandidate>,
    request: &SelectionRequest<'_>,
) -> Option<PlatformFontMatch> {
    // GNU scores every entity independently. Equal scores retain the entity's
    // own Fontconfig discovery order; a different named instance in the same
    // variable file must not donate an earlier ordinal to the winner.
    let selected = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(ordinal, candidate)| {
            let score = candidate_score(&candidate, request)?;
            Some((ordinal, score, candidate.matched))
        })
        .min_by_key(|(ordinal, score, _)| (*score, *ordinal));
    if let Some((ordinal, score, matched)) = selected.as_ref() {
        tracing::trace!(
            target: "font_boundary",
            family = matched.family(),
            identity = %matched.identity.stable_key,
            weight = matched.weight(),
            slant = ?matched.slant(),
            ordinal,
            ?score,
            "shared font resolver selected platform candidate"
        );
    }
    selected.map(|(_, _, matched)| matched)
}

fn family_search_order(
    backend: &dyn FontBackend,
    requested_family: &str,
    spec: &StoredFontSpec,
) -> Vec<Option<String>> {
    if let Some(spec_family) = spec.family.map(resolve_sym) {
        return vec![Some(backend.resolve_family(spec_family))];
    }
    if requested_family.is_empty() {
        return vec![None];
    }

    let mut order = Vec::new();
    for family in alternative_font_families(requested_family) {
        let resolved = backend.resolve_family(&family);
        if resolved != family {
            order.push(Some(resolved));
        }
        order.push(Some(family));
    }
    // `None` asks the native backend for its ordered cascade from the base
    // family. This is discovery, not policy: it is reached only after every
    // GNU fontset/alternative-family pass has failed.
    order.push(None);
    order
}

fn candidate_score(
    candidate: &FontCandidate,
    request: &SelectionRequest<'_>,
) -> Option<CandidateSelectionScore> {
    let candidate_weight = candidate.matched.weight().unwrap_or(400);
    let compatibility =
        spacing_score(
            request.spacing,
            candidate.matched.metadata.spacing,
            request.prefer_monospace,
        ) + family_affinity_score(request.queried_family, candidate.matched.family());
    candidate_selection_score(
        compatibility,
        request.size.device_px_26_6(),
        request.weight,
        request.slant,
        request.width,
        candidate_weight,
        candidate.matched.slant(),
        candidate.matched.metadata.width,
        candidate.matched.metadata.size,
    )
}

fn spacing_score(
    requested_spacing: Option<i32>,
    candidate_spacing: Option<i32>,
    prefer_monospace: bool,
) -> u32 {
    let requested = requested_spacing.and_then(normalize_spacing);
    let candidate = candidate_spacing.and_then(normalize_spacing);
    match (requested, candidate) {
        (Some(requested), Some(candidate)) if requested == candidate => 0,
        (Some(SpacingClass::Mono | SpacingClass::Charcell), Some(SpacingClass::Dual))
            if prefer_monospace =>
        {
            25
        }
        (Some(SpacingClass::Dual), Some(SpacingClass::Mono | SpacingClass::Charcell))
            if prefer_monospace =>
        {
            25
        }
        (Some(_), None) if prefer_monospace => 800,
        (Some(requested), Some(candidate)) => spacing_distance(requested, candidate),
        // GNU ftfont does not turn a monospace family into an implicit exact
        // FC_SPACING request. `prefer_monospace` only relaxes an explicit
        // mono/dual request above.
        _ => 0,
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SpacingClass {
    Proportional,
    Dual,
    Mono,
    Charcell,
}

fn normalize_spacing(spacing: i32) -> Option<SpacingClass> {
    match spacing {
        i32::MIN..=-1 => None,
        0..=89 => Some(SpacingClass::Proportional),
        90..=99 => Some(SpacingClass::Dual),
        100..=109 => Some(SpacingClass::Mono),
        _ => Some(SpacingClass::Charcell),
    }
}

fn spacing_distance(requested: SpacingClass, candidate: SpacingClass) -> u32 {
    use SpacingClass::{Charcell, Dual, Mono, Proportional};
    match (requested, candidate) {
        (Proportional, Dual) | (Dual, Proportional) => 500,
        (Proportional, Mono) | (Mono, Proportional) => 800,
        (Proportional, Charcell) | (Charcell, Proportional) => 1_000,
        (Dual, Mono) | (Mono, Dual) => 200,
        (Dual, Charcell) | (Charcell, Dual) => 250,
        (Mono, Charcell) | (Charcell, Mono) => 100,
        _ => 0,
    }
}

fn family_affinity_score(queried_family: Option<&str>, candidate_family: &str) -> u32 {
    let Some(queried_family) = queried_family.filter(|family| !family.is_empty()) else {
        return 0;
    };
    let queried = queried_family.to_ascii_lowercase();
    let candidate = candidate_family.to_ascii_lowercase();
    if candidate == queried {
        0
    } else if candidate.starts_with(&queried) || queried.starts_with(&candidate) {
        5
    } else if candidate.contains(&queried) || queried.contains(&candidate) {
        15
    } else {
        80
    }
}

#[cfg(test)]
#[path = "resolver_test.rs"]
mod tests;
