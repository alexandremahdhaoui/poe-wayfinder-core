//! Parser stages that only PoE1 prints.
//!
//! Ported from `parseFoulborn`, `parseVestigial`, `parseMapTier`,
//! `parseAccessory`, `parseMemoryStrandsNested`, `parseTincture`,
//! `parseImbuedGem`, `parseHeistContract` and `parseSplit` in Awakened PoE
//! Trade's `renderer/src/parser/Parser.ts`.
//!
//! # Why these were missing
//!
//! Exiled Exchange 2 is a PoE2 fork. None of these lines exist in PoE2, so its
//! copy of the parser drops all nine stages. Porting only that reference
//! measured 100 percent parity while PoE1 could not read a heist contract, a
//! split item or a tincture.
//!
//! An unread line is not an error. It falls through to the modifier stages,
//! fails to match a stat and lands in `unknown_modifiers`, where nobody looks.

use crate::controller::parse::{ParseError, ParseOutcome, ParserState};
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::{HeistContract, ItemRarity};
use crate::util::number::leading_int;

/// Strip the `Foulborn ` prefix off a unique's name.
///
/// The prefix is a property of the item and not part of its name, so leaving
/// it on makes the database lookup miss and the unique price as an unknown.
pub fn parse_foulborn(state: &mut ParserState) -> Result<(), ParseError> {
    if state.item.rarity != Some(ItemRarity::Unique) || state.item.is_unidentified {
        return Ok(());
    }

    if let Some(rest) = state.name.strip_prefix(cs::ITEM_FOULBORN) {
        state.name = rest.to_string();
        state.item.is_foulborn = true;
    }

    Ok(())
}

/// Strip the `Vestigial ` prefix off a unique's base type.
///
/// The name, not the base, is what the reference checks for Foulborn. This one
/// is on the base, because that is where the game prints it.
pub fn parse_vestigial(state: &mut ParserState) -> Result<(), ParseError> {
    if state.item.rarity != Some(ItemRarity::Unique) {
        return Ok(());
    }

    let Some(base) = &state.base_type else {
        return Ok(());
    };

    if let Some(rest) = base.strip_prefix(cs::ITEM_VESTIGIAL) {
        state.base_type = Some(rest.to_string());
        state.item.is_vestigial = true;
    }

    Ok(())
}

/// Read and remove a ` (Tier 5)` suffix from the name plate.
///
/// The current PoE1 atlas prints the tier inside the base name rather than on
/// its own line. Leaving it on makes every tier of one map a different base to
/// the database, so none of them resolve.
pub fn parse_map_tier(state: &mut ParserState) -> Result<(), ParseError> {
    let target = match &state.base_type {
        Some(base) => base.clone(),
        None => state.name.clone(),
    };

    let Some((rest, tier)) = split_tier_suffix(&target) else {
        return Ok(());
    };

    state.item.map.tier = Some(tier);

    if state.base_type.is_some() {
        state.base_type = Some(rest);
    } else {
        state.name = rest;
    }

    Ok(())
}

/// Split a trailing ` (Tier N)` off a name.
///
/// Returns the name without it and the tier. Written as a scanner rather than
/// a regex, like every other prefix and suffix in this parser, so there is no
/// pattern to get subtly wrong.
fn split_tier_suffix(text: &str) -> Option<(String, u32)> {
    let head = text.strip_suffix(')')?;
    let (before, digits) = head.rsplit_once(" (Tier ")?;

    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    Some((before.to_string(), digits.parse().ok()?))
}

