//! Rune and soul core sockets.
//!
//! Ported from `parseAugmentSockets`, `applyAugmentSockets`, `getMaxSockets`
//! and `isArmourOrWeaponOrCaster` in `renderer/src/parser/Parser.ts`.
//!
//! # Why empty sockets matter
//!
//! A PoE2 base with an empty socket is worth more than the same base with a
//! bad rune already in it, because a rune cannot be removed. The trade site
//! filters on the socket count, so the parser has to work out how many the
//! base has and how many are filled.
//!
//! The game does not print an empty socket count. It prints the sockets that
//! exist, so an item with no socket line at all still has its base allowance
//! available.

use crate::controller::parse::{ParseError, ParseOutcome, ParserState};
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::{AugmentSockets, ItemRarity};
use crate::types::modifier::ModifierType;

/// Which broad group a category belongs to for socket purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketGroup {
    Armour,
    Weapon,
    Caster,
}

/// The broad group a category belongs to.
///
/// Ported from `isArmourOrWeaponOrCaster`. Returns None for anything that
/// cannot take a rune at all.
pub fn socket_group(category: Option<ItemCategory>) -> Option<SocketGroup> {
    use ItemCategory::*;

    let category = category?;

    let group = match category {
        BodyArmour | Boots | Gloves | Helmet | Shield | Focus | Buckler => SocketGroup::Armour,

        OneHandedAxe | OneHandedMace | OneHandedSword | TwoHandedAxe | TwoHandedMace
        | TwoHandedSword | Bow | Crossbow | Claw | Dagger | Spear | Flail | Warstaff | Talisman => {
            SocketGroup::Weapon
        }

        Wand | Sceptre | Staff => SocketGroup::Caster,

        _ => return None,
    };

    Some(group)
}

/// How many sockets a base can have.
///
/// Ported from `getMaxSockets`. Three uniques break the pattern and are named
/// explicitly, which is how the reference does it too.
pub fn max_sockets(category: Option<ItemCategory>, reference_name: &str) -> u32 {
    // These three carry a socket count their category does not imply.
    match reference_name {
        "Darkness Enthroned" => return 2,
        "Grasping Ring" | "Corona Amulet" => return 1,
        _ => {}
    }

    use ItemCategory::*;

    let Some(category) = category else {
        return 0;
    };

    match category {
        BodyArmour | TwoHandedAxe | TwoHandedMace | TwoHandedSword | Crossbow | Bow | Warstaff
        | Staff | Talisman => 2,

        Helmet | Shield | Gloves | Boots | OneHandedAxe | OneHandedMace | OneHandedSword | Claw
        | Dagger | Focus | Spear | Flail | Wand | Buckler | Sceptre => 1,

        _ => 0,
    }
}

/// Read the `Sockets: S S` line on a rune bearing base.
///
/// A base that takes runes and prints no socket line still reports its full
/// allowance as empty, because the sockets exist whether or not the game
/// mentions them.
pub fn parse_augment_sockets(section: &[String], state: &mut ParserState<'_>) -> ParseOutcome {
    let category_max = max_sockets(state.item.category, &state.item.info.reference_name);

    let takes_runes = category_max > 0
        && (socket_group(state.item.category).is_some()
            || state.item.info.reference_name == "Darkness Enthroned");

    if !takes_runes {
        return ParseOutcome::ParserSkipped;
    }

    let Some(first) = section.first() else {
        return ParseOutcome::SectionSkipped;
    };

    if let Some(rest) = first.strip_prefix(cs::SOCKETS) {
        // The game prints one S per socket that exists on the item.
        let current = rest.trim_end().matches('S').count() as u32;

        state.item.augment_sockets = Some(AugmentSockets {
            empty: 0,
            current,
            normal: category_max,
        });

        return ParseOutcome::SectionParsed;
    }

    // No socket line. A craftable base still has its full allowance, and a
    // corrupted one cannot gain sockets so it has none to offer.
    if state.item.is_modifiable() {
        state.item.augment_sockets = Some(AugmentSockets {
            empty: category_max,
            current: 0,
            normal: category_max,
        });
    }

    ParseOutcome::SectionSkipped
}

/// Work out how many sockets are still empty.
///
/// Ported from `applyAugmentSockets`. A socket holding a rune shows up as an
/// augment modifier, so the presence of any augment modifier means the sockets
/// are filled.
///
/// The reference calls its own version of this a hack, because rune tiers make
/// an exact count impossible from the item text alone. Treating any augment
/// modifier as "sockets are used" is the same approximation, and it errs
/// toward reporting fewer empty sockets, which understates the item rather
/// than overstating it.
pub fn apply_augment_sockets(state: &mut ParserState<'_>) -> Result<(), ParseError> {
    let Some(sockets) = state.item.augment_sockets else {
        return Ok(());
    };

    let has_runes = state
        .item
        .modifiers
        .iter()
        .any(|m| m.info.kind == Some(ModifierType::Augment));

    let empty = if has_runes {
        0
    } else {
        sockets.normal.max(sockets.current)
    };

    state.item.augment_sockets = Some(AugmentSockets { empty, ..sockets });

    Ok(())
}

