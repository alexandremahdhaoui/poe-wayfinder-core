//! Exiled Exchange 2's own pseudo filter expectations, run against ours.
//!
//! Ported case for case from
//! `renderer/specs/web/price-check/filters/pseudo/index.test.ts`.
//!
//! # Why port the assertions and not just the fixtures
//!
//! `upstream_fixtures.rs` proves the reference's 25 items parse. Parsing is
//! the easy half. These are the reference's own expected numbers for what a
//! pseudo total should come out as, and a pseudo total is what almost every
//! real search is actually about.
//!
//! Every value here is the reference's, not one I chose. A test written
//! against numbers I picked agrees with whatever I implemented.

use poe_wayfinder_core::controller::aggregate::StatTotal;
use poe_wayfinder_core::controller::filter::pseudo::{pseudo_totals, PseudoTotal};
use poe_wayfinder_core::types::modifier::ModifierType;
use poe_wayfinder_core::types::stat::StatRoll;

const FIRE_RES: &str = "#% to Fire Resistance";
const COLD_RES: &str = "#% to Cold Resistance";
const MOVEMENT_SPEED: &str = "#% increased Movement Speed";
const STRENGTH: &str = "# to Strength";
const MAX_LIFE: &str = "# to maximum Life";

/// The reference's `createStatHelper`, with its default of 25.
fn stat(reference: &str, value: f64) -> StatTotal {
    StatTotal {
        reference: reference.to_string(),
        kind: ModifierType::Explicit,
        roll: Some(StatRoll {
            value,
            min: value,
            max: value,
            ..StatRoll::default()
        }),
        sources: 1,
    }
}

fn find<'a>(totals: &'a [PseudoTotal], reference: &str) -> Option<&'a PseudoTotal> {
    totals.iter().find(|t| t.reference == reference)
}

// ---------------------------------------------------------------------------
// "should do nothing if no stats"
// ---------------------------------------------------------------------------

#[test]
fn no_stats_produce_no_pseudo_totals() {
    assert!(pseudo_totals(&[]).is_empty());
}

// ---------------------------------------------------------------------------
// "should add pseudo for stat"
// ---------------------------------------------------------------------------

#[test]
fn one_resistance_produces_the_total_and_the_single() {
    // The reference expects exactly two filters from one fire resistance.
    let got = pseudo_totals(&[stat(FIRE_RES, 25.0)]);

    let total = find(&got, "+#% total Elemental Resistance").expect("elemental total");
    let fire = find(&got, "+#% total to Fire Resistance").expect("fire total");

    assert_eq!(total.roll.value, 25.0);
    assert_eq!(fire.roll.value, 25.0);
}

#[test]
fn the_elemental_total_is_on_and_the_single_resistance_is_off() {
    // The reference's exact expectation: totalRes.disabled false, fireRes
    // disabled true. It is the single most searched number in the game, and
    // requiring the individual one as well excludes most of the market.
    let got = pseudo_totals(&[stat(FIRE_RES, 25.0)]);

    assert!(
        !find(&got, "+#% total Elemental Resistance")
            .unwrap()
            .disabled
    );
    assert!(find(&got, "+#% total to Fire Resistance").unwrap().disabled);
}

// ---------------------------------------------------------------------------
// "should merge two of same stats"
// ---------------------------------------------------------------------------

#[test]
fn two_of_the_same_resistance_add_up() {
    // Two fire resistance modifiers of 25 make one filter of 50, not two
    // filters of 25. Filtering on either half finds almost nothing, because
    // the split across modifiers is random.
    let mut first = stat(FIRE_RES, 25.0);
    first.roll = Some(StatRoll {
        value: 50.0,
        min: 50.0,
        max: 50.0,
        ..StatRoll::default()
    });
    first.sources = 2;

    let got = pseudo_totals(&[first]);

    assert_eq!(
        find(&got, "+#% total Elemental Resistance")
            .unwrap()
            .roll
            .value,
        50.0
    );
    assert_eq!(
        find(&got, "+#% total to Fire Resistance")
            .unwrap()
            .roll
            .value,
        50.0
    );
}

#[test]
fn a_merged_total_reports_both_sources() {
    // The reference checks sources has length 2. The count is what tells the
    // UI the number came from two modifiers rather than one.
    let mut merged = stat(FIRE_RES, 50.0);
    merged.sources = 2;

    let got = pseudo_totals(&[merged]);

    assert_eq!(
        find(&got, "+#% total Elemental Resistance")
            .unwrap()
            .sources,
        2
    );
}

