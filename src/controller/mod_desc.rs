use crate::controller::filter::roll_math::{round_roll, Precision};
use crate::types::client_strings as cs;
use crate::types::modifier::{Generation, ModifierInfo, ModifierType};
use crate::types::stat::ParsedStat;
use crate::util::number::leading_float;

const FIELD_SEPARATOR: char = '\u{2014}';

pub fn is_mod_info_line(line: &str) -> bool {
    line.starts_with('{') && line.ends_with('}')
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupedMod {
    pub mod_line: String,
    pub stat_lines: Vec<String>,
}

pub fn group_lines_by_mod(lines: &[String]) -> Vec<GroupedMod> {
    if lines.first().is_none_or(|l| !is_mod_info_line(l)) {
        return Vec::new();
    }

    let mut out: Vec<GroupedMod> = Vec::new();

    for line in lines {
        if is_mod_info_line(line) {
            out.push(GroupedMod {
                mod_line: line.clone(),
                stat_lines: Vec::new(),
            });
        } else if let Some(last) = out.last_mut() {
            last.stat_lines.push(line.clone());
        }
    }

    out
}

pub fn remove_lines_ending(lines: &[String], ending: &str) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.strip_suffix(ending).unwrap_or(l).to_string())
        .collect()
}

pub fn parse_mod_type(lines: &[String]) -> (ModifierType, Vec<String>) {
    if lines
        .first()
        .is_some_and(|l| l == cs::VEILED_PREFIX || l == cs::VEILED_SUFFIX)
    {
        return (ModifierType::Veiled, lines.to_vec());
    }

    let ordered = [
        ModifierType::Scourge,
        ModifierType::Enchant,
        ModifierType::Implicit,
        ModifierType::Fractured,
        ModifierType::Crafted,
        ModifierType::Augment,
        ModifierType::AddedAugment,
        ModifierType::Desecrated,
    ];

    for kind in ordered {
        let Some(suffix) = kind.line_suffix() else {
            continue;
        };

        if lines.iter().any(|l| l.ends_with(suffix)) {
            return (kind, remove_lines_ending(lines, suffix));
        }
    }

    (ModifierType::Explicit, lines.to_vec())
}

pub fn parse_mod_info_line(line: &str, kind: ModifierType) -> ModifierInfo {
    let inner = line
        .strip_prefix('{')
        .and_then(|l| l.strip_suffix('}'))
        .unwrap_or(line);

    let mut fields = inner.split(FIELD_SEPARATOR).map(str::trim);

    let head = fields.next().unwrap_or("");
    let second = fields.next();
    let third = fields.next();

    let mut info = ModifierInfo {
        kind: Some(kind),
        ..ModifierInfo::default()
    };

    if let Some(rank) = eldritch_rank(head) {
        info.generation = Some(Generation::Eldritch);
        info.rank = Some(rank);
    } else {
        read_head(head, &mut info);
    }

    let incr_text = match (second, third) {
        (_, Some(third)) => Some(third),
        (Some(second), None) if parse_increased(second).is_some() => Some(second),
        _ => None,
    };

    let tags_text = match (second, incr_text) {
        (Some(second), Some(incr)) if second == incr => None,
        (Some(second), _) => Some(second),
        _ => None,
    };

    info.tags = tags_text
        .map(|t| t.split(", ").map(str::to_string).collect())
        .unwrap_or_default();
    info.roll_incr = incr_text.and_then(parse_increased);

    info
}

fn strip_crafted(type_word: &str) -> Option<&str> {
    type_word
        .strip_prefix(cs::MASTER_CRAFTED_MODIFIER)
        .or_else(|| type_word.strip_prefix(cs::CRAFTED_MODIFIER))
        .map(str::trim)
}

