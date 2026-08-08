use crate::controller::parse::{ParseError, ParseOutcome, ParserState};
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::{HeistContract, ItemRarity};
use crate::util::number::leading_int;

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

fn split_tier_suffix(text: &str) -> Option<(String, u32)> {
    let head = text.strip_suffix(')')?;
    let (before, digits) = head.rsplit_once(" (Tier ")?;

    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    Some((before.to_string(), digits.parse().ok()?))
}

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

pub fn parse_tincture(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.category != Some(ItemCategory::Tincture) {
        return ParseOutcome::ParserSkipped;
    }

    let before = state.item.quality;

    crate::controller::parse::shared::levels::parse_quality_nested(section, state);

    if state.item.quality != before {
        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

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

fn line_is_a_known_stat(line: &str, state: &ParserState) -> bool {
    crate::controller::stat_match::placeholder::candidates(line)
        .iter()
        .any(|c| state.data.stat_by_matcher(&c.template).is_some())
}

pub fn parse_heist_contract(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.category != Some(ItemCategory::HeistContract) {
        return ParseOutcome::ParserSkipped;
    }

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

fn read_job(line: &str) -> Option<(String, u32)> {
    let line = line.replace(" (unmet)", "");

    let rest = line.strip_prefix(cs::HEIST_CONTRACT_REQUIRES)?;
    let rest = rest.trim_end().strip_suffix(')')?;

    let (job, level) = rest.trim_end().rsplit_once(" (Level ")?;

    let job = cs::HEIST_JOBS
        .iter()
        .find(|(name, _)| *name == job)
        .map(|(name, _)| (*name).to_string())?;

    Some((job, level.parse().ok()?))
}

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
        let mut s = state();
        s.item.rarity = Some(ItemRarity::Unique);
        s.item.is_unidentified = true;
        s.name = "Foulborn Rusted Sword".into();

        parse_foulborn(&mut s).unwrap();

        assert_eq!(s.name, "Foulborn Rusted Sword");
    }

    #[test]
    fn a_rare_named_foulborn_something_keeps_its_name() {
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

    #[test]
    fn a_tier_suffix_is_read_and_removed() {
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
        let mut s = contract();

        let got = parse_heist_contract(&sec(&["Requires Agility (Level 3)"]), &mut s);

        assert_eq!(got, ParseOutcome::SectionSkipped);
        assert_eq!(s.item.heist_contract, None);
    }

    #[test]
    fn a_job_the_trade_site_does_not_index_is_not_read() {
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
        let mut ids: Vec<&str> = cs::HEIST_JOBS.iter().map(|(_, id)| *id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();

        assert_eq!(ids.len(), before);
    }
}
