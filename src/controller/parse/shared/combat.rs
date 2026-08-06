//! Stages that read the defensive and offensive numbers.
//!
//! Ported from `parseArmour`, `parseWeapon` and `parseCaster` in
//! `renderer/src/parser/Parser.ts`.
//!
//! # A unique's numbers are read like anything else
//!
//! These stages used to clear the armour and weapon values whenever the item
//! was unique, reasoning that a base roll filter built from them would exclude
//! every copy of the item.
//!
//! That is a filter decision made in the wrong place. The reference reads the
//! numbers and asserts it does: `parsers.test.ts` "Unique Armour" expects an
//! energy shield of 44 on a unique focus. Throwing them away here meant a
//! unique's defences never reached the panel and could never be shown, and the
//! decision about whether to filter on them could not be revisited.
//!
//! The filter layer makes that decision now, in `item_property.rs`, where the
//! item's rarity is already known.

use crate::controller::parse::shared::levels::parse_quality_nested;
use crate::controller::parse::{ParseOutcome, ParserState};
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::{ArmourStats, WeaponStats};
use crate::util::number::{damage_list_sum, damage_range_avg, leading_float, leading_int};

/// Armour, evasion, energy shield, ward and block.
pub fn parse_armour(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let mut parsed = false;

    for line in section {
        let hit = read_armour_line(line, &mut state.item.armour);
        parsed |= hit;
    }

    if !parsed {
        return ParseOutcome::SectionSkipped;
    }

    parse_quality_nested(section, state);

    ParseOutcome::SectionParsed
}

fn read_armour_line(line: &str, out: &mut ArmourStats) -> bool {
    if let Some(rest) = line.strip_prefix(cs::ARMOUR) {
        out.ar = leading_int(rest).map(|v| v as f64);

        return out.ar.is_some();
    }

    if let Some(rest) = line.strip_prefix(cs::EVASION) {
        out.ev = leading_int(rest).map(|v| v as f64);

        return out.ev.is_some();
    }

    if let Some(rest) = line.strip_prefix(cs::ENERGY_SHIELD) {
        out.es = leading_int(rest).map(|v| v as f64);

        return out.es.is_some();
    }

    if let Some(rest) = line.strip_prefix(cs::BLOCK_CHANCE) {
        out.block = leading_int(rest).map(|v| v as f64);

        return out.block.is_some();
    }

    if let Some(rest) = line.strip_prefix(cs::RUNIC_WARD) {
        out.rw = leading_int(rest).map(|v| v as f64);

        return out.rw.is_some();
    }

    false
}

/// Crit, attack speed, damage, reload and spirit.
///
/// Fire, cold and lightning damage each add into the elemental total rather
/// than replacing it, because a weapon can print several element lines.
pub fn parse_weapon(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let mut parsed = false;

    for line in section {
        let hit = read_weapon_line(line, &mut state.item.weapon);
        parsed |= hit;
    }

    if !parsed {
        return ParseOutcome::SectionSkipped;
    }

    parse_quality_nested(section, state);

    ParseOutcome::SectionParsed
}

