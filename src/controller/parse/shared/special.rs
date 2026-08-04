//! Items whose modifier block does not follow the normal rules.
//!
//! Ported from `parseMirroredTablet`, `parseFilledCoffin`, `parseLogbookArea`,
//! `parseAtzoatlRooms`, `parseFractured`, `parseBlightedMap` and
//! `calcBasePercentile` in `renderer/src/parser/Parser.ts`.
//!
//! Each of these is one named base that prints something no other item does.
//! They are gated on the base name so an ordinary item can never fall into
//! them, and each is small enough that the alternative is a pile of special
//! cases inside the general modifier stage.

use crate::controller::calc::quality::{prop_percentile, PropContributions};
use crate::controller::parse::shared::modifiers::match_stat_lines;
use crate::controller::parse::{ParseError, ParseOutcome, ParserState};
use crate::controller::stat_match::placeholder::StatString;
use crate::controller::stat_match::resolve::try_parse_translation;
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::{ItemRarity, MapBlighted, UnknownModifier};
use crate::types::modifier::{ModifierInfo, ModifierType};

use super::modifiers::ParsedModifier;

/// The Mirrored Tablet's pseudo modifier block.
///
/// It prints eight or more lines that are each a whole modifier, with no
/// metadata and no suffix. Nothing marks them, so the only signal is the base
/// name and the section length.
pub fn parse_mirrored_tablet(section: &[String], state: &mut ParserState<'_>) -> ParseOutcome {
    if state.item.info.reference_name != "Mirrored Tablet" {
        return ParseOutcome::ParserSkipped;
    }

    // Eight is the reference's threshold. A shorter section on this base is
    // something else, such as the item level block.
    if section.len() < 8 {
        return ParseOutcome::SectionSkipped;
    }

    for line in section {
        // Every line is its own modifier and none of them scale.
        let stat_line = StatString {
            string: line.clone(),
            unscalable: true,
        };

        match try_parse_translation(&stat_line, state.data) {
            Some(parsed) => state.item.modifiers.push(ParsedModifier {
                info: ModifierInfo {
                    kind: Some(ModifierType::Pseudo),
                    ..ModifierInfo::default()
                },
                stats: vec![parsed],
            }),
            None => state.item.unknown_modifiers.push(UnknownModifier {
                text: line.clone(),
                kind: ModifierType::Pseudo,
            }),
        }
    }

    ParseOutcome::SectionParsed
}

/// The Filled Coffin's necropolis modifier block.
///
/// It prints its modifiers with an implicit suffix, which the general stage
/// would read as implicits. They are necropolis modifiers and the trade site
/// files them under their own namespace.
pub fn parse_filled_coffin(section: &[String], state: &mut ParserState<'_>) -> ParseOutcome {
    if state.item.info.reference_name != "Filled Coffin" {
        return ParseOutcome::ParserSkipped;
    }

    let suffix = ModifierType::Implicit
        .line_suffix()
        .expect("implicit always has a suffix");

    if !section.iter().any(|l| l.ends_with(suffix)) {
        return ParseOutcome::SectionSkipped;
    }

    let lines: Vec<String> = section
        .iter()
        .map(|l| l.strip_suffix(suffix).unwrap_or(l).to_string())
        .collect();

    let (stats, unknown) = match_stat_lines(&lines, state.data);

    if !stats.is_empty() {
        state.item.modifiers.push(ParsedModifier {
            info: ModifierInfo {
                kind: Some(ModifierType::Necropolis),
                ..ModifierInfo::default()
            },
            stats,
        });
    }

    for text in unknown {
        state.item.unknown_modifiers.push(UnknownModifier {
            text,
            kind: ModifierType::Necropolis,
        });
    }

    ParseOutcome::SectionParsed
}

/// An Expedition Logbook's area modifier block.
///
/// A logbook carries several areas, each with its own faction and its own
/// modifiers. The stage appears three times in the pipeline, once per area,
/// which is why it claims one section at a time.
pub fn parse_logbook_area(section: &[String], state: &mut ParserState<'_>) -> ParseOutcome {
    if state.item.info.reference_name != "Expedition Logbook" {
        return ParseOutcome::ParserSkipped;
    }

    // The reference requires three lines: the area name, the faction and at
    // least one modifier.
    if section.len() < 3 {
        return ParseOutcome::SectionSkipped;
    }

    let (stats, unknown) = match_stat_lines(section, state.data);

    if stats.is_empty() && unknown.len() == section.len() {
        // Nothing matched at all, so this is not an area block.
        return ParseOutcome::SectionSkipped;
    }

    if !stats.is_empty() {
        state.item.modifiers.push(ParsedModifier {
            info: ModifierInfo {
                kind: Some(ModifierType::Explicit),
                ..ModifierInfo::default()
            },
            stats,
        });
    }

    ParseOutcome::SectionParsed
}

