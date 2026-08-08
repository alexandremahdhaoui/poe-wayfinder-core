use crate::controller::parse::{ParseOutcome, ParserState};
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::Requirements;
use crate::util::number::leading_int;

pub fn parse_quality_nested(section: &[String], state: &mut ParserState) {
    for line in section {
        if let Some(rest) = line.strip_prefix(cs::QUALITY) {
            if let Some(v) = leading_int(rest) {
                state.item.quality = Some(v as i32);
            }

            return;
        }
    }
}

pub fn parse_item_level(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let prefix = if state.item.info.reference_name == "Filled Coffin" {
        cs::CORPSE_LEVEL
    } else {
        cs::ITEM_LEVEL
    };

    for line in section {
        if let Some(rest) = line.strip_prefix(prefix) {
            state.item.item_level = leading_int(rest).map(|v| v as u32);

            return ParseOutcome::SectionParsed;
        }
    }

    ParseOutcome::SectionSkipped
}

pub fn parse_talisman_tier(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    if let Some(rest) = first.strip_prefix(cs::TALISMAN_TIER) {
        state.item.talisman_tier = leading_int(rest).map(|v| v as u32);

        return ParseOutcome::SectionParsed;
    }

    ParseOutcome::SectionSkipped
}

pub fn parse_gem(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let category = state.item.category;

    let applies = matches!(
        category,
        Some(ItemCategory::Gem) | Some(ItemCategory::MetaGem) | Some(ItemCategory::UncutGem)
    );

    if !applies {
        return ParseOutcome::ParserSkipped;
    }

    let row = match category {
        Some(ItemCategory::Gem) | Some(ItemCategory::MetaGem) => 1,
        _ => 0,
    };

    let Some(line) = section.get(row) else {
        return ParseOutcome::SectionSkipped;
    };

    let Some(rest) = line.strip_prefix(cs::GEM_LEVEL) else {
        return ParseOutcome::SectionSkipped;
    };

    state.item.gem_level = leading_int(rest).map(|v| v as u32);

    parse_quality_nested(section, state);

    ParseOutcome::SectionParsed
}

pub fn parse_requirements(section: &[String], state: &mut ParserState) -> ParseOutcome {
    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    let Some(rest) = first.strip_prefix(cs::REQUIRES) else {
        return ParseOutcome::SectionSkipped;
    };

    if state.item.category.is_some_and(ItemCategory::is_gem) {
        return ParseOutcome::SectionSkipped;
    }

    state.item.requires = Some(parse_requires_line(rest));

    ParseOutcome::SectionParsed
}

