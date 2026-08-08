use crate::controller::filter::brackets::{ceil_to_bracket, floor_to_bracket, ITEM_LEVEL_BRACKETS};
use crate::controller::filter::common::max_useful_item_level;
use crate::types::category::ItemCategory;
use crate::types::item::{ItemRarity, ParsedItem};
use crate::types::query::{
    category_trade_ids, Filters, NameField, Range, Status, TradeQuery, TypeFilters,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterOptions {
    pub include_offline: bool,
    pub roll_tolerance: f64,
    pub filter_item_level: bool,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            include_offline: false,
            roll_tolerance: 0.1,
            filter_item_level: false,
        }
    }
}

pub fn build_query(item: &ParsedItem, options: FilterOptions) -> TradeQuery {
    let mut query = TradeQuery {
        status: if options.include_offline {
            Status::Any
        } else {
            Status::Online
        },
        ..TradeQuery::default()
    };

    apply_identity(item, &mut query);
    apply_type_filters(item, options, &mut query.filters.type_filters);
    apply_misc_filters(item, &mut query.filters);
    apply_equipment_filters(item, options, &mut query.filters);
    apply_map_filters(item, options, &mut query.filters);

    query
}

fn apply_identity(item: &ParsedItem, query: &mut TradeQuery) {
    let discriminator = item.info.trade_discriminator.as_deref();

    match item.rarity {
        Some(ItemRarity::Unique) if !item.is_unidentified => {
            query.name = Some(NameField::new(&item.info.name, None));

            if let Some(base) = &item.info.unique_base {
                query.type_name = Some(NameField::new(base, discriminator));
            }
        }
        _ => {
            if !item.info.reference_name.is_empty() {
                query.type_name = Some(NameField::new(&item.info.reference_name, discriminator));
            }
        }
    }
}

fn item_level_range(item: &ParsedItem) -> Option<Range> {
    let level = item.item_level?;

    if max_useful_item_level(item.category) == 1 {
        return None;
    }

    if item.rarity == Some(ItemRarity::Unique) {
        return None;
    }

    if item.category == Some(ItemCategory::ClusterJewel) {
        return Some(Range {
            min: Some(f64::from(floor_to_bracket(level, ITEM_LEVEL_BRACKETS))),
            max: Some(f64::from(ceil_to_bracket(level, CLUSTER_UPPER_BRACKETS))),
        });
    }

    Some(Range::at_least(f64::from(
        level.min(max_useful_item_level(item.category)),
    )))
}

const CLUSTER_UPPER_BRACKETS: &[u32] = &[49, 67, 74, 100];

fn apply_type_filters(item: &ParsedItem, options: FilterOptions, out: &mut TypeFilters) {
    if let Some(category) = item.category {
        out.category = category_trade_ids()
            .get(category.as_str())
            .map(|s| (*s).to_string());
    }

    if item.is_foil {
        out.rarity = Some("uniquefoil".to_string());
    } else if item.rarity != Some(ItemRarity::Unique) && item.category.is_some() {
        out.rarity = Some("nonunique".to_string());
    }

    if options.filter_item_level {
        if let Some(range) = item_level_range(item) {
            out.ilvl = range;
        }
    }

    if item.category.is_some_and(ItemCategory::is_gem) {
        if let Some(quality) = item.quality {
            out.quality = Range::at_least(f64::from(quality));
        }
    }
}

fn apply_misc_filters(item: &ParsedItem, out: &mut Filters) {
    let misc = &mut out.misc_filters;

    misc.corrupted = Some(item.is_corrupted);

    if item.is_mirrored {
        misc.mirrored = Some(true);
    }

    if item.is_sanctified {
        misc.sanctified = Some(true);
    }

    if item.is_veiled {
        misc.veiled = Some(true);
    }

    if item.is_fractured {
        misc.fractured_item = Some(true);
    }

    match item.unidentified_tier {
        Some(tier) => misc.unidentified_tier = Range::exactly(f64::from(tier)),
        None if item.is_unidentified => misc.identified = Some(false),
        None => {}
    }

    if let Some(level) = item.gem_level {
        misc.gem_level = Range::at_least(f64::from(level));
    }

    if let Some(sockets) = item.gem_sockets {
        if sockets.number > 0 {
            misc.gem_sockets = Range::at_least(f64::from(sockets.number));
        }
    }

    if let Some(level) = item.area_level {
        misc.area_level = Range::at_least(f64::from(level));
    }
}

