//! Exiled Exchange 2's modifier slot expectations, run against ours.
//!
//! Ported case for case from
//! `renderer/specs/web/price-check/filters/create-stat-filters.test.ts`, which
//! is a table of eighteen combinations of the two stats that change how many
//! modifiers an item can hold.
//!
//! # Why the slot count is worth a table
//!
//! A rare with five modifiers and one open prefix is worth more than the same
//! rare with six, because the buyer can craft into the gap. The trade site has
//! a filter for exactly that, and whether to send it depends on comparing what
//! the item has against what it can hold.
//!
//! What it can hold is not fixed. `+1 Prefix Modifier allowed` makes room for
//! a fourth prefix and `-1` takes one away, and an item can carry several of
//! them at once. Using the base count alone reports a craftable item as
//! finished and a widened one as over full, and both send the wrong filter.
//!
//! The reference's table is worth porting whole because the interesting cases
//! are the ones nobody writes by hand: two of the same stat stacking, a
//! negative that would take the budget below zero, and one of each pulling in
//! opposite directions.

use poe_wayfinder_core::controller::filter::slots::{
    max_modifiers_by_slot, PREFIX_ALLOWED, SUFFIX_ALLOWED,
};
use poe_wayfinder_core::types::item::ItemRarity;

/// One row of the reference's table: rarity, the stats on the item, and the
/// prefix and suffix budgets it expects.
type Row = (ItemRarity, Vec<(String, f64)>, usize, usize);

/// A stat and its roll, in the reference's shape.
fn stat(reference: &str, roll: f64) -> (String, f64) {
    (reference.to_string(), roll)
}

/// The reference's base expectation: normal none, magic one, rare three.
#[test]
fn the_base_count_matches_the_reference() {
    for (rarity, expected) in [
        (ItemRarity::Normal, 0),
        (ItemRarity::Magic, 1),
        (ItemRarity::Rare, 3),
    ] {
        let got = max_modifiers_by_slot(None, Some(rarity), &[]);

        assert_eq!(got.prefixes, expected, "{rarity:?}");
        assert_eq!(got.suffixes, expected, "{rarity:?}");
        // The reference returns [2 * base, base, base]: the total is the two
        // halves added, which is what `total` computes.
        assert_eq!(got.total(), expected * 2, "{rarity:?}");
    }
}

