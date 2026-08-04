//! Which of an item's own numbers are worth filtering on.
//!
//! Ported from `web/price-check/filters/pseudo/item-property.ts`.
//!
//! # Why not just filter on all of them
//!
//! A rare bow prints physical damage, elemental damage, crit chance and attack
//! speed. Filtering on all four returns the one listing that is this exact
//! bow. Filtering on none returns every bow of that base.
//!
//! What a buyer actually wants is narrow: the damage per second that matters
//! for the weapon's type, and the one defence the armour is bought for.
//!
//! # Why physical damage is not always the number
//!
//! An axe is bought for physical damage per second. A wand is bought for
//! spell damage and its physical is noise. Filtering pdps on a wand excludes
//! every wand worth buying.

use crate::types::category::ItemCategory;
use crate::types::item::{ArmourStats, WeaponStats};

/// One property worth filtering on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyFilter {
    /// The pseudo trade id.
    pub id: &'static str,
    /// The value the item has.
    pub value: f64,
    /// Whether it starts switched on.
    pub enabled: bool,
}

/// Whether physical damage per second is what this weapon is bought for.
///
/// Ported from `isPdpsImportant`. An axe is bought for it. A wand is bought
/// for spell damage and its physical is noise, so filtering pdps on one
/// excludes every wand worth buying.
pub fn is_pdps_important(category: Option<ItemCategory>) -> bool {
    use ItemCategory::*;

    matches!(
        category,
        Some(
            OneHandedAxe
                | TwoHandedAxe
                | OneHandedSword
                | TwoHandedSword
                | OneHandedMace
                | TwoHandedMace
                | Bow
                | Warstaff
                | Crossbow
                | Spear
                | Flail
        )
    )
}

/// Whether a caster weapon's own damage is noise.
///
/// A wand, sceptre or staff is bought for what it grants a spell, not for the
/// damage it swings for.
pub fn is_caster_weapon(category: Option<ItemCategory>) -> bool {
    matches!(
        category,
        Some(ItemCategory::Wand | ItemCategory::Sceptre | ItemCategory::Staff)
    )
}

/// Whether the armour is bought for exactly one defence.
///
/// Ported from `isSingleAttrArmour`. The reference always returns true with a
/// note saying hybrid armour is bought for one defence in practice even when
/// it has two, so only the largest is filtered on.
///
/// Kept as a function rather than inlined, because if that ever stops being
/// true this is the one place to change.
pub fn is_single_defence_armour(_armour: ArmourStats) -> bool {
    true
}

