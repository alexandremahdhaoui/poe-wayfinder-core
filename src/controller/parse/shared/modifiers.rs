//! The modifier stage.
//!
//! Ported from `parseModifiers`, `parseStatsFromMod` and `linesToStatStrings`
//! in `renderer/src/parser/Parser.ts`.
//!
//! This is the stage that appears five times in the reference pipeline. Each
//! occurrence eats a different modifier section, because a section is consumed
//! at most once and an item can carry enchant, rune, implicit, granted skill
//! and explicit blocks all at once.
//!
//! # Multi line stats
//!
//! One stat can span several lines. The matcher therefore tries the current
//! line, then that line joined with the next, and so on, taking the longest
//! run the data recognises. Trying single lines only would leave every
//! multi line stat unknown.

use crate::adapter::data_adapter::StatLookup;
use crate::controller::mod_desc::{
    group_lines_by_mod, is_mod_info_line, parse_mod_info_line, parse_mod_type,
};
use crate::controller::parse::{ParseOutcome, ParserState};
use crate::controller::stat_match::placeholder::{closes_reminder, opens_reminder, StatString};
use crate::controller::stat_match::resolve::try_parse_translation;
use crate::types::client_strings as cs;
use crate::types::item::UnknownModifier;
use crate::types::modifier::{ModifierInfo, ModifierType};
use crate::types::stat::ParsedStat;

/// One modifier and every stat it granted.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedModifier {
    pub info: ModifierInfo,
    pub stats: Vec<ParsedStat>,
}

/// What reading a modifier section produced.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModifierSection {
    pub modifiers: Vec<ParsedModifier>,
    /// Lines that matched no stat.
    ///
    /// Kept rather than dropped. An unknown modifier means our data is older
    /// than the game, and the UI has to say so instead of quietly pricing a
    /// worse item.
    pub unknown: Vec<UnknownModifier>,
}

impl ModifierSection {
    /// Whether the section produced anything at all.
    pub fn is_empty(&self) -> bool {
        self.modifiers.is_empty() && self.unknown.is_empty()
    }
}

/// Read one modifier section.
///
/// Returns nothing when no line in the section matched a stat and no metadata
/// line was present, which means this is not a modifier section and another
/// stage should get it.
pub fn read_modifier_section(section: &[String], data: &dyn StatLookup) -> ModifierSection {
    let (kind, lines) = parse_mod_type(section);

    let groups = group_lines_by_mod(&lines);

    let mut out = ModifierSection::default();

    if groups.is_empty() {
        // No metadata lines. The whole section is one modifier whose type the
        // line suffixes already gave us.
        let info = ModifierInfo {
            kind: Some(kind),
            ..ModifierInfo::default()
        };

        read_one(&info, &lines, kind, data, &mut out);

        return out;
    }

    for group in groups {
        let info = parse_mod_info_line(&group.mod_line, kind);
        let group_kind = info.kind.unwrap_or(kind);

        read_one(&info, &group.stat_lines, group_kind, data, &mut out);
    }

    out
}

/// Match one modifier's stat lines.
fn read_one(
    info: &ModifierInfo,
    lines: &[String],
    kind: ModifierType,
    data: &dyn StatLookup,
    out: &mut ModifierSection,
) {
    // A veiled modifier has no readable stats. The game hides them.
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

/// How many lines one stat may span.
///
/// Measured against the real data: the longest multi line matcher GGG ships is
/// three lines. Four gives a margin.
///
/// This is a cap and not an optimisation. Without it the scan is quadratic in
/// the number of lines and each step joins a growing string, so a clipboard of
/// twenty thousand lines hangs the overlay. Clipboard text is fully attacker
/// controlled, and a freeze in a tool running alongside a game is a hang the
/// user blames on the game.
pub const MAX_STAT_LINES: usize = 4;

/// Match every stat in a run of lines.
///
/// Returns the stats it matched and the lines it could not. Reminder strings
/// are skipped entirely, because they are explanatory text and matching them
/// would waste a lookup on every one.
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

        // Try the longest run of lines the data recognises, shortest first,
        // matching the reference. A stat spanning three lines only matches
        // when all three are offered together.
        //
        // The window is capped. See MAX_STAT_LINES.
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

/// Split the unscalable marker off a joined run of lines.
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

/// Whether a section looks like it holds modifiers at all.
///
/// Used to decide whether the modifier stage should claim a section. A section
/// of one line that matched nothing is far more likely to be something another
/// stage wants.
pub fn looks_like_modifiers(section: &[String]) -> bool {
    section
        .iter()
        .any(|l| is_mod_info_line(l) || has_type_suffix(l))
}

/// Whether a line ends in a modifier type suffix.
///
/// Explicit is absent because it has no suffix. That is exactly why this
/// function cannot identify an explicit modifier and the stage has to fall
/// back to whether anything matched.
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

/// The modifier stage.
///
/// Appears five times in each pipeline. Each occurrence claims one modifier
/// section, because a section is consumed at most once and an item can carry
/// enchant, rune, implicit, granted skill and explicit blocks at once.
///
/// A section is claimed only when it produced a modifier or a marked unknown.
/// Claiming an unrecognised section would starve the later occurrences.
pub fn parse_modifiers(section: &[String], state: &mut ParserState<'_>) -> ParseOutcome {
    let read = read_modifier_section(section, state.data);

    if read.is_empty() {
        return ParseOutcome::SectionSkipped;
    }

    // A section that matched nothing and carries no modifier marker is far
    // more likely to be something another stage wants. Only claim it when the
    // game itself said it holds modifiers.
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
        // One crafted modifier among explicit ones must not make the whole
        // section crafted, and must not stay explicit either.
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
        // Dropping it would price the item as if the modifier were not there.
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
        // The suffix is stripped before the line is recorded, so a later
        // lookup against fresher data can succeed.
        assert_eq!(got.unknown[0].text, "Something New");
    }

    #[test]
    fn the_join_window_is_capped_so_a_huge_section_cannot_hang_the_parser() {
        // Clipboard text is fully attacker controlled. Without the cap this
        // scan is quadratic and each step joins a growing string, so a large
        // paste freezes the overlay.
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
        // Documents the cap's cost. No stat GGG ships is this long, and the
        // alternative is a parser that hangs on a large paste.
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
        // Offering single lines only would leave both halves unknown.
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
        // Both a one line and a two line form are in the table. Taking the
        // longer one would swallow the next stat.
        let data = table(&["# to maximum Life", "# to Dexterity"]);

        let got = read_modifier_section(&sec(&["+25 to maximum Life", "+30 to Dexterity"]), &data);

        assert_eq!(got.modifiers[0].stats.len(), 2);
    }

    #[test]
    fn a_reminder_string_is_skipped_entirely() {
        // It is explanatory text. Recording it as unknown would warn the user
        // about a modifier that does not exist.
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
        // The game hides them. Recording nothing would lose the fact that the
        // item has an unrevealed modifier, which changes its price.
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
        // A blank line is not an unknown modifier.
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
        // An explicit modifier carries no marker at all, so this check cannot
        // identify one. The stage falls back to whether anything matched.
        assert!(!looks_like_modifiers(&sec(&["Item Level: 84"])));
        assert!(!looks_like_modifiers(&sec(&["+25 to maximum Life"])));
    }
}
