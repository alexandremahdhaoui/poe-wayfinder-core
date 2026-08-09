//! Exiled Exchange 2's affix tier expectations, run against ours.
//!
//! Ported from
//! `renderer/specs/web/price-check/trade/pathofexile-trade.test.ts`, whose
//! four cases assert the exact tier strings a listing tooltip should show.
//!
//! The fixture is the reference's own `docs/fetchResponses.json`, trimmed to
//! the four listings it asserts on and to the fields the tier join needs.
//! These are real responses from the trade site's fetch endpoint.
//!
//! # Why the tier join is worth pinning
//!
//! The response does not pair a printed line with its tier. It sends the
//! printed lines in one array, the modifiers in another, and a third array
//! saying which modifiers produced each line. Joining them by position looks
//! right on a simple item and is wrong the moment a modifier grants two lines
//! or a line comes from two modifiers.
//!
//! Both happen, and the reference has a named case for each. A wrong tier is
//! not an error: it is a plausible number next to the wrong modifier, which is
//! exactly the kind of thing a person acts on before noticing.

use poe_wayfinder_core::controller::listing::{mod_block, LineKind};

const REAL: &str = include_str!("fixtures/upstream_listings.json");

/// One listing and what the reference says its tiers are.
struct Case {
    id: String,
    expected: Vec<String>,
    lines: Vec<String>,
    /// The tier of each modifier, in the order the response lists them.
    tiers: Vec<Option<String>>,
    /// Which modifiers produced each printed line, in line order.
    hashes: Vec<Vec<usize>>,
}

fn cases() -> Vec<Case> {
    let parsed: Vec<serde_json::Value> =
        serde_json::from_str(REAL).expect("the fixture is valid JSON");

    parsed
        .into_iter()
        .map(|case| Case {
            id: case["id"].as_str().unwrap_or_default().to_string(),
            expected: strings(&case["expectedTiers"]),
            lines: strings(&case["explicitMods"]),
            tiers: case["modTiers"]
                .as_array()
                .expect("modTiers is an array")
                .iter()
                .map(|t| t.as_str().map(str::to_string))
                .collect(),
            hashes: case["hashes"]
                .as_array()
                .expect("hashes is an array")
                .iter()
                .map(|group| {
                    group
                        .as_array()
                        .expect("each hash group is an array")
                        .iter()
                        .filter_map(|i| i.as_u64().map(|i| i as usize))
                        .collect()
                })
                .collect(),
        })
        .collect()
}

fn strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .expect("an array of strings")
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect()
}

fn tiers_of(case: &Case) -> Vec<Option<String>> {
    mod_block(&case.lines, LineKind::Normal, &case.tiers, &case.hashes)
        .into_iter()
        .map(|line| line.tier)
        .collect()
}

#[test]
fn the_reference_cases_are_all_present() {
    // Four in the reference. A trimmed fixture that dropped one would make the
    // rest pass on what was left.
    assert_eq!(cases().len(), 4);
}

#[test]
fn every_tier_matches_the_reference_exactly() {
    // The reference's four expectations, character for character.
    for case in cases() {
        let got: Vec<String> = tiers_of(&case)
            .into_iter()
            .map(|t| t.unwrap_or_default())
            .collect();

        assert_eq!(got, case.expected, "{}", case.id);
    }
}

#[test]
fn a_hybrid_modifier_repeats_its_tier_on_both_lines() {
    // "reuses the same affix tier for hybrid modifier lines". One modifier
    // grants two printed lines and both carry its tier. Joining by position
    // would put the second modifier's tier on the second line.
    let case = cases()
        .into_iter()
        .find(|c| c.id == "ece06af2")
        .expect("the hybrid case is in the fixture");

    let got = tiers_of(&case);

    assert_eq!(got[0], got[1], "the two hybrid lines disagree: {got:?}");
    assert_eq!(got[0].as_deref(), Some("P7"));
}

#[test]
fn a_line_from_two_modifiers_shows_both_tiers() {
    // "can do many tiers to one mod". Two modifiers add into one printed line
    // and the tooltip has to say so, because the line's value is the sum and
    // neither tier explains it alone.
    let case = cases()
        .into_iter()
        .find(|c| c.id == "2fa2f1ee")
        .expect("the many to one case is in the fixture");

    let got = tiers_of(&case);

    assert_eq!(got[0].as_deref(), Some("P2 + P5"));
}

#[test]
fn one_modifier_granting_several_lines_tiers_all_of_them() {
    // "can do many mods from one tier". Three lines come from one suffix and
    // all three carry S1.
    let case = cases()
        .into_iter()
        .find(|c| c.id == "91c9f13")
        .expect("the one to many case is in the fixture");

    let got = tiers_of(&case);

    assert_eq!(got[1], got[2]);
    assert_eq!(got[2], got[3]);
    assert_eq!(got[1].as_deref(), Some("S1"));
}

#[test]
fn every_printed_line_gets_a_line_back() {
    // The tooltip shows what the seller's item says. Dropping a line because
    // its tier could not be resolved would hide a modifier the buyer is paying
    // for.
    for case in cases() {
        let got = mod_block(&case.lines, LineKind::Normal, &case.tiers, &case.hashes);

        assert_eq!(got.len(), case.lines.len(), "{}", case.id);
    }
}

#[test]
fn no_tier_is_invented_for_a_line_that_has_none() {
    // A join that reached past the end of the tier list would produce a tier
    // out of nowhere, which reads as fact.
    for case in cases() {
        let got = tiers_of(&case);

        for (index, tier) in got.iter().enumerate() {
            let Some(tier) = tier else {
                continue;
            };

            // Every tier shown has to be one the response actually sent.
            for part in tier.split(" + ") {
                assert!(
                    case.tiers.iter().flatten().any(|t| t == part),
                    "{} line {index} shows tier {part}, which the response never sent",
                    case.id
                );
            }
        }
    }
}

#[test]
fn the_fixture_holds_real_responses_and_not_empty_shells() {
    // A fixture trimmed too far would pass every test above by having nothing
    // to check.
    for case in cases() {
        assert!(!case.lines.is_empty(), "{} has no printed lines", case.id);
        assert!(!case.tiers.is_empty(), "{} has no modifiers", case.id);
        assert_eq!(
            case.hashes.len(),
            case.lines.len(),
            "{} has a hash group per line",
            case.id
        );
    }
}
