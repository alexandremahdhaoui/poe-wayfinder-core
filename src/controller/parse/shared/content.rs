use crate::controller::parse::{ParseError, ParseOutcome, ParserState};
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::{Heist, HeistTarget, ItemRarity, Trials, UltimatumHint};
use crate::util::number::leading_int;

pub fn parse_map(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    if let Some(rest) = first.strip_prefix(cs::MAP_TIER) {
        state.item.map.tier = leading_int(rest).map(|v| v as u32);

        return ParseOutcome::SectionParsed;
    }

    for line in section {
        if let Some(reward) = line.strip_prefix(cs::MAP_COMPLETION_REWARD) {
            state.item.map_completion_reward = Some(reward.to_string());

            return ParseOutcome::SectionParsed;
        }
    }

    ParseOutcome::SectionSkipped
}

pub fn parse_waystone(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    if !first.starts_with(cs::WAYSTONE_REVIVES) {
        return ParseOutcome::SectionSkipped;
    }

    let Some(base_tier) = state.item.info.map_tier else {
        return ParseOutcome::SectionSkipped;
    };

    state.item.map.tier = Some(base_tier);

    for line in section {
        if let Some(rest) = line.strip_prefix(cs::WAYSTONE_REVIVES) {
            state.item.map.revives = leading_int(rest).map(|v| v as u32);
        } else if let Some(rest) = line.strip_prefix(cs::WAYSTONE_PACK_SIZE) {
            state.item.map.pack_size = leading_int(rest).map(|v| v as f64);
        } else if let Some(rest) = line.strip_prefix(cs::WAYSTONE_MAGIC_MONSTERS) {
            state.item.map.magic_monsters = leading_int(rest).map(|v| v as f64);
        } else if let Some(rest) = line.strip_prefix(cs::WAYSTONE_RARE_MONSTERS) {
            state.item.map.rare_monsters = leading_int(rest).map(|v| v as f64);
        } else if let Some(rest) = line.strip_prefix(cs::WAYSTONE_DROP_CHANCE) {
            state.item.map.drop_chance = leading_int(rest).map(|v| v as f64);
        } else if let Some(rest) = line.strip_prefix(cs::WAYSTONE_RARITY) {
            state.item.map.item_rarity = leading_int(rest).map(|v| v as f64);
        } else if let Some(rest) = line.strip_prefix(cs::WAYSTONE_GOLD) {
            state.item.map.gold = leading_int(rest).map(|v| v as f64);
        } else if let Some(rest) = line.strip_prefix(cs::WAYSTONE_MONSTER_RARITY) {
            state.item.map.monster_rarity = leading_int(rest).map(|v| v as f64);
        } else if let Some(rest) = line.strip_prefix(cs::WAYSTONE_EFFECTIVENESS) {
            state.item.map.effectiveness = leading_int(rest).map(|v| v as f64);
        }
    }

    ParseOutcome::SectionParsed
}

pub fn parse_category_by_help_text(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    let category = match first.as_str() {
        cs::BEAST_HELP => ItemCategory::CapturedBeast,
        cs::METAMORPH_HELP => ItemCategory::MetamorphSample,
        cs::VOIDSTONE_HELP => ItemCategory::Voidstone,
        _ => return ParseOutcome::SectionSkipped,
    };

    state.item.category = Some(category);

    ParseOutcome::SectionParsed
}

fn read_area_level(section: &[String], state: &mut ParserState) {
    for line in section {
        if let Some(rest) = line.strip_prefix(cs::AREA_LEVEL) {
            state.item.area_level = leading_int(rest).map(|v| v as u32);

            return;
        }
    }
}

