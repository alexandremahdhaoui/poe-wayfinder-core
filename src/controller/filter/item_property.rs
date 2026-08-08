use crate::types::category::ItemCategory;
use crate::types::item::{ArmourStats, MapStats, WeaponStats};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyFilter {
    pub id: &'static str,
    pub value: f64,
    pub enabled: bool,
}

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

pub fn is_caster_weapon(category: Option<ItemCategory>) -> bool {
    matches!(
        category,
        Some(ItemCategory::Wand | ItemCategory::Sceptre | ItemCategory::Staff)
    )
}

pub fn is_single_defence_armour(_armour: ArmourStats) -> bool {
    true
}

pub fn primary_defence(armour: ArmourStats) -> Option<(&'static str, f64)> {
    let candidates = [
        ("item.armour", armour.ar),
        ("item.evasion_rating", armour.ev),
        ("item.energy_shield", armour.es),
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

    [
        ("item.armour", armour.ar),
        ("item.evasion_rating", armour.ev),
        ("item.energy_shield", armour.es),
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

pub fn weapon_filters(category: Option<ItemCategory>, weapon: WeaponStats) -> Vec<PropertyFilter> {
    let Some(aps) = weapon.attack_speed else {
        return Vec::new();
    };

    if is_caster_weapon(category) {
        return weapon
            .spirit
            .filter(|s| *s > 0.0)
            .map(|spirit| {
                vec![PropertyFilter {
                    id: "item.spirit",
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
        id: "item.total_dps",
        value: total,
        enabled: !pdps_matters && elemental <= 0.0,
    });

    if physical > 0.0 {
        out.push(PropertyFilter {
            id: "item.physical_dps",
            value: physical,
            enabled: pdps_matters,
        });
    }

    if elemental > 0.0 {
        out.push(PropertyFilter {
            id: "item.elemental_dps",
            value: elemental,
            enabled: !pdps_matters,
        });
    }

    out
}

pub fn map_filters(map: MapStats) -> Vec<PropertyFilter> {
    let mut out = Vec::new();

    let mut push = |id: &'static str, value: Option<f64>| {
        if let Some(value) = value.filter(|v| *v != 0.0) {
            out.push(PropertyFilter {
                id,
                value,
                enabled: false,
            });
        }
    };

    push("item.map_revives", map.revives.map(f64::from));
    push("item.map_pack_size", map.pack_size);
    push("item.map_magic_monsters", map.magic_monsters);
    push("item.map_rare_monsters", map.rare_monsters);
    push("item.map_drop_chance", map.drop_chance);
    push("item.map_item_rarity", map.item_rarity);

    out
}

pub fn base_percentile_filter(percentile: Option<f64>) -> Option<PropertyFilter> {
    percentile.map(|value| PropertyFilter {
        id: "item.base_percentile",
        value,
        enabled: value >= 50.0,
    })
}

pub fn remove_used_stats(stats: &mut Vec<String>, used: &[&str]) {
    stats.retain(|reference| !used.contains(&reference.as_str()));
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
        let got = primary_defence(armour(Some(800.0), None, Some(40.0))).unwrap();

        assert_eq!(got.0, "item.armour");
        assert_eq!(got.1, 800.0);
    }

    #[test]
    fn each_defence_can_be_the_primary_one() {
        assert_eq!(
            primary_defence(armour(Some(100.0), None, None)).unwrap().0,
            "item.armour"
        );
        assert_eq!(
            primary_defence(armour(None, Some(100.0), None)).unwrap().0,
            "item.evasion_rating"
        );
        assert_eq!(
            primary_defence(armour(None, None, Some(100.0))).unwrap().0,
            "item.energy_shield"
        );
    }

    #[test]
    fn a_zero_defence_is_not_the_primary_one() {
        let got = primary_defence(armour(Some(0.0), Some(50.0), None)).unwrap();

        assert_eq!(got.0, "item.evasion_rating");
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
        let got = weapon_filters(
            Some(ItemCategory::Bow),
            weapon(Some(100.0), None, Some(1.5)),
        );

        let pdps = got.iter().find(|f| f.id == "item.physical_dps").unwrap();

        assert_eq!(pdps.value, 150.0);
        assert!(pdps.enabled);
    }

    #[test]
    fn a_weapon_with_no_attack_speed_gets_no_filters() {
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
        assert_eq!(got[0].id, "item.spirit");
        assert!(got[0].enabled);
    }

    #[test]
    fn a_physical_weapon_leaves_its_elemental_filter_off() {
        let got = weapon_filters(
            Some(ItemCategory::TwoHandedAxe),
            weapon(Some(200.0), Some(20.0), Some(1.2)),
        );

        let elemental = got.iter().find(|f| f.id == "item.elemental_dps").unwrap();

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

        let total = got.iter().find(|f| f.id == "item.total_dps").unwrap();

        assert_eq!(total.value, 300.0);
    }

    #[test]
    fn exactly_one_weapon_filter_is_enabled() {
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
        assert!(is_single_defence_armour(armour(
            Some(100.0),
            Some(100.0),
            None
        )));
    }

    fn map_with(pack_size: Option<f64>, rarity: Option<f64>) -> MapStats {
        MapStats {
            pack_size,
            item_rarity: rarity,
            ..MapStats::default()
        }
    }

    fn ids(filters: &[PropertyFilter]) -> Vec<&str> {
        filters.iter().map(|f| f.id).collect()
    }

    #[test]
    fn a_map_is_filtered_on_what_it_drops() {
        let got = map_filters(map_with(Some(30.0), Some(80.0)));

        assert_eq!(ids(&got), ["item.map_pack_size", "item.map_item_rarity"]);
    }

    #[test]
    fn every_map_filter_starts_switched_off() {
        for filter in map_filters(map_with(Some(30.0), Some(80.0))) {
            assert!(!filter.enabled, "{}", filter.id);
        }
    }

    #[test]
    fn a_number_the_map_does_not_have_is_not_filtered() {
        assert!(map_filters(MapStats::default()).is_empty());
    }

    #[test]
    fn a_zero_is_not_a_number_worth_filtering_on() {
        assert!(map_filters(map_with(Some(0.0), None)).is_empty());
    }

    #[test]
    fn a_map_carries_its_value_through() {
        let got = map_filters(map_with(Some(37.0), None));

        assert_eq!(got[0].value, 37.0);
    }

    #[test]
    fn the_poe1_only_numbers_are_still_read() {
        let got = map_filters(MapStats {
            magic_monsters: Some(20.0),
            rare_monsters: Some(15.0),
            revives: Some(6),
            drop_chance: Some(50.0),
            ..MapStats::default()
        });

        assert_eq!(
            ids(&got),
            [
                "item.map_revives",
                "item.map_magic_monsters",
                "item.map_rare_monsters",
                "item.map_drop_chance"
            ]
        );
    }

    #[test]
    fn every_map_filter_id_is_distinct() {
        let got = map_filters(MapStats {
            pack_size: Some(1.0),
            item_rarity: Some(1.0),
            revives: Some(1),
            drop_chance: Some(1.0),
            magic_monsters: Some(1.0),
            rare_monsters: Some(1.0),
            ..MapStats::default()
        });

        let mut seen = ids(&got);
        seen.sort_unstable();
        let count = seen.len();
        seen.dedup();

        assert_eq!(seen.len(), count);
    }

    #[test]
    fn a_well_rolled_base_starts_switched_on() {
        let got = base_percentile_filter(Some(85.0)).expect("a percentile filters");

        assert_eq!(got.id, "item.base_percentile");
        assert_eq!(got.value, 85.0);
        assert!(got.enabled);
    }

    #[test]
    fn a_below_average_base_starts_switched_off() {
        assert!(
            !base_percentile_filter(Some(20.0))
                .expect("a percentile filters")
                .enabled
        );
    }

    #[test]
    fn a_base_exactly_at_the_halfway_mark_starts_switched_on() {
        assert!(
            base_percentile_filter(Some(50.0))
                .expect("a percentile filters")
                .enabled
        );
    }

    #[test]
    fn an_unknown_percentile_produces_no_filter() {
        assert_eq!(base_percentile_filter(None), None);
    }

    #[test]
    fn a_stat_a_property_already_covers_is_dropped() {
        let mut stats = vec![
            "#% increased Physical Damage".to_string(),
            "+# to maximum Life".to_string(),
        ];

        remove_used_stats(&mut stats, &["#% increased Physical Damage"]);

        assert_eq!(stats, ["+# to maximum Life"]);
    }

    #[test]
    fn removing_nothing_leaves_the_list_alone() {
        let mut stats = vec!["+# to maximum Life".to_string()];

        remove_used_stats(&mut stats, &[]);

        assert_eq!(stats.len(), 1);
    }

    #[test]
    fn removing_a_stat_that_is_not_there_changes_nothing() {
        let mut stats = vec!["+# to maximum Life".to_string()];

        remove_used_stats(&mut stats, &["#% increased Armour"]);

        assert_eq!(stats.len(), 1);
    }

    #[test]
    fn every_copy_of_a_used_stat_is_dropped() {
        let mut stats = vec!["a".to_string(), "b".to_string(), "a".to_string()];

        remove_used_stats(&mut stats, &["a"]);

        assert_eq!(stats, ["b"]);
    }
}