/// Fold added elemental damage modifiers into the weapon's element totals.
///
/// Ported from `applyElementalAdded`.
///
/// The game prints the total elemental damage on the weapon line but does not
/// break it down. The breakdown comes from the modifiers, and a filter on fire
/// damage specifically needs it.
///
/// Skipped on a unique, whose numbers come from the unique and not the base.
pub fn apply_elemental_added(state: &mut ParserState<'_>) -> Result<(), ParseError> {
    if state.item.weapon.elemental.is_none() || state.item.rarity == Some(ItemRarity::Unique) {
        return Ok(());
    }

    let mut fire = 0.0;
    let mut cold = 0.0;
    let mut lightning = 0.0;

    for modifier in &state.item.modifiers {
        for stat in &modifier.stats {
            let Some(roll) = stat.roll else {
                continue;
            };

            match stat.reference.as_str() {
                "Adds # to # Fire Damage" => fire += roll.value,
                "Adds # to # Cold Damage" => cold += roll.value,
                "Adds # to # Lightning Damage" => lightning += roll.value,
                _ => {}
            }
        }
    }

    add_into(&mut state.item.weapon.fire, fire);
    add_into(&mut state.item.weapon.cold, cold);
    add_into(&mut state.item.weapon.lightning, lightning);

    Ok(())
}