fn apply_equipment_filters(item: &ParsedItem, options: FilterOptions, out: &mut Filters) {
    let eq = &mut out.equipment_filters;
    let tolerance = options.roll_tolerance;

    set_with_tolerance(&mut eq.ar, item.armour.ar, tolerance);
    set_with_tolerance(&mut eq.ev, item.armour.ev, tolerance);
    set_with_tolerance(&mut eq.es, item.armour.es, tolerance);
    set_with_tolerance(&mut eq.block, item.armour.block, tolerance);

    set_with_tolerance(&mut eq.aps, item.weapon.attack_speed, tolerance);
    set_with_tolerance(&mut eq.crit, item.weapon.crit, tolerance);
    set_with_tolerance(&mut eq.spirit, item.weapon.spirit, tolerance);
    set_with_tolerance(&mut eq.reload_time, item.weapon.reload, tolerance);

    if let Some(aps) = item.weapon.attack_speed {
        set_with_tolerance(
            &mut eq.pdps,
            item.weapon.physical.map(|d| d * aps),
            tolerance,
        );
        set_with_tolerance(
            &mut eq.edps,
            item.weapon.elemental.map(|d| d * aps),
            tolerance,
        );

        let total = item.weapon.physical.unwrap_or(0.0) + item.weapon.elemental.unwrap_or(0.0);

        if total > 0.0 {
            set_with_tolerance(&mut eq.dps, Some(total * aps), tolerance);
        }
    }

    if let Some(sockets) = item.augment_sockets {
        if sockets.normal > 0 {
            eq.rune_sockets = Range::at_least(f64::from(sockets.normal));
        }
    }
}

fn apply_map_filters(item: &ParsedItem, options: FilterOptions, out: &mut Filters) {
    let map = &mut out.map_filters;

    if let Some(tier) = item.map.tier {
        map.map_tier = Range::exactly(f64::from(tier));
    }

    let tolerance = options.roll_tolerance;

    set_with_tolerance(&mut map.map_packsize, item.map.pack_size, tolerance);
    set_with_tolerance(&mut map.map_iir, item.map.item_rarity, tolerance);
    set_with_tolerance(&mut map.map_bonus, item.map.drop_chance, tolerance);
    set_with_tolerance(
        &mut map.map_magic_monsters,
        item.map.magic_monsters,
        tolerance,
    );
    set_with_tolerance(
        &mut map.map_rare_monsters,
        item.map.rare_monsters,
        tolerance,
    );

    if let Some(revives) = item.map.revives {
        map.map_revives = Range::at_least(f64::from(revives));
    }

    if let Some(trials) = &item.trials {
        map.ultimatum_hint = trials.ultimatum_hint.map(|h| {
            match h {
                crate::types::item::UltimatumHint::Victorious => "Victorious",
                crate::types::item::UltimatumHint::Cowardly => "Cowardly",
                crate::types::item::UltimatumHint::Deadly => "Deadly",
            }
            .to_string()
        });
    }
}

