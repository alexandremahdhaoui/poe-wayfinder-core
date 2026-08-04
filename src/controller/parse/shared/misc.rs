//! Stages that read a small section or exist only to consume one.
//!
//! Ported from `parseStackSize`, `parseSockets`, `parsePriceNote`,
//! `parseSentinelCharge`, `parseFlask`, `parseJewelery`, `parseCharmSlots`,
//! `parseSpirit` and `parseTimelostRadius` in `renderer/src/parser/Parser.ts`.
//!
//! Several of these record nothing. Their whole job is to claim a section so
//! the modifier stages never see it. A charm slot line read as a modifier
//! becomes an unknown modifier and the UI then warns the user for no reason.

use crate::controller::parse::shared::levels::parse_quality_nested;
use crate::controller::parse::{ParseOutcome, ParserState};
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::{GemSockets, ItemRarity, StackSize};
use crate::util::number::{digits_only, leading_int};

/// `Stack Size: 2,448/40`.
///
/// The separator inside the number varies with the client locale, so both
/// halves are read digits only.
pub fn parse_stack_size(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let applies = state.item.rarity == Some(ItemRarity::Normal)
        || matches!(
            state.item.category,
            Some(ItemCategory::Currency)
                | Some(ItemCategory::DivinationCard)
                | Some(ItemCategory::MapFragment)
        );

    if !applies {
        return ParseOutcome::ParserSkipped;
    }

    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    let Some(rest) = first.strip_prefix(cs::STACK_SIZE) else {
        return ParseOutcome::SectionSkipped;
    };

    let mut parts = rest.split('/');
    let value = parts.next().and_then(digits_only);
    let max = parts.next().and_then(digits_only);

    if let (Some(value), Some(max)) = (value, max) {
        state.item.stack_size = Some(StackSize {
            value: value as u32,
            max: max as u32,
        });
    }

    ParseOutcome::SectionParsed
}

/// `Sockets: G-G-G-R-R B` on a gem bearing base.
///
/// Only the link pattern matters for pricing, and only a five link or a six
/// link is worth searching on.
pub fn parse_sockets(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if !state.item.category.is_some_and(ItemCategory::is_gem) {
        return ParseOutcome::SectionSkipped;
    }

    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    let Some(rest) = first.strip_prefix(cs::SOCKETS) else {
        return ParseOutcome::SectionSkipped;
    };

    let raw = rest.trim_end();

    // Every colour becomes a hash so only the shape is left. Spaces and
    // hyphens are the shape.
    let shape: String = raw
        .chars()
        .map(|c| if c == ' ' || c == '-' { c } else { '#' })
        .collect();

    let linked = match shape.as_str() {
        "#-#-#-#-#-#" => Some(6),
        "# #-#-#-#-#" | "#-#-#-#-# #" | "#-#-#-#-#" => Some(5),
        _ => None,
    };

    state.item.gem_sockets = Some(GemSockets {
        number: shape.matches('#').count() as u32,
        white: raw.matches('W').count() as u32,
        linked,
    });

    ParseOutcome::SectionParsed
}

/// `Note: ~price 5 divine`.
///
/// Kept verbatim. The seller wrote it and any rewriting loses information.
pub fn parse_price_note(section: &[String], state: &mut ParserState) -> ParseOutcome {
    for line in section {
        if let Some(rest) = line.strip_prefix(cs::PRICE_NOTE) {
            state.item.note = Some(rest.to_string());

            return ParseOutcome::SectionParsed;
        }
    }

    ParseOutcome::SectionSkipped
}

/// `Charge: 42` on a sentinel.
pub fn parse_sentinel_charge(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.category != Some(ItemCategory::Sentinel) {
        return ParseOutcome::ParserSkipped;
    }

    if section.len() != 1 {
        return ParseOutcome::SectionSkipped;
    }

    let Some(rest) = section[0].strip_prefix(cs::SENTINEL_CHARGE) else {
        return ParseOutcome::SectionSkipped;
    };

    state.item.sentinel_charge = leading_int(rest).map(|v| v as u32);

    ParseOutcome::SectionParsed
}

/// A flask buff section.
///
/// Recorded value: none. It exists to stop `Currently has 12 Charges` and the
/// buff text below it being read as modifiers.
pub fn parse_flask(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let has_charges = section
        .iter()
        .any(|l| l.starts_with("Currently has ") && l.ends_with(" Charges"));

    if !has_charges {
        return ParseOutcome::SectionSkipped;
    }

    parse_quality_nested(section, state);

    ParseOutcome::SectionParsed
}

