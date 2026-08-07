//! Turning a parsed item into a trade query.
//!
//! Ported from `create-item-filters.ts` and the query assembly half of
//! `pathofexile-trade.ts`.
//!
//! # The rule that governs everything here
//!
//! **Filter on what identifies the item, not on everything it has.**
//!
//! An over specified query returns nothing, and an empty result reads as "this
//! item is worthless" when it means "nobody else has this exact roll". So a
//! rare searches by base type and category, never by name, because its name is
//! random. A unique searches by name, because its name is the item.

use crate::controller::filter::brackets::{ceil_to_bracket, floor_to_bracket, ITEM_LEVEL_BRACKETS};
use crate::controller::filter::common::max_useful_item_level;
use crate::types::category::ItemCategory;
use crate::types::item::{ItemRarity, ParsedItem};
use crate::types::query::{
    category_trade_ids, Filters, NameField, Range, Status, TradeQuery, TypeFilters,
};

/// How wide to search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterOptions {
    /// Include offline sellers.
    pub include_offline: bool,
    /// Widen every numeric filter by this fraction.
    ///
    /// A query pinned to the exact roll matches only identical items. Ten
    /// percent is the reference's default and finds comparable ones.
    pub roll_tolerance: f64,
    /// Filter on item level.
    ///
    /// Off by default. Item level rarely changes a price and pinning it cuts
    /// the result count hard.
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

/// Build a trade query from a parsed item.
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

