//! Platform-neutral GNU font-entity scoring.
//!
//! GNU `font_score` stores four independent seven-bit distances. Their bit
//! positions make comparison lexicographic in `face-font-selection-order`
//! (by default: width, size, weight, slant); unrelated properties are not an
//! additive bag of penalties. Rust field order plus derived [`Ord`] expresses
//! that policy directly and prevents a refactor from silently reordering it.

use neovm_core::face::{FontSlant, FontWidth};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PropertyDistance(u32);

/// GNU's default style-property priority, represented without bit arithmetic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct GnuStyleScore {
    width: PropertyDistance,
    size: PropertyDistance,
    weight: PropertyDistance,
    slant: PropertyDistance,
}

/// Complete candidate score before the stable discovery-order tie break.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CandidateSelectionScore {
    /// Non-style compatibility supplied by the caller (family/spacing).
    compatibility: u32,
    style: GnuStyleScore,
}

pub(crate) fn candidate_selection_score(
    compatibility: u32,
    requested_weight: u16,
    requested_slant: FontSlant,
    requested_width: Option<FontWidth>,
    candidate_weight: u16,
    candidate_slant: FontSlant,
    candidate_width: Option<FontWidth>,
) -> CandidateSelectionScore {
    CandidateSelectionScore {
        compatibility,
        style: GnuStyleScore {
            width: PropertyDistance(requested_width.map_or(0, |requested| {
                u32::from(
                    candidate_width
                        .unwrap_or(FontWidth::Normal)
                        .gnu_numeric()
                        .abs_diff(requested.gnu_numeric()),
                )
            })),
            // Candidate enumeration does not currently expose bitmap sizes;
            // scalable entities therefore have GNU distance zero here.
            size: PropertyDistance(0),
            weight: PropertyDistance(u32::from(candidate_weight.abs_diff(requested_weight))),
            slant: PropertyDistance(slant_distance(requested_slant, candidate_slant)),
        },
    }
}

fn slant_distance(requested: FontSlant, candidate: FontSlant) -> u32 {
    use FontSlant::{Italic, Normal, Oblique, ReverseItalic, ReverseOblique};
    match (requested, candidate) {
        (Normal, Normal) => 0,
        (Italic, Italic) | (Italic, Oblique) => 0,
        (Oblique, Oblique) | (Oblique, Italic) => 0,
        (ReverseItalic, ReverseItalic) | (ReverseItalic, ReverseOblique) => 0,
        (ReverseOblique, ReverseOblique) | (ReverseOblique, ReverseItalic) => 0,
        (Normal, _) => 350,
        (_, Normal) => 250,
        _ => 75,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_distance_precedes_weight_distance() {
        let wrong_weight = candidate_selection_score(
            0,
            400,
            FontSlant::Normal,
            Some(FontWidth::Normal),
            900,
            FontSlant::Normal,
            Some(FontWidth::Normal),
        );
        let wrong_width = candidate_selection_score(
            0,
            400,
            FontSlant::Normal,
            Some(FontWidth::Normal),
            400,
            FontSlant::Normal,
            Some(FontWidth::Expanded),
        );
        assert!(wrong_weight < wrong_width);
    }

    #[test]
    fn weight_distance_precedes_slant_distance() {
        let wrong_slant = candidate_selection_score(
            0,
            400,
            FontSlant::Normal,
            Some(FontWidth::Normal),
            400,
            FontSlant::Italic,
            Some(FontWidth::Normal),
        );
        let wrong_weight = candidate_selection_score(
            0,
            400,
            FontSlant::Normal,
            Some(FontWidth::Normal),
            500,
            FontSlant::Normal,
            Some(FontWidth::Normal),
        );
        assert!(wrong_slant < wrong_weight);
    }
}