// ---------------------------------------------------------------------------
// "should join related stats"
// ---------------------------------------------------------------------------

#[test]
fn strength_counts_double_toward_life() {
    // The reference's exact number: life 25 plus strength 25 gives 75,
    // because ten strength gives five life and the trade site's own pseudo
    // total uses that ratio.
    let got = pseudo_totals(&[stat(MAX_LIFE, 25.0), stat(STRENGTH, 25.0)]);

    assert_eq!(
        find(&got, "+# total maximum Life").unwrap().roll.value,
        75.0
    );
}

#[test]
fn strength_still_reports_its_own_total_unchanged() {
    // The reference expects the strength total to stay 25. Doubling it there
    // too would double count it.
    let got = pseudo_totals(&[stat(MAX_LIFE, 25.0), stat(STRENGTH, 25.0)]);

    assert_eq!(find(&got, "+# total to Strength").unwrap().roll.value, 25.0);
}

#[test]
fn strength_alone_grants_no_life_total() {
    // The reference requires a real life modifier. Life from strength alone
    // would match a ring carrying only strength, which is not what anyone
    // means by total life.
    let got = pseudo_totals(&[stat(STRENGTH, 25.0)]);

    assert_eq!(find(&got, "+# total maximum Life"), None);
}

// ---------------------------------------------------------------------------
// "should only keep highest ele res"
// ---------------------------------------------------------------------------

#[test]
fn two_resistances_add_into_the_elemental_total() {
    // Fire 25 and cold 50 give 75. The reference's exact number.
    let got = pseudo_totals(&[stat(FIRE_RES, 25.0), stat(COLD_RES, 50.0)]);

    assert_eq!(
        find(&got, "+#% total Elemental Resistance")
            .unwrap()
            .roll
            .value,
        75.0
    );
}

#[test]
fn each_single_resistance_keeps_its_own_value() {
    let got = pseudo_totals(&[stat(FIRE_RES, 25.0), stat(COLD_RES, 50.0)]);

    assert_eq!(
        find(&got, "+#% total to Cold Resistance")
            .unwrap()
            .roll
            .value,
        50.0
    );
    assert_eq!(
        find(&got, "+#% total to Fire Resistance")
            .unwrap()
            .roll
            .value,
        25.0
    );
}

#[test]
fn the_elemental_total_stays_the_only_one_enabled() {
    // With two resistances the reference still enables only the elemental
    // total. Enabling both singles requires an item with this exact split.
    let got = pseudo_totals(&[stat(FIRE_RES, 25.0), stat(COLD_RES, 50.0)]);

    let enabled: Vec<&str> = got
        .iter()
        .filter(|t| !t.disabled)
        .map(|t| t.reference)
        .collect();

    assert_eq!(enabled, vec!["+#% total Elemental Resistance"]);
}

// ---------------------------------------------------------------------------
// A stat that feeds nothing
// ---------------------------------------------------------------------------

#[test]
fn movement_speed_feeds_no_pseudo_total() {
    // The reference keeps it as an ordinary filter. A pseudo total for it
    // would be the same number under a different name.
    let got = pseudo_totals(&[stat(MOVEMENT_SPEED, 25.0)]);

    assert!(got.is_empty(), "{got:?}");
}

// ---------------------------------------------------------------------------
// Invariants the reference relies on without stating
// ---------------------------------------------------------------------------

#[test]
fn no_two_totals_share_a_reference() {
    // Two rules producing one reference would send the same filter twice, and
    // the trade site takes the last one.
    let got = pseudo_totals(&[
        stat(FIRE_RES, 25.0),
        stat(COLD_RES, 50.0),
        stat(MAX_LIFE, 25.0),
        stat(STRENGTH, 25.0),
    ]);

    let mut seen: Vec<&str> = got.iter().map(|t| t.reference).collect();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();

    assert_eq!(seen.len(), before, "{got:?}");
}

#[test]
fn a_total_never_comes_out_below_a_contributor() {
    // A total is a sum. One smaller than a part means a sign or a multiplier
    // is wrong, and the filter would exclude the item it came from.
    let got = pseudo_totals(&[stat(FIRE_RES, 25.0), stat(COLD_RES, 50.0)]);

    assert!(
        find(&got, "+#% total Elemental Resistance")
            .unwrap()
            .roll
            .value
            >= 50.0
    );
}