/// A jewellery quality section.
///
/// Rings, amulets and belts print `Quality (Attack Modifiers): +20%` and
/// similar. Matching on the bare word covers every variant.
pub fn parse_jewelery(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let applies = matches!(
        state.item.category,
        Some(ItemCategory::Amulet) | Some(ItemCategory::Ring) | Some(ItemCategory::Belt)
    );

    if !applies {
        return ParseOutcome::ParserSkipped;
    }

    // The reference slices the quality prefix up to its colon, which leaves
    // the bare word. A catalyst quality line carries a qualifier before the
    // colon, so the full prefix would miss it.
    let bare = cs::QUALITY.split(':').next().unwrap_or(cs::QUALITY);

    if section.iter().any(|l| l.starts_with(bare)) {
        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// `Charm Slots: 1` on a belt.
///
/// Recorded value: none. Consumed so it is not read as a modifier.
pub fn parse_charm_slots(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.category != Some(ItemCategory::Belt) {
        return ParseOutcome::ParserSkipped;
    }

    if section.iter().any(|l| l.starts_with(cs::CHARM_SLOTS)) {
        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// `Spirit: 100` on a sceptre.
///
/// Recorded value: none here. The weapon stage records the number when the
/// line shares a section with the other weapon numbers. This stage catches the
/// case where it stands alone.
pub fn parse_spirit(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.category != Some(ItemCategory::Sceptre) {
        return ParseOutcome::ParserSkipped;
    }

    if section.iter().any(|l| l.starts_with(cs::BASE_SPIRIT)) {
        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// `Radius: Large` on a timeless jewel.
///
/// Recorded value: none. Consumed so it is not read as a modifier.
pub fn parse_timelost_radius(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.category != Some(ItemCategory::Jewel) {
        return ParseOutcome::ParserSkipped;
    }

    if section.iter().any(|l| l.starts_with(cs::TIMELESS_RADIUS)) {
        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sec(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn currency() -> ParserState<'static> {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Currency);

        s
    }

    #[test]
    fn a_stack_size_is_read() {
        let mut s = currency();

        assert_eq!(
            parse_stack_size(&sec(&["Stack Size: 12/20"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert_eq!(s.item.stack_size, Some(StackSize { value: 12, max: 20 }));
    }

    #[test]
    fn a_thousands_separator_does_not_break_a_stack_size() {
        // Portal scrolls stack to 40 but the game still groups the digits.
        let mut s = currency();

        parse_stack_size(&sec(&["Stack Size: 2,448/40"]), &mut s);

        assert_eq!(
            s.item.stack_size,
            Some(StackSize {
                value: 2448,
                max: 40
            })
        );
    }

    #[test]
    fn a_normal_item_can_carry_a_stack_size() {
        let mut s = ParserState::default();
        s.item.rarity = Some(ItemRarity::Normal);

        parse_stack_size(&sec(&["Stack Size: 3/10"]), &mut s);

        assert_eq!(s.item.stack_size, Some(StackSize { value: 3, max: 10 }));
    }

    #[test]
    fn a_rare_item_skips_the_stack_size_stage() {
        let mut s = ParserState::default();
        s.item.rarity = Some(ItemRarity::Rare);

        assert_eq!(
            parse_stack_size(&sec(&["Stack Size: 3/10"]), &mut s),
            ParseOutcome::ParserSkipped
        );
        assert_eq!(s.item.stack_size, None);
    }

    #[test]
    fn a_malformed_stack_size_still_claims_the_section() {
        // The reference claims the section on the prefix alone. Leaving it for
        // the modifier stages would turn it into an unknown modifier.
        let mut s = currency();

        assert_eq!(
            parse_stack_size(&sec(&["Stack Size: lots"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert_eq!(s.item.stack_size, None);
    }

    #[test]
    fn a_six_link_is_recognised() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Gem);

        parse_sockets(&sec(&["Sockets: G-G-G-R-R-B"]), &mut s);

        let sockets = s.item.gem_sockets.unwrap();
        assert_eq!(sockets.number, 6);
        assert_eq!(sockets.linked, Some(6));
    }

    #[test]
    fn every_five_link_shape_is_recognised() {
        for shape in ["R G-G-G-R-R", "G-G-G-R-R R", "G-G-G-R-R"] {
            let mut s = ParserState::default();
            s.item.category = Some(ItemCategory::Gem);

            parse_sockets(&sec(&[&format!("Sockets: {shape}")]), &mut s);

            assert_eq!(s.item.gem_sockets.unwrap().linked, Some(5), "{shape}");
        }
    }

    #[test]
    fn an_unlinked_group_reports_no_link() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Gem);

        parse_sockets(&sec(&["Sockets: G G G R"]), &mut s);

        let sockets = s.item.gem_sockets.unwrap();
        assert_eq!(sockets.number, 4);
        assert_eq!(sockets.linked, None);
    }

    #[test]
    fn white_sockets_are_counted() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Gem);

        parse_sockets(&sec(&["Sockets: W-W-G"]), &mut s);

        assert_eq!(s.item.gem_sockets.unwrap().white, 2);
    }

    #[test]
    fn a_non_gem_base_is_skipped_by_the_socket_stage() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Bow);

        assert_eq!(
            parse_sockets(&sec(&["Sockets: G-G-G"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert_eq!(s.item.gem_sockets, None);
    }

    #[test]
    fn a_price_note_is_kept_verbatim() {
        let mut s = ParserState::default();

        parse_price_note(&sec(&["Note: ~price 5 divine"]), &mut s);

        assert_eq!(s.item.note.as_deref(), Some("~price 5 divine"));
    }

    #[test]
    fn a_price_note_is_found_anywhere_in_the_section() {
        let mut s = ParserState::default();

        parse_price_note(&sec(&["Corrupted", "Note: ~b/o 1 chaos"]), &mut s);

        assert_eq!(s.item.note.as_deref(), Some("~b/o 1 chaos"));
    }

    #[test]
    fn a_section_with_no_note_is_skipped() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_price_note(&sec(&["Corrupted"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn a_sentinel_charge_is_read() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Sentinel);

        parse_sentinel_charge(&sec(&["Charge: 42"]), &mut s);

        assert_eq!(s.item.sentinel_charge, Some(42));
    }

    #[test]
    fn a_multi_line_sentinel_section_is_skipped() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Sentinel);

        assert_eq!(
            parse_sentinel_charge(&sec(&["Charge: 42", "Corrupted"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn a_non_sentinel_skips_the_charge_stage() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_sentinel_charge(&sec(&["Charge: 42"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    #[test]
    fn a_flask_charge_section_is_consumed() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_flask(&sec(&["Quality: +20%", "Currently has 12 Charges"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert_eq!(s.item.quality, Some(20));
    }

    #[test]
    fn a_section_without_a_charge_line_is_left_for_the_modifier_stages() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_flask(&sec(&["Consumes 10 Charges on use"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn jewellery_quality_is_consumed_with_or_without_a_qualifier() {
        for line in ["Quality: +20%", "Quality (Attack Modifiers): +20%"] {
            let mut s = ParserState::default();
            s.item.category = Some(ItemCategory::Ring);

            assert_eq!(
                parse_jewelery(&sec(&[line]), &mut s),
                ParseOutcome::SectionParsed,
                "{line}"
            );
        }
    }

    #[test]
    fn every_jewellery_base_is_covered() {
        for c in [ItemCategory::Amulet, ItemCategory::Ring, ItemCategory::Belt] {
            let mut s = ParserState::default();
            s.item.category = Some(c);

            assert_eq!(
                parse_jewelery(&sec(&["Quality: +5%"]), &mut s),
                ParseOutcome::SectionParsed,
                "{c:?}"
            );
        }
    }

    #[test]
    fn a_non_jewellery_base_skips_the_stage() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Bow);

        assert_eq!(
            parse_jewelery(&sec(&["Quality: +20%"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    #[test]
    fn a_charm_slot_line_is_consumed_on_a_belt() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Belt);

        assert_eq!(
            parse_charm_slots(&sec(&["Charm Slots: 1"]), &mut s),
            ParseOutcome::SectionParsed
        );
    }

    #[test]
    fn a_charm_slot_line_on_anything_else_skips_the_stage() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Ring);

        assert_eq!(
            parse_charm_slots(&sec(&["Charm Slots: 1"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    #[test]
    fn a_lone_spirit_line_is_consumed_on_a_sceptre() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Sceptre);

        assert_eq!(
            parse_spirit(&sec(&["Spirit: 100"]), &mut s),
            ParseOutcome::SectionParsed
        );
    }

    #[test]
    fn a_timeless_radius_is_consumed_on_a_jewel() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Jewel);

        assert_eq!(
            parse_timelost_radius(&sec(&["Radius: Large"]), &mut s),
            ParseOutcome::SectionParsed
        );
    }

    #[test]
    fn a_timeless_radius_on_a_non_jewel_skips_the_stage() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Ring);

        assert_eq!(
            parse_timelost_radius(&sec(&["Radius: Large"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }
}