fn set_with_tolerance(range: &mut Range, value: Option<f64>, tolerance: f64) {
    let Some(value) = value else {
        return;
    };

    if value <= 0.0 {
        return;
    }

    let floor = (value * (1.0 - tolerance) * 10.0).floor() / 10.0;

    *range = Range::at_least(floor);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::item::{
        ArmourStats, AugmentSockets, BaseInfo, MapStats, Trials, UltimatumHint, WeaponStats,
    };

    fn rare_bow() -> ParsedItem {
        ParsedItem {
            rarity: Some(ItemRarity::Rare),
            category: Some(ItemCategory::Bow),
            item_level: Some(84),
            info: BaseInfo {
                name: "Doom Fletch".into(),
                reference_name: "Spine Bow".into(),
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        }
    }

    #[test]
    fn a_rare_searches_by_base_type_and_never_by_name() {
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert!(q.name.is_none());
        assert_eq!(q.type_name.as_ref().unwrap().name(), "Spine Bow");
    }

    #[test]
    fn a_rare_excludes_uniques() {
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert_eq!(q.filters.type_filters.rarity.as_deref(), Some("nonunique"));
    }

    #[test]
    fn a_unique_searches_by_name() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            category: Some(ItemCategory::BodyArmour),
            info: BaseInfo {
                name: "Kaom's Heart".into(),
                reference_name: "Kaom's Heart".into(),
                unique_base: Some("Glorious Plate".into()),
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.name.as_ref().unwrap().name(), "Kaom's Heart");
        assert_eq!(q.type_name.as_ref().unwrap().name(), "Glorious Plate");
        assert_eq!(q.filters.type_filters.rarity, None);
    }

    #[test]
    fn a_unique_never_sends_its_own_name_as_the_type() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            category: Some(ItemCategory::BodyArmour),
            info: BaseInfo {
                name: "Kaom's Heart".into(),
                reference_name: "Kaom's Heart".into(),
                unique_base: Some("Glorious Plate".into()),
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_ne!(q.type_name.as_ref().unwrap().name(), "Kaom's Heart");
    }

    #[test]
    fn a_unique_whose_base_is_unknown_searches_by_name_alone() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            category: Some(ItemCategory::BodyArmour),
            info: BaseInfo {
                name: "Kaom's Heart".into(),
                reference_name: "Kaom's Heart".into(),
                unique_base: None,
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.name.as_ref().unwrap().name(), "Kaom's Heart");
        assert_eq!(q.type_name, None);
    }

    #[test]
    fn an_unidentified_item_says_so() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Rare),
            is_unidentified: true,
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.misc_filters.identified, Some(false));
        assert_eq!(q.filters.misc_filters.unidentified_tier, Range::default());
    }

    #[test]
    fn a_tiered_unidentified_item_sends_the_tier_instead_of_the_flag() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Rare),
            is_unidentified: true,
            unidentified_tier: Some(4),
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.misc_filters.identified, None);
        assert_eq!(
            q.filters.misc_filters.unidentified_tier,
            Range::exactly(4.0)
        );
    }

    #[test]
    fn an_unidentified_unique_searches_by_base_type_only() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            is_unidentified: true,
            category: Some(ItemCategory::BodyArmour),
            info: BaseInfo {
                name: "Glorious Plate".into(),
                reference_name: "Glorious Plate".into(),
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert!(q.name.is_none());
        assert_eq!(q.filters.misc_filters.identified, Some(false));
    }

    #[test]
    fn a_foil_unique_searches_the_foil_market() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            is_foil: true,
            category: Some(ItemCategory::BodyArmour),
            info: BaseInfo {
                name: "Kaom's Heart".into(),
                reference_name: "Glorious Plate".into(),
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.type_filters.rarity.as_deref(), Some("uniquefoil"));
    }

    #[test]
    fn a_discriminated_base_keeps_its_discriminator() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Rare),
            category: Some(ItemCategory::Ring),
            info: BaseInfo {
                reference_name: "Two-Stone Ring".into(),
                trade_discriminator: Some("fire_cold".into()),
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        match q.type_name.unwrap() {
            NameField::Discriminated(d) => assert_eq!(d.discriminator, "fire_cold"),
            other => panic!("discriminator was dropped: {other:?}"),
        }
    }

    #[test]
    fn the_category_becomes_its_trade_id() {
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert_eq!(
            q.filters.type_filters.category.as_deref(),
            Some("weapon.bow")
        );
    }

    #[test]
    fn a_category_with_no_trade_id_sets_none() {
        let item = ParsedItem {
            category: Some(ItemCategory::Currency),
            info: BaseInfo {
                reference_name: "Chaos Orb".into(),
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.type_filters.category, None);
    }

    #[test]
    fn item_level_is_not_filtered_by_default() {
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert!(q.filters.type_filters.ilvl.is_empty());
    }

    #[test]
    fn item_level_is_filtered_when_asked_for() {
        let options = FilterOptions {
            filter_item_level: true,
            ..FilterOptions::default()
        };

        let q = build_query(&rare_bow(), options);

        assert_eq!(q.filters.type_filters.ilvl.min, Some(82.0));
    }

    #[test]
    fn a_wand_caps_lower_than_a_bow() {
        let options = FilterOptions {
            filter_item_level: true,
            ..FilterOptions::default()
        };

        let wand = ParsedItem {
            category: Some(ItemCategory::Wand),
            ..rare_bow()
        };

        assert_eq!(
            build_query(&wand, options).filters.type_filters.ilvl.min,
            Some(81.0)
        );
    }

    #[test]
    fn an_item_below_its_cap_keeps_its_own_level() {
        let options = FilterOptions {
            filter_item_level: true,
            ..FilterOptions::default()
        };

        let low = ParsedItem {
            item_level: Some(60),
            ..rare_bow()
        };

        assert_eq!(
            build_query(&low, options).filters.type_filters.ilvl.min,
            Some(60.0)
        );
    }

    #[test]
    fn a_base_whose_level_says_nothing_is_not_filtered() {
        let options = FilterOptions {
            filter_item_level: true,
            ..FilterOptions::default()
        };

        for category in [ItemCategory::Jewel, ItemCategory::Map, ItemCategory::Tablet] {
            let item = ParsedItem {
                category: Some(category),
                ..rare_bow()
            };

            assert!(
                build_query(&item, options)
                    .filters
                    .type_filters
                    .ilvl
                    .is_empty(),
                "{category:?}"
            );
        }
    }

    #[test]
    fn a_unique_is_not_filtered_by_item_level() {
        let options = FilterOptions {
            filter_item_level: true,
            ..FilterOptions::default()
        };

        let unique = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            ..rare_bow()
        };

        assert!(build_query(&unique, options)
            .filters
            .type_filters
            .ilvl
            .is_empty());
    }

    #[test]
    fn a_cluster_jewel_gets_a_two_ended_band() {
        let options = FilterOptions {
            filter_item_level: true,
            ..FilterOptions::default()
        };

        let cluster = ParsedItem {
            category: Some(ItemCategory::ClusterJewel),
            item_level: Some(70),
            ..rare_bow()
        };

        let ilvl = build_query(&cluster, options).filters.type_filters.ilvl;

        assert_eq!(ilvl.min, Some(68.0));
        assert_eq!(ilvl.max, Some(74.0));
    }

    #[test]
    fn a_cluster_jewel_band_always_contains_its_own_level() {
        let options = FilterOptions {
            filter_item_level: true,
            ..FilterOptions::default()
        };

        for level in 1..=100u32 {
            let cluster = ParsedItem {
                category: Some(ItemCategory::ClusterJewel),
                item_level: Some(level),
                ..rare_bow()
            };

            let ilvl = build_query(&cluster, options).filters.type_filters.ilvl;

            assert!(ilvl.min.unwrap() <= f64::from(level), "{level}");
            assert!(ilvl.max.unwrap() >= f64::from(level), "{level}");
        }
    }

    #[test]
    fn corruption_is_always_stated_in_both_directions() {
        let plain = build_query(&rare_bow(), FilterOptions::default());
        assert_eq!(plain.filters.misc_filters.corrupted, Some(false));

        let item = ParsedItem {
            is_corrupted: true,
            ..rare_bow()
        };
        let corrupted = build_query(&item, FilterOptions::default());
        assert_eq!(corrupted.filters.misc_filters.corrupted, Some(true));
    }

    #[test]
    fn an_identified_item_states_nothing_about_identification() {
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert_eq!(q.filters.misc_filters.identified, None);
    }

    #[test]
    fn each_rare_flag_is_only_sent_when_true() {
        let q = build_query(&rare_bow(), FilterOptions::default());
        let misc = &q.filters.misc_filters;

        assert_eq!(misc.mirrored, None);
        assert_eq!(misc.sanctified, None);
        assert_eq!(misc.veiled, None);
        assert_eq!(misc.fractured_item, None);
    }

    #[test]
    fn a_fractured_item_says_so() {
        let item = ParsedItem {
            is_fractured: true,
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.misc_filters.fractured_item, Some(true));
    }

    #[test]
    fn a_defensive_number_becomes_a_floor_below_the_roll() {
        let item = ParsedItem {
            armour: ArmourStats {
                ar: Some(500.0),
                ..ArmourStats::default()
            },
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.equipment_filters.ar.min, Some(450.0));
        assert_eq!(q.filters.equipment_filters.ar.max, None);
    }

    #[test]
    fn a_zero_tolerance_pins_the_floor_to_the_roll() {
        let options = FilterOptions {
            roll_tolerance: 0.0,
            ..FilterOptions::default()
        };
        let item = ParsedItem {
            armour: ArmourStats {
                es: Some(220.0),
                ..ArmourStats::default()
            },
            ..rare_bow()
        };

        let q = build_query(&item, options);

        assert_eq!(q.filters.equipment_filters.es.min, Some(220.0));
    }

    #[test]
    fn an_absent_number_sets_no_filter() {
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert!(q.filters.equipment_filters.ar.is_empty());
        assert!(q.filters.equipment_filters.es.is_empty());
    }

    #[test]
    fn a_zero_number_sets_no_filter() {
        let item = ParsedItem {
            armour: ArmourStats {
                ar: Some(0.0),
                ..ArmourStats::default()
            },
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        assert!(q.filters.equipment_filters.ar.is_empty());
    }

    #[test]
    fn damage_is_filtered_as_damage_per_second() {
        let item = ParsedItem {
            weapon: WeaponStats {
                physical: Some(100.0),
                attack_speed: Some(1.5),
                ..WeaponStats::default()
            },
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.equipment_filters.pdps.min, Some(135.0));
        assert_eq!(q.filters.equipment_filters.dps.min, Some(135.0));
    }

    #[test]
    fn physical_and_elemental_damage_both_reach_the_total() {
        let item = ParsedItem {
            weapon: WeaponStats {
                physical: Some(100.0),
                elemental: Some(50.0),
                attack_speed: Some(2.0),
                ..WeaponStats::default()
            },
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.equipment_filters.pdps.min, Some(180.0));
        assert_eq!(q.filters.equipment_filters.edps.min, Some(90.0));
        assert_eq!(q.filters.equipment_filters.dps.min, Some(270.0));
    }

    #[test]
    fn damage_without_an_attack_speed_sets_no_dps_filter() {
        let item = ParsedItem {
            weapon: WeaponStats {
                physical: Some(100.0),
                ..WeaponStats::default()
            },
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        assert!(q.filters.equipment_filters.pdps.is_empty());
        assert!(q.filters.equipment_filters.dps.is_empty());
    }

    #[test]
    fn rune_sockets_are_filtered_on_the_base_count() {
        let item = ParsedItem {
            augment_sockets: Some(AugmentSockets {
                empty: 1,
                current: 2,
                normal: 3,
            }),
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.equipment_filters.rune_sockets.min, Some(3.0));
    }

    #[test]
    fn a_map_tier_is_pinned_exactly() {
        let item = ParsedItem {
            category: Some(ItemCategory::Map),
            map: MapStats {
                tier: Some(16),
                ..MapStats::default()
            },
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(q.filters.map_filters.map_tier.min, Some(16.0));
        assert_eq!(q.filters.map_filters.map_tier.max, Some(16.0));
    }

    #[test]
    fn waystone_numbers_get_a_floor_with_tolerance() {
        let item = ParsedItem {
            category: Some(ItemCategory::Waystone),
            map: MapStats {
                tier: Some(15),
                pack_size: Some(20.0),
                item_rarity: Some(40.0),
                drop_chance: Some(100.0),
                revives: Some(6),
                ..MapStats::default()
            },
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());
        let map = &q.filters.map_filters;

        assert_eq!(map.map_packsize.min, Some(18.0));
        assert_eq!(map.map_iir.min, Some(36.0));
        assert_eq!(map.map_bonus.min, Some(90.0));
        assert_eq!(map.map_revives.min, Some(6.0));
    }

    #[test]
    fn an_ultimatum_hint_reaches_the_query() {
        let item = ParsedItem {
            trials: Some(Trials {
                number_of_trials: Some(10),
                ultimatum_hint: Some(UltimatumHint::Victorious),
            }),
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        assert_eq!(
            q.filters.map_filters.ultimatum_hint.as_deref(),
            Some("Victorious")
        );
    }

    #[test]
    fn gem_quality_is_filtered_and_weapon_quality_is_not() {
        let gem = ParsedItem {
            category: Some(ItemCategory::Gem),
            quality: Some(20),
            gem_level: Some(21),
            ..rare_bow()
        };

        let q = build_query(&gem, FilterOptions::default());
        assert_eq!(q.filters.type_filters.quality.min, Some(20.0));
        assert_eq!(q.filters.misc_filters.gem_level.min, Some(21.0));

        let weapon = ParsedItem {
            quality: Some(20),
            ..rare_bow()
        };

        let q = build_query(&weapon, FilterOptions::default());
        assert!(q.filters.type_filters.quality.is_empty());
    }

    #[test]
    fn the_default_search_is_online_sellers_only() {
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert_eq!(q.status, Status::Online);
    }

    #[test]
    fn offline_sellers_are_included_when_asked_for() {
        let options = FilterOptions {
            include_offline: true,
            ..FilterOptions::default()
        };

        let q = build_query(&rare_bow(), options);

        assert_eq!(q.status, Status::Any);
    }

    #[test]
    fn an_item_with_no_base_name_sends_no_type() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Rare),
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert!(q.type_name.is_none());
        assert!(q.name.is_none());
    }
}
