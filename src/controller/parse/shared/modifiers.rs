use crate::adapter::data_adapter::StatLookup;
use crate::controller::mod_desc::{
    group_lines_by_mod, is_mod_info_line, parse_mod_info_line, parse_mod_type,
};
use crate::controller::parse::{ParseOutcome, ParserState};
use crate::controller::stat_match::placeholder::{closes_reminder, opens_reminder, StatString};
use crate::controller::stat_match::resolve::try_parse_translation;
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::UnknownModifier;
use crate::types::modifier::{ModifierInfo, ModifierType};
use crate::types::stat::ParsedStat;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedModifier {
    pub info: ModifierInfo,
    pub stats: Vec<ParsedStat>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModifierSection {
    pub modifiers: Vec<ParsedModifier>,
    pub unknown: Vec<UnknownModifier>,
}

impl ModifierSection {
    pub fn is_empty(&self) -> bool {
        self.modifiers.is_empty() && self.unknown.is_empty()
    }
}

pub fn read_modifier_section(section: &[String], data: &dyn StatLookup) -> ModifierSection {
    let groups = group_lines_by_mod(section);

    let mut out = ModifierSection::default();

    if groups.is_empty() {
        let (section_kind, lines) = parse_mod_type(section);

        let info = ModifierInfo {
            kind: Some(section_kind),
            ..ModifierInfo::default()
        };

        read_one(&info, &lines, section_kind, data, &mut out);

        return out;
    }

    for group in groups {
        let (group_kind, group_lines) = parse_mod_type(&group.stat_lines);

        let info = parse_mod_info_line(&group.mod_line, group_kind);
        let kind = info.kind.unwrap_or(group_kind);

        read_one(&info, &group_lines, kind, data, &mut out);
    }

    out
}

fn read_one(
    info: &ModifierInfo,
    lines: &[String],
    kind: ModifierType,
    data: &dyn StatLookup,
    out: &mut ModifierSection,
) {
    if kind == ModifierType::Veiled {
        out.modifiers.push(ParsedModifier {
            info: info.clone(),
            stats: Vec::new(),
        });

        return;
    }

    let (stats, unknown) = match_stat_lines(lines, data);

    if !stats.is_empty() {
        out.modifiers.push(ParsedModifier {
            info: info.clone(),
            stats,
        });
    }

    for text in unknown {
        out.unknown.push(UnknownModifier { text, kind });
    }
}

pub const MAX_STAT_LINES: usize = 4;

pub fn match_stat_lines(lines: &[String], data: &dyn StatLookup) -> (Vec<ParsedStat>, Vec<String>) {
    let mut stats = Vec::new();
    let mut unknown = Vec::new();

    let mut in_reminder = false;
    let mut start = 0;

    while start < lines.len() {
        let line = &lines[start];

        if opens_reminder(line) {
            in_reminder = true;
        }

        if in_reminder {
            if closes_reminder(line) {
                in_reminder = false;
            }

            start += 1;

            continue;
        }

        let mut matched_to = None;

        let last = (start + MAX_STAT_LINES).min(lines.len());

        for end in start..last {
            let joined = lines[start..=end].join("\n");
            let stat_string = split_unscalable_marker(&joined);

            if let Some(parsed) = try_parse_translation(&stat_string, data) {
                stats.push(parsed);
                matched_to = Some(end);

                break;
            }
        }

        match matched_to {
            Some(end) => start = end + 1,
            None => {
                if !line.is_empty() {
                    unknown.push(line.clone());
                }

                start += 1;
            }
        }
    }

    (stats, unknown)
}

fn split_unscalable_marker(text: &str) -> StatString {
    match text.strip_suffix(cs::UNSCALABLE_VALUE) {
        Some(rest) => StatString {
            string: rest.to_string(),
            unscalable: true,
        },
        None => StatString {
            string: text.to_string(),
            unscalable: false,
        },
    }
}

pub fn looks_like_modifiers(section: &[String]) -> bool {
    section
        .iter()
        .any(|l| l.starts_with(cs::GRANTS_SKILL) || is_mod_info_line(l) || has_type_suffix(l))
}

fn has_type_suffix(line: &str) -> bool {
    [
        ModifierType::Scourge,
        ModifierType::Enchant,
        ModifierType::Implicit,
        ModifierType::Fractured,
        ModifierType::Crafted,
        ModifierType::Augment,
        ModifierType::AddedAugment,
        ModifierType::Desecrated,
    ]
    .iter()
    .filter_map(|k| k.line_suffix())
    .any(|s| line.ends_with(s))
}

fn whole_section_kind(section: &[String]) -> Option<ModifierType> {
    for line in section {
        if line.starts_with(cs::GRANTS_SKILL) {
            return Some(ModifierType::Skill);
        }

        for kind in [
            ModifierType::Enchant,
            ModifierType::Scourge,
            ModifierType::AddedAugment,
            ModifierType::Augment,
        ] {
            if kind.line_suffix().is_some_and(|s| line.ends_with(s)) {
                return Some(kind);
            }
        }
    }

    None
}

pub fn read_modifier_section_poe2(
    section: &[String],
    category: Option<ItemCategory>,
    data: &dyn StatLookup,
) -> ModifierSection {
    if let Some(kind) = whole_section_kind(section) {
        let (_, lines) = parse_mod_type(section);

        let info = ModifierInfo {
            kind: Some(kind),
            ..ModifierInfo::default()
        };

        let mut out = ModifierSection::default();
        read_one(&info, &lines, kind, data, &mut out);

        return out;
    }

    if section.iter().any(|line| is_mod_info_line(line)) {
        return read_modifier_section(section, data);
    }

    let mut out = ModifierSection::default();

    for line in section {
        let own = std::slice::from_ref(line);
        let (mut kind, lines) = parse_mod_type(own);

        if kind == ModifierType::Explicit && category == Some(ItemCategory::Relic) {
            kind = ModifierType::Sanctum;
        }

        let info = ModifierInfo {
            kind: Some(kind),
            ..ModifierInfo::default()
        };

        read_one(&info, &lines, kind, data, &mut out);
    }

    out
}

pub fn parse_modifiers_poe2(section: &[String], state: &mut ParserState<'_>) -> ParseOutcome {
    if state.item.rarity.is_none() {
        return ParseOutcome::ParserSkipped;
    }

    let read = read_modifier_section_poe2(section, state.item.category, state.data);

    if read.is_empty() {
        return ParseOutcome::SectionSkipped;
    }

    if read.modifiers.is_empty() && !looks_like_modifiers(section) {
        return ParseOutcome::SectionSkipped;
    }

    for m in read.modifiers {
        if m.info.kind == Some(ModifierType::Fractured) {
            state.item.is_fractured = true;
        }

        if m.info.kind == Some(ModifierType::Veiled) {
            state.item.is_veiled = true;
        }

        state.item.modifiers.push(m);
    }

    state.item.unknown_modifiers.extend(read.unknown);

    ParseOutcome::SectionParsed
}

pub fn parse_modifiers(section: &[String], state: &mut ParserState<'_>) -> ParseOutcome {
    let read = read_modifier_section(section, state.data);

    if read.is_empty() {
        return ParseOutcome::SectionSkipped;
    }

    if read.modifiers.is_empty() && !looks_like_modifiers(section) {
        return ParseOutcome::SectionSkipped;
    }

    for m in read.modifiers {
        if m.info.kind == Some(ModifierType::Fractured) {
            state.item.is_fractured = true;
        }

        if m.info.kind == Some(ModifierType::Veiled) {
            state.item.is_veiled = true;
        }

        state.item.modifiers.push(m);
    }

    state.item.unknown_modifiers.extend(read.unknown);

    ParseOutcome::SectionParsed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::stat::{Stat, StatHit, StatMatcher};

    struct FakeStats {
        stats: Vec<Stat>,
    }

    impl StatLookup for FakeStats {
        fn stat_by_matcher(&self, template: &str) -> Option<StatHit<'_>> {
            for stat in &self.stats {
                for matcher in &stat.matchers {
                    if matcher.string == template {
                        return Some(StatHit { stat, matcher });
                    }
                }
            }

            None
        }
    }

    fn table(refs: &[&str]) -> FakeStats {
        FakeStats {
            stats: refs
                .iter()
                .map(|r| Stat {
                    reference: (*r).to_string(),
                    matchers: vec![StatMatcher {
                        string: (*r).to_string(),
                        ..StatMatcher::default()
                    }],
                    ..Stat::default()
                })
                .collect(),
        }
    }

    fn sec(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_plain_explicit_section_is_read() {
        let data = table(&["# to maximum Life", "# to Dexterity"]);

        let got = read_modifier_section(&sec(&["+25 to maximum Life", "+30 to Dexterity"]), &data);

        assert_eq!(got.modifiers.len(), 1);
        assert_eq!(got.modifiers[0].stats.len(), 2);
        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Explicit));
        assert!(got.unknown.is_empty());
    }

    #[test]
    fn an_implicit_suffix_types_the_whole_modifier() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section(&sec(&["+25 to maximum Life (implicit)"]), &data);

        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Implicit));
        assert_eq!(got.modifiers[0].stats.len(), 1);
    }

    #[test]
    fn metadata_lines_split_a_section_into_several_modifiers() {
        let data = table(&["# to maximum Life", "# to Dexterity"]);

        let got = read_modifier_section(
            &sec(&[
                "{Prefix Modifier \"Rotund\" (Tier: 3)}",
                "+25 to maximum Life",
                "{Suffix Modifier \"of the Lynx\" (Tier: 1)}",
                "+30 to Dexterity",
            ]),
            &data,
        );

        assert_eq!(got.modifiers.len(), 2);
        assert_eq!(got.modifiers[0].info.name.as_deref(), Some("Rotund"));
        assert_eq!(got.modifiers[1].info.name.as_deref(), Some("of the Lynx"));
    }

    #[test]
    fn a_metadata_line_can_retype_its_own_modifier() {
        let data = table(&["# to maximum Life", "# to Dexterity"]);

        let got = read_modifier_section(
            &sec(&[
                "{Prefix Modifier \"Rotund\"}",
                "+25 to maximum Life",
                "{Master Crafted Suffix Modifier}",
                "+30 to Dexterity",
            ]),
            &data,
        );

        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Explicit));
        assert_eq!(got.modifiers[1].info.kind, Some(ModifierType::Crafted));
    }

    #[test]
    fn an_unmatched_line_is_kept_as_an_unknown_modifier() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section(
            &sec(&["+25 to maximum Life", "Grants Sudden Enlightenment"]),
            &data,
        );

        assert_eq!(got.modifiers[0].stats.len(), 1);
        assert_eq!(got.unknown.len(), 1);
        assert_eq!(got.unknown[0].text, "Grants Sudden Enlightenment");
        assert_eq!(got.unknown[0].kind, ModifierType::Explicit);
    }

    #[test]
    fn an_unknown_line_carries_the_section_type() {
        let data = table(&[]);

        let got = read_modifier_section(&sec(&["Something New (implicit)"]), &data);

        assert_eq!(got.unknown[0].kind, ModifierType::Implicit);
        assert_eq!(got.unknown[0].text, "Something New");
    }

    #[test]
    fn the_join_window_is_capped_so_a_huge_section_cannot_hang_the_parser() {
        let data = table(&[]);
        let lines: Vec<String> = (0..20_000).map(|i| format!("line {i}")).collect();

        let started = std::time::Instant::now();
        let got = read_modifier_section(&lines, &data);

        assert_eq!(got.unknown.len(), 20_000);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "the scan took {:?}, which means the cap is not working",
            started.elapsed()
        );
    }

    #[test]
    fn a_stat_longer_than_the_cap_is_not_matched() {
        let long = (0..MAX_STAT_LINES + 1)
            .map(|i| format!("part {i}"))
            .collect::<Vec<_>>()
            .join("\n");

        let data = table(&[&long]);
        let lines: Vec<String> = (0..MAX_STAT_LINES + 1)
            .map(|i| format!("part {i}"))
            .collect();

        let got = read_modifier_section(&lines, &data);

        assert!(got.modifiers.is_empty());
    }

    #[test]
    fn a_stat_exactly_at_the_cap_still_matches() {
        let exact = (0..MAX_STAT_LINES)
            .map(|i| format!("part {i}"))
            .collect::<Vec<_>>()
            .join("\n");

        let data = table(&[&exact]);
        let lines: Vec<String> = (0..MAX_STAT_LINES).map(|i| format!("part {i}")).collect();

        let got = read_modifier_section(&lines, &data);

        assert_eq!(got.modifiers.len(), 1);
        assert!(got.unknown.is_empty());
    }

    #[test]
    fn a_multi_line_stat_matches_as_one() {
        let data = table(&["Passives in Radius of Wicked Ward\ncan be Allocated"]);

        let got = read_modifier_section(
            &sec(&["Passives in Radius of Wicked Ward", "can be Allocated"]),
            &data,
        );

        assert_eq!(got.modifiers[0].stats.len(), 1);
        assert!(got.unknown.is_empty());
    }

    #[test]
    fn the_shortest_matching_run_wins() {
        let data = table(&["# to maximum Life", "# to Dexterity"]);

        let got = read_modifier_section(&sec(&["+25 to maximum Life", "+30 to Dexterity"]), &data);

        assert_eq!(got.modifiers[0].stats.len(), 2);
    }

    #[test]
    fn a_reminder_string_is_skipped_entirely() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section(
            &sec(&["+25 to maximum Life", "(Fire Resistance is capped at 75%)"]),
            &data,
        );

        assert_eq!(got.modifiers[0].stats.len(), 1);
        assert!(got.unknown.is_empty());
    }

    #[test]
    fn a_multi_line_reminder_string_is_skipped_entirely() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section(
            &sec(&[
                "+25 to maximum Life",
                "(This is a long reminder",
                "that runs across two lines)",
            ]),
            &data,
        );

        assert!(got.unknown.is_empty());
    }

    #[test]
    fn a_veiled_modifier_records_itself_with_no_stats() {
        let data = table(&[]);

        let got = read_modifier_section(&sec(&["Desecrated Prefix", "Veiled Stat"]), &data);

        assert_eq!(got.modifiers.len(), 1);
        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Veiled));
        assert!(got.modifiers[0].stats.is_empty());
        assert!(got.unknown.is_empty());
    }

    #[test]
    fn an_empty_section_produces_nothing() {
        let data = table(&[]);

        assert!(read_modifier_section(&[], &data).is_empty());
    }

    #[test]
    fn a_section_of_blank_lines_produces_nothing() {
        let data = table(&[]);

        assert!(read_modifier_section(&sec(&["", ""]), &data).is_empty());
    }

    #[test]
    fn a_modifier_with_no_matching_stats_is_not_recorded_as_a_modifier() {
        let data = table(&[]);

        let got = read_modifier_section(&sec(&["{Prefix Modifier}", "Unknown Thing"]), &data);

        assert!(got.modifiers.is_empty());
        assert_eq!(got.unknown.len(), 1);
    }

    #[test]
    fn an_unscalable_marker_is_split_off_before_matching() {
        let data = table(&["# to maximum Life"]);
        let line = format!("+25 to maximum Life{}", cs::UNSCALABLE_VALUE);

        let got = read_modifier_section(&sec(&[&line]), &data);

        assert!(got.modifiers[0].stats[0].roll.unwrap().unscalable);
    }

    #[test]
    fn a_rune_suffix_is_recognised() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section(&sec(&["+25 to maximum Life (rune)"]), &data);

        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Augment));
    }

    #[test]
    fn a_section_with_a_metadata_line_looks_like_modifiers() {
        assert!(looks_like_modifiers(&sec(&["{Prefix Modifier}"])));
    }

    #[test]
    fn a_section_with_a_type_suffix_looks_like_modifiers() {
        assert!(looks_like_modifiers(&sec(&["+25 to Life (implicit)"])));
    }

    #[test]
    fn a_plain_section_does_not_look_like_modifiers_on_its_own() {
        assert!(!looks_like_modifiers(&sec(&["Item Level: 84"])));
        assert!(!looks_like_modifiers(&sec(&["+25 to maximum Life"])));
    }

    #[test]
    fn every_poe2_explicit_line_is_its_own_modifier() {
        let data = table(&["# to maximum Life", "# to Dexterity"]);

        let got = read_modifier_section_poe2(
            &sec(&["+25 to maximum Life", "+30 to Dexterity"]),
            None,
            &data,
        );

        assert_eq!(got.modifiers.len(), 2);
        assert_eq!(got.modifiers[0].stats.len(), 1);
        assert_eq!(got.modifiers[1].stats.len(), 1);
    }

    #[test]
    fn a_tagged_poe2_section_stays_one_modifier() {
        let data = table(&["# to maximum Life", "# to Dexterity"]);

        let got = read_modifier_section_poe2(
            &sec(&["+25 to maximum Life (rune)", "+30 to Dexterity"]),
            None,
            &data,
        );

        assert_eq!(got.modifiers.len(), 1);
        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Augment));
    }

    #[test]
    fn an_added_rune_is_not_read_as_a_plain_rune() {
        let data = table(&["# to maximum Life"]);

        let got =
            read_modifier_section_poe2(&sec(&["+25 to maximum Life (added rune)"]), None, &data);

        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::AddedAugment));
    }

    #[test]
    fn an_enchant_section_stays_whole() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section_poe2(&sec(&["+25 to maximum Life (enchant)"]), None, &data);

        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Enchant));
    }

    #[test]
    fn a_relic_explicit_is_a_sanctum_modifier() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section_poe2(
            &sec(&["+25 to maximum Life"]),
            Some(ItemCategory::Relic),
            &data,
        );

        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Sanctum));
    }

    #[test]
    fn a_relic_implicit_is_left_as_an_implicit() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section_poe2(
            &sec(&["+25 to maximum Life (implicit)"]),
            Some(ItemCategory::Relic),
            &data,
        );

        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Implicit));
    }

    #[test]
    fn a_ring_explicit_is_not_turned_into_a_sanctum_modifier() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section_poe2(
            &sec(&["+25 to maximum Life"]),
            Some(ItemCategory::Ring),
            &data,
        );

        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Explicit));
    }

    #[test]
    fn a_poe2_section_with_metadata_keeps_the_shared_grouping() {
        let data = table(&["# to maximum Life", "# to Dexterity"]);

        let section = sec(&[
            "{ Prefix Modifier \"Rotund\" (Tier: 3) }",
            "+25 to maximum Life",
            "+30 to Dexterity",
        ]);

        let got = read_modifier_section_poe2(&section, None, &data);

        assert_eq!(got.modifiers.len(), 1);
        assert_eq!(got.modifiers[0].stats.len(), 2);
    }

    #[test]
    fn an_unmatched_poe2_line_is_reported_rather_than_dropped() {
        let data = table(&["# to maximum Life"]);

        let got =
            read_modifier_section_poe2(&sec(&["+25 to maximum Life", "Mystery line"]), None, &data);

        assert_eq!(got.modifiers.len(), 1);
        assert_eq!(got.unknown.len(), 1);
        assert_eq!(got.unknown[0].text, "Mystery line");
    }

    #[test]
    fn an_empty_poe2_section_produces_nothing() {
        assert!(read_modifier_section_poe2(&[], None, &table(&[])).is_empty());
    }

    #[test]
    fn a_granted_skill_section_stays_whole() {
        let data = table(&["# to maximum Life"]);

        let got = read_modifier_section_poe2(
            &sec(&["Grants Skill: Level 20 Grace", "+25 to maximum Life"]),
            None,
            &data,
        );

        assert_eq!(got.modifiers.len(), 1);
        assert_eq!(got.modifiers[0].info.kind, Some(ModifierType::Skill));
    }

    #[test]
    fn a_granted_skill_the_data_does_not_know_is_still_reported() {
        let got =
            read_modifier_section_poe2(&sec(&["Grants Skill: Raise Shield"]), None, &table(&[]));

        assert!(got.modifiers.is_empty());
        assert_eq!(got.unknown.len(), 1);
        assert_eq!(got.unknown[0].kind, ModifierType::Skill);
    }

    #[test]
    fn a_granted_skill_line_marks_a_section_as_modifiers() {
        assert!(looks_like_modifiers(&sec(&["Grants Skill: Raise Shield"])));
    }

    #[test]
    fn an_ordinary_line_still_does_not_look_like_modifiers() {
        assert!(!looks_like_modifiers(&sec(&["Requires: Level 54"])));
    }
}
