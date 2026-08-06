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
use crate::types::category::ItemCategory;
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
    // Grouped from the raw section, not from the stripped lines. Stripping
    // first removes the `(desecrated)` and `(crafted)` markers, and each
    // modifier's type is decided by the marker on its own lines.
    let groups = group_lines_by_mod(section);

    let mut out = ModifierSection::default();

    if groups.is_empty() {
        let (section_kind, lines) = parse_mod_type(section);

        // No metadata lines. The whole section is one modifier whose type the
        // line suffixes already gave us.
        let info = ModifierInfo {
            kind: Some(section_kind),
            ..ModifierInfo::default()
        };

        read_one(&info, &lines, section_kind, data, &mut out);

        return out;
    }

    for group in groups {
        // Each modifier is typed from its own stat lines, not from the
        // section's. A section can mix types: one line ending in
        // `(desecrated)` types that modifier and no other.
        //
        // Reading the section's type here gave a body armour with one
        // desecrated modifier five more of them, and the query then asked for
        // six desecrated modifiers that do not exist.
        let (group_kind, group_lines) = parse_mod_type(&group.stat_lines);

        let info = parse_mod_info_line(&group.mod_line, group_kind);
        let kind = info.kind.unwrap_or(group_kind);

        read_one(&info, &group_lines, kind, data, &mut out);
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
    section.iter().any(|l| {
        // A granted skill marks itself at the front rather than at the end.
        // Leaving it out here meant a shield whose granted skill was not in
        // the stat table lost the line entirely: no modifier, no unknown
        // modifier, nothing for the panel to warn about.
        l.starts_with(cs::GRANTS_SKILL) || is_mod_info_line(l) || has_type_suffix(l)
    })
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

/// Whether a section carries a tag that makes the whole of it one modifier.
///
/// A PoE2 enchant, rune or granted skill prints its marker once and the lines
/// under it belong together. Splitting them makes each line its own modifier
/// and the item then claims four runes where it has one.
fn whole_section_kind(section: &[String]) -> Option<ModifierType> {
    for line in section {
        // Checked before the suffixes because a granted skill marks itself at
        // the front of the line rather than the end.
        if line.starts_with(cs::GRANTS_SKILL) {
            return Some(ModifierType::Skill);
        }

        // The reference's order. Added rune is checked before rune because its
        // suffix ends with the rune suffix, so the looser test would claim it.
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

/// Read a PoE2 modifier section.
///
/// Ported from `parseModifiersPoe2`.
///
/// # How PoE2 differs
///
/// PoE1 prints one modifier per section and marks each with a metadata line
/// when advanced descriptions are on. PoE2 prints every explicit in one
/// section with no markers at all, so each line there is its own modifier.
///
/// Treating that section as one modifier makes a rare with six explicits look
/// like a single six line modifier, and the tier and name of every one of them
/// is then attributed to the first.
///
/// A section that does carry a tag is the exception and stays whole.
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

    // The section carries metadata, so the shared reader already groups it
    // correctly and splitting per line would throw the grouping away.
    if section.iter().any(|line| is_mod_info_line(line)) {
        return read_modifier_section(section, data);
    }

    let mut out = ModifierSection::default();

    for line in section {
        let own = std::slice::from_ref(line);
        let (mut kind, lines) = parse_mod_type(own);

        // A relic's explicits are sanctum modifiers. They trade under their
        // own ids, so sending them as explicit finds nothing.
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

/// The PoE2 modifier stage.
///
/// Same claiming rule as the shared stage. It differs only in how it splits
/// the section into modifiers.
pub fn parse_modifiers_poe2(section: &[String], state: &mut ParserState<'_>) -> ParseOutcome {
    // Only a real item carries modifiers. A currency stack or a map device
    // section reaching here would otherwise be read as a modifier block.
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

    // -----------------------------------------------------------------
    // PoE2 splitting
    // -----------------------------------------------------------------

    #[test]
    fn every_poe2_explicit_line_is_its_own_modifier() {
        // Treating the section as one modifier makes a rare with two explicits
        // look like a single two line modifier, and the tier and name of both
        // are then attributed to the first.
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
        // A rune prints its marker once and the lines under it belong
        // together. Splitting them claims two runes where the item has one.
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
        // Its suffix ends with the rune suffix, so the looser test claims it.
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
        // Sanctum modifiers trade under their own ids, so sending them as
        // explicit finds nothing.
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
        // Only the explicits become sanctum modifiers.
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
        // The metadata already says where each modifier starts, and splitting
        // per line would throw that grouping away.
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
        // A shield printing "Grants Skill: Raise Shield" against a data file
        // that spells it "Grants Skill: Level # Raise Shield" lost the line
        // entirely: no modifier, no unknown modifier, nothing for the panel to
        // warn about. The user saw a price built from an item the tool had
        // only half read, and nothing said so.
        let got =
            read_modifier_section_poe2(&sec(&["Grants Skill: Raise Shield"]), None, &table(&[]));

        assert!(got.modifiers.is_empty());
        assert_eq!(got.unknown.len(), 1);
        assert_eq!(got.unknown[0].kind, ModifierType::Skill);
    }

    #[test]
    fn a_granted_skill_line_marks_a_section_as_modifiers() {
        // The marker is at the front of the line, not the end, so the suffix
        // test alone never sees it.
        assert!(looks_like_modifiers(&sec(&["Grants Skill: Raise Shield"])));
    }

    #[test]
    fn an_ordinary_line_still_does_not_look_like_modifiers() {
        // The guard has to stay narrow. Treating any section as modifiers
        // makes the stage eat the first section it is offered.
        assert!(!looks_like_modifiers(&sec(&["Requires: Level 54"])));
    }
}