fn parse_requires_line(rest: &str) -> Requirements {
    let mut out = Requirements::default();

    for part in rest.split(',') {
        let part = part.trim();

        if let Some(level) = part.strip_prefix("Level") {
            out.level = leading_int(level).unwrap_or(0) as u32;

            continue;
        }

        let Some(value) = leading_int(part) else {
            continue;
        };
        let value = value as u32;

        if part.contains("Strength") || part.contains("Str") {
            out.str = value;
        } else if part.contains("Dexterity") || part.contains("Dex") {
            out.dex = value;
        } else if part.contains("Intelligence") || part.contains("Int") {
            out.int = value;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sec(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn item_level_is_read() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_item_level(&sec(&["Item Level: 84"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert_eq!(s.item.item_level, Some(84));
    }

    #[test]
    fn item_level_is_found_anywhere_in_the_section() {
        let mut s = ParserState::default();

        parse_item_level(&sec(&["Quality: +20%", "Item Level: 84"]), &mut s);

        assert_eq!(s.item.item_level, Some(84));
    }

    #[test]
    fn a_filled_coffin_reads_its_corpse_level_instead() {
        let mut s = ParserState::default();
        s.item.info.reference_name = "Filled Coffin".into();

        parse_item_level(&sec(&["Corpse Level: 68"]), &mut s);

        assert_eq!(s.item.item_level, Some(68));
    }

    #[test]
    fn a_filled_coffin_ignores_a_plain_item_level_line() {
        let mut s = ParserState::default();
        s.item.info.reference_name = "Filled Coffin".into();

        assert_eq!(
            parse_item_level(&sec(&["Item Level: 68"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert_eq!(s.item.item_level, None);
    }

    #[test]
    fn a_section_with_no_item_level_is_skipped() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_item_level(&sec(&["Corrupted"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn talisman_tier_is_read_from_the_first_line_only() {
        let mut s = ParserState::default();

        parse_talisman_tier(&sec(&["Talisman Tier: 3"]), &mut s);
        assert_eq!(s.item.talisman_tier, Some(3));

        let mut s = ParserState::default();
        assert_eq!(
            parse_talisman_tier(&sec(&["Corrupted", "Talisman Tier: 3"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn quality_is_read_out_of_its_decoration() {
        let mut s = ParserState::default();

        parse_quality_nested(&sec(&["Quality: +20% (augmented)"]), &mut s);

        assert_eq!(s.item.quality, Some(20));
    }

    #[test]
    fn a_negative_quality_keeps_its_sign() {
        let mut s = ParserState::default();

        parse_quality_nested(&sec(&["Quality: -10%"]), &mut s);

        assert_eq!(s.item.quality, Some(-10));
    }

    #[test]
    fn only_the_first_quality_line_counts() {
        let mut s = ParserState::default();

        parse_quality_nested(&sec(&["Quality: +20%", "Quality: +5%"]), &mut s);

        assert_eq!(s.item.quality, Some(20));
    }

    #[test]
    fn a_section_with_no_quality_leaves_the_field_alone() {
        let mut s = ParserState::default();

        parse_quality_nested(&sec(&["Corrupted"]), &mut s);

        assert_eq!(s.item.quality, None);
    }

    #[test]
    fn a_cut_gem_reads_its_level_from_the_second_line() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Gem);

        assert_eq!(
            parse_gem(&sec(&["Spell, AoE, Fire", "Level: 20 (Max)"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert_eq!(s.item.gem_level, Some(20));
    }

    #[test]
    fn an_uncut_gem_reads_its_level_from_the_first_line() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::UncutGem);

        parse_gem(&sec(&["Level: 15"]), &mut s);

        assert_eq!(s.item.gem_level, Some(15));
    }

    #[test]
    fn a_gem_picks_up_its_quality_at_the_same_time() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Gem);

        parse_gem(
            &sec(&["Spell, AoE", "Level: 20 (Max)", "Quality: +23% (augmented)"]),
            &mut s,
        );

        assert_eq!(s.item.gem_level, Some(20));
        assert_eq!(s.item.quality, Some(23));
    }

    #[test]
    fn a_non_gem_skips_the_gem_stage_entirely() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Bow);

        assert_eq!(
            parse_gem(&sec(&["Level: 20"]), &mut s),
            ParseOutcome::ParserSkipped
        );
        assert_eq!(s.item.gem_level, None);
    }

    #[test]
    fn a_gem_section_without_a_level_line_is_skipped() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Gem);

        assert_eq!(
            parse_gem(&sec(&["Spell, AoE", "Corrupted"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn a_short_gem_section_is_skipped_rather_than_panicking() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Gem);

        assert_eq!(
            parse_gem(&sec(&["Spell"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn requirements_read_every_attribute() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_requirements(&sec(&["Requires: Level 68, 155 Str, 62 Dex"]), &mut s),
            ParseOutcome::SectionParsed
        );
        assert_eq!(
            s.item.requires,
            Some(Requirements {
                level: 68,
                str: 155,
                dex: 62,
                int: 0
            })
        );
    }

    #[test]
    fn requirements_accept_the_long_attribute_names() {
        let mut s = ParserState::default();

        parse_requirements(
            &sec(&["Requires: Level 62, 100 Intelligence, 50 Dexterity"]),
            &mut s,
        );

        assert_eq!(
            s.item.requires,
            Some(Requirements {
                level: 62,
                str: 0,
                dex: 50,
                int: 100
            })
        );
    }

    #[test]
    fn requirements_accept_a_level_only_line() {
        let mut s = ParserState::default();

        parse_requirements(&sec(&["Requires: Level 12"]), &mut s);

        assert_eq!(
            s.item.requires,
            Some(Requirements {
                level: 12,
                str: 0,
                dex: 0,
                int: 0
            })
        );
    }

    #[test]
    fn requirements_do_not_depend_on_attribute_order() {
        let mut s = ParserState::default();

        parse_requirements(&sec(&["Requires: 50 Int, Level 30, 20 Str"]), &mut s);

        assert_eq!(
            s.item.requires,
            Some(Requirements {
                level: 30,
                str: 20,
                dex: 0,
                int: 50
            })
        );
    }

    #[test]
    fn a_gem_skips_requirements() {
        let mut s = ParserState::default();
        s.item.category = Some(ItemCategory::Gem);

        assert_eq!(
            parse_requirements(&sec(&["Requires: Level 68"]), &mut s),
            ParseOutcome::SectionSkipped
        );
        assert_eq!(s.item.requires, None);
    }

    #[test]
    fn a_section_that_is_not_a_requires_line_is_skipped() {
        let mut s = ParserState::default();

        assert_eq!(
            parse_requirements(&sec(&["Item Level: 84"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }
}