fn read_head(head: &str, info: &mut ModifierInfo) {
    let (type_word, rest) = split_type_word(head);

    let mut type_word = type_word;

    if let Some(stripped) = type_word.strip_prefix(cs::FRACTURED_MODIFIER) {
        type_word = stripped.trim();
        info.kind = Some(ModifierType::Fractured);
    } else if let Some(stripped) = type_word.strip_prefix(cs::DESECRATED_MODIFIER) {
        type_word = stripped.trim();

        if info.kind != Some(ModifierType::Fractured) {
            info.kind = Some(ModifierType::Desecrated);
        }
    } else if let Some(stripped) = strip_crafted(type_word) {
        type_word = stripped;

        if info.kind != Some(ModifierType::Fractured) {
            info.kind = Some(ModifierType::Crafted);
        }
    }

    match type_word {
        cs::PREFIX_MODIFIER => info.generation = Some(Generation::Prefix),
        cs::SUFFIX_MODIFIER => info.generation = Some(Generation::Suffix),
        cs::CORRUPTED_IMPLICIT => {
            info.generation = Some(Generation::Corrupted);
            info.kind = Some(ModifierType::Enchant);
        }
        cs::IMPLICIT_MODIFIER => info.kind = Some(ModifierType::Implicit),
        cs::VAAL_UNIQUE_MODIFIER => info.generation = Some(Generation::Mutated),
        _ => {}
    }

    info.name = read_quoted_name(head);
    info.tier = read_parenthesised(rest, "Tier");
    info.rank = read_parenthesised(rest, "Rank");
}

fn split_type_word(head: &str) -> (&str, &str) {
    let end = head.find(['"', '(']).unwrap_or(head.len());

    (head[..end].trim(), &head[end..])
}

fn read_quoted_name(head: &str) -> Option<String> {
    let start = head.find('"')? + 1;
    let end = start + head[start..].find('"')?;
    let name = &head[start..end];

    (!name.is_empty()).then(|| name.to_string())
}

fn read_parenthesised(text: &str, key: &str) -> Option<u32> {
    let needle = format!("({key}:");
    let start = text.find(&needle)? + needle.len();
    let end = start + text[start..].find(')')?;

    text[start..end].trim().parse().ok()
}

fn parse_increased(text: &str) -> Option<f64> {
    let value = text.trim().strip_suffix("% Increased")?;

    leading_float(value)
}

fn eldritch_rank(head: &str) -> Option<u32> {
    let rest = head
        .strip_prefix("Eater of Worlds Implicit Modifier")
        .or_else(|| head.strip_prefix("Searing Exarch Implicit Modifier"))?
        .trim();

    let word = rest.strip_prefix('(')?.strip_suffix(')')?.trim();

    cs::ELDRITCH_MOD_RANKS
        .iter()
        .position(|&r| r == word)
        .map(|i| i as u32 + 1)
}

pub fn incr_roll(value: f64, percent: f64, decimals: u32) -> f64 {
    let raised = value + (value * percent) / 100.0;

    round_roll(raised, Precision::Fixed(decimals))
}