fn read_weapon_line(line: &str, out: &mut WeaponStats) -> bool {
    // Decimal fields. Crit chance and attack speed are never whole numbers, so
    // leading_int would silently truncate 1.35 to 1.
    if let Some(rest) = line.strip_prefix(cs::CRIT_CHANCE) {
        out.crit = leading_float(rest);

        return out.crit.is_some();
    }

    if let Some(rest) = line.strip_prefix(cs::ATTACK_SPEED) {
        out.attack_speed = leading_float(rest);

        return out.attack_speed.is_some();
    }

    if let Some(rest) = line.strip_prefix(cs::RELOAD_SPEED) {
        out.reload = leading_float(rest);

        return out.reload.is_some();
    }

    if let Some(rest) = line.strip_prefix(cs::PHYSICAL_DAMAGE) {
        out.physical = damage_range_avg(rest);

        return true;
    }

    if let Some(rest) = line.strip_prefix(cs::ELEMENTAL_DAMAGE) {
        add_elemental(out, damage_list_sum(rest));

        return true;
    }

    // Each element line records itself AND adds into the elemental total, so a
    // filter can ask for fire damage on its own or for elemental as a whole.
    if let Some(rest) = line.strip_prefix(cs::FIRE_DAMAGE) {
        out.fire = damage_list_sum(rest);
        add_elemental(out, out.fire);

        return true;
    }

    if let Some(rest) = line.strip_prefix(cs::COLD_DAMAGE) {
        out.cold = damage_list_sum(rest);
        add_elemental(out, out.cold);

        return true;
    }

    if let Some(rest) = line.strip_prefix(cs::LIGHTNING_DAMAGE) {
        out.lightning = damage_list_sum(rest);
        add_elemental(out, out.lightning);

        return true;
    }

    if let Some(rest) = line.strip_prefix(cs::BASE_SPIRIT) {
        out.spirit = leading_int(rest).map(|v| v as f64);

        return out.spirit.is_some();
    }

    false
}

/// Add one element's damage into the elemental total.
fn add_elemental(out: &mut WeaponStats, value: Option<f64>) {
    if let Some(v) = value {
        out.elemental = Some(out.elemental.unwrap_or(0.0) + v);
    }
}