/// The accessory section, which carries memory strands.
///
/// Consumes the section so the strand count is not read as a modifier.
pub fn parse_accessory(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if !matches!(
        state.item.category,
        Some(ItemCategory::Ring) | Some(ItemCategory::Amulet) | Some(ItemCategory::Belt)
    ) {
        return ParseOutcome::ParserSkipped;
    }

    for line in section {
        let Some(rest) = line.strip_prefix(cs::MEMORY_STRANDS) else {
            continue;
        };

        state.item.memory_strands = leading_int(rest).map(|v| v as u32);

        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// The tincture section.
///
/// Exists to read the quality and consume the section. A tincture's buff lines
/// are not modifiers and reading them as such fills the panel with stats that
/// cannot be searched.
pub fn parse_tincture(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.category != Some(ItemCategory::Tincture) {
        return ParseOutcome::ParserSkipped;
    }

    let before = state.item.quality;

    crate::controller::parse::shared::levels::parse_quality_nested(section, state);

    // The section is only this one when the quality line was in it. Consuming
    // it either way would eat the tincture's real modifier block.
    if state.item.quality != before {
        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

/// A one line section on a gem that is a support stat.
///
/// An imbued gem carries a support the gem does not normally have, printed on
/// its own with no modifier heading. The stage is gated on the section being
/// one line and on that line matching a stat, so an ordinary gem passes
/// through untouched.
pub fn parse_imbued_gem(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if !state.item.category.is_some_and(|c| c.is_gem()) {
        return ParseOutcome::ParserSkipped;
    }

    if section.len() != 1 {
        return ParseOutcome::SectionSkipped;
    }

    if !line_is_a_known_stat(&section[0], state) {
        return ParseOutcome::SectionSkipped;
    }

    state.item.is_imbued_gem = true;

    ParseOutcome::SectionParsed
}

/// Whether the data file recognises a line as a stat.
fn line_is_a_known_stat(line: &str, state: &ParserState) -> bool {
    crate::controller::stat_match::placeholder::candidates(line)
        .iter()
        .any(|c| state.data.stat_by_matcher(&c.template).is_some())
}

/// The heist contract section.
///
/// The job and its level are most of what a contract is worth. A buyer wants
/// one job at one level, so a contract searched without them returns every
/// contract in the game.
pub fn parse_heist_contract(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.category != Some(ItemCategory::HeistContract) {
        return ParseOutcome::ParserSkipped;
    }

    // The area level line is what identifies this section. Without it this is
    // some other section that happens to sit on a contract.
    let Some(area_level) = section.iter().find_map(|l| {
        l.strip_prefix(cs::AREA_LEVEL)
            .and_then(leading_int)
            .map(|v| v as u32)
    }) else {
        return ParseOutcome::SectionSkipped;
    };

    state.item.area_level = Some(area_level);

    let mut contract = HeistContract::default();

    for line in section {
        if let Some((job, level)) = read_job(line) {
            contract.required_job = Some(job);
            contract.job_level = Some(level);

            continue;
        }

        if line.starts_with(cs::HEIST_CONTRACT_TARGET)
            && line.ends_with(&format!("({})", cs::HEIST_TARGET_PRICELESS))
        {
            contract.priceless = true;
        }
    }

    state.item.heist_contract = Some(contract);

    ParseOutcome::SectionParsed
}

/// Read `Requires Lockpicking (Level 3)` into its two halves.
///
/// The reference allows a trailing ` (unmet)`, which the game prints when the
/// character cannot run the contract yet. The contract is worth the same
/// either way, so the marker is dropped rather than recorded.
fn read_job(line: &str) -> Option<(String, u32)> {
    // Removed first so the rest of the read does not have to know where the
    // game puts it. The reference's pattern allows it inside the level
    // parentheses, and dropping it up front handles either placement.
    let line = line.replace(" (unmet)", "");

    let rest = line.strip_prefix(cs::HEIST_CONTRACT_REQUIRES)?;
    let rest = rest.trim_end().strip_suffix(')')?;

    let (job, level) = rest.trim_end().rsplit_once(" (Level ")?;

    // Only the nine jobs the trade site indexes. Anything else is a different
    // line that happens to read like this one, and inventing a tenth job would
    // send a trade id that does not exist.
    let job = cs::HEIST_JOBS
        .iter()
        .find(|(name, _)| *name == job)
        .map(|(name, _)| (*name).to_string())?;

    Some((job, level.parse().ok()?))
}

/// A one line section reading `Split`.
///
/// A split item cannot be split again, so the trade site indexes it and a
/// buyer filters on it.
pub fn parse_split(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if section.len() != 1 || section[0] != cs::SPLIT {
        return ParseOutcome::SectionSkipped;
    }

    state.item.is_split = true;

    ParseOutcome::SectionParsed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ParserState<'static> {
        ParserState {
            game: crate::types::game::GameVersion::Poe1,
            ..ParserState::default()
        }
    }

    fn sec(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    // -----------------------------------------------------------------
    // Foulborn and Vestigial
    // -----------------------------------------------------------------

    #[test]
    fn a_foulborn_unique_loses_the_prefix_from_its_name() {
        let mut s = state();
        s.item.rarity = Some(ItemRarity::Unique);
        s.name = "Foulborn Brightbeak".into();

        parse_foulborn(&mut s).unwrap();

        assert_eq!(s.name, "Brightbeak");
        assert!(s.item.is_foulborn);
    }

    #[test]
    fn an_unidentified_unique_keeps_the_foulborn_prefix() {
        // Its name is not printed yet, so whatever is there is the base type
        // and stripping a word off it breaks the lookup.
        let mut s = state();
        s.item.rarity = Some(ItemRarity::Unique);
        s.item.is_unidentified = true;
        s.name = "Foulborn Rusted Sword".into();

        parse_foulborn(&mut s).unwrap();

        assert_eq!(s.name, "Foulborn Rusted Sword");
    }

    #[test]
    fn a_rare_named_foulborn_something_keeps_its_name() {
        // A rare's name is randomly generated and can start with any word.
        let mut s = state();
        s.item.rarity = Some(ItemRarity::Rare);
        s.name = "Foulborn Grasp".into();

        parse_foulborn(&mut s).unwrap();

        assert_eq!(s.name, "Foulborn Grasp");
        assert!(!s.item.is_foulborn);
    }

    #[test]
    fn a_vestigial_unique_loses_the_prefix_from_its_base() {
        let mut s = state();
        s.item.rarity = Some(ItemRarity::Unique);
        s.base_type = Some("Vestigial Iron Ring".into());

        parse_vestigial(&mut s).unwrap();

        assert_eq!(s.base_type.as_deref(), Some("Iron Ring"));
        assert!(s.item.is_vestigial);
    }

    #[test]
    fn a_unique_with_no_base_line_is_left_alone() {
        let mut s = state();
        s.item.rarity = Some(ItemRarity::Unique);
        s.name = "Vestigial Something".into();

        parse_vestigial(&mut s).unwrap();

        assert!(!s.item.is_vestigial);
        assert_eq!(s.name, "Vestigial Something");
    }

    // -----------------------------------------------------------------
    // Map tier suffix
    // -----------------------------------------------------------------

    #[test]
    fn a_tier_suffix_is_read_and_removed() {
        // Leaving it on makes every tier of one map a different base and none
        // of them resolve.
        let mut s = state();
        s.name = "Toxic Map (Tier 14)".into();

        parse_map_tier(&mut s).unwrap();

        assert_eq!(s.name, "Toxic Map");
        assert_eq!(s.item.map.tier, Some(14));
    }

    #[test]
    fn a_tier_suffix_on_the_base_line_is_taken_from_there() {
        let mut s = state();
        s.name = "Torment Vault".into();
        s.base_type = Some("Toxic Map (Tier 3)".into());

        parse_map_tier(&mut s).unwrap();

        assert_eq!(s.base_type.as_deref(), Some("Toxic Map"));
        assert_eq!(s.name, "Torment Vault");
        assert_eq!(s.item.map.tier, Some(3));
    }

    #[test]
    fn a_name_with_no_tier_suffix_is_untouched() {
        let mut s = state();
        s.name = "Sapphire Ring".into();

        parse_map_tier(&mut s).unwrap();

        assert_eq!(s.name, "Sapphire Ring");
        assert_eq!(s.item.map.tier, None);
    }

    #[test]
    fn a_tier_suffix_with_no_number_is_not_a_tier() {
        // A unique could be called anything. Reading "(Tier of Doom)" as a
        // tier would strip a word off its name.
        let mut s = state();
        s.name = "Thing (Tier of Doom)".into();

        parse_map_tier(&mut s).unwrap();

        assert_eq!(s.name, "Thing (Tier of Doom)");
        assert_eq!(s.item.map.tier, None);
    }

    #[test]
    fn a_tier_in_the_middle_of_a_name_is_not_a_suffix() {
        let mut s = state();
        s.name = "Map (Tier 3) of Doom".into();

        parse_map_tier(&mut s).unwrap();

        assert_eq!(s.name, "Map (Tier 3) of Doom");
    }

    // -----------------------------------------------------------------
    // Memory strands
    // -----------------------------------------------------------------

    #[test]
    fn an_amulet_reports_its_memory_strands() {
        let mut s = state();
        s.item.category = Some(ItemCategory::Amulet);

        let got = parse_accessory(&sec(&["Memory Strands: 47"]), &mut s);

        assert_eq!(got, ParseOutcome::SectionParsed);
        assert_eq!(s.item.memory_strands, Some(47));
    }

    #[test]
    fn a_strand_line_is_consumed_so_it_is_not_read_as_a_modifier() {
        // An unconsumed line lands in unknown_modifiers, where nobody looks.
        let mut s = state();
        s.item.category = Some(ItemCategory::Ring);

        assert_eq!(
            parse_accessory(&sec(&["Memory Strands: 1"]), &mut s),
            ParseOutcome::SectionParsed
        );
    }

    #[test]
    fn a_non_accessory_skips_the_strand_stage_entirely() {
        let mut s = state();
        s.item.category = Some(ItemCategory::BodyArmour);

        assert_eq!(
            parse_accessory(&sec(&["Memory Strands: 47"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    #[test]
    fn an_accessory_section_without_strands_is_left_for_the_next_stage() {
        let mut s = state();
        s.item.category = Some(ItemCategory::Ring);

        assert_eq!(
            parse_accessory(&sec(&["+40 to maximum Life"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    // -----------------------------------------------------------------
    // Split
    // -----------------------------------------------------------------

    #[test]
    fn a_split_item_says_so() {
        let mut s = state();

        assert_eq!(
            parse_split(&sec(&["Split"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert!(s.item.is_split);
    }

    #[test]
    fn a_section_that_only_mentions_splitting_is_not_the_split_line() {
        let mut s = state();

        assert_eq!(
            parse_split(&sec(&["Split Arrow"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert!(!s.item.is_split);
    }

    #[test]
    fn a_two_line_section_is_not_the_split_line() {
        let mut s = state();

        assert_eq!(
            parse_split(&sec(&["Split", "Mirrored"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    // -----------------------------------------------------------------
    // Heist contract
    // -----------------------------------------------------------------

    fn contract() -> ParserState<'static> {
        let mut s = state();
        s.item.category = Some(ItemCategory::HeistContract);

        s
    }

    #[test]
    fn a_contract_reports_its_job_and_level() {
        let mut s = contract();

        let got = parse_heist_contract(
            &sec(&["Requires Lockpicking (Level 3)", "Area Level: 83"]),
            &mut s,
        );

        assert_eq!(got, ParseOutcome::SectionParsed);

        let c = s.item.heist_contract.unwrap();
        assert_eq!(c.required_job.as_deref(), Some("Lockpicking"));
        assert_eq!(c.job_level, Some(3));
        assert_eq!(s.item.area_level, Some(83));
    }

    #[test]
    fn an_unmet_job_requirement_reads_the_same() {
        // The game marks a job the character cannot do yet. The contract is
        // worth the same either way. The reference's pattern puts the marker
        // inside the level parentheses, so both placements are read.
        for line in [
            "Requires Deception (Level 4 (unmet))",
            "Requires Deception (Level 4) (unmet)",
        ] {
            let mut s = contract();

            parse_heist_contract(&sec(&[line, "Area Level: 83"]), &mut s);

            let c = s.item.heist_contract.unwrap();
            assert_eq!(c.required_job.as_deref(), Some("Deception"), "{line}");
            assert_eq!(c.job_level, Some(4), "{line}");
        }
    }

    #[test]
    fn a_hyphenated_job_is_read() {
        // Counter-Thaumaturgy is the one job whose name is not two plain
        // words, and its trade id drops the hyphen.
        let mut s = contract();

        parse_heist_contract(
            &sec(&["Requires Counter-Thaumaturgy (Level 2)", "Area Level: 80"]),
            &mut s,
        );

        assert_eq!(
            s.item.heist_contract.unwrap().required_job.as_deref(),
            Some("Counter-Thaumaturgy")
        );
    }

    #[test]
    fn a_priceless_target_is_recorded() {
        let mut s = contract();

        parse_heist_contract(
            &sec(&["Heist Target: Brooch (Priceless)", "Area Level: 83"]),
            &mut s,
        );

        assert!(s.item.heist_contract.unwrap().priceless);
    }

    #[test]
    fn an_ordinary_target_is_not_priceless() {
        let mut s = contract();

        parse_heist_contract(
            &sec(&["Heist Target: Brooch (Common)", "Area Level: 83"]),
            &mut s,
        );

        assert!(!s.item.heist_contract.unwrap().priceless);
    }

    #[test]
    fn a_section_with_no_area_level_is_not_the_contract_section() {
        // Without it this is some other section that happens to sit on a
        // contract, and consuming it would eat a real modifier block.
        let mut s = contract();

        let got = parse_heist_contract(&sec(&["Requires Agility (Level 3)"]), &mut s);

        assert_eq!(got, ParseOutcome::SectionSkipped);
        assert_eq!(s.item.heist_contract, None);
    }

    #[test]
    fn a_job_the_trade_site_does_not_index_is_not_read() {
        // Inventing a tenth job would send a trade id that does not exist and
        // fail the whole query.
        let mut s = contract();

        parse_heist_contract(
            &sec(&["Requires Juggling (Level 3)", "Area Level: 83"]),
            &mut s,
        );

        assert_eq!(s.item.heist_contract.unwrap().required_job, None);
    }

    #[test]
    fn a_non_contract_skips_the_stage() {
        let mut s = state();
        s.item.category = Some(ItemCategory::Map);

        assert_eq!(
            parse_heist_contract(&sec(&["Area Level: 83"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    #[test]
    fn every_job_the_reference_knows_is_read() {
        for (name, _) in cs::HEIST_JOBS {
            let mut s = contract();

            parse_heist_contract(
                &sec(&[&format!("Requires {name} (Level 5)"), "Area Level: 83"]),
                &mut s,
            );

            assert_eq!(
                s.item.heist_contract.unwrap().required_job.as_deref(),
                Some(name),
                "{name} was not read"
            );
        }
    }

    #[test]
    fn every_job_has_a_distinct_trade_id() {
        // A copied id would search for the wrong job.
        let mut ids: Vec<&str> = cs::HEIST_JOBS.iter().map(|(_, id)| *id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();

        assert_eq!(ids.len(), before);
    }
}
