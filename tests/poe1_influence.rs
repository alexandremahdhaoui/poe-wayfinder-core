//! Our influence spellings against the live PoE1 trade data.
//!
//! `fixtures/poe1_influence_stats.ndjson` is the six influence records exactly
//! as `www.pathofexile.com/api/trade/data/stats` returns them.
//!
//! # Why a fixture and not a unit test
//!
//! The filter looks a stat up by its text. A wrong spelling produces no filter
//! and no error, so the item prices against every plain base of its type and
//! nothing anywhere reports a problem. A unit test written against a spelling
//! I invented would agree with itself.
//!
//! Influence is most of what a PoE1 base is worth. Getting this wrong is an
//! order of magnitude on the price, not a rounding error.

use poe_trader_core::controller::filter::influence::{filterable, stat_reference};
use poe_trader_core::types::item::Influence;

const REAL: &str = include_str!("fixtures/poe1_influence_stats.ndjson");

const ALL: [Influence; 6] = [
    Influence::Crusader,
    Influence::Elder,
    Influence::Hunter,
    Influence::Redeemer,
    Influence::Shaper,
    Influence::Warlord,
];

#[test]
fn every_spelling_appears_in_the_real_data() {
    for influence in ALL {
        let reference = stat_reference(influence);

        assert!(
            REAL.contains(&format!(r#""ref":"{reference}""#)),
            "{reference} is not a stat the trade site knows"
        );
    }
}

#[test]
fn every_influence_carries_a_pseudo_trade_id() {
    // The filter reads the pseudo id. A record filed under any other namespace
    // would look correct here and send nothing.
    for line in REAL.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            line.contains(r#""pseudo":["pseudo.pseudo_has_"#),
            "an influence record carries no pseudo id: {line}"
        );
    }
}

#[test]
fn the_fixture_holds_one_record_per_influence() {
    // Five records would mean one influence silently has no filter.
    let count = REAL.lines().filter(|l| !l.trim().is_empty()).count();

    assert_eq!(count, ALL.len());
}

#[test]
fn each_record_matches_exactly_one_of_our_spellings() {
    // A record we do not name is an influence the overlay cannot filter on.
    for line in REAL.lines().filter(|l| !l.trim().is_empty()) {
        let hits = ALL
            .iter()
            .filter(|i| line.contains(&format!(r#""ref":"{}""#, stat_reference(**i))))
            .count();

        assert_eq!(hits, 1, "no single spelling claims: {line}");
    }
}

#[test]
fn a_double_influenced_base_filters_on_both() {
    // Shaper plus Elder is the classic double influenced item and both halves
    // are the reason it is worth anything.
    let both = [Influence::Shaper, Influence::Elder];

    for influence in filterable(&both) {
        assert!(REAL.contains(&format!(r#""ref":"{}""#, stat_reference(*influence))));
    }

    assert_eq!(filterable(&both).len(), 2);
}
