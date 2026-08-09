//! Exiled Exchange 2's stat translation expectations, run against ours.
//!
//! Ported from `renderer/specs/Parser/stat-translation.test.ts`, using the
//! same three stats it builds by hand: a physical damage stat with a negating
//! matcher, an exact match stat with no roll, and an option stat whose
//! matchers bake their value into the text.
//!
//! # What this step does
//!
//! It turns one printed line into a stat id and a roll. `180(170-190)%
//! increased Physical Damage` has to become the physical damage stat with a
//! roll of 180 and a range of 170 to 190, and the same line reading 220 has to
//! come out marked legacy, because a roll above the stated range means the
//! item predates a balance change and cannot be made again.
//!
//! # What is not ported
//!
//! Two of the reference's ten cases exercise a second lookup table it keeps
//! for stats that exist on the trade site but not in its own stat file. Our
//! data builder merges both into one table, so there is no second tier to fall
//! back to and no behaviour to compare against.

use poe_wayfinder_core::adapter::data_adapter::StatLookup;
use poe_wayfinder_core::controller::stat_match::placeholder::StatString;
use poe_wayfinder_core::controller::stat_match::resolve::try_parse_translation;
use poe_wayfinder_core::types::stat::{Stat, StatBetter, StatHit, StatMatcher, TradeInfo};

/// The reference's `PHYS_DAMAGE_STAT`, including its negating matcher and the
/// one that bakes in minus a hundred.
fn phys_damage() -> Stat {
    let mut trade = TradeInfo::default();
    trade
        .ids
        .insert("explicit".into(), vec!["explicit.stat_1509134228".into()]);

    Stat {
        reference: "#% increased Physical Damage".into(),
        better: StatBetter::PositiveRoll,
        matchers: vec![
            StatMatcher {
                string: "#% increased Physical Damage".into(),
                ..StatMatcher::default()
            },
            StatMatcher {
                string: "#% reduced Physical Damage".into(),
                negate: true,
                ..StatMatcher::default()
            },
            StatMatcher {
                string: "No Physical Damage".into(),
                value: Some(-100.0),
                ..StatMatcher::default()
            },
        ],
        trade,
        ..Stat::default()
    }
}

/// The reference's `EXACT_MATCH_STAT`. No placeholder anywhere in it.
fn exact_match() -> Stat {
    let mut trade = TradeInfo::default();
    trade
        .ids
        .insert("explicit".into(), vec!["explicit.stat_4103440490".into()]);

    Stat {
        reference: "Players are Cursed with Enfeeble".into(),
        better: StatBetter::PositiveRoll,
        matchers: vec![StatMatcher {
            string: "Players are Cursed with Enfeeble".into(),
            ..StatMatcher::default()
        }],
        trade,
        ..Stat::default()
    }
}

/// The reference's `OPTION_STAT`. Each matcher carries its own fixed value
/// rather than a placeholder, because the game prints a word and the trade
/// site wants a number.
fn option_stat() -> Stat {
    let mut trade = TradeInfo::default();
    trade
        .ids
        .insert("explicit".into(), vec!["explicit.stat_3891355829".into()]);
    trade.option = true;

    Stat {
        reference: "Upgrades Radius to #".into(),
        better: StatBetter::PositiveRoll,
        matchers: vec![
            StatMatcher {
                string: "Upgrades Radius to Large".into(),
                value: Some(2.0),
                ..StatMatcher::default()
            },
            StatMatcher {
                string: "Upgrades Radius to Medium".into(),
                value: Some(1.0),
                ..StatMatcher::default()
            },
        ],
        trade,
        ..Stat::default()
    }
}

/// A table holding whatever stats a case needs.
struct Table {
    stats: Vec<Stat>,
}

