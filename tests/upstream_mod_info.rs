//! Exiled Exchange 2's modifier heading expectations, run against ours.
//!
//! Ported from `renderer/specs/Parser/advanced-mod-desc.test.ts`, which runs
//! five assertions over every entry in `renderer/specs/Parser/mods.ts`: the
//! modifier's type, generation, name, tags and tier.
//!
//! The fixture is that file, harvested. Forty real headings across five
//! modifier types.
//!
//! # What a heading is and why it is fiddly
//!
//! With Advanced Item Description on, the game prints a line above each
//! modifier saying what it is:
//!
//! ```text
//! { Prefix Modifier "Crackling" (Tier: 7) — Damage, Elemental, Lightning, Attack }
//! ```
//!
//! Five separate facts in one line, with an em dash separator, quoted names
//! that contain spaces, and type words that prefix the generation word rather
//! than replacing it. `Desecrated Prefix Modifier` is a prefix that is also
//! desecrated, and reading it as a type called `Desecrated Prefix` loses the
//! generation.
//!
//! Every one of these is what the trade query filters on. A modifier read with
//! the wrong type goes to the wrong trade namespace and matches nothing.

use poe_wayfinder_core::controller::mod_desc::{parse_mod_info_line, parse_mod_type};
use poe_wayfinder_core::types::modifier::{Generation, ModifierType};

const REAL: &str = include_str!("fixtures/upstream_mod_info.json");

/// One heading and what the reference expects from it.
struct Case {
    lines: Vec<String>,
    kind: Option<ModifierType>,
    generation: Option<Generation>,
    name: Option<String>,
    tier: Option<u32>,
    tags: Vec<String>,
}

fn cases() -> Vec<Case> {
    let parsed: Vec<serde_json::Value> =
        serde_json::from_str(REAL).expect("the fixture is valid JSON");

    parsed
        .into_iter()
        .map(|case| Case {
            lines: case["rawText"]
                .as_str()
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect(),
            kind: case["type"].as_str().and_then(kind_named),
            generation: case["generation"].as_str().and_then(generation_named),
            name: case["name"].as_str().map(str::to_string),
            tier: case["tier"].as_u64().map(|t| t as u32),
            tags: case["tags"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

fn kind_named(name: &str) -> Option<ModifierType> {
    Some(match name {
        "Explicit" => ModifierType::Explicit,
        "Implicit" => ModifierType::Implicit,
        "Crafted" => ModifierType::Crafted,
        "Fractured" => ModifierType::Fractured,
        "Desecrated" => ModifierType::Desecrated,
        "Enchant" => ModifierType::Enchant,
        "Scourge" => ModifierType::Scourge,
        "Veiled" => ModifierType::Veiled,
        _ => return None,
    })
}

fn generation_named(name: &str) -> Option<Generation> {
    Some(match name {
        "prefix" => Generation::Prefix,
        "suffix" => Generation::Suffix,
        "corrupted" => Generation::Corrupted,
        "eldritch" => Generation::Eldritch,
        "mutated" => Generation::Mutated,
        _ => return None,
    })
}

/// Run a case the way the reference does: type the section, then read the
/// heading with that type as the default.
fn info_for(case: &Case) -> poe_wayfinder_core::types::modifier::ModifierInfo {
    let (kind, lines) = parse_mod_type(&case.lines);

    parse_mod_info_line(lines.first().map(String::as_str).unwrap_or_default(), kind)
}

#[test]
fn the_whole_fixture_is_present() {
    // Forty in the reference. A harvest that dropped most of them would make
    // every test below pass on what was left.
    assert_eq!(cases().len(), 40);
}

#[test]
fn every_case_carries_a_heading_line() {
    // A case whose text did not survive the harvest asserts nothing.
    for case in cases() {
        let first = case.lines.first().expect("a heading line");

        assert!(first.starts_with('{'), "not a heading: {first}");
    }
}

#[test]
fn every_modifier_type_matches_the_reference() {
    // The type decides which trade namespace the filter goes to. A crafted
    // modifier sent as explicit matches nothing at all.
    for case in cases() {
        let got = info_for(&case);

        assert_eq!(got.kind, case.kind, "{}", case.lines[0]);
    }
}

#[test]
fn every_generation_matches_the_reference() {
    // Prefix and suffix decide which half of an item's budget a modifier came
    // from, which is how a buyer judges whether it can be improved.
    for case in cases() {
        let got = info_for(&case);

        assert_eq!(got.generation, case.generation, "{}", case.lines[0]);
    }
}

#[test]
fn every_name_matches_the_reference() {
    // The affix name is quoted and contains spaces, so a split on whitespace
    // truncates it to its first word.
    for case in cases() {
        let got = info_for(&case);

        assert_eq!(got.name, case.name, "{}", case.lines[0]);
    }
}

#[test]
fn every_tier_matches_the_reference() {
    for case in cases() {
        let got = info_for(&case);

        assert_eq!(got.tier, case.tier, "{}", case.lines[0]);
    }
}

#[test]
fn every_tag_list_matches_the_reference() {
    // The tags are comma separated after an em dash, which is not the hyphen
    // it looks like in most fonts.
    for case in cases() {
        let got = info_for(&case);

        assert_eq!(got.tags, case.tags, "{}", case.lines[0]);
    }
}

// ---------------------------------------------------------------------------
// The cases the reference singles out
// ---------------------------------------------------------------------------

#[test]
fn a_type_word_in_front_of_the_generation_keeps_both() {
    // `Desecrated Prefix Modifier` is a prefix that is also desecrated.
    // Reading the whole thing as one type word loses the generation, and the
    // reference has three desecrated cases precisely to pin this.
    let desecrated: Vec<Case> = cases()
        .into_iter()
        .filter(|c| c.kind == Some(ModifierType::Desecrated))
        .collect();

    assert!(!desecrated.is_empty(), "the fixture has no desecrated case");

    for case in desecrated {
        let got = info_for(&case);

        assert_eq!(
            got.kind,
            Some(ModifierType::Desecrated),
            "{}",
            case.lines[0]
        );

        // Every one of them still knows which half of the item it came from.
        assert!(
            got.generation.is_some(),
            "{} lost its generation",
            case.lines[0]
        );
    }
}

#[test]
fn a_fractured_heading_is_still_a_prefix_or_a_suffix() {
    for case in cases()
        .into_iter()
        .filter(|c| c.kind == Some(ModifierType::Fractured))
    {
        let got = info_for(&case);

        assert_eq!(got.kind, Some(ModifierType::Fractured), "{}", case.lines[0]);
        assert_eq!(got.generation, case.generation, "{}", case.lines[0]);
    }
}

#[test]
fn an_implicit_has_no_generation() {
    // An implicit is not rolled from the prefix or suffix budget, so claiming
    // one would tell a buyer the item has room it does not have.
    for case in cases()
        .into_iter()
        .filter(|c| c.kind == Some(ModifierType::Implicit))
    {
        assert_eq!(case.generation, None, "the fixture disagrees");

        let got = info_for(&case);

        assert_eq!(got.generation, None, "{}", case.lines[0]);
    }
}

#[test]
fn no_case_comes_back_completely_empty() {
    // A heading that parsed into nothing at all would pass any single field
    // check that expects None, so the whole set is checked for signs of life.
    for case in cases() {
        let got = info_for(&case);

        assert!(
            got.kind.is_some() || got.generation.is_some() || got.name.is_some(),
            "{} produced nothing",
            case.lines[0]
        );
    }
}