/// Every row of the reference's table: rarity, stats, expected prefixes,
/// expected suffixes.
#[test]
fn every_row_of_the_reference_table_matches() {
    let cases: Vec<Row> = vec![
        (ItemRarity::Rare, vec![], 3, 3),
        // A stat that is not one of the two changes nothing.
        (
            ItemRarity::Rare,
            vec![stat("# to all Attributes", 1.0)],
            3,
            3,
        ),
        (ItemRarity::Magic, vec![stat(SUFFIX_ALLOWED, 2.0)], 1, 3),
        // A magic item pushed below zero prefixes clamps at zero.
        (ItemRarity::Magic, vec![stat(PREFIX_ALLOWED, -1.0)], 0, 1),
        (ItemRarity::Rare, vec![stat(SUFFIX_ALLOWED, 2.0)], 3, 5),
        (ItemRarity::Rare, vec![stat(SUFFIX_ALLOWED, 1.0)], 3, 4),
        (ItemRarity::Rare, vec![stat(SUFFIX_ALLOWED, -1.0)], 3, 2),
        (ItemRarity::Rare, vec![stat(SUFFIX_ALLOWED, -2.0)], 3, 1),
        (ItemRarity::Rare, vec![stat(SUFFIX_ALLOWED, -3.0)], 3, 0),
        (ItemRarity::Rare, vec![stat(PREFIX_ALLOWED, 2.0)], 5, 3),
        (ItemRarity::Rare, vec![stat(PREFIX_ALLOWED, 1.0)], 4, 3),
        (ItemRarity::Rare, vec![stat(PREFIX_ALLOWED, -1.0)], 2, 3),
        (ItemRarity::Rare, vec![stat(PREFIX_ALLOWED, -2.0)], 1, 3),
        (ItemRarity::Rare, vec![stat(PREFIX_ALLOWED, -3.0)], 0, 3),
        // One of each, both up.
        (
            ItemRarity::Rare,
            vec![stat(PREFIX_ALLOWED, 1.0), stat(SUFFIX_ALLOWED, 1.0)],
            4,
            4,
        ),
        // Pulling in opposite directions.
        (
            ItemRarity::Rare,
            vec![stat(PREFIX_ALLOWED, 2.0), stat(SUFFIX_ALLOWED, -1.0)],
            5,
            2,
        ),
        (
            ItemRarity::Rare,
            vec![stat(PREFIX_ALLOWED, -1.0), stat(SUFFIX_ALLOWED, 2.0)],
            2,
            5,
        ),
        (
            ItemRarity::Rare,
            vec![stat(PREFIX_ALLOWED, -2.0), stat(SUFFIX_ALLOWED, -2.0)],
            1,
            1,
        ),
        (
            ItemRarity::Rare,
            vec![stat(PREFIX_ALLOWED, -3.0), stat(SUFFIX_ALLOWED, 2.0)],
            0,
            5,
        ),
        // The same stat twice. Both apply.
        (
            ItemRarity::Rare,
            vec![stat(SUFFIX_ALLOWED, 2.0), stat(SUFFIX_ALLOWED, 2.0)],
            3,
            7,
        ),
    ];

    for (rarity, stats, prefixes, suffixes) in cases {
        let got = max_modifiers_by_slot(None, Some(rarity), &stats);

        assert_eq!(got.prefixes, prefixes, "{rarity:?} {stats:?}");
        assert_eq!(got.suffixes, suffixes, "{rarity:?} {stats:?}");
        assert_eq!(got.total(), prefixes + suffixes, "{rarity:?} {stats:?}");
    }
}

// ---------------------------------------------------------------------------
// The invariants the table depends on
// ---------------------------------------------------------------------------

#[test]
fn a_budget_never_goes_below_zero() {
    // The reference clamps. A negative count has no meaning and would make
    // every comparison against it read as full, so an item with room would be
    // reported as finished.
    let got = max_modifiers_by_slot(
        None,
        Some(ItemRarity::Rare),
        &[stat(PREFIX_ALLOWED, -99.0), stat(SUFFIX_ALLOWED, -99.0)],
    );

    assert_eq!(got.prefixes, 0);
    assert_eq!(got.suffixes, 0);
    assert_eq!(got.total(), 0);
}

#[test]
fn a_prefix_allowance_never_touches_the_suffixes() {
    // Crossing them would move the open slot to the wrong half of the item,
    // and the filter would look right while finding the wrong items.
    let got = max_modifiers_by_slot(None, Some(ItemRarity::Rare), &[stat(PREFIX_ALLOWED, 2.0)]);

    assert_eq!(got.suffixes, 3);
}

#[test]
fn an_unrelated_stat_changes_nothing() {
    // Every modifier on the item is offered to this, so anything that is not
    // one of the two has to pass straight through.
    let got = max_modifiers_by_slot(
        None,
        Some(ItemRarity::Rare),
        &[
            stat("# to maximum Life", 79.0),
            stat("#% to Fire Resistance", 45.0),
            stat("Prefix Modifier allowed", 1.0),
        ],
    );

    assert_eq!(got.prefixes, 3);
    assert_eq!(got.suffixes, 3);
}

#[test]
fn a_normal_item_holds_nothing_however_it_is_adjusted_downward() {
    let got = max_modifiers_by_slot(
        None,
        Some(ItemRarity::Normal),
        &[stat(PREFIX_ALLOWED, -1.0)],
    );

    assert_eq!(got.prefixes, 0);
}
