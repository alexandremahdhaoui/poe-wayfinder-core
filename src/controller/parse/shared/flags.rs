//! Stages that read a whole line flag.
//!
//! Ported from `parseCorrupted`, `parseFoil`, `parseUnidentified`,
//! `parseInfluence`, `parseMirrored`, `parseSanctified` and `parseSynthesised`
//! in `renderer/src/parser/Parser.ts`.

use crate::controller::parse::{ParseOutcome, ParserState};
use crate::types::client_strings as cs;
use crate::types::item::{Influence, ItemRarity};
use crate::util::number::leading_int;

/// `Corrupted`, `Twice Corrupted` or `Unmodifiable`.
///
/// Unmodifiable implies corrupted. An unmodifiable item cannot take a craft,
/// which is the only thing corruption changes for pricing.
pub fn parse_corrupted(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    let trimmed = first.trim();

    if trimmed == cs::CORRUPTED || trimmed == cs::DOUBLE_CORRUPTED {
        state.item.is_corrupted = true;

        return ParseOutcome::SectionParsed;
    }

    // Not trimmed, matching the reference. Unmodifiable is printed alone.
    if first == cs::UNMODIFIABLE {
        state.item.is_corrupted = true;
        state.item.is_unmodifiable = true;

        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// `Mirrored`.
pub fn parse_mirrored(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if section.first().is_some_and(|l| l == cs::MIRRORED) {
        state.item.is_mirrored = true;

        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// `Sanctified`.
pub fn parse_sanctified(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if section.first().is_some_and(|l| l == cs::SANCTIFIED) {
        state.item.is_sanctified = true;

        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// `Foil Unique`.
///
/// Only uniques can be foil, so the stage bails on everything else rather than
/// walking every section for a line that cannot be there.
pub fn parse_foil(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.rarity != Some(ItemRarity::Unique) {
        return ParseOutcome::ParserSkipped;
    }

    if section.first().is_some_and(|l| l == cs::FOIL_UNIQUE) {
        state.item.is_foil = true;

        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// `Fractured Item`.
pub fn parse_fractured_flag(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if section.first().is_some_and(|l| l == cs::FRACTURED_ITEM) {
        state.item.is_fractured = true;

        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// `Unidentified`, optionally with a tier.
///
/// PoE2 prints `Unidentified (Tier 3)` on waystones. The tier survives
/// identification, so it is worth capturing.
pub fn parse_unidentified(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    let Some(rest) = first.strip_prefix("Unidentified") else {
        return ParseOutcome::SectionSkipped;
    };

    let rest = rest.trim();

    if rest.is_empty() {
        state.item.is_unidentified = true;

        return ParseOutcome::SectionParsed;
    }

    // The only accepted decoration is a tier in parentheses. Anything else is
    // a different line that happens to start with the same word.
    let Some(inner) = rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else {
        return ParseOutcome::SectionSkipped;
    };

    let Some(tier_text) = inner.trim().strip_prefix("Tier") else {
        return ParseOutcome::SectionSkipped;
    };

    let Some(tier) = leading_int(tier_text) else {
        return ParseOutcome::SectionSkipped;
    };

    state.item.is_unidentified = true;
    state.item.unidentified_tier = Some(tier as u32);

    ParseOutcome::SectionParsed
}

/// The six PoE1 influence lines.
///
/// An item can carry two, so the section is scanned whole. The reference only
/// looks at sections of two lines or fewer, because a longer section holding
/// the word `Shaper Item` is a modifier and not an influence.
pub fn parse_influence(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if section.len() > 2 {
        return ParseOutcome::SectionSkipped;
    }

    let before = state.item.influences.len();

    for line in section {
        let found = match line.as_str() {
            cs::INFLUENCE_CRUSADER => Some(Influence::Crusader),
            cs::INFLUENCE_ELDER => Some(Influence::Elder),
            cs::INFLUENCE_SHAPER => Some(Influence::Shaper),
            cs::INFLUENCE_HUNTER => Some(Influence::Hunter),
            cs::INFLUENCE_REDEEMER => Some(Influence::Redeemer),
            cs::INFLUENCE_WARLORD => Some(Influence::Warlord),
            _ => None,
        };

        if let Some(influence) = found {
            state.item.influences.push(influence);
        }
    }

    if state.item.influences.len() > before {
        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// `Synthesised Item`, plus the `Synthesised ` name prefix.
///
/// The name prefix has to come off or the database lookup misses. A synthesised
/// Vaal Regalia is still a Vaal Regalia.
pub fn parse_synthesised(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if !section
        .first()
        .is_some_and(|l| l == cs::SECTION_SYNTHESISED)
    {
        return ParseOutcome::SectionSkipped;
    }

    state.item.is_synthesised = true;

    match state.base_type.as_deref() {
        Some(base) => {
            if let Some(stripped) = base.strip_prefix(cs::ITEM_SYNTHESISED) {
                state.base_type = Some(stripped.to_string());
            }
        }
        None => {
            if let Some(stripped) = state.name.strip_prefix(cs::ITEM_SYNTHESISED) {
                state.name = stripped.to_string();
            }
        }
    }

    ParseOutcome::SectionParsed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sec(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn state() -> ParserState {
        ParserState::default()
    }

    #[test]
    fn corrupted_is_recognised() {
        let mut s = state();

        assert_eq!(
            parse_corrupted(&sec(&["Corrupted"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert!(s.item.is_corrupted);
        assert!(!s.item.is_unmodifiable);
    }

    #[test]
    fn twice_corrupted_is_still_corrupted() {
        let mut s = state();

        parse_corrupted(&sec(&["Twice Corrupted"]), &mut s);

        assert!(s.item.is_corrupted);
    }

    #[test]
    fn surrounding_whitespace_does_not_hide_corruption() {
        let mut s = state();

        assert_eq!(
            parse_corrupted(&sec(&["  Corrupted  "]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert!(s.item.is_corrupted);
    }

    #[test]
    fn unmodifiable_implies_corrupted() {
        let mut s = state();

        parse_corrupted(&sec(&["Unmodifiable"]), &mut s);

        assert!(s.item.is_corrupted);
        assert!(s.item.is_unmodifiable);
    }

    #[test]
    fn another_line_is_skipped() {
        let mut s = state();

        assert_eq!(
            parse_corrupted(&sec(&["Item Level: 84"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert!(!s.item.is_corrupted);
    }

    #[test]
    fn an_empty_section_is_skipped_rather_than_panicking() {
        let mut s = state();

        assert_eq!(parse_corrupted(&[], &mut s), ParseOutcome::SectionSkipped);
    }

    #[test]
    fn mirrored_and_sanctified_are_recognised() {
        let mut s = state();
        parse_mirrored(&sec(&["Mirrored"]), &mut s);
        assert!(s.item.is_mirrored);

        let mut s = state();
        parse_sanctified(&sec(&["Sanctified"]), &mut s);
        assert!(s.item.is_sanctified);
    }

    #[test]
    fn foil_only_applies_to_uniques() {
        let mut s = state();
        s.item.rarity = Some(ItemRarity::Rare);

        assert_eq!(
            parse_foil(&sec(&["Foil Unique"]), &mut s),
            ParseOutcome::ParserSkipped
        );
        assert!(!s.item.is_foil);
    }

    #[test]
    fn foil_is_recognised_on_a_unique() {
        let mut s = state();
        s.item.rarity = Some(ItemRarity::Unique);

        assert_eq!(
            parse_foil(&sec(&["Foil Unique"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert!(s.item.is_foil);
    }

    #[test]
    fn a_unique_without_the_foil_line_keeps_looking() {
        let mut s = state();
        s.item.rarity = Some(ItemRarity::Unique);

        assert_eq!(
            parse_foil(&sec(&["Corrupted"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn the_fractured_flag_is_recognised() {
        let mut s = state();

        parse_fractured_flag(&sec(&["Fractured Item"]), &mut s);

        assert!(s.item.is_fractured);
    }

    #[test]
    fn unidentified_with_no_tier() {
        let mut s = state();

        assert_eq!(
            parse_unidentified(&sec(&["Unidentified"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert!(s.item.is_unidentified);
        assert_eq!(s.item.unidentified_tier, None);
    }

    #[test]
    fn unidentified_with_a_tier() {
        let mut s = state();

        parse_unidentified(&sec(&["Unidentified (Tier 3)"]), &mut s);

        assert!(s.item.is_unidentified);
        assert_eq!(s.item.unidentified_tier, Some(3));
    }

    #[test]
    fn unidentified_tolerates_extra_spacing() {
        let mut s = state();

        parse_unidentified(&sec(&["Unidentified (Tier  12)"]), &mut s);

        assert_eq!(s.item.unidentified_tier, Some(12));
    }

    #[test]
    fn a_line_merely_starting_with_unidentified_is_skipped() {
        // Guarding against a modifier such as "Unidentified Items drop..."
        let mut s = state();

        assert_eq!(
            parse_unidentified(&sec(&["Unidentified Items are worth more"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert!(!s.item.is_unidentified);
    }

    #[test]
    fn a_malformed_tier_is_skipped() {
        let mut s = state();

        for line in ["Unidentified (Tier)", "Unidentified (3)", "Unidentified ("] {
            assert_eq!(
                parse_unidentified(&sec(&[line]), &mut s),
                ParseOutcome::SectionSkipped,
                "{line}"
            );
        }

        assert!(!s.item.is_unidentified);
    }

    #[test]
    fn one_influence_is_recognised() {
        let mut s = state();

        assert_eq!(
            parse_influence(&sec(&["Shaper Item"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert_eq!(s.item.influences, vec![Influence::Shaper]);
    }

    #[test]
    fn two_influences_are_both_kept() {
        let mut s = state();

        parse_influence(&sec(&["Shaper Item", "Elder Item"]), &mut s);

        assert_eq!(s.item.influences, vec![Influence::Shaper, Influence::Elder]);
    }

    #[test]
    fn every_influence_line_is_wired_up() {
        for (line, want) in [
            ("Crusader Item", Influence::Crusader),
            ("Elder Item", Influence::Elder),
            ("Shaper Item", Influence::Shaper),
            ("Hunter Item", Influence::Hunter),
            ("Redeemer Item", Influence::Redeemer),
            ("Warlord Item", Influence::Warlord),
        ] {
            let mut s = state();
            parse_influence(&sec(&[line]), &mut s);

            assert_eq!(s.item.influences, vec![want], "{line}");
        }
    }

    #[test]
    fn a_long_section_is_skipped_even_when_it_holds_an_influence_line() {
        // A modifier section can contain the exact words. Reading it as an
        // influence would put a filter on the trade query that the item does
        // not have.
        let mut s = state();

        assert_eq!(
            parse_influence(&sec(&["Shaper Item", "Elder Item", "Hunter Item"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert!(s.item.influences.is_empty());
    }

    #[test]
    fn a_section_with_no_influence_is_skipped() {
        let mut s = state();

        assert_eq!(
            parse_influence(&sec(&["Corrupted"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn synthesised_strips_the_prefix_from_the_base_type() {
        let mut s = ParserState {
            name: "Doom Shell".into(),
            base_type: Some("Synthesised Vaal Regalia".into()),
            ..ParserState::default()
        };

        assert_eq!(
            parse_synthesised(&sec(&["Synthesised Item"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert!(s.item.is_synthesised);
        assert_eq!(s.base_type.as_deref(), Some("Vaal Regalia"));
        assert_eq!(s.name, "Doom Shell");
    }

    #[test]
    fn synthesised_strips_the_prefix_from_the_name_when_there_is_no_base_type() {
        let mut s = ParserState {
            name: "Synthesised Vaal Regalia".into(),
            ..ParserState::default()
        };

        parse_synthesised(&sec(&["Synthesised Item"]), &mut s);

        assert_eq!(s.name, "Vaal Regalia");
    }

    #[test]
    fn synthesised_leaves_a_name_that_lacks_the_prefix_alone() {
        let mut s = ParserState {
            name: "Vaal Regalia".into(),
            ..ParserState::default()
        };

        parse_synthesised(&sec(&["Synthesised Item"]), &mut s);

        assert!(s.item.is_synthesised);
        assert_eq!(s.name, "Vaal Regalia");
    }

    #[test]
    fn a_section_without_the_synthesised_line_is_skipped() {
        let mut s = ParserState {
            name: "Synthesised Vaal Regalia".into(),
            ..ParserState::default()
        };

        assert_eq!(
            parse_synthesised(&sec(&["Corrupted"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert!(!s.item.is_synthesised);
        assert_eq!(s.name, "Synthesised Vaal Regalia");
    }
}
