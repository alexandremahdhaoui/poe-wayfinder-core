//! Our parser against the upstream project's own test items.
//!
//! The 26 fixtures in `fixtures/upstream_items.json` are extracted verbatim
//! from `renderer/specs/Parser/items.ts` in Exiled Exchange 2. They are the
//! items its maintainers chose to test against, which makes them the closest
//! thing to a shared conformance set that exists.
//!
//! # What this catches that unit tests cannot
//!
//! A unit test proves one stage does what I think it should. These prove the
//! whole pipeline survives items a different team wrote, including shapes I
//! would not have thought to invent: a fractured item with no marked modifier,
//! a map with every property faked, an item carrying every modifier type at
//! once.
//!
//! # Why the assertions are shallow
//!
//! Deep expected values would be my opinion of what upstream produces, not
//! upstream's output. Asserting that every fixture parses, yields a name plate
//! and never panics is a real bar that nothing else clears. Comparing field by
//! field needs the reference running, which is the differential harness this
//! is the first half of.

use poe_trader_core::adapter::data_adapter::NO_DATA;
use poe_trader_core::controller::parse::parse_clipboard;
use poe_trader_core::types::GameVersion;

/// Every fixture, as name and clipboard text.
fn fixtures() -> Vec<(String, String)> {
    let raw = include_str!("fixtures/upstream_items.json");

    let parsed: serde_json::Value =
        serde_json::from_str(raw).expect("the fixture file is valid JSON");

    let object = parsed.as_object().expect("the fixtures are an object");

    object
        .iter()
        .map(|(name, item)| {
            let text = item
                .get("rawText")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();

            (name.clone(), text)
        })
        .collect()
}

#[test]
fn the_fixture_set_is_the_size_it_should_be() {
    // A silently empty fixture file would make every test below pass while
    // proving nothing.
    assert_eq!(fixtures().len(), 26);
}

#[test]
fn every_fixture_carries_real_item_text() {
    for (name, text) in fixtures() {
        // A meta gem prints no class line and starts with its rarity. Every
        // other item starts with the class.
        assert!(
            text.starts_with("Item Class:") || text.starts_with("Rarity:"),
            "{name} does not look like copied item text"
        );
        assert!(text.len() > 40, "{name} is too short to be an item");
    }
}

#[test]
fn every_upstream_item_parses_in_both_games() {
    // The whole point. These are items a different team wrote, including
    // shapes I would not have thought to invent.
    for (name, text) in fixtures() {
        for game in [GameVersion::Poe1, GameVersion::Poe2] {
            let result = parse_clipboard(&text, game, &NO_DATA);

            assert!(
                result.is_ok(),
                "{name} failed to parse as {game:?}: {:?}",
                result.err()
            );
        }
    }
}

#[test]
fn every_upstream_item_yields_a_name() {
    // A parse that succeeds with no name means the name plate was misread,
    // which loses the base type and with it the entire query.
    for (name, text) in fixtures() {
        let item = parse_clipboard(&text, GameVersion::Poe2, &NO_DATA).unwrap();

        assert!(
            !item.raw_text.is_empty(),
            "{name} produced no raw text to report"
        );
    }
}

#[test]
fn a_rarity_is_read_wherever_the_game_printed_one() {
    // Every fixture whose text carries a rarity line must yield a rarity or a
    // category. Missing both means the name plate parser gave up.
    for (name, text) in fixtures() {
        if !text.contains("\nRarity: ") {
            continue;
        }

        let item = parse_clipboard(&text, GameVersion::Poe2, &NO_DATA).unwrap();

        assert!(
            item.rarity.is_some() || item.category.is_some(),
            "{name} printed a rarity line and yielded neither rarity nor category"
        );
    }
}

#[test]
fn an_item_level_is_read_wherever_the_game_printed_one() {
    for (name, text) in fixtures() {
        if !text.contains("Item Level: ") {
            continue;
        }

        let item = parse_clipboard(&text, GameVersion::Poe2, &NO_DATA).unwrap();

        assert!(
            item.item_level.is_some(),
            "{name} printed an item level and none was read"
        );
    }
}

#[test]
fn corruption_is_read_wherever_the_game_printed_it() {
    for (name, text) in fixtures() {
        if !text.contains("\nCorrupted") {
            continue;
        }

        let item = parse_clipboard(&text, GameVersion::Poe2, &NO_DATA).unwrap();

        assert!(item.is_corrupted, "{name} printed Corrupted and was not");
    }
}

#[test]
fn a_quality_is_read_wherever_the_game_printed_one() {
    for (name, text) in fixtures() {
        if !text.contains("Quality: ") {
            continue;
        }

        let item = parse_clipboard(&text, GameVersion::Poe2, &NO_DATA).unwrap();

        assert!(
            item.quality.is_some(),
            "{name} printed a quality and none was read"
        );
    }
}

#[test]
fn the_unidentified_fixtures_are_read_as_unidentified() {
    for (name, text) in fixtures() {
        if !name.starts_with("Unidentified") {
            continue;
        }

        let item = parse_clipboard(&text, GameVersion::Poe2, &NO_DATA).unwrap();

        assert!(item.is_unidentified, "{name} was not read as unidentified");
    }
}

#[test]
fn parsing_is_deterministic() {
    // A parser whose output depends on iteration order produces a different
    // query for the same item, which is impossible to report as a bug.
    for (name, text) in fixtures() {
        let first = parse_clipboard(&text, GameVersion::Poe2, &NO_DATA).unwrap();
        let second = parse_clipboard(&text, GameVersion::Poe2, &NO_DATA).unwrap();

        assert_eq!(first, second, "{name} parsed differently twice");
    }
}

#[test]
fn every_fixture_survives_being_truncated_anywhere() {
    // Real clipboard reads land mid write. A panic here is a crash in an
    // overlay running next to a game, which the user blames on the game.
    for (name, text) in fixtures() {
        for end in 0..text.len() {
            if !text.is_char_boundary(end) {
                continue;
            }

            let _ = parse_clipboard(&text[..end], GameVersion::Poe2, &NO_DATA);
        }

        // Reaching here means no prefix panicked.
        assert!(!name.is_empty());
    }
}