impl StatLookup for Table {
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

fn table(stats: Vec<Stat>) -> Table {
    Table { stats }
}

fn line(text: &str) -> StatString {
    StatString {
        string: text.to_string(),
        unscalable: false,
    }
}

// ---------------------------------------------------------------------------
// "should return undefined if no translation is found"
// ---------------------------------------------------------------------------

#[test]
fn a_line_no_stat_matches_produces_nothing() {
    // Nothing rather than a guess. A guessed stat sends a filter for a
    // modifier the item does not have.
    let got = try_parse_translation(&line("Some words nothing knows"), &table(vec![]));

    assert!(got.is_none());
}

// ---------------------------------------------------------------------------
// "should parse roll from stat, when by match str"
// ---------------------------------------------------------------------------

#[test]
fn a_roll_with_its_range_reads_the_roll() {
    // The reference's exact line and number.
    let got = try_parse_translation(
        &line("180(170-190)% increased Physical Damage"),
        &table(vec![phys_damage()]),
    )
    .expect("the stat is in the table");

    assert_eq!(got.reference, "#% increased Physical Damage");
    assert_eq!(got.roll.expect("a roll").value, 180.0);
}

#[test]
fn a_roll_with_its_range_reads_the_range() {
    // The range is what tells a buyer whether 180 is good. Without it every
    // roll looks the same.
    let roll = try_parse_translation(
        &line("180(170-190)% increased Physical Damage"),
        &table(vec![phys_damage()]),
    )
    .and_then(|s| s.roll)
    .expect("a roll");

    assert_eq!(roll.min, 170.0);
    assert_eq!(roll.max, 190.0);
}

#[test]
fn a_roll_inside_its_range_is_not_legacy() {
    let roll = try_parse_translation(
        &line("180(170-190)% increased Physical Damage"),
        &table(vec![phys_damage()]),
    )
    .and_then(|s| s.roll)
    .expect("a roll");

    assert!(!roll.legacy);
}

// ---------------------------------------------------------------------------
// "should handle legacy rolls"
// ---------------------------------------------------------------------------

#[test]
fn a_roll_above_its_range_is_legacy() {
    // The reference's exact case: 220 where the range says 170 to 190. The
    // item predates a balance change, cannot be made again, and is usually
    // worth more for it. Reporting it as an ordinary roll hides that.
    let roll = try_parse_translation(
        &line("220(170-190)% increased Physical Damage"),
        &table(vec![phys_damage()]),
    )
    .and_then(|s| s.roll)
    .expect("a roll");

    assert_eq!(roll.value, 220.0);
    assert!(roll.legacy);
}

// ---------------------------------------------------------------------------
// "should prefer match str on exact match"
// ---------------------------------------------------------------------------

#[test]
fn a_line_with_no_numbers_matches_its_stat_whole() {
    // Nothing to placehold, so the only template that can match is the line
    // itself.
    let got = try_parse_translation(
        &line("Players are Cursed with Enfeeble"),
        &table(vec![exact_match()]),
    )
    .expect("the stat is in the table");

    assert_eq!(got.reference, "Players are Cursed with Enfeeble");
}

// ---------------------------------------------------------------------------
// "should handle options in match str"
// ---------------------------------------------------------------------------

#[test]
fn a_matcher_that_bakes_its_value_in_reports_that_value() {
    // The game prints a word and the trade site wants a number. The reference
    // expects `Upgrades Radius to Large` to come out as 2.
    let roll = try_parse_translation(
        &line("Upgrades Radius to Large"),
        &table(vec![option_stat()]),
    )
    .and_then(|s| s.roll)
    .expect("a roll");

    assert_eq!(roll.value, 2.0);
}

#[test]
fn each_baked_matcher_reports_its_own_value() {
    // Two matchers on one stat, different values. Taking the first would make
    // every radius upgrade read as large.
    let roll = try_parse_translation(
        &line("Upgrades Radius to Medium"),
        &table(vec![option_stat()]),
    )
    .and_then(|s| s.roll)
    .expect("a roll");

    assert_eq!(roll.value, 1.0);
}

// ---------------------------------------------------------------------------
// The negating matcher, which the reference builds but never asserts on
// ---------------------------------------------------------------------------

#[test]
fn a_negating_matcher_flips_the_sign() {
    // `#% reduced Physical Damage` is the same stat as increased, counted the
    // other way. The trade site has one id for both, so a reduced roll has to
    // arrive negative or it searches for an improvement.
    let roll = try_parse_translation(
        &line("30% reduced Physical Damage"),
        &table(vec![phys_damage()]),
    )
    .and_then(|s| s.roll)
    .expect("a roll");

    assert_eq!(roll.value, -30.0);
}

#[test]
fn a_matcher_baking_a_negative_reports_it() {
    // `No Physical Damage` carries minus one hundred. A line with no number at
    // all still has to produce one.
    let roll = try_parse_translation(&line("No Physical Damage"), &table(vec![phys_damage()]))
        .and_then(|s| s.roll)
        .expect("a roll");

    assert_eq!(roll.value, -100.0);
}

// ---------------------------------------------------------------------------
// Invariants the cases above rely on
// ---------------------------------------------------------------------------

#[test]
fn the_matched_text_is_carried_back() {
    // The filter builder looks the stat up again by the literal form that
    // matched, so losing it drops the filter.
    let got = try_parse_translation(
        &line("180(170-190)% increased Physical Damage"),
        &table(vec![phys_damage()]),
    )
    .expect("a stat");

    assert!(!got.matched.is_empty());
    assert!(
        table(vec![phys_damage()])
            .stat_by_matcher(&got.matched)
            .is_some(),
        "the matched text does not find the stat again: {}",
        got.matched
    );
}

#[test]
fn a_stat_in_the_table_never_matches_a_line_for_a_different_stat() {
    // The table holds all three. A line for one must not resolve to another.
    let all = table(vec![phys_damage(), exact_match(), option_stat()]);

    for (text, expected) in [
        (
            "180(170-190)% increased Physical Damage",
            "#% increased Physical Damage",
        ),
        (
            "Players are Cursed with Enfeeble",
            "Players are Cursed with Enfeeble",
        ),
        ("Upgrades Radius to Large", "Upgrades Radius to #"),
    ] {
        let got = try_parse_translation(&line(text), &all).expect(text);

        assert_eq!(got.reference, expected, "{text}");
    }
}