pub fn parse_area_level(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let applies = matches!(
        state.item.info.reference_name.as_str(),
        "Chronicle of Atzoatl" | "Expedition Logbook" | "Mirrored Tablet" | "Forbidden Tome"
    );

    if !applies {
        return ParseOutcome::ParserSkipped;
    }

    read_area_level(section, state);

    if state.item.area_level.is_some() {
        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

pub fn parse_heist_blueprint(section: &[String], state: &mut ParserState) -> ParseOutcome {
    if state.item.category != Some(ItemCategory::HeistBlueprint) {
        return ParseOutcome::ParserSkipped;
    }

    read_area_level(section, state);

    if state.item.area_level.is_none() {
        return ParseOutcome::SectionSkipped;
    }

    let mut heist = Heist::default();

    for line in section {
        if let Some(rest) = line.strip_prefix(cs::HEIST_TARGET) {
            heist.target = match rest {
                cs::HEIST_BLUEPRINT_ENCHANTS => Some(HeistTarget::Enchants),
                cs::HEIST_BLUEPRINT_GEMS => Some(HeistTarget::Gems),
                cs::HEIST_BLUEPRINT_REPLICAS => Some(HeistTarget::Replicas),
                cs::HEIST_BLUEPRINT_TRINKETS => Some(HeistTarget::Trinkets),
                _ => None,
            };
        } else if let Some(rest) = line.strip_prefix(cs::HEIST_WINGS_REVEALED) {
            heist.wings_revealed = leading_int(rest).map(|v| v as u32);
        }
    }

    state.item.heist = Some(heist);

    ParseOutcome::SectionParsed
}

pub fn parse_trials(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let name = state.item.info.reference_name.as_str();
    let is_ultimatum = name == "Inscribed Ultimatum";

    if name != "Djinn Barya" && !is_ultimatum {
        return ParseOutcome::ParserSkipped;
    }

    read_area_level(section, state);

    if state.item.area_level.is_none() {
        return ParseOutcome::SectionSkipped;
    }

    let mut trials = Trials::default();

    for line in section {
        if let Some(rest) = line.strip_prefix(cs::TRIAL_COUNT) {
            trials.number_of_trials = leading_int(rest).map(|v| v as u32);
        } else if is_ultimatum {
            if line.starts_with(cs::ULTIMATUM_VICTORIOUS) {
                trials.ultimatum_hint = Some(UltimatumHint::Victorious);
            } else if line.starts_with(cs::ULTIMATUM_COWARDLY) {
                trials.ultimatum_hint = Some(UltimatumHint::Cowardly);
            } else if line.starts_with(cs::ULTIMATUM_DEADLY) {
                trials.ultimatum_hint = Some(UltimatumHint::Deadly);
            }
        }
    }

    state.item.trials = Some(trials);

    ParseOutcome::SectionParsed
}

fn name_prefix_is_strippable(state: &ParserState) -> bool {
    match state.item.rarity {
        Some(ItemRarity::Normal) => true,
        Some(ItemRarity::Magic) | Some(ItemRarity::Rare) | Some(ItemRarity::Unique) => {
            state.item.is_unidentified
        }
        None => false,
    }
}

pub fn parse_superior(state: &mut ParserState) -> Result<(), ParseError> {
    strip_name_prefix(state, cs::ITEM_SUPERIOR);

    Ok(())
}

pub fn parse_exceptional(state: &mut ParserState) -> Result<(), ParseError> {
    strip_name_prefix(state, cs::ITEM_EXCEPTIONAL);

    Ok(())
}

pub fn parse_runeforged(_state: &mut ParserState) -> Result<(), ParseError> {
    Ok(())
}

fn strip_name_prefix(state: &mut ParserState, prefix: &str) {
    if !name_prefix_is_strippable(state) {
        return;
    }

    if let Some(stripped) = state.name.strip_prefix(prefix) {
        state.name = stripped.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sec(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn normal(name: &str) -> ParserState<'static> {
        let mut s = ParserState {
            name: name.into(),
            ..ParserState::default()
        };
        s.item.rarity = Some(ItemRarity::Normal);

        s
    }

    fn named(name: &str) -> ParserState<'static> {
        let mut item = crate::types::item::ParsedItem::default();
        item.info.reference_name = name.into();

        ParserState {
            item,
            ..ParserState::default()
        }
    }

    #[test]
    fn a_map_tier_is_read() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_map(&sec(&["Map Tier: 16"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert_eq!(s.item.map.tier, Some(16));
    }

    #[test]
    fn a_map_tier_is_read_from_the_first_line_only() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_map(&sec(&["Corrupted", "Map Tier: 16"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn a_waystone_takes_its_tier_from_the_base() {
        let mut s = ParserState::default();
        s.item.info.map_tier = Some(15);

        let out = parse_waystone(
            &sec(&[
                "Revives Available: 6",
                "Pack Size: +20%",
                "Item Rarity: +40%",
                "Waystone Drop Chance: +100%",
                "Gold Found: +75%",
                "Monster Rarity: +32%",
                "Monster Effectiveness: +45%",
            ]),
            &mut s,
        );

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(s.item.map.tier, Some(15));
        assert_eq!(s.item.map.revives, Some(6));
        assert_eq!(s.item.map.pack_size, Some(20.0));
        assert_eq!(s.item.map.monster_rarity, Some(32.0));
        assert_eq!(s.item.map.effectiveness, Some(45.0));
        assert_eq!(s.item.map.item_rarity, Some(40.0));
        assert_eq!(s.item.map.drop_chance, Some(100.0));
        assert_eq!(s.item.map.gold, Some(75.0));
    }

    #[test]
    fn a_waystone_with_no_base_tier_is_skipped() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_waystone(&sec(&["Revives Available: 6"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert_eq!(s.item.map.revives, None);
    }

    #[test]
    fn a_section_not_starting_with_revives_is_skipped() {
        let mut s = ParserState::default();
        s.item.info.map_tier = Some(15);

        assert_eq!(
            parse_waystone(&sec(&["Monster Pack Size: +20%"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn magic_and_rare_monster_counts_are_read() {
        let mut s = ParserState::default();
        s.item.info.map_tier = Some(15);

        parse_waystone(
            &sec(&[
                "Revives Available: 6",
                "Magic Monsters: +30%",
                "Rare Monsters: +12%",
            ]),
            &mut s,
        );

        assert_eq!(s.item.map.magic_monsters, Some(30.0));
        assert_eq!(s.item.map.rare_monsters, Some(12.0));
    }

    #[test]
    fn each_help_line_sets_its_category() {
        for (line, want) in [
            (cs::BEAST_HELP, ItemCategory::CapturedBeast),
            (cs::METAMORPH_HELP, ItemCategory::MetamorphSample),
            (cs::VOIDSTONE_HELP, ItemCategory::Voidstone),
        ] {
            let mut s = ParserState::default();

            assert_eq!(
                parse_category_by_help_text(&sec(&[line]), &mut s),
                ParseOutcome::SectionParsed,
                "{line}"
            );
            assert_eq!(s.item.category, Some(want));
        }
    }

    #[test]
    fn a_help_line_below_the_first_row_is_ignored() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_category_by_help_text(&sec(&["Corrupted", cs::BEAST_HELP]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert_eq!(s.item.category, None);
    }

    #[test]
    fn area_level_is_read_on_the_four_bases_that_carry_one() {
        for name in [
            "Chronicle of Atzoatl",
            "Expedition Logbook",
            "Mirrored Tablet",
            "Forbidden Tome",
        ] {
            let mut s = named(name);

            assert_eq!(
                parse_area_level(&sec(&["Area Level: 83"]), &mut s),
                ParseOutcome::SectionParsed,
                "{name}"
            );
            assert_eq!(s.item.area_level, Some(83));
        }
    }

    #[test]
    fn any_other_base_skips_the_area_level_stage() {
        let mut s = named("Spine Bow");

        assert_eq!(
            parse_area_level(&sec(&["Area Level: 83"]), &mut s),
            ParseOutcome::ParserSkipped
        );
        assert_eq!(s.item.area_level, None);
    }

    #[test]
    fn an_area_level_base_without_the_line_is_skipped() {
        let mut s = named("Forbidden Tome");

        assert_eq!(
            parse_area_level(&sec(&["Corrupted"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn a_heist_blueprint_reads_its_target_and_wings() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::HeistBlueprint);

        let out = parse_heist_blueprint(
            &sec(&[
                "Area Level: 83",
                "Wings Revealed: 3",
                "Heist Target: Unusual Gems",
            ]),
            &mut s,
        );

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(s.item.area_level, Some(83));
        assert_eq!(
            s.item.heist,
            Some(Heist {
                wings_revealed: Some(3),
                target: Some(HeistTarget::Gems)
            })
        );
    }

    #[test]
    fn every_heist_target_is_wired_up() {
        for (text, want) in [
            (cs::HEIST_BLUEPRINT_ENCHANTS, HeistTarget::Enchants),
            (cs::HEIST_BLUEPRINT_GEMS, HeistTarget::Gems),
            (cs::HEIST_BLUEPRINT_REPLICAS, HeistTarget::Replicas),
            (cs::HEIST_BLUEPRINT_TRINKETS, HeistTarget::Trinkets),
        ] {
            let mut s = ParserState::default();
            s.item.category = Some(ItemCategory::HeistBlueprint);

            parse_heist_blueprint(
                &sec(&["Area Level: 80", &format!("Heist Target: {text}")]),
                &mut s,
            );

            assert_eq!(s.item.heist.unwrap().target, Some(want), "{text}");
        }
    }

    #[test]
    fn a_heist_section_with_no_area_level_is_skipped() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::HeistBlueprint);

        assert_eq!(
            parse_heist_blueprint(&sec(&["Wings Revealed: 3"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert_eq!(s.item.heist, None);
    }

    #[test]
    fn a_non_blueprint_skips_the_heist_stage() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_heist_blueprint(&sec(&["Area Level: 83"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    #[test]
    fn an_inscribed_ultimatum_reads_its_hint() {
        let mut s = named("Inscribed Ultimatum");

        parse_trials(
            &sec(&["Area Level: 80", "Number of Trials: 10", "Victorious"]),
            &mut s,
        );

        assert_eq!(
            s.item.trials,
            Some(Trials {
                number_of_trials: Some(10),
                ultimatum_hint: Some(UltimatumHint::Victorious)
            })
        );
    }

    #[test]
    fn every_ultimatum_hint_is_wired_up() {
        for (text, want) in [
            (cs::ULTIMATUM_VICTORIOUS, UltimatumHint::Victorious),
            (cs::ULTIMATUM_COWARDLY, UltimatumHint::Cowardly),
            (cs::ULTIMATUM_DEADLY, UltimatumHint::Deadly),
        ] {
            let mut s = named("Inscribed Ultimatum");

            parse_trials(&sec(&["Area Level: 80", text]), &mut s);

            assert_eq!(s.item.trials.unwrap().ultimatum_hint, Some(want), "{text}");
        }
    }

    #[test]
    fn a_djinn_barya_reads_no_hint() {
        let mut s = named("Djinn Barya");

        parse_trials(
            &sec(&["Area Level: 80", "Number of Trials: 4", "Victorious"]),
            &mut s,
        );

        let trials = s.item.trials.unwrap();
        assert_eq!(trials.number_of_trials, Some(4));
        assert_eq!(trials.ultimatum_hint, None);
    }

    #[test]
    fn any_other_base_skips_the_trials_stage() {
        let mut s = named("Spine Bow");

        assert_eq!(
            parse_trials(&sec(&["Area Level: 80"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    #[test]
    fn a_trials_section_with_no_area_level_is_skipped() {
        let mut s = named("Djinn Barya");

        assert_eq!(
            parse_trials(&sec(&["Number of Trials: 4"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert_eq!(s.item.trials, None);
    }

    #[test]
    fn superior_is_stripped_off_a_normal_item() {
        let mut s = normal("Superior Iron Hat");

        parse_superior(&mut s).unwrap();

        assert_eq!(s.name, "Iron Hat");
    }

    #[test]
    fn superior_is_stripped_off_an_unidentified_rare() {
        let mut s = ParserState {
            name: "Superior Iron Hat".into(),
            ..ParserState::default()
        };
        s.item.rarity = Some(ItemRarity::Rare);
        s.item.is_unidentified = true;

        parse_superior(&mut s).unwrap();

        assert_eq!(s.name, "Iron Hat");
    }

    #[test]
    fn superior_is_kept_on_an_identified_rare() {
        let mut s = ParserState {
            name: "Superior Doom Crown".into(),
            ..ParserState::default()
        };
        s.item.rarity = Some(ItemRarity::Rare);

        parse_superior(&mut s).unwrap();

        assert_eq!(s.name, "Superior Doom Crown");
    }

    #[test]
    fn an_item_with_no_rarity_keeps_its_prefix() {
        let mut s = ParserState {
            name: "Superior Iron Hat".into(),
            ..ParserState::default()
        };

        parse_superior(&mut s).unwrap();

        assert_eq!(s.name, "Superior Iron Hat");
    }

    #[test]
    fn exceptional_is_stripped_the_same_way() {
        let mut s = normal("Exceptional Iron Hat");

        parse_exceptional(&mut s).unwrap();

        assert_eq!(s.name, "Iron Hat");
    }

    #[test]
    fn a_name_without_the_prefix_is_left_alone() {
        let mut s = normal("Iron Hat");

        parse_superior(&mut s).unwrap();

        assert_eq!(s.name, "Iron Hat");
    }

    #[test]
    fn runeforged_strips_nothing() {
        let mut s = normal("Runeforged Iron Hat");

        parse_runeforged(&mut s).unwrap();

        assert_eq!(s.name, "Runeforged Iron Hat");
    }
}