/// A caster weapon section holding nothing but a quality line.
///
/// A wand with no spirit and no damage prints only its quality. Without this
/// stage that section survives to the modifier stages and is read as a
/// modifier.
pub fn parse_caster(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let applies = matches!(
        state.item.category,
        Some(ItemCategory::Wand) | Some(ItemCategory::Sceptre) | Some(ItemCategory::Staff)
    );

    if !applies {
        return ParseOutcome::ParserSkipped;
    }

    if section.len() == 1 && section[0].starts_with(cs::QUALITY) {
        parse_quality_nested(section, state);

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

    #[test]
    fn armour_reads_every_defensive_number() {
        let mut s = ParserState::default();

        let out = parse_armour(
            &sec(&[
                "Armour: 512",
                "Evasion Rating: 130",
                "Energy Shield: 44",
                "Block chance: 25",
                "Runic Ward: 60",
            ]),
            &mut s,
        );

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(s.item.armour.ar, Some(512.0));
        assert_eq!(s.item.armour.ev, Some(130.0));
        assert_eq!(s.item.armour.es, Some(44.0));
        assert_eq!(s.item.armour.block, Some(25.0));
        assert_eq!(s.item.armour.rw, Some(60.0));
    }

    #[test]
    fn armour_picks_up_quality_from_the_same_section() {
        let mut s = ParserState::default();

        parse_armour(&sec(&["Quality: +20% (augmented)", "Armour: 512"]), &mut s);

        assert_eq!(s.item.quality, Some(20));
    }

    #[test]
    fn armour_reads_an_augmented_number() {
        let mut s = ParserState::default();

        parse_armour(&sec(&["Armour: 512 (augmented)"]), &mut s);

        assert_eq!(s.item.armour.ar, Some(512.0));
    }

    #[test]
    fn a_unique_keeps_its_armour_numbers() {
        // The reference reads them and asserts it does. Clearing them here
        // meant a unique's defences never reached the panel, and it put a
        // filter decision in the parser. The filter layer decides.
        let mut s = ParserState::default();
        s.item.rarity = Some(crate::types::item::ItemRarity::Unique);

        let out = parse_armour(&sec(&["Armour: 512", "Quality: +20%"]), &mut s);

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(s.item.armour.ar, Some(512.0));
        assert_eq!(s.item.quality, Some(20));
    }

    #[test]
    fn a_section_with_no_defensive_number_is_skipped() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_armour(&sec(&["Quality: +20%"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        // Quality is not read either, because the stage never claimed the
        // section.
        assert_eq!(s.item.quality, None);
    }

    #[test]
    fn weapon_reads_crit_and_attack_speed_with_decimals() {
        let mut s = ParserState::default();

        parse_weapon(
            &sec(&["Critical Hit Chance: 6.50%", "Attacks per Second: 1.35"]),
            &mut s,
        );

        assert_eq!(s.item.weapon.crit, Some(6.5));
        assert_eq!(s.item.weapon.attack_speed, Some(1.35));
    }

    #[test]
    fn physical_damage_averages_its_range() {
        let mut s = ParserState::default();

        parse_weapon(&sec(&["Physical Damage: 45-90"]), &mut s);

        assert_eq!(s.item.weapon.physical, Some(67.5));
    }

    #[test]
    fn elemental_damage_sums_every_range_on_the_line() {
        let mut s = ParserState::default();

        parse_weapon(&sec(&["Elemental Damage: 10-20, 30-40"]), &mut s);

        assert_eq!(s.item.weapon.elemental, Some(15.0 + 35.0));
    }

    #[test]
    fn each_element_line_adds_into_the_elemental_total() {
        let mut s = ParserState::default();

        parse_weapon(
            &sec(&[
                "Fire Damage: 10-20",
                "Cold Damage: 30-40",
                "Lightning Damage: 1-99",
            ]),
            &mut s,
        );

        assert_eq!(s.item.weapon.fire, Some(15.0));
        assert_eq!(s.item.weapon.cold, Some(35.0));
        assert_eq!(s.item.weapon.lightning, Some(50.0));
        assert_eq!(s.item.weapon.elemental, Some(100.0));
    }

    #[test]
    fn an_element_line_adds_on_top_of_an_elemental_total_line() {
        let mut s = ParserState::default();

        parse_weapon(
            &sec(&["Elemental Damage: 10-20", "Fire Damage: 30-40"]),
            &mut s,
        );

        assert_eq!(s.item.weapon.elemental, Some(15.0 + 35.0));
    }

    #[test]
    fn reload_and_spirit_are_read() {
        let mut s = ParserState::default();

        parse_weapon(&sec(&["Reload Time: 0.75", "Spirit: 100"]), &mut s);

        assert_eq!(s.item.weapon.reload, Some(0.75));
        assert_eq!(s.item.weapon.spirit, Some(100.0));
    }

    #[test]
    fn a_unique_keeps_its_weapon_numbers() {
        let mut s = ParserState::default();
        s.item.rarity = Some(crate::types::item::ItemRarity::Unique);

        parse_weapon(&sec(&["Physical Damage: 45-90", "Quality: +20%"]), &mut s);

        assert_eq!(s.item.weapon.physical, Some(67.5));
        assert_eq!(s.item.quality, Some(20));
    }

    #[test]
    fn a_section_with_no_weapon_number_is_skipped() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_weapon(&sec(&["Item Level: 84"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn a_caster_weapon_claims_a_lone_quality_section() {
        // Without this the section reaches the modifier stages and a quality
        // line is read as an unknown modifier.
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Wand);

        assert_eq!(
            parse_caster(&sec(&["Quality: +20% (augmented)"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert_eq!(s.item.quality, Some(20));
    }

    #[test]
    fn every_caster_base_is_covered() {
        for c in [
            ItemCategory::Wand,
            ItemCategory::Sceptre,
            ItemCategory::Staff,
        ] {
            let mut s = ParserState::default();
            s.item.category = Some(c);

            assert_eq!(
                parse_caster(&sec(&["Quality: +5%"]), &mut s),
                ParseOutcome::SectionParsed,
                "{c:?}"
            );
        }
    }

    #[test]
    fn a_non_caster_skips_the_caster_stage() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Bow);

        assert_eq!(
            parse_caster(&sec(&["Quality: +20%"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    #[test]
    fn a_caster_section_with_more_than_one_line_is_skipped() {
        // A longer section belongs to the weapon stage, which reads the
        // damage numbers as well as the quality.
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Wand);

        assert_eq!(
            parse_caster(&sec(&["Quality: +20%", "Spirit: 100"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }
}