/// A Chronicle of Atzoatl's room list.
///
/// It prints `Open Rooms:` then a list, and optionally `Obstructed Rooms:`
/// then another. The rooms are what the item is worth, so the block is
/// recorded rather than merely consumed.
pub fn parse_atzoatl_rooms(section: &[String], state: &mut ParserState<'_>) -> ParseOutcome {
    if state.item.info.reference_name != "Chronicle of Atzoatl" {
        return ParseOutcome::ParserSkipped;
    }

    if !section
        .first()
        .is_some_and(|l| l.trim() == cs::INCURSION_OPEN)
    {
        return ParseOutcome::SectionSkipped;
    }

    let mut rooms = Vec::new();
    let mut open = true;

    for line in section.iter().skip(1) {
        let trimmed = line.trim();

        if trimmed == cs::INCURSION_OBSTRUCTED {
            open = false;

            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        rooms.push(AtzoatlRoom {
            name: trimmed.to_string(),
            open,
        });
    }

    state.item.atzoatl_rooms = rooms;

    ParseOutcome::SectionParsed
}

/// One room in a Chronicle of Atzoatl.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtzoatlRoom {
    pub name: String,
    /// Open rooms are usable. Obstructed ones need opening first and are worth
    /// less.
    pub open: bool,
}

/// Set the fractured flag from the modifiers.
///
/// Ported from `parseFractured`. A fractured modifier cannot be removed, which
/// changes what the item is worth, and the flag drives the query filter.
pub fn parse_fractured(state: &mut ParserState<'_>) -> Result<(), ParseError> {
    if state
        .item
        .modifiers
        .iter()
        .any(|m| m.info.kind == Some(ModifierType::Fractured))
    {
        state.item.is_fractured = true;
    }

    Ok(())
}

/// Work out whether a map is blighted from its implicit.
///
/// Ported from `parseBlightedMap`. The name prefix is stripped earlier, so on
/// a map whose prefix was already removed the implicit is the only signal
/// left.
///
/// A roll of nine means blight ravaged. Anything else means blighted.
pub fn parse_blighted_map(state: &mut ParserState<'_>) -> Result<(), ParseError> {
    if state.item.category != Some(ItemCategory::Map) {
        return Ok(());
    }

    // Already known from the name prefix. That signal is more direct.
    if state.item.map.blighted.is_some() {
        return Ok(());
    }

    for modifier in &state.item.modifiers {
        if modifier.info.kind != Some(ModifierType::Implicit) {
            continue;
        }

        for stat in &modifier.stats {
            if !stat
                .reference
                .starts_with("Area is infested with Fungal Growths")
            {
                continue;
            }

            let value = stat.roll.map_or(0.0, |r| r.value);

            state.item.map.blighted = Some(if value == 9.0 {
                MapBlighted::BlightRavaged
            } else {
                MapBlighted::Blighted
            });

            return Ok(());
        }
    }

    Ok(())
}