pub fn apply_incr(info: &ModifierInfo, stat: &ParsedStat) -> Option<ParsedStat> {
    let increase = info.roll_incr?;
    let roll = stat.roll?;

    if increase == 0.0 {
        return None;
    }

    if roll.unscalable {
        return None;
    }

    let decimals = if roll.decimals { 2 } else { 0 };

    let mut scaled = roll;
    scaled.value = incr_roll(roll.value, increase, decimals);
    scaled.min = incr_roll(roll.min, increase, decimals);
    scaled.max = incr_roll(roll.max, increase, decimals);

    Some(ParsedStat {
        reference: stat.reference.clone(),
        matched: stat.matched.clone(),
        roll: Some(scaled),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::stat::StatRoll;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_metadata_line_is_recognised_by_its_braces() {
        assert!(is_mod_info_line("{Prefix Modifier}"));
        assert!(!is_mod_info_line("+25 to maximum Life"));
        assert!(!is_mod_info_line("{unterminated"));
        assert!(!is_mod_info_line("unopened}"));
    }

    #[test]
    fn each_metadata_line_collects_the_stats_beneath_it() {
        let got = group_lines_by_mod(&lines(&[
            "{Prefix Modifier \"Rotund\" (Tier: 3)}",
            "+25 to maximum Life",
            "{Suffix Modifier \"of the Lynx\" (Tier: 1)}",
            "+30 to Dexterity",
            "+12% to Cold Resistance",
        ]));

        assert_eq!(got.len(), 2);
        assert_eq!(got[0].stat_lines, vec!["+25 to maximum Life"]);
        assert_eq!(got[1].stat_lines.len(), 2);
    }

    #[test]
    fn a_section_that_does_not_start_with_metadata_is_not_grouped() {
        let got = group_lines_by_mod(&lines(&["+25 to maximum Life", "{Prefix Modifier}"]));

        assert!(got.is_empty());
    }

    #[test]
    fn an_empty_section_is_not_grouped() {
        assert!(group_lines_by_mod(&[]).is_empty());
    }

    #[test]
    fn a_metadata_line_with_no_stats_still_yields_a_group() {
        let got = group_lines_by_mod(&lines(&["{Prefix Modifier}"]));

        assert_eq!(got.len(), 1);
        assert!(got[0].stat_lines.is_empty());
    }

    #[test]
    fn a_line_suffix_names_the_type_and_is_stripped() {
        let (kind, out) = parse_mod_type(&lines(&["+25 to maximum Life (implicit)"]));

        assert_eq!(kind, ModifierType::Implicit);
        assert_eq!(out, vec!["+25 to maximum Life"]);
    }

    #[test]
    fn both_games_spellings_of_the_crafted_marker_are_read() {
        for line in [
            "{ Crafted Prefix Modifier \"Gentian\" (Tier: 6) — Mana }",
            "{ Master Crafted Prefix Modifier \"Gentian\" (Tier: 6) — Mana }",
        ] {
            let got = parse_mod_info_line(line, ModifierType::Explicit);

            assert_eq!(got.kind, Some(ModifierType::Crafted), "{line}");
            assert_eq!(got.generation, Some(Generation::Prefix), "{line}");
            assert_eq!(got.name.as_deref(), Some("Gentian"), "{line}");
        }
    }

    #[test]
    fn the_longer_crafted_spelling_is_tried_first() {
        let got = parse_mod_info_line(
            "{ Master Crafted Suffix Modifier \"of Mana\" (Tier: 1) }",
            ModifierType::Explicit,
        );

        assert_eq!(got.generation, Some(Generation::Suffix));
    }

    #[test]
    fn every_suffix_is_wired_up() {
        for (suffix, want) in [
            (" (scourge)", ModifierType::Scourge),
            (" (enchant)", ModifierType::Enchant),
            (" (implicit)", ModifierType::Implicit),
            (" (fractured)", ModifierType::Fractured),
            (" (crafted)", ModifierType::Crafted),
            (" (rune)", ModifierType::Augment),
            (" (added rune)", ModifierType::AddedAugment),
            (" (desecrated)", ModifierType::Desecrated),
        ] {
            let (kind, out) = parse_mod_type(&lines(&[&format!("+25 to Life{suffix}")]));

            assert_eq!(kind, want, "{suffix}");
            assert_eq!(out, vec!["+25 to Life"], "{suffix}");
        }
    }

    #[test]
    fn a_section_with_no_suffix_is_explicit_and_unchanged() {
        let (kind, out) = parse_mod_type(&lines(&["+25 to maximum Life"]));

        assert_eq!(kind, ModifierType::Explicit);
        assert_eq!(out, vec!["+25 to maximum Life"]);
    }

    #[test]
    fn one_suffixed_line_types_the_whole_section() {
        let (kind, out) = parse_mod_type(&lines(&[
            "+25 to maximum Life (implicit)",
            "+30 to Dexterity",
        ]));

        assert_eq!(kind, ModifierType::Implicit);
        assert_eq!(out[1], "+30 to Dexterity");
    }

    #[test]
    fn a_veiled_marker_beats_every_suffix() {
        let (kind, out) = parse_mod_type(&lines(&["Desecrated Prefix", "+25 to Life (implicit)"]));

        assert_eq!(kind, ModifierType::Veiled);
        assert_eq!(out[1], "+25 to Life (implicit)");
    }

    #[test]
    fn the_first_matching_suffix_in_reference_order_wins() {
        let (kind, _) = parse_mod_type(&lines(&["A (implicit)", "B (enchant)"]));

        assert_eq!(kind, ModifierType::Enchant);
    }

    #[test]
    fn removing_a_suffix_leaves_lines_that_lack_it_alone() {
        let got = remove_lines_ending(&lines(&["A (implicit)", "B"]), " (implicit)");

        assert_eq!(got, vec!["A", "B"]);
    }

    #[test]
    fn a_prefix_with_a_name_and_a_tier_is_read() {
        let got = parse_mod_info_line(
            "{Prefix Modifier \"Rotund\" (Tier: 3)}",
            ModifierType::Explicit,
        );

        assert_eq!(got.generation, Some(Generation::Prefix));
        assert_eq!(got.name.as_deref(), Some("Rotund"));
        assert_eq!(got.tier, Some(3));
        assert_eq!(got.kind, Some(ModifierType::Explicit));
    }

    #[test]
    fn a_suffix_is_read() {
        let got = parse_mod_info_line(
            "{Suffix Modifier \"of the Lynx\" (Tier: 1)}",
            ModifierType::Explicit,
        );

        assert_eq!(got.generation, Some(Generation::Suffix));
        assert_eq!(got.name.as_deref(), Some("of the Lynx"));
        assert_eq!(got.tier, Some(1));
    }

    #[test]
    fn a_rank_is_read_alongside_a_tier() {
        let got = parse_mod_info_line(
            "{Prefix Modifier \"Rotund\" (Tier: 3) (Rank: 2)}",
            ModifierType::Explicit,
        );

        assert_eq!(got.tier, Some(3));
        assert_eq!(got.rank, Some(2));
    }

    #[test]
    fn an_empty_quoted_name_reads_as_absent() {
        let got = parse_mod_info_line("{Implicit Modifier \"\"}", ModifierType::Explicit);

        assert_eq!(got.name, None);
    }

    #[test]
    fn a_metadata_line_with_no_name_reads_as_absent() {
        let got = parse_mod_info_line("{Implicit Modifier}", ModifierType::Explicit);

        assert_eq!(got.name, None);
        assert_eq!(got.kind, Some(ModifierType::Implicit));
    }

    #[test]
    fn a_crafted_marker_overrides_the_section_type() {
        let got = parse_mod_info_line(
            "{Master Crafted Prefix Modifier \"Upgraded\" (Tier: 1)}",
            ModifierType::Explicit,
        );

        assert_eq!(got.kind, Some(ModifierType::Crafted));
        assert_eq!(got.generation, Some(Generation::Prefix));
    }

    #[test]
    fn a_fractured_marker_overrides_the_section_type() {
        let got = parse_mod_info_line(
            "{Fractured Prefix Modifier \"Rotund\" (Tier: 3)}",
            ModifierType::Explicit,
        );

        assert_eq!(got.kind, Some(ModifierType::Fractured));
        assert_eq!(got.generation, Some(Generation::Prefix));
    }

    #[test]
    fn a_desecrated_marker_overrides_the_section_type() {
        let got = parse_mod_info_line(
            "{Desecrated Suffix Modifier \"of Rot\"}",
            ModifierType::Explicit,
        );

        assert_eq!(got.kind, Some(ModifierType::Desecrated));
        assert_eq!(got.generation, Some(Generation::Suffix));
    }

    #[test]
    fn a_corrupted_implicit_becomes_an_enchant() {
        let got = parse_mod_info_line("{Corruption Implicit Modifier}", ModifierType::Explicit);

        assert_eq!(got.kind, Some(ModifierType::Enchant));
        assert_eq!(got.generation, Some(Generation::Corrupted));
    }

    #[test]
    fn a_vaal_unique_modifier_is_mutated() {
        let got = parse_mod_info_line("{Vaal Unique Modifier}", ModifierType::Explicit);

        assert_eq!(got.generation, Some(Generation::Mutated));
    }

    #[test]
    fn tags_are_split_on_comma_space() {
        let got = parse_mod_info_line(
            "{Prefix Modifier \"Rotund\" (Tier: 3) \u{2014} Life, Defences}",
            ModifierType::Explicit,
        );

        assert_eq!(got.tags, vec!["Life", "Defences"]);
        assert_eq!(got.roll_incr, None);
    }

    #[test]
    fn a_roll_increase_in_the_third_field_is_read() {
        let got = parse_mod_info_line(
            "{Prefix Modifier \"Rotund\" \u{2014} Life \u{2014} 20% Increased}",
            ModifierType::Explicit,
        );

        assert_eq!(got.tags, vec!["Life"]);
        assert_eq!(got.roll_incr, Some(20.0));
    }

    #[test]
    fn a_roll_increase_in_the_second_field_is_not_read_as_a_tag() {
        let got = parse_mod_info_line(
            "{Prefix Modifier \"Rotund\" \u{2014} 20% Increased}",
            ModifierType::Explicit,
        );

        assert!(got.tags.is_empty());
        assert_eq!(got.roll_incr, Some(20.0));
    }

    #[test]
    fn a_fractional_roll_increase_is_read() {
        let got = parse_mod_info_line(
            "{Prefix Modifier \u{2014} 12.5% Increased}",
            ModifierType::Explicit,
        );

        assert_eq!(got.roll_incr, Some(12.5));
    }

    #[test]
    fn an_eldritch_implicit_reports_its_rank() {
        for (word, rank) in [
            ("Lesser", 1),
            ("Greater", 2),
            ("Grand", 3),
            ("Exceptional", 4),
            ("Exquisite", 5),
            ("Perfect", 6),
        ] {
            let line = format!("{{Eater of Worlds Implicit Modifier ({word})}}");
            let got = parse_mod_info_line(&line, ModifierType::Implicit);

            assert_eq!(got.generation, Some(Generation::Eldritch), "{word}");
            assert_eq!(got.rank, Some(rank), "{word}");
        }
    }

    #[test]
    fn a_searing_exarch_implicit_is_read_the_same_way() {
        let got = parse_mod_info_line(
            "{Searing Exarch Implicit Modifier (Perfect)}",
            ModifierType::Implicit,
        );

        assert_eq!(got.generation, Some(Generation::Eldritch));
        assert_eq!(got.rank, Some(6));
    }

    #[test]
    fn an_unknown_eldritch_rank_word_falls_through_to_the_normal_reader() {
        let got = parse_mod_info_line(
            "{Eater of Worlds Implicit Modifier (Sublime)}",
            ModifierType::Implicit,
        );

        assert_eq!(got.generation, None);
        assert_eq!(got.rank, None);
    }

    #[test]
    fn a_line_without_braces_is_still_read() {
        let got = parse_mod_info_line("Prefix Modifier \"Rotund\"", ModifierType::Explicit);

        assert_eq!(got.name.as_deref(), Some("Rotund"));
    }

    #[test]
    fn an_empty_metadata_line_yields_only_the_incoming_type() {
        let got = parse_mod_info_line("{}", ModifierType::Implicit);

        assert_eq!(got.kind, Some(ModifierType::Implicit));
        assert_eq!(got.generation, None);
        assert_eq!(got.name, None);
        assert!(got.tags.is_empty());
    }

    fn stat_with(roll: Option<StatRoll>) -> ParsedStat {
        ParsedStat {
            reference: "# to maximum Life".into(),
            matched: "# to maximum Life".into(),
            roll,
        }
    }

    fn roll_of(value: f64) -> StatRoll {
        StatRoll {
            value,
            min: value,
            max: value,
            ..StatRoll::default()
        }
    }

    fn info_with_incr(percent: Option<f64>) -> ModifierInfo {
        ModifierInfo {
            roll_incr: percent,
            ..ModifierInfo::default()
        }
    }

    #[test]
    fn a_roll_increase_rescales_the_whole_range() {
        let got = apply_incr(
            &info_with_incr(Some(20.0)),
            &stat_with(Some(roll_of(100.0))),
        )
        .expect("an increase applies");

        let roll = got.roll.expect("the roll survives");

        assert_eq!(roll.value, 120.0);
        assert_eq!(roll.min, 120.0);
        assert_eq!(roll.max, 120.0);
    }

    #[test]
    fn the_bounds_are_scaled_independently() {
        let mut roll = roll_of(0.0);
        roll.value = 50.0;
        roll.min = 40.0;
        roll.max = 60.0;

        let got = apply_incr(&info_with_incr(Some(50.0)), &stat_with(Some(roll)))
            .expect("an increase applies")
            .roll
            .expect("the roll survives");

        assert_eq!((got.min, got.value, got.max), (60.0, 75.0, 90.0));
    }

    #[test]
    fn a_modifier_with_no_increase_changes_nothing() {
        assert_eq!(
            apply_incr(&info_with_incr(None), &stat_with(Some(roll_of(100.0)))),
            None
        );
    }

    #[test]
    fn a_stat_with_no_roll_changes_nothing() {
        assert_eq!(
            apply_incr(&info_with_incr(Some(20.0)), &stat_with(None)),
            None
        );
    }

    #[test]
    fn an_unscalable_roll_is_left_alone() {
        let mut roll = roll_of(100.0);
        roll.unscalable = true;

        assert_eq!(
            apply_incr(&info_with_incr(Some(20.0)), &stat_with(Some(roll))),
            None
        );
    }

    #[test]
    fn a_decimal_roll_keeps_its_decimals() {
        let mut roll = roll_of(10.0);
        roll.decimals = true;

        let got = apply_incr(&info_with_incr(Some(15.0)), &stat_with(Some(roll)))
            .expect("an increase applies")
            .roll
            .expect("the roll survives");

        assert_eq!(got.value, 11.5);
    }

    #[test]
    fn a_whole_roll_stays_whole() {
        let got = apply_incr(&info_with_incr(Some(15.0)), &stat_with(Some(roll_of(10.0))))
            .expect("an increase applies")
            .roll
            .expect("the roll survives");

        assert_eq!(got.value, 11.0);
    }

    #[test]
    fn scaling_keeps_the_stat_identity() {
        let stat = stat_with(Some(roll_of(100.0)));

        let got = apply_incr(&info_with_incr(Some(20.0)), &stat).expect("an increase applies");

        assert_eq!(got.reference, stat.reference);
        assert_eq!(got.matched, stat.matched);
    }

    #[test]
    fn scaling_keeps_the_roll_flags() {
        let mut roll = roll_of(100.0);
        roll.legacy = true;

        let got = apply_incr(&info_with_incr(Some(20.0)), &stat_with(Some(roll)))
            .expect("an increase applies")
            .roll
            .expect("the roll survives");

        assert!(got.legacy);
    }

    #[test]
    fn a_zero_increase_leaves_the_value_where_it_was() {
        let got = apply_incr(&info_with_incr(Some(0.0)), &stat_with(Some(roll_of(100.0))));

        assert_eq!(got, None);
    }

    #[test]
    fn a_negative_roll_scales_away_from_zero() {
        let got = apply_incr(
            &info_with_incr(Some(20.0)),
            &stat_with(Some(roll_of(-10.0))),
        )
        .expect("an increase applies")
        .roll
        .expect("the roll survives");

        assert_eq!(got.value, -12.0);
    }

    #[test]
    fn a_roll_increase_scales_a_whole_number() {
        assert_eq!(incr_roll(100.0, 20.0, 0), 120.0);
    }

    #[test]
    fn scaling_truncates_and_does_not_round() {
        assert_eq!(incr_roll(10.0, 15.0, 0), 11.0);
    }

    #[test]
    fn scaling_keeps_the_requested_decimals() {
        assert_eq!(incr_roll(10.0, 15.0, 2), 11.5);
    }

    #[test]
    fn a_zero_increase_changes_nothing() {
        assert_eq!(incr_roll(37.0, 0.0, 0), 37.0);
    }

    #[test]
    fn scaling_a_negative_value_moves_it_further_from_zero() {
        assert_eq!(incr_roll(-10.0, 20.0, 0), -12.0);
    }
}
