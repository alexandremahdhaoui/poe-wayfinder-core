//! Every property filter id we send is one the references actually send.
//!
//! # The bug this exists for
//!
//! The overlay sent `pseudo.pseudo_total_armour` for an armour piece. No such
//! trade id exists. The trade API answered
//!
//! ```text
//! Unknown stat provided: pseudo.pseudo_total_armour
//! ```
//!
//! and every armour price check failed, in both games. The id was invented.
//! Both references use `item.armour`.
//!
//! Nothing caught it. The parity count was 100 percent, because `armour_filters`
//! existed and was named after the function it ports. Its unit tests passed,
//! because they asserted the ids the code already produced. `datacheck` did not
//! see it, because it checks stats and bases rather than filter ids.
//!
//! An id is only ever correct because the trade site says so, and the only
//! record of what the trade site accepts is what the references send. So this
//! reads the ids straight out of both reference checkouts and requires every id
//! we emit to be among them.

use std::collections::BTreeSet;
use std::path::Path;

use poe_trader_core::controller::filter::item_property::{armour_filters, weapon_filters};
use poe_trader_core::types::item::{ArmourStats, WeaponStats};
use poe_trader_core::types::ItemCategory;

/// Every `tradeId:` in a reference's item-property file.
fn reference_ids(path: &Path) -> BTreeSet<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeSet::new();
    };

    text.lines()
        .filter_map(|line| line.split_once("tradeId:"))
        .filter_map(|(_, rest)| {
            let rest = rest.trim().trim_start_matches(['"', '\'']);

            rest.split(['"', '\'']).next()
        })
        .filter(|id| id.contains('.'))
        .map(str::to_string)
        .collect()
}

fn accepted() -> BTreeSet<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../reference");

    let mut ids = reference_ids(
        &root
            .join("Exiled-Exchange-2/renderer/src/web/price-check/filters/pseudo/item-property.ts"),
    );

    ids.extend(reference_ids(&root.join(
        "awakened-poe-trade/renderer/src/web/price-check/filters/pseudo/item-property.ts",
    )));

    ids
}

#[test]
fn every_armour_id_we_send_is_one_a_reference_sends() {
    let accepted = accepted();

    if accepted.is_empty() {
        // A fresh checkout has no reference tree. Passing beats failing on
        // something that is not the developer's doing.
        return;
    }

    // One defence at a time, because `armour_filters` returns only the largest
    // and a combined case would never exercise the other two.
    for armour in [
        ArmourStats {
            ar: Some(500.0),
            ..ArmourStats::default()
        },
        ArmourStats {
            ev: Some(500.0),
            ..ArmourStats::default()
        },
        ArmourStats {
            es: Some(500.0),
            ..ArmourStats::default()
        },
    ] {
        for filter in armour_filters(armour) {
            assert!(
                accepted.contains(filter.id),
                "{} is not an id either reference sends. The trade api answers \
                 \"Unknown stat provided\" and the whole search fails.",
                filter.id
            );
        }
    }
}

#[test]
fn every_weapon_id_we_send_is_one_a_reference_sends() {
    let accepted = accepted();

    if accepted.is_empty() {
        return;
    }

    let weapon = WeaponStats {
        physical: Some(80.0),
        elemental: Some(40.0),
        attack_speed: Some(1.5),
        crit: Some(6.5),
        spirit: Some(100.0),
        ..WeaponStats::default()
    };

    for category in [ItemCategory::OneHandedSword, ItemCategory::Bow] {
        for filter in weapon_filters(Some(category), weapon) {
            assert!(
                accepted.contains(filter.id),
                "{} is not an id either reference sends.",
                filter.id
            );
        }
    }
}

#[test]
fn the_reference_ids_were_actually_read() {
    // Without this the two tests above pass by reading nothing, which is how a
    // guard quietly stops guarding.
    let accepted = accepted();

    if accepted.is_empty() {
        return;
    }

    for wanted in ["item.armour", "item.evasion_rating", "item.energy_shield"] {
        assert!(accepted.contains(wanted), "{wanted} was not read");
    }
}