/// Add into an optional total, leaving it absent when there is nothing to add.
fn add_into(slot: &mut Option<f64>, value: f64) {
    if value == 0.0 {
        return;
    }

    *slot = Some(slot.unwrap_or(0.0) + value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::parse::shared::modifiers::ParsedModifier;
    use crate::types::item::{BaseInfo, WeaponStats};
    use crate::types::modifier::ModifierInfo;
    use crate::types::stat::{ParsedStat, StatRoll};

    fn sec(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn state_for(category: ItemCategory, name: &str) -> ParserState<'static> {
        let mut s = ParserState::default();
        s.item.category = Some(category);
        s.item.info = BaseInfo {
            reference_name: name.into(),
            craftable: true,
            ..BaseInfo::default()
        };

        s
    }

    // -----------------------------------------------------------------
    // Socket allowance
    // -----------------------------------------------------------------

    #[test]
    fn a_two_handed_base_takes_two_sockets() {
        for c in [
            ItemCategory::BodyArmour,
            ItemCategory::TwoHandedAxe,
            ItemCategory::Bow,
            ItemCategory::Crossbow,
            ItemCategory::Staff,
            ItemCategory::Talisman,
        ] {
            assert_eq!(max_sockets(Some(c), ""), 2, "{c:?}");
        }
    }

    #[test]
    fn a_one_handed_base_takes_one_socket() {
        for c in [
            ItemCategory::Helmet,
            ItemCategory::Gloves,
            ItemCategory::Boots,
            ItemCategory::Shield,
            ItemCategory::Wand,
            ItemCategory::Sceptre,
        ] {
            assert_eq!(max_sockets(Some(c), ""), 1, "{c:?}");
        }
    }

    #[test]
    fn a_base_that_cannot_take_runes_reports_none() {
        for c in [
            ItemCategory::Ring,
            ItemCategory::Amulet,
            ItemCategory::Currency,
            ItemCategory::Map,
        ] {
            assert_eq!(max_sockets(Some(c), ""), 0, "{c:?}");
        }

        assert_eq!(max_sockets(None, ""), 0);
    }

    #[test]
    fn the_three_uniques_that_break_the_pattern_are_named() {
        // Each carries a socket count its category does not imply. Missing one
        // would report zero sockets on an item whose sockets are the reason
        // people buy it.
        assert_eq!(
            max_sockets(Some(ItemCategory::Belt), "Darkness Enthroned"),
            2
        );
        assert_eq!(max_sockets(Some(ItemCategory::Ring), "Grasping Ring"), 1);
        assert_eq!(max_sockets(Some(ItemCategory::Amulet), "Corona Amulet"), 1);
    }

    #[test]
    fn a_named_unique_beats_its_category() {
        // Darkness Enthroned is a belt, and belts take no runes.
        assert_eq!(max_sockets(Some(ItemCategory::Belt), ""), 0);
        assert_eq!(
            max_sockets(Some(ItemCategory::Belt), "Darkness Enthroned"),
            2
        );
    }

    #[test]
    fn each_category_lands_in_the_right_socket_group() {
        assert_eq!(
            socket_group(Some(ItemCategory::BodyArmour)),
            Some(SocketGroup::Armour)
        );
        assert_eq!(
            socket_group(Some(ItemCategory::Bow)),
            Some(SocketGroup::Weapon)
        );
        assert_eq!(
            socket_group(Some(ItemCategory::Wand)),
            Some(SocketGroup::Caster)
        );
        assert_eq!(socket_group(Some(ItemCategory::Ring)), None);
        assert_eq!(socket_group(None), None);
    }

    // -----------------------------------------------------------------
    // Reading the socket line
    // -----------------------------------------------------------------

    #[test]
    fn a_socket_line_reports_how_many_are_filled() {
        let mut s = state_for(ItemCategory::BodyArmour, "Vaal Regalia");

        let out = parse_augment_sockets(&sec(&["Sockets: S S"]), &mut s);

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(
            s.item.augment_sockets,
            Some(AugmentSockets {
                empty: 0,
                current: 2,
                normal: 2
            })
        );
    }

    #[test]
    fn one_filled_socket_on_a_two_socket_base_is_read() {
        let mut s = state_for(ItemCategory::BodyArmour, "Vaal Regalia");

        parse_augment_sockets(&sec(&["Sockets: S"]), &mut s);

        let sockets = s.item.augment_sockets.unwrap();
        assert_eq!(sockets.current, 1);
        assert_eq!(sockets.normal, 2);
    }

    #[test]
    fn a_craftable_base_with_no_socket_line_offers_its_full_allowance() {
        // The sockets exist whether or not the game mentions them, and an item
        // with empty sockets is worth more than one with a bad rune in it.
        let mut s = state_for(ItemCategory::Helmet, "Iron Hat");

        let out = parse_augment_sockets(&sec(&["Item Level: 80"]), &mut s);

        assert_eq!(out, ParseOutcome::SectionSkipped);
        assert_eq!(
            s.item.augment_sockets,
            Some(AugmentSockets {
                empty: 1,
                current: 0,
                normal: 1
            })
        );
    }

    #[test]
    fn a_corrupted_base_with_no_socket_line_offers_nothing() {
        // It cannot gain sockets, so reporting an allowance would overstate it.
        let mut s = state_for(ItemCategory::Helmet, "Iron Hat");
        s.item.is_corrupted = true;

        parse_augment_sockets(&sec(&["Item Level: 80"]), &mut s);

        assert_eq!(s.item.augment_sockets, None);
    }

    #[test]
    fn a_base_that_cannot_take_runes_skips_the_stage() {
        let mut s = state_for(ItemCategory::Ring, "Sapphire Ring");

        assert_eq!(
            parse_augment_sockets(&sec(&["Sockets: S"]), &mut s),
            ParseOutcome::ParserSkipped
        );
        assert_eq!(s.item.augment_sockets, None);
    }

    #[test]
    fn darkness_enthroned_takes_runes_despite_being_a_belt() {
        let mut s = state_for(ItemCategory::Belt, "Darkness Enthroned");

        let out = parse_augment_sockets(&sec(&["Sockets: S S"]), &mut s);

        assert_eq!(out, ParseOutcome::SectionParsed);
        assert_eq!(s.item.augment_sockets.unwrap().normal, 2);
    }

    // -----------------------------------------------------------------
    // Working out what is empty
    // -----------------------------------------------------------------

    fn with_rune() -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(ModifierType::Augment),
                ..ModifierInfo::default()
            },
            stats: vec![ParsedStat {
                reference: "# to maximum Life".into(),
                matched: "# to maximum Life".into(),
                roll: Some(StatRoll {
                    value: 20.0,
                    ..StatRoll::default()
                }),
            }],
        }
    }

    #[test]
    fn a_base_with_no_runes_reports_every_socket_empty() {
        let mut s = state_for(ItemCategory::BodyArmour, "Vaal Regalia");
        s.item.augment_sockets = Some(AugmentSockets {
            empty: 0,
            current: 0,
            normal: 2,
        });

        apply_augment_sockets(&mut s).unwrap();

        assert_eq!(s.item.augment_sockets.unwrap().empty, 2);
    }

    #[test]
    fn a_base_carrying_a_rune_reports_none_empty() {
        // Rune tiers make an exact count impossible from the item text, so any
        // rune means the sockets are used. That understates the item rather
        // than overstating it.
        let mut s = state_for(ItemCategory::BodyArmour, "Vaal Regalia");
        s.item.augment_sockets = Some(AugmentSockets {
            empty: 0,
            current: 2,
            normal: 2,
        });
        s.item.modifiers.push(with_rune());

        apply_augment_sockets(&mut s).unwrap();

        assert_eq!(s.item.augment_sockets.unwrap().empty, 0);
    }

    #[test]
    fn a_non_rune_modifier_does_not_fill_a_socket() {
        let mut s = state_for(ItemCategory::BodyArmour, "Vaal Regalia");
        s.item.augment_sockets = Some(AugmentSockets {
            empty: 0,
            current: 0,
            normal: 2,
        });
        s.item.modifiers.push(ParsedModifier {
            info: ModifierInfo {
                kind: Some(ModifierType::Explicit),
                ..ModifierInfo::default()
            },
            stats: Vec::new(),
        });

        apply_augment_sockets(&mut s).unwrap();

        assert_eq!(s.item.augment_sockets.unwrap().empty, 2);
    }

    #[test]
    fn an_item_with_no_sockets_at_all_is_left_alone() {
        let mut s = state_for(ItemCategory::Ring, "Sapphire Ring");

        apply_augment_sockets(&mut s).unwrap();

        assert_eq!(s.item.augment_sockets, None);
    }

    // -----------------------------------------------------------------
    // Elemental breakdown
    // -----------------------------------------------------------------

    fn added(reference: &str, value: f64) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(ModifierType::Explicit),
                ..ModifierInfo::default()
            },
            stats: vec![ParsedStat {
                reference: reference.into(),
                matched: reference.into(),
                roll: Some(StatRoll {
                    value,
                    ..StatRoll::default()
                }),
            }],
        }
    }

    #[test]
    fn added_elemental_modifiers_fill_in_the_breakdown() {
        // The weapon line prints the total but not the breakdown, and a filter
        // on fire damage specifically needs it.
        let mut s = state_for(ItemCategory::Bow, "Spine Bow");
        s.item.weapon = WeaponStats {
            elemental: Some(100.0),
            ..WeaponStats::default()
        };
        s.item
            .modifiers
            .push(added("Adds # to # Fire Damage", 30.0));
        s.item
            .modifiers
            .push(added("Adds # to # Cold Damage", 45.0));
        s.item
            .modifiers
            .push(added("Adds # to # Lightning Damage", 25.0));

        apply_elemental_added(&mut s).unwrap();

        assert_eq!(s.item.weapon.fire, Some(30.0));
        assert_eq!(s.item.weapon.cold, Some(45.0));
        assert_eq!(s.item.weapon.lightning, Some(25.0));
    }

    #[test]
    fn two_modifiers_of_the_same_element_add_together() {
        let mut s = state_for(ItemCategory::Bow, "Spine Bow");
        s.item.weapon = WeaponStats {
            elemental: Some(100.0),
            ..WeaponStats::default()
        };
        s.item
            .modifiers
            .push(added("Adds # to # Fire Damage", 30.0));
        s.item
            .modifiers
            .push(added("Adds # to # Fire Damage", 20.0));

        apply_elemental_added(&mut s).unwrap();

        assert_eq!(s.item.weapon.fire, Some(50.0));
    }

    #[test]
    fn a_breakdown_the_weapon_line_already_gave_is_added_to() {
        let mut s = state_for(ItemCategory::Bow, "Spine Bow");
        s.item.weapon = WeaponStats {
            elemental: Some(100.0),
            fire: Some(10.0),
            ..WeaponStats::default()
        };
        s.item
            .modifiers
            .push(added("Adds # to # Fire Damage", 30.0));

        apply_elemental_added(&mut s).unwrap();

        assert_eq!(s.item.weapon.fire, Some(40.0));
    }

    #[test]
    fn a_weapon_with_no_elemental_total_is_left_alone() {
        let mut s = state_for(ItemCategory::Bow, "Spine Bow");
        s.item
            .modifiers
            .push(added("Adds # to # Fire Damage", 30.0));

        apply_elemental_added(&mut s).unwrap();

        assert_eq!(s.item.weapon.fire, None);
    }

    #[test]
    fn a_unique_keeps_no_breakdown() {
        // Its numbers come from the unique and not from the base.
        let mut s = state_for(ItemCategory::Bow, "Spine Bow");
        s.item.rarity = Some(ItemRarity::Unique);
        s.item.weapon = WeaponStats {
            elemental: Some(100.0),
            ..WeaponStats::default()
        };
        s.item
            .modifiers
            .push(added("Adds # to # Fire Damage", 30.0));

        apply_elemental_added(&mut s).unwrap();

        assert_eq!(s.item.weapon.fire, None);
    }

    #[test]
    fn an_unrelated_modifier_changes_nothing() {
        let mut s = state_for(ItemCategory::Bow, "Spine Bow");
        s.item.weapon = WeaponStats {
            elemental: Some(100.0),
            ..WeaponStats::default()
        };
        s.item.modifiers.push(added("# to maximum Life", 50.0));

        apply_elemental_added(&mut s).unwrap();

        assert_eq!(s.item.weapon.fire, None);
        assert_eq!(s.item.weapon.cold, None);
    }
}