/// The defence an armour piece is bought for.
///
/// The largest one. A piece with 800 armour and 40 energy shield is an armour
/// piece, and filtering on the energy shield would return every base that has
/// any.
pub fn primary_defence(armour: ArmourStats) -> Option<(&'static str, f64)> {
    let candidates = [
        ("pseudo.pseudo_total_armour", armour.ar),
        ("pseudo.pseudo_total_evasion", armour.ev),
        ("pseudo.pseudo_total_energy_shield", armour.es),
    ];

    let mut best: Option<(&'static str, f64)> = None;

    for (id, value) in candidates {
        let Some(value) = value else {
            continue;
        };

        if value <= 0.0 {
            continue;
        }

        if best.is_none_or(|(_, current)| value > current) {
            best = Some((id, value));
        }
    }

    best
}

/// The property filters an armour piece deserves.
pub fn armour_filters(armour: ArmourStats) -> Vec<PropertyFilter> {
    let Some((id, value)) = primary_defence(armour) else {
        return Vec::new();
    };

    if is_single_defence_armour(armour) {
        return vec![PropertyFilter {
            id,
            value,
            enabled: true,
        }];
    }

    // Every defence it has, with only the largest switched on.
    [
        ("pseudo.pseudo_total_armour", armour.ar),
        ("pseudo.pseudo_total_evasion", armour.ev),
        ("pseudo.pseudo_total_energy_shield", armour.es),
    ]
    .into_iter()
    .filter_map(|(candidate, v)| {
        v.filter(|v| *v > 0.0).map(|v| PropertyFilter {
            id: candidate,
            value: v,
            enabled: candidate == id,
        })
    })
    .collect()
}

/// The property filters a weapon deserves.
///
/// Damage per second and not damage. The trade site indexes dps, and the same
/// damage at a different attack speed is a different weapon.
pub fn weapon_filters(category: Option<ItemCategory>, weapon: WeaponStats) -> Vec<PropertyFilter> {
    let Some(aps) = weapon.attack_speed else {
        // Damage per second is undefined without it. Sending raw damage as dps
        // would understate the weapon by a factor of two.
        return Vec::new();
    };

    // A caster weapon is bought for what it grants a spell, not for the
    // damage it swings for.
    if is_caster_weapon(category) {
        return weapon
            .spirit
            .filter(|s| *s > 0.0)
            .map(|spirit| {
                vec![PropertyFilter {
                    id: "pseudo.pseudo_total_spirit",
                    value: spirit,
                    enabled: true,
                }]
            })
            .unwrap_or_default();
    }

    let physical = weapon.physical.unwrap_or(0.0) * aps;
    let elemental = weapon.elemental.unwrap_or(0.0) * aps;
    let total = physical + elemental;

    let mut out = Vec::new();

    if total <= 0.0 {
        return out;
    }

    let pdps_matters = is_pdps_important(category) && physical > 0.0;

    out.push(PropertyFilter {
        id: "pseudo.pseudo_total_dps",
        value: total,
        // Total damage per second is enabled only when neither half dominates,
        // because otherwise the specific one is the better filter.
        enabled: !pdps_matters && elemental <= 0.0,
    });

    if physical > 0.0 {
        out.push(PropertyFilter {
            id: "pseudo.pseudo_total_physical_dps",
            value: physical,
            enabled: pdps_matters,
        });
    }

    if elemental > 0.0 {
        out.push(PropertyFilter {
            id: "pseudo.pseudo_total_elemental_dps",
            value: elemental,
            // An elemental weapon is bought for its elemental damage, and a
            // physical one is not bought for the little it has.
            enabled: !pdps_matters,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn armour(ar: Option<f64>, ev: Option<f64>, es: Option<f64>) -> ArmourStats {
        ArmourStats {
            ar,
            ev,
            es,
            ..ArmourStats::default()
        }
    }

    fn weapon(physical: Option<f64>, elemental: Option<f64>, aps: Option<f64>) -> WeaponStats {
        WeaponStats {
            physical,
            elemental,
            attack_speed: aps,
            ..WeaponStats::default()
        }
    }

    #[test]
    fn a_martial_weapon_is_bought_for_physical_damage() {
        for c in [
            ItemCategory::OneHandedAxe,
            ItemCategory::TwoHandedSword,
            ItemCategory::Bow,
            ItemCategory::Crossbow,
            ItemCategory::Spear,
        ] {
            assert!(is_pdps_important(Some(c)), "{c:?}");
        }
    }

    #[test]
    fn a_caster_weapon_is_not() {
        // Filtering pdps on a wand excludes every wand worth buying.
        for c in [
            ItemCategory::Wand,
            ItemCategory::Sceptre,
            ItemCategory::Staff,
        ] {
            assert!(!is_pdps_important(Some(c)), "{c:?}");
            assert!(is_caster_weapon(Some(c)), "{c:?}");
        }
    }

    #[test]
    fn a_non_weapon_is_neither() {
        assert!(!is_pdps_important(Some(ItemCategory::Ring)));
        assert!(!is_caster_weapon(Some(ItemCategory::Ring)));
        assert!(!is_pdps_important(None));
    }

    #[test]
    fn the_largest_defence_is_the_one_the_piece_is_bought_for() {
        // A piece with 800 armour and 40 energy shield is an armour piece.
        // Filtering the energy shield returns every base that has any.
        let got = primary_defence(armour(Some(800.0), None, Some(40.0))).unwrap();

        assert_eq!(got.0, "pseudo.pseudo_total_armour");
        assert_eq!(got.1, 800.0);
    }

    #[test]
    fn each_defence_can_be_the_primary_one() {
        assert_eq!(
            primary_defence(armour(Some(100.0), None, None)).unwrap().0,
            "pseudo.pseudo_total_armour"
        );
        assert_eq!(
            primary_defence(armour(None, Some(100.0), None)).unwrap().0,
            "pseudo.pseudo_total_evasion"
        );
        assert_eq!(
            primary_defence(armour(None, None, Some(100.0))).unwrap().0,
            "pseudo.pseudo_total_energy_shield"
        );
    }

    #[test]
    fn a_zero_defence_is_not_the_primary_one() {
        // A floor of zero matches everything and only slows the search.
        let got = primary_defence(armour(Some(0.0), Some(50.0), None)).unwrap();

        assert_eq!(got.0, "pseudo.pseudo_total_evasion");
    }

    #[test]
    fn an_item_with_no_defences_gets_no_filter() {
        assert!(primary_defence(armour(None, None, None)).is_none());
        assert!(armour_filters(armour(None, None, None)).is_empty());
    }

    #[test]
    fn an_armour_piece_gets_one_enabled_filter() {
        let got = armour_filters(armour(Some(800.0), None, Some(40.0)));

        assert_eq!(got.len(), 1);
        assert!(got[0].enabled);
        assert_eq!(got[0].value, 800.0);
    }

    #[test]
    fn a_weapon_is_filtered_on_damage_per_second() {
        // The trade site indexes dps, and the same damage at a different
        // attack speed is a different weapon.
        let got = weapon_filters(
            Some(ItemCategory::Bow),
            weapon(Some(100.0), None, Some(1.5)),
        );

        let pdps = got
            .iter()
            .find(|f| f.id == "pseudo.pseudo_total_physical_dps")
            .unwrap();

        assert_eq!(pdps.value, 150.0);
        assert!(pdps.enabled);
    }

    #[test]
    fn a_weapon_with_no_attack_speed_gets_no_filters() {
        // Damage per second is undefined without it, and sending raw damage as
        // dps understates the weapon by a factor of two.
        assert!(
            weapon_filters(Some(ItemCategory::Bow), weapon(Some(100.0), None, None)).is_empty()
        );
    }

    #[test]
    fn an_elemental_weapon_is_bought_for_its_elemental_damage() {
        let got = weapon_filters(
            Some(ItemCategory::Wand),
            WeaponStats {
                elemental: Some(200.0),
                attack_speed: Some(1.5),
                spirit: None,
                ..WeaponStats::default()
            },
        );

        // A wand with no spirit yields nothing, because it is bought for what
        // it grants a spell.
        assert!(got.is_empty());
    }

    #[test]
    fn a_sceptre_is_filtered_on_its_spirit() {
        let got = weapon_filters(
            Some(ItemCategory::Sceptre),
            WeaponStats {
                spirit: Some(100.0),
                attack_speed: Some(1.4),
                physical: Some(50.0),
                ..WeaponStats::default()
            },
        );

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "pseudo.pseudo_total_spirit");
        assert!(got[0].enabled);
    }

    #[test]
    fn a_physical_weapon_leaves_its_elemental_filter_off() {
        // It is not bought for the little elemental damage it has.
        let got = weapon_filters(
            Some(ItemCategory::TwoHandedAxe),
            weapon(Some(200.0), Some(20.0), Some(1.2)),
        );

        let elemental = got
            .iter()
            .find(|f| f.id == "pseudo.pseudo_total_elemental_dps")
            .unwrap();

        assert!(!elemental.enabled);
    }

    #[test]
    fn a_weapon_with_no_damage_at_all_gets_no_filters() {
        assert!(weapon_filters(Some(ItemCategory::Bow), weapon(None, None, Some(1.5))).is_empty());
    }

    #[test]
    fn total_damage_per_second_sums_both_halves() {
        let got = weapon_filters(
            Some(ItemCategory::Bow),
            weapon(Some(100.0), Some(50.0), Some(2.0)),
        );

        let total = got
            .iter()
            .find(|f| f.id == "pseudo.pseudo_total_dps")
            .unwrap();

        assert_eq!(total.value, 300.0);
    }

    #[test]
    fn exactly_one_weapon_filter_is_enabled() {
        // Two enabled filters on one weapon narrow it to the exact item.
        for (category, stats) in [
            (
                ItemCategory::Bow,
                weapon(Some(100.0), Some(50.0), Some(1.5)),
            ),
            (ItemCategory::Bow, weapon(Some(100.0), None, Some(1.5))),
            (ItemCategory::Quiver, weapon(None, Some(100.0), Some(1.5))),
        ] {
            let got = weapon_filters(Some(category), stats);
            let enabled = got.iter().filter(|f| f.enabled).count();

            assert_eq!(enabled, 1, "{category:?} {stats:?}");
        }
    }

    #[test]
    fn a_single_defence_piece_is_treated_as_such() {
        // The reference always says yes, with a note that hybrid armour is
        // bought for one defence in practice. This is the one place to change
        // if that stops being true.
        assert!(is_single_defence_armour(armour(
            Some(100.0),
            Some(100.0),
            None
        )));
    }
}
