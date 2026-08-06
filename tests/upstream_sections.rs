//! Exiled Exchange 2's own section parser expectations, run against ours.
//!
//! Ported case for case from `renderer/specs/Parser/sectionUnit.test.ts`.
//!
//! # What a section stage has to get right
//!
//! Three answers, not two. `SectionParsed` consumes the section and removes
//! it. `SectionSkipped` leaves it for the next section. `ParserSkipped` gives
//! up on the whole stage.
//!
//! Returning the wrong one is invisible in a passing parse. A stage that
//! consumes a section it should have skipped eats the modifier block that was
//! going to follow, and the item comes out with no modifiers and no error.
//!
//! Every input and every expected answer here is the reference's.

use poe_trader_core::controller::parse::shared::content::parse_trials;
use poe_trader_core::controller::parse::shared::flags::parse_unidentified;
use poe_trader_core::controller::parse::{ParseOutcome, ParserState};
use poe_trader_core::types::game::GameVersion;
use poe_trader_core::types::item::UltimatumHint;

fn state() -> ParserState<'static> {
    ParserState {
        game: GameVersion::Poe2,
        ..ParserState::default()
    }
}

fn named(reference_name: &str) -> ParserState<'static> {
    let mut s = state();
    s.item.info.reference_name = reference_name.to_string();

    s
}

fn sec(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|l| l.to_string()).collect()
}

// ---------------------------------------------------------------------------
// "Parse Unidentified Sections"
// ---------------------------------------------------------------------------

#[test]
fn a_plain_unidentified_line_is_read() {
    let mut s = state();

    let got = parse_unidentified(&sec(&["Unidentified"]), &mut s);

    assert_eq!(got, ParseOutcome::SectionParsed);
    assert!(s.item.is_unidentified);
    assert_eq!(s.item.unidentified_tier, None);
}

#[test]
fn an_unidentified_line_with_a_tier_reports_the_tier() {
    let mut s = state();

    let got = parse_unidentified(&sec(&["Unidentified (Tier 4)"]), &mut s);

    assert_eq!(got, ParseOutcome::SectionParsed);
    assert!(s.item.is_unidentified);
    assert_eq!(s.item.unidentified_tier, Some(4));
}

#[test]
fn the_three_sections_the_reference_says_are_not_unidentified() {
    // The reference's exact three. Consuming any of them would eat the item
    // level, an implicit or the whole name plate.
    let cases: [&[&str]; 3] = [
        &["Item Level: 54"],
        &[
            "{ Implicit Modifier — Elemental, Cold, Resistance }",
            "+23(20-30)% to Cold Resistance",
        ],
        &["Item Class: Rings", "Rarity: Magic", "Sapphire Ring"],
    ];

    for lines in cases {
        let mut s = state();

        let got = parse_unidentified(&sec(lines), &mut s);

        assert_eq!(got, ParseOutcome::SectionSkipped, "{lines:?}");
        assert!(!s.item.is_unidentified, "{lines:?}");
        assert_eq!(s.item.unidentified_tier, None, "{lines:?}");
    }
}

// ---------------------------------------------------------------------------
// "parseTrials", Djinn Barya
// ---------------------------------------------------------------------------

#[test]
fn every_djinn_barya_the_reference_lists_is_read() {
    for (lines, area, count) in [
        (["Area Level: 25", "Number of Trials: 1"], 25, 1),
        (["Area Level: 58", "Number of Trials: 2"], 58, 2),
        (["Area Level: 70", "Number of Trials: 3"], 70, 3),
        (["Area Level: 80", "Number of Trials: 4"], 80, 4),
    ] {
        let mut s = named("Djinn Barya");

        let got = parse_trials(&sec(&lines), &mut s);

        assert_eq!(got, ParseOutcome::SectionParsed, "{lines:?}");
        assert_eq!(s.item.area_level, Some(area), "{lines:?}");
        assert_eq!(
            s.item.trials.expect("trials").number_of_trials,
            Some(count),
            "{lines:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// "parseTrials", Inscribed Ultimatum
// ---------------------------------------------------------------------------

#[test]
fn an_ultimatum_without_a_hint_reports_none() {
    for (lines, area, count) in [
        (["Area Level: 27", "Number of Trials: 1"], 27, 1),
        (["Area Level: 56", "Number of Trials: 2"], 56, 2),
        (["Area Level: 69", "Number of Trials: 3"], 69, 3),
    ] {
        let mut s = named("Inscribed Ultimatum");

        let got = parse_trials(&sec(&lines), &mut s);

        assert_eq!(got, ParseOutcome::SectionParsed, "{lines:?}");
        assert_eq!(s.item.area_level, Some(area), "{lines:?}");

        let trials = s.item.trials.expect("trials");
        assert_eq!(trials.number_of_trials, Some(count), "{lines:?}");
        assert_eq!(trials.ultimatum_hint, None, "{lines:?}");
    }
}

#[test]
fn every_ultimatum_hint_the_reference_lists_is_read() {
    // The hint is the reward tier and is most of what the item is worth.
    for (lines, area, hint) in [
        (
            ["Area Level: 75", "Number of Trials: 4", "Victorious"],
            75,
            UltimatumHint::Victorious,
        ),
        (
            ["Area Level: 76", "Number of Trials: 4", "Cowardly"],
            76,
            UltimatumHint::Cowardly,
        ),
        (
            ["Area Level: 79", "Number of Trials: 4", "Deadly"],
            79,
            UltimatumHint::Deadly,
        ),
    ] {
        let mut s = named("Inscribed Ultimatum");

        let got = parse_trials(&sec(&lines), &mut s);

        assert_eq!(got, ParseOutcome::SectionParsed, "{lines:?}");
        assert_eq!(s.item.area_level, Some(area), "{lines:?}");

        let trials = s.item.trials.expect("trials");
        assert_eq!(trials.number_of_trials, Some(4), "{lines:?}");
        assert_eq!(trials.ultimatum_hint, Some(hint), "{lines:?}");
    }
}

// ---------------------------------------------------------------------------
// "should skip non trial items"
// ---------------------------------------------------------------------------

#[test]
fn every_non_trial_item_the_reference_lists_skips_the_stage() {
    // ParserSkipped and not SectionSkipped. A trial stage that only skipped
    // the section would keep trying the same lines against every other
    // section on the item.
    for (name, lines) in [
        (
            "Divine Crown",
            vec!["Area Level: 27", "Number of Trials: 1"],
        ),
        (
            "Withered Wand",
            vec!["Area Level: 56", "Number of Trials: 2"],
        ),
        (
            "Uncut Support Gem (Level 5)",
            vec!["Area Level: 69", "Number of Trials: 3"],
        ),
        (
            "Ornate Belt",
            vec!["Area Level: 75", "Number of Trials: 4", "Victorious"],
        ),
        (
            "Rider Bow",
            vec!["Area Level: 76", "Number of Trials: 4", "Cowardly"],
        ),
        ("", vec!["Area Level: 79", "Number of Trials: 4", "Deadly"]),
    ] {
        let mut s = named(name);

        let got = parse_trials(&sec(&lines), &mut s);

        assert_eq!(got, ParseOutcome::ParserSkipped, "{name}");
        assert_eq!(s.item.area_level, None, "{name}");
        assert_eq!(s.item.trials, None, "{name}");
    }
}