/// Where the base's defensive roll sits inside its possible range.
///
/// Ported from `calcBasePercentile`.
///
/// One percentile covers every defence, because a base rolls them together.
/// Armour is tried first, then evasion, then energy shield, because a larger
/// number carries more precision and armour is usually the largest.
///
/// Needs the base's roll ranges, which come from the game bundles and are not
/// in the trade API. Without them this reports nothing rather than guessing.
pub fn calc_base_percentile(state: &mut ParserState<'_>) -> Result<(), ParseError> {
    let bounds = state.item.info.armour_bounds;
    let quality = f64::from(state.item.quality.unwrap_or(0));

    let pairs = [
        (state.item.armour.ar, bounds.ar),
        (state.item.armour.ev, bounds.ev),
        (state.item.armour.es, bounds.es),
    ];

    for (rolled, range) in pairs {
        let (Some(rolled), Some(range)) = (rolled, range) else {
            continue;
        };

        if rolled <= 0.0 {
            continue;
        }

        state.item.base_percentile = Some(prop_percentile(
            rolled,
            range,
            PropContributions::default(),
            quality,
        ));

        return Ok(());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::data_adapter::{ItemLookup, Namespace, StatLookup};
    use crate::types::item::{ArmourBounds, ArmourStats, BaseInfo};
    use crate::types::stat::{ParsedStat, Stat, StatHit, StatMatcher, StatRoll};
    use crate::types::GameVersion;

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

    impl ItemLookup for FakeStats {
        fn items_by_name(&self, _: &str, _: Namespace, _: GameVersion) -> Vec<BaseInfo> {
            Vec::new()
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

    fn named<'a>(data: &'a FakeStats, name: &str) -> ParserState<'a> {
        let mut s = ParserState {
            data,
            ..ParserState::default()
        };
        s.item.info.reference_name = name.into();

        s
    }

    // -----------------------------------------------------------------
    // Mirrored Tablet
    // -----------------------------------------------------------------

    #[test]
    fn a_mirrored_tablet_reads_every_line_as_its_own_modifier() {
        let data = table(&["# to maximum Life"]);
        let mut s = named(&data, "Mirrored Tablet");

        let lines = sec(&["# to maximum Life"; 8]);
        let out = parse_mirrored_tablet(&lines, &mut s);

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(s.item.modifiers.len(), 8);
        assert_eq!(s.item.modifiers[0].info.kind, Some(ModifierType::Pseudo));
    }

    #[test]
    fn a_mirrored_tablet_line_that_matches_nothing_is_kept_as_unknown() {
        let data = table(&[]);
        let mut s = named(&data, "Mirrored Tablet");

        parse_mirrored_tablet(&sec(&["Unknown thing"; 8]), &mut s);

        assert_eq!(s.item.unknown_modifiers.len(), 8);
        assert_eq!(s.item.unknown_modifiers[0].kind, ModifierType::Pseudo);
    }

    #[test]
    fn a_short_section_on_a_mirrored_tablet_is_not_its_modifier_block() {
        // The item level block would otherwise be read as modifiers.
        let data = table(&[]);
        let mut s = named(&data, "Mirrored Tablet");

        assert_eq!(
            parse_mirrored_tablet(&sec(&["Item Level: 80"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn any_other_base_skips_the_mirrored_tablet_stage() {
        let data = table(&[]);
        let mut s = named(&data, "Sapphire Ring");

        assert_eq!(
            parse_mirrored_tablet(&sec(&["x"; 10]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    // -----------------------------------------------------------------
    // Filled Coffin
    // -----------------------------------------------------------------

    #[test]
    fn a_filled_coffin_files_its_modifiers_under_necropolis() {
        // They print with an implicit suffix. Reading them as implicits sends
        // the query to the wrong namespace and it returns nothing.
        let data = table(&["# to maximum Life"]);
        let mut s = named(&data, "Filled Coffin");

        let out = parse_filled_coffin(&sec(&["# to maximum Life (implicit)"]), &mut s);

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(
            s.item.modifiers[0].info.kind,
            Some(ModifierType::Necropolis)
        );
    }

    #[test]
    fn a_filled_coffin_section_with_no_implicit_suffix_is_skipped() {
        let data = table(&["# to maximum Life"]);
        let mut s = named(&data, "Filled Coffin");

        assert_eq!(
            parse_filled_coffin(&sec(&["# to maximum Life"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn any_other_base_skips_the_filled_coffin_stage() {
        let data = table(&[]);
        let mut s = named(&data, "Sapphire Ring");

        assert_eq!(
            parse_filled_coffin(&sec(&["x (implicit)"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    // -----------------------------------------------------------------
    // Expedition Logbook
    // -----------------------------------------------------------------

    #[test]
    fn a_logbook_area_block_is_read() {
        let data = table(&["# to maximum Life"]);
        let mut s = named(&data, "Expedition Logbook");

        let out = parse_logbook_area(
            &sec(&[
                "Rotting Temple",
                "Order of the Chalice",
                "# to maximum Life",
            ]),
            &mut s,
        );

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(s.item.modifiers.len(), 1);
    }

    #[test]
    fn a_short_logbook_section_is_not_an_area_block() {
        let data = table(&["# to maximum Life"]);
        let mut s = named(&data, "Expedition Logbook");

        assert_eq!(
            parse_logbook_area(&sec(&["Item Level: 80", "x"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn a_logbook_section_where_nothing_matches_is_left_for_another_stage() {
        let data = table(&[]);
        let mut s = named(&data, "Expedition Logbook");

        assert_eq!(
            parse_logbook_area(&sec(&["a", "b", "c"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    // -----------------------------------------------------------------
    // Chronicle of Atzoatl
    // -----------------------------------------------------------------

    #[test]
    fn atzoatl_rooms_are_recorded_with_their_state() {
        // The rooms are what the item is worth, so they are recorded rather
        // than merely consumed.
        let data = table(&[]);
        let mut s = named(&data, "Chronicle of Atzoatl");

        let out = parse_atzoatl_rooms(
            &sec(&[
                "Open Rooms:",
                "Hall of Metallurgy",
                "Obstructed Rooms:",
                "Sparring Room",
            ]),
            &mut s,
        );

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(
            s.item.atzoatl_rooms,
            vec![
                AtzoatlRoom {
                    name: "Hall of Metallurgy".into(),
                    open: true
                },
                AtzoatlRoom {
                    name: "Sparring Room".into(),
                    open: false
                },
            ]
        );
    }

    #[test]
    fn an_atzoatl_with_only_open_rooms_reads_them_all_as_open() {
        let data = table(&[]);
        let mut s = named(&data, "Chronicle of Atzoatl");

        parse_atzoatl_rooms(&sec(&["Open Rooms:", "A", "B"]), &mut s);

        assert!(s.item.atzoatl_rooms.iter().all(|r| r.open));
        assert_eq!(s.item.atzoatl_rooms.len(), 2);
    }

    #[test]
    fn a_section_that_does_not_start_with_open_rooms_is_skipped() {
        let data = table(&[]);
        let mut s = named(&data, "Chronicle of Atzoatl");

        assert_eq!(
            parse_atzoatl_rooms(&sec(&["Item Level: 80"]), &mut s),
            ParseOutcome::SectionSkipped
        );
    }

    #[test]
    fn any_other_base_skips_the_atzoatl_stage() {
        let data = table(&[]);
        let mut s = named(&data, "Sapphire Ring");

        assert_eq!(
            parse_atzoatl_rooms(&sec(&["Open Rooms:", "A"]), &mut s),
            ParseOutcome::ParserSkipped
        );
    }

    // -----------------------------------------------------------------
    // Fractured
    // -----------------------------------------------------------------

    fn modifier_of(kind: ModifierType) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(kind),
                ..ModifierInfo::default()
            },
            stats: Vec::new(),
        }
    }

    #[test]
    fn a_fractured_modifier_sets_the_item_flag() {
        // It cannot be removed, which changes what the item is worth, and the
        // flag drives the query filter.
        let data = table(&[]);
        let mut s = named(&data, "Spine Bow");
        s.item.modifiers.push(modifier_of(ModifierType::Fractured));

        parse_fractured(&mut s).unwrap();

        assert!(s.item.is_fractured);
    }

    #[test]
    fn an_item_with_no_fractured_modifier_keeps_the_flag_clear() {
        let data = table(&[]);
        let mut s = named(&data, "Spine Bow");
        s.item.modifiers.push(modifier_of(ModifierType::Explicit));

        parse_fractured(&mut s).unwrap();

        assert!(!s.item.is_fractured);
    }

    // -----------------------------------------------------------------
    // Blighted maps
    // -----------------------------------------------------------------

    fn infested(value: f64) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(ModifierType::Implicit),
                ..ModifierInfo::default()
            },
            stats: vec![ParsedStat {
                reference: "Area is infested with Fungal Growths".into(),
                matched: "Area is infested with Fungal Growths".into(),
                roll: Some(StatRoll {
                    value,
                    ..StatRoll::default()
                }),
            }],
        }
    }

    #[test]
    fn a_roll_of_nine_means_blight_ravaged() {
        let data = table(&[]);
        let mut s = named(&data, "Toxic Map");
        s.item.category = Some(ItemCategory::Map);
        s.item.modifiers.push(infested(9.0));

        parse_blighted_map(&mut s).unwrap();

        assert_eq!(s.item.map.blighted, Some(MapBlighted::BlightRavaged));
    }

    #[test]
    fn any_other_roll_means_merely_blighted() {
        let data = table(&[]);
        let mut s = named(&data, "Toxic Map");
        s.item.category = Some(ItemCategory::Map);
        s.item.modifiers.push(infested(3.0));

        parse_blighted_map(&mut s).unwrap();

        assert_eq!(s.item.map.blighted, Some(MapBlighted::Blighted));
    }

    #[test]
    fn the_name_prefix_wins_over_the_implicit() {
        // The prefix is the more direct signal and was read earlier.
        let data = table(&[]);
        let mut s = named(&data, "Toxic Map");
        s.item.category = Some(ItemCategory::Map);
        s.item.map.blighted = Some(MapBlighted::BlightRavaged);
        s.item.modifiers.push(infested(3.0));

        parse_blighted_map(&mut s).unwrap();

        assert_eq!(s.item.map.blighted, Some(MapBlighted::BlightRavaged));
    }

    #[test]
    fn a_non_map_is_never_marked_blighted() {
        let data = table(&[]);
        let mut s = named(&data, "Sapphire Ring");
        s.item.category = Some(ItemCategory::Ring);
        s.item.modifiers.push(infested(9.0));

        parse_blighted_map(&mut s).unwrap();

        assert_eq!(s.item.map.blighted, None);
    }

    #[test]
    fn an_explicit_infestation_modifier_does_not_count() {
        // Only the implicit marks a blighted map.
        let data = table(&[]);
        let mut s = named(&data, "Toxic Map");
        s.item.category = Some(ItemCategory::Map);

        let mut m = infested(9.0);
        m.info.kind = Some(ModifierType::Explicit);
        s.item.modifiers.push(m);

        parse_blighted_map(&mut s).unwrap();

        assert_eq!(s.item.map.blighted, None);
    }

    // -----------------------------------------------------------------
    // Base percentile
    // -----------------------------------------------------------------

    #[test]
    fn a_base_roll_in_the_middle_of_its_range_is_the_fiftieth_percentile() {
        let data = table(&[]);
        let mut s = named(&data, "Iron Hat");
        s.item.armour = ArmourStats {
            ar: Some(150.0),
            ..ArmourStats::default()
        };
        s.item.info.armour_bounds = ArmourBounds {
            ar: Some((100.0, 200.0)),
            ..ArmourBounds::default()
        };

        calc_base_percentile(&mut s).unwrap();

        assert_eq!(s.item.base_percentile, Some(50.0));
    }

    #[test]
    fn armour_is_preferred_over_evasion_and_energy_shield() {
        // A base rolls its defences together, so one percentile covers them
        // all. The largest number carries the most precision.
        let data = table(&[]);
        let mut s = named(&data, "Hybrid Base");
        s.item.armour = ArmourStats {
            ar: Some(200.0),
            ev: Some(100.0),
            ..ArmourStats::default()
        };
        s.item.info.armour_bounds = ArmourBounds {
            ar: Some((100.0, 200.0)),
            ev: Some((100.0, 200.0)),
            ..ArmourBounds::default()
        };

        calc_base_percentile(&mut s).unwrap();

        // From armour, which is at the top of its range.
        assert_eq!(s.item.base_percentile, Some(100.0));
    }

    #[test]
    fn evasion_is_used_when_there_is_no_armour() {
        let data = table(&[]);
        let mut s = named(&data, "Leather Cap");
        s.item.armour = ArmourStats {
            ev: Some(200.0),
            ..ArmourStats::default()
        };
        s.item.info.armour_bounds = ArmourBounds {
            ev: Some((100.0, 200.0)),
            ..ArmourBounds::default()
        };

        calc_base_percentile(&mut s).unwrap();

        assert_eq!(s.item.base_percentile, Some(100.0));
    }

    #[test]
    fn quality_is_removed_before_the_percentile_is_taken() {
        let data = table(&[]);
        let mut s = named(&data, "Iron Hat");
        s.item.quality = Some(20);
        s.item.armour = ArmourStats {
            ar: Some(120.0),
            ..ArmourStats::default()
        };
        s.item.info.armour_bounds = ArmourBounds {
            ar: Some((100.0, 200.0)),
            ..ArmourBounds::default()
        };

        calc_base_percentile(&mut s).unwrap();

        // 120 at 20 quality is 100 base, the bottom of the range.
        assert_eq!(s.item.base_percentile, Some(0.0));
    }

    #[test]
    fn a_base_with_no_known_range_reports_nothing_rather_than_guessing() {
        // The ranges come from the game bundles and are not in the trade API.
        let data = table(&[]);
        let mut s = named(&data, "Iron Hat");
        s.item.armour = ArmourStats {
            ar: Some(150.0),
            ..ArmourStats::default()
        };

        calc_base_percentile(&mut s).unwrap();

        assert_eq!(s.item.base_percentile, None);
    }

    #[test]
    fn an_item_with_no_defences_reports_nothing() {
        let data = table(&[]);
        let mut s = named(&data, "Spine Bow");
        s.item.info.armour_bounds = ArmourBounds {
            ar: Some((100.0, 200.0)),
            ..ArmourBounds::default()
        };

        calc_base_percentile(&mut s).unwrap();

        assert_eq!(s.item.base_percentile, None);
    }
}