/// Decide what names the item.
///
/// A unique is its name. A rare's name is random, so it searches by base type
/// only. Sending a rare's name returns exactly the one listing that is the
/// same item, or none.
fn apply_identity(item: &ParsedItem, query: &mut TradeQuery) {
    let discriminator = item.info.trade_discriminator.as_deref();

    match item.rarity {
        Some(ItemRarity::Unique) if !item.is_unidentified => {
            query.name = Some(NameField::new(&item.info.name, None));

            // The base, not the unique's own name. `Kaom's Heart` is a name
            // and `Glorious Plate` is a type, and sending the first as the
            // type matches nothing. When the base is unknown the type is left
            // off, because the name alone still finds the item.
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

/// Category, rarity, item level and quality.
/// The item level filter, or nothing when the level says nothing about price.
///
/// # When item level is not worth sending
///
/// A tablet, a jewel and a map roll the same modifiers at every level, so the
/// level narrows nothing and only excludes listings. A unique's level says
/// nothing about its rolls at all.
///
/// A cluster jewel is the exception that gets a two ended range. Its passive
/// count is tied to bands of item level, so a buyer wants the band and not a
/// floor.
fn item_level_range(item: &ParsedItem) -> Option<Range> {
    let level = item.item_level?;

    // These roll the same modifiers at every level.
    if max_useful_item_level(item.category) == 1 {
        return None;
    }

    // A unique's level says nothing about its rolls.
    if item.rarity == Some(ItemRarity::Unique) {
        return None;
    }

    if item.category == Some(ItemCategory::ClusterJewel) {
        return Some(Range {
            min: Some(f64::from(floor_to_bracket(level, ITEM_LEVEL_BRACKETS))),
            // The upper brackets run downwards because they mark the top of
            // each band rather than the bottom.
            max: Some(f64::from(ceil_to_bracket(level, CLUSTER_UPPER_BRACKETS))),
        });
    }

    // Capped. Item level stops mattering once every modifier the base can roll
    // is reachable, and filtering above the cap excludes cheaper listings that
    // are just as good.
    Some(Range::at_least(f64::from(
        level.min(max_useful_item_level(item.category)),
    )))
}

/// The tops of the cluster jewel item level bands.
///
/// Descending because each marks the top of a band. Rounding up through them
/// in order finds the band the level sits in.
const CLUSTER_UPPER_BRACKETS: &[u32] = &[49, 67, 74, 100];

fn apply_type_filters(item: &ParsedItem, options: FilterOptions, out: &mut TypeFilters) {
    if let Some(category) = item.category {
        out.category = category_trade_ids()
            .get(category.as_str())
            .map(|s| (*s).to_string());
    }

    // A foil unique is a different market from a plain one.
    if item.is_foil {
        out.rarity = Some("uniquefoil".to_string());
    } else if item.rarity != Some(ItemRarity::Unique) && item.category.is_some() {
        // Excluding uniques stops a rare search matching a unique of the same
        // base, which is almost always priced very differently.
        out.rarity = Some("nonunique".to_string());
    }

    if options.filter_item_level {
        if let Some(range) = item_level_range(item) {
            out.ilvl = range;
        }
    }

    // Quality only matters where it changes the numbers. On a gem it is most
    // of the price.
    if item.category.is_some_and(ItemCategory::is_gem) {
        if let Some(quality) = item.quality {
            out.quality = Range::at_least(f64::from(quality));
        }
    }
}

/// Corruption, identification, gem level and the rest.
fn apply_misc_filters(item: &ParsedItem, out: &mut Filters) {
    let misc = &mut out.misc_filters;

    // Corruption is always filtered, both ways. A corrupted item cannot be
    // crafted further and is worth less, so mixing the two prices badly.
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

    // Only stated when the item is unidentified. An identified item is the
    // normal case and saying so narrows nothing.
    //
    // The tier replaces the flag rather than joining it. A tiered
    // unidentified item is unidentified by definition, so the flag adds
    // nothing, and the reference asserts the two are never sent together.
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

/// Defences, offence and rune sockets.
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

    // Damage per second, not damage. The trade site indexes dps, and an item
    // with the same damage at a different attack speed is a different item.
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

/// Map tier and the waystone numbers.
fn apply_map_filters(item: &ParsedItem, options: FilterOptions, out: &mut Filters) {
    let map = &mut out.map_filters;

    // Tier is pinned exactly. A tier 15 and a tier 16 map are different items
    // with different prices, so a tolerance here would be wrong.
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

/// Set a floor `tolerance` below the value.
///
/// A query pinned to the exact roll matches only identical items. The floor is
/// rounded down so a value that renders with one decimal still matches itself.
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
        // A rare's name is random. Sending it returns the one listing that is
        // this exact item, or none.
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert!(q.name.is_none());
        assert_eq!(q.type_name.as_ref().unwrap().name(), "Spine Bow");
    }

    #[test]
    fn a_rare_excludes_uniques() {
        // A unique of the same base is almost always priced very differently.
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert_eq!(q.filters.type_filters.rarity.as_deref(), Some("nonunique"));
    }

    #[test]
    fn a_unique_searches_by_name() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            category: Some(ItemCategory::BodyArmour),
            info: BaseInfo {
                // What the parser actually produces. The unique's entry names
                // the unique in both fields, and the base comes from the line
                // under it.
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
        // A unique search must not exclude uniques.
        assert_eq!(q.filters.type_filters.rarity, None);
    }

    #[test]
    fn a_unique_never_sends_its_own_name_as_the_type() {
        // No base is called Kaom's Heart, so a query typed that way matches
        // nothing. This is what the overlay sent for every unique in both
        // games.
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
        // A name with no type still finds the item. A name with a wrong type
        // finds nothing, so leaving the type off is the safe answer.
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
        // The reference asserts the two are never sent together. A tiered
        // unidentified item is unidentified by definition, so the flag adds
        // nothing to a filter that already implies it.
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
        // Its name is not printed yet, so there is nothing to search by.
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
        // Currency trades by name. A guessed id returns nothing.
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
        // It rarely changes a price and pinning it cuts the result count hard.
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

        // Capped at 82. Item level stops mattering once every modifier a bow
        // can roll is reachable, and 84 excludes cheaper listings that are
        // just as good.
        assert_eq!(q.filters.type_filters.ilvl.min, Some(82.0));
    }

    #[test]
    fn a_wand_caps_lower_than_a_bow() {
        // Its last modifier tier unlocks at 81.
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
        // A jewel and a map roll the same modifiers at every level, so the
        // filter narrows nothing and only excludes listings.
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
        // Its level says nothing about its rolls.
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
        // Its passive count is tied to bands of item level, so a buyer wants
        // the band and not a floor.
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
        // A band that excluded the item it came from would return nothing.
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
        // A corrupted item cannot be crafted further and is worth less, so
        // mixing the two prices badly in both directions.
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
        // It is the normal case and saying so narrows nothing.
        let q = build_query(&rare_bow(), FilterOptions::default());

        assert_eq!(q.filters.misc_filters.identified, None);
    }

    #[test]
    fn each_rare_flag_is_only_sent_when_true() {
        // Sending false would exclude every item that has the property, which
        // is a very different search.
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
        // Pinning the exact roll matches only identical items.
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
        // A floor of zero matches everything and only slows the search.
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
        // The trade site indexes dps. The same damage at a different attack
        // speed is a different item.
        let item = ParsedItem {
            weapon: WeaponStats {
                physical: Some(100.0),
                attack_speed: Some(1.5),
                ..WeaponStats::default()
            },
            ..rare_bow()
        };

        let q = build_query(&item, FilterOptions::default());

        // 100 * 1.5 = 150 pdps, minus ten percent.
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
        // Damage per second is undefined without it, and sending the raw
        // damage as dps would understate the weapon by a factor of two.
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
        // A tier 15 and a tier 16 map are different items with different
        // prices, so a tolerance would be wrong.
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
        // On a gem quality is most of the price. On a weapon the numbers it
        // scales are already filtered, so filtering it too narrows twice.
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
        // Offline listings never reply, and including them makes every price
        // look lower than it is.
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
        // Sending an empty string searches for an item called "", which
        // returns nothing and looks like the item is worthless.
        let item = ParsedItem {
            rarity: Some(ItemRarity::Rare),
            ..ParsedItem::default()
        };

        let q = build_query(&item, FilterOptions::default());

        assert!(q.type_name.is_none());
        assert!(q.name.is_none());
    }
}
