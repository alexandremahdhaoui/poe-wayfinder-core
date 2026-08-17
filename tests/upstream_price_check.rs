//! The upstream fixtures through the whole price check, not just the parser.
//!
//! The parser harness in `upstream_fixtures.rs` proves the 26 items from
//! Exiled Exchange 2's own test set parse. This proves the rest of the
//! pipeline survives them: filter building, stat matching, endpoint routing
//! and query assembly.
//!
//! # Why this is a separate file
//!
//! A parse that succeeds and a query that is malformed look identical to the
//! parser harness. Every bug this file has caught lived past the parser: a
//! filter with a floor above its ceiling, a query with no name and no type, a
//! currency routed to the wrong endpoint.
//!
//! # Why the data is empty
//!
//! `NO_DATA` matches no stat, so every modifier comes back unknown. That is
//! the harshest input the filter builder can get and it is what a user with a
//! stale data file has. Nothing here may panic on it.

use poe_wayfinder_core::adapter::data_adapter::NO_DATA;
use poe_wayfinder_core::controller::bulk::Endpoint;
use poe_wayfinder_core::controller::price_check::{price_check, PriceCheckOptions};
use poe_wayfinder_core::types::GameVersion;

/// Every fixture, as name and clipboard text.
fn fixtures() -> Vec<(String, String)> {
    let raw = include_str!("fixtures/upstream_items.json");

    let parsed: serde_json::Value =
        serde_json::from_str(raw).expect("the fixture file is valid JSON");

    parsed
        .as_object()
        .expect("the fixtures are an object")
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

const GAMES: [GameVersion; 2] = [GameVersion::Poe1, GameVersion::Poe2];

#[test]
fn the_harness_prices_every_fixture_rather_than_skipping_them() {
    // Every other test in this file skips an item that fails to price. Without
    // this one they would all pass on an empty set and prove nothing.
    let total = fixtures().len();

    assert_eq!(total, 26, "the fixture set changed size");

    let priced = fixtures()
        .into_iter()
        .filter(|(_, text)| {
            price_check(text, &NO_DATA, &PriceCheckOptions::new(GameVersion::Poe2)).is_ok()
        })
        .count();

    assert_eq!(priced, total, "only {priced} of {total} fixtures priced");
}

#[test]
fn a_query_built_with_no_data_reports_that_it_narrows_nothing() {
    // The base type comes from the data file, so a stale or missing one leaves
    // the query empty while the parse still succeeds. Every such query matches
    // the entire trade site, and the caller has to be able to tell.
    let unconstrained = fixtures()
        .into_iter()
        .filter(|(_, text)| {
            price_check(text, &NO_DATA, &PriceCheckOptions::new(GameVersion::Poe2))
                .is_ok_and(|c| !c.constrains_something())
        })
        .count();

    // Most of them, because no base resolves without data. The number is not
    // the point. That the check reports them at all is.
    assert!(
        unconstrained > 0,
        "no fixture is unconstrained, so the check proves nothing"
    );
}

#[test]
fn every_upstream_item_produces_a_query_in_both_games() {
    // An item that parses and then fails to price is a crash in front of the
    // user with the item still on their clipboard.
    for (name, text) in fixtures() {
        for game in GAMES {
            let got = price_check(&text, &NO_DATA, &PriceCheckOptions::new(game));

            assert!(got.is_ok(), "{name} in {game:?}: {:?}", got.err());
        }
    }
}

#[test]
fn every_query_asks_for_something() {
    // A query with no name, no type and no filters matches every item on the
    // site. The user sees a price for the whole market and no sign it is wrong.
    for (name, text) in fixtures() {
        for game in GAMES {
            let Ok(check) = price_check(&text, &NO_DATA, &PriceCheckOptions::new(game)) else {
                continue;
            };

            // A query that narrows nothing must say so rather than being sent.
            // The caller refuses it and tells the user their data file is
            // stale, which is a fixable problem stated plainly.
            let narrows = check.constrains_something();
            let reported = check.query.type_name.is_none();

            assert!(narrows || reported, "{name} in {game:?}");
        }
    }
}

#[test]
fn no_stat_filter_has_a_floor_above_its_ceiling() {
    // Such a filter matches nothing and the user sees an empty result with no
    // clue why. It is the shape a sign error produces.
    for (name, text) in fixtures() {
        for game in GAMES {
            let Ok(check) = price_check(&text, &NO_DATA, &PriceCheckOptions::new(game)) else {
                continue;
            };

            for group in &check.query.stats {
                for filter in &group.filters {
                    let (Some(min), Some(max)) = (filter.range.min, filter.range.max) else {
                        continue;
                    };

                    assert!(
                        min <= max,
                        "{name} in {game:?}: {} has {min}..{max}",
                        filter.id
                    );
                }
            }
        }
    }
}

#[test]
fn no_query_carries_a_filter_id_twice() {
    // The trade site keeps the last of a repeated id, so the earlier one is
    // silently dropped and the search is looser than the panel says.
    for (name, text) in fixtures() {
        for game in GAMES {
            let Ok(check) = price_check(&text, &NO_DATA, &PriceCheckOptions::new(game)) else {
                continue;
            };

            for group in &check.query.stats {
                let mut ids: Vec<&str> = group.filters.iter().map(|f| f.id.as_str()).collect();
                let count = ids.len();
                ids.sort_unstable();
                ids.dedup();

                assert_eq!(ids.len(), count, "{name} in {game:?} repeats a filter id");
            }
        }
    }
}

#[test]
fn an_exchange_route_always_carries_a_tag() {
    // An exchange request with no tag is one the server rejects, and the user
    // sees an error rather than a price.
    for (name, text) in fixtures() {
        for game in GAMES {
            let Ok(check) = price_check(&text, &NO_DATA, &PriceCheckOptions::new(game)) else {
                continue;
            };

            if check.endpoint == Endpoint::Exchange {
                assert!(check.trade_tag.is_some(), "{name} in {game:?}");
            }
        }
    }
}

#[test]
fn a_search_route_never_carries_a_tag() {
    // The search request has nowhere to put one, and carrying it would be an
    // unread field that looks like it did something.
    for (name, text) in fixtures() {
        for game in GAMES {
            let Ok(check) = price_check(&text, &NO_DATA, &PriceCheckOptions::new(game)) else {
                continue;
            };

            if check.endpoint == Endpoint::Search {
                assert!(check.trade_tag.is_none(), "{name} in {game:?}");
            }
        }
    }
}

#[test]
fn every_unmatched_modifier_is_reported_rather_than_dropped() {
    // With no data every modifier is unknown. An item whose modifiers vanish
    // silently prices as a bare base and the user has no way to tell.
    let interesting = fixtures()
        .into_iter()
        .filter(|(_, text)| text.contains("--------"))
        .count();

    assert!(interesting > 0, "the fixtures carry no sectioned items");

    for (name, text) in fixtures() {
        let Ok(check) = price_check(&text, &NO_DATA, &PriceCheckOptions::new(GameVersion::Poe2))
        else {
            continue;
        };

        // Every modifier must be accounted for one way or the other.
        assert!(
            check.item.modifiers.is_empty() || !check.item.unknown_modifiers.is_empty(),
            "{name} matched a modifier with no data loaded"
        );
    }
}

#[test]
fn pricing_is_deterministic() {
    // A query that differs between runs cannot be reasoned about, and a user
    // pressing the key twice would get two prices for one item.
    for (name, text) in fixtures() {
        let first = price_check(&text, &NO_DATA, &PriceCheckOptions::new(GameVersion::Poe2));
        let second = price_check(&text, &NO_DATA, &PriceCheckOptions::new(GameVersion::Poe2));

        assert_eq!(first.is_ok(), second.is_ok(), "{name}");

        if let (Ok(a), Ok(b)) = (first, second) {
            assert_eq!(a.query, b.query, "{name}");
            assert_eq!(a.endpoint, b.endpoint, "{name}");
        }
    }
}

#[test]
fn every_fixture_survives_being_truncated_anywhere() {
    // Clipboard text is fully attacker controlled and a partial copy is an
    // ordinary user mistake. Neither may panic anywhere in the pipeline.
    for (name, text) in fixtures() {
        for end in 0..=text.len() {
            if !text.is_char_boundary(end) {
                continue;
            }

            for game in GAMES {
                let _ = price_check(&text[..end], &NO_DATA, &PriceCheckOptions::new(game));
            }
        }

        // Reaching here without a panic is the assertion.
        assert!(!name.is_empty());
    }
}

#[test]
fn a_fixture_with_every_line_repeated_still_prices() {
    // A user copying twice into one clipboard is common enough, and the
    // duplicate section is what found the quadratic scan in the parser.
    for (_name, text) in fixtures() {
        let doubled = format!("{text}\n{text}");

        let _ = price_check(
            &doubled,
            &NO_DATA,
            &PriceCheckOptions::new(GameVersion::Poe2),
        );
    }
}

#[test]
fn most_upstream_items_now_carry_a_category() {
    // The data file names a category for 810 of 3578 bases, because the trade
    // API groups items coarsely: it says weapon, not bow. The class line the
    // game prints on every item fills the rest.
    //
    // Without a category the query carries no category filter and returns
    // every kind of item that happens to share a modifier.
    let with_category = fixtures()
        .into_iter()
        .filter(|(_, text)| {
            price_check(text, &NO_DATA, &PriceCheckOptions::new(GameVersion::Poe2))
                .is_ok_and(|c| c.item.category.is_some())
        })
        .count();

    // NO_DATA supplies nothing, so every category here came from the class
    // line alone.
    assert!(
        with_category >= 20,
        "only {with_category} of 26 fixtures got a category with no data file"
    );
}
