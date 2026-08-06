//! Small decisions shared by every filter builder.
//!
//! Ported from `web/price-check/filters/common.ts`.

use crate::types::category::ItemCategory;
use crate::types::query::StatFilter;

/// The highest item level worth filtering on for this base.
///
/// Ported from `maxUsefulItemLevel`.
///
/// # Why a cap at all
///
/// Item level stops mattering once every modifier a base can roll is already
/// reachable. Past that point a higher level is the same item, so a filter
/// pinned to it excludes cheaper listings that are just as good.
///
/// The caps are not uniform. A wand and a staff top out at 81 because their
/// last modifier tier unlocks there. A tablet, a jewel and a map roll the same
/// modifiers at every level, so their level says nothing at all.
pub fn max_useful_item_level(category: Option<ItemCategory>) -> u32 {
    match category {
        // The last modifier tier on these unlocks at 81.
        Some(ItemCategory::Wand | ItemCategory::Staff) => 81,
        Some(ItemCategory::Relic) => 80,
        // These roll the same modifiers at every level, so the level says
        // nothing and filtering on it only excludes listings.
        Some(ItemCategory::Tablet | ItemCategory::Jewel | ItemCategory::Map) => 1,
        _ => 82,
    }
}

/// Switch on every filter the user can see.
///
/// Ported from `enableAllFilters`. Used by the "match everything" key, where
/// the user wants the strictest search the item supports.
///
/// A hidden filter stays off. It is hidden because filtering on it narrows the
/// search to items identical to the one in hand, and turning it on behind the
/// user's back would return nothing with no visible cause.
pub fn enable_all(filters: &mut [StatFilter]) {
    for filter in filters.iter_mut() {
        if !filter.hidden {
            filter.disabled = false;
        }
    }
}

/// What an item needs done to its filters once they are all built.
///
/// Ported from `finalFilterTweaks`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FinalTweaks {
    /// Apply the cluster jewel rules.
    ///
    /// A cluster jewel is bought for its passive count and its notables, and
    /// every other roll on it is noise a buyer does not filter on.
    pub cluster_jewel_rules: bool,
    /// Apply the flask rules.
    pub flask_rules: bool,
    /// Hide every rune filter.
    ///
    /// A unique's runes are part of the item as listed. Filtering on them
    /// searches for a unique someone else socketed the same way, which is a
    /// different and much rarer item than the one the user is pricing.
    pub hide_augments: bool,
    /// Offer the empty modifier filter.
    pub show_empty_modifier: bool,
}

/// Uniques whose runes are worth filtering on.
///
/// Both are bought for what is socketed in them and nothing else, so hiding
/// their rune filters would hide the only thing that sets one apart.
const AUGMENT_UNIQUES: &[&str] = &["Morior Invictus", "Darkness Enthroned"];

/// Work out the final tweaks for an item.
///
/// Ported from `finalFilterTweaks`. The cluster jewel and flask rules are
/// exclusive: an item is one or the other and never both.
pub fn final_filter_tweaks(
    category: Option<ItemCategory>,
    rarity: Option<crate::types::item::ItemRarity>,
    reference_name: &str,
    has_empty_slot: bool,
) -> FinalTweaks {
    use crate::types::item::ItemRarity;

    let unique = rarity == Some(ItemRarity::Unique);

    FinalTweaks {
        // A unique cluster jewel has fixed passives, so the rules that pick
        // which rolls matter do not apply to it.
        cluster_jewel_rules: category == Some(ItemCategory::ClusterJewel) && !unique,
        flask_rules: category == Some(ItemCategory::Flask)
            && category != Some(ItemCategory::ClusterJewel),
        hide_augments: unique && !AUGMENT_UNIQUES.contains(&reference_name),
        show_empty_modifier: has_empty_slot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::query::Range;

    fn filter(hidden: bool) -> StatFilter {
        let mut f = StatFilter::range("x", Range::at_least(1.0));
        f.disabled = true;
        f.hidden = hidden;

        f
    }

    #[test]
    fn most_bases_cap_at_eighty_two() {
        assert_eq!(max_useful_item_level(Some(ItemCategory::Ring)), 82);
        assert_eq!(max_useful_item_level(Some(ItemCategory::BodyArmour)), 82);
    }

    #[test]
    fn an_unknown_base_caps_at_eighty_two() {
        // Guessing lower would exclude listings that are genuinely better.
        assert_eq!(max_useful_item_level(None), 82);
    }

    #[test]
    fn a_wand_and_a_staff_cap_at_eighty_one() {
        // Their last modifier tier unlocks there.
        assert_eq!(max_useful_item_level(Some(ItemCategory::Wand)), 81);
        assert_eq!(max_useful_item_level(Some(ItemCategory::Staff)), 81);
    }

    #[test]
    fn a_relic_caps_at_eighty() {
        assert_eq!(max_useful_item_level(Some(ItemCategory::Relic)), 80);
    }

    #[test]
    fn a_base_whose_level_says_nothing_caps_at_one() {
        // These roll the same modifiers at every level, so filtering on the
        // level only excludes listings.
        for category in [ItemCategory::Tablet, ItemCategory::Jewel, ItemCategory::Map] {
            assert_eq!(max_useful_item_level(Some(category)), 1, "{category:?}");
        }
    }

    #[test]
    fn every_cap_is_a_level_the_game_can_reach() {
        for category in [
            None,
            Some(ItemCategory::Wand),
            Some(ItemCategory::Relic),
            Some(ItemCategory::Map),
        ] {
            let cap = max_useful_item_level(category);

            assert!((1..=100).contains(&cap), "{category:?} gave {cap}");
        }
    }

    #[test]
    fn enabling_all_switches_on_a_visible_filter() {
        let mut filters = vec![filter(false)];

        enable_all(&mut filters);

        assert!(!filters[0].disabled);
    }

    #[test]
    fn enabling_all_leaves_a_hidden_filter_off() {
        // Turning it on would return nothing with no visible cause.
        let mut filters = vec![filter(true)];

        enable_all(&mut filters);

        assert!(filters[0].disabled);
    }

    #[test]
    fn enabling_all_of_nothing_does_nothing() {
        let mut filters: Vec<StatFilter> = Vec::new();

        enable_all(&mut filters);

        assert!(filters.is_empty());
    }

    // -----------------------------------------------------------------
    // Final tweaks
    // -----------------------------------------------------------------

    use crate::types::item::ItemRarity;

    fn tweaks(category: ItemCategory, rarity: ItemRarity, name: &str) -> FinalTweaks {
        final_filter_tweaks(Some(category), Some(rarity), name, false)
    }

    #[test]
    fn a_rare_cluster_jewel_gets_the_cluster_rules() {
        // It is bought for its passive count and its notables, and every other
        // roll on it is noise a buyer does not filter on.
        assert!(tweaks(ItemCategory::ClusterJewel, ItemRarity::Rare, "x").cluster_jewel_rules);
    }

    #[test]
    fn a_unique_cluster_jewel_does_not() {
        // Its passives are fixed, so the rules that pick which rolls matter do
        // not apply.
        assert!(!tweaks(ItemCategory::ClusterJewel, ItemRarity::Unique, "x").cluster_jewel_rules);
    }

    #[test]
    fn a_flask_gets_the_flask_rules() {
        assert!(tweaks(ItemCategory::Flask, ItemRarity::Magic, "x").flask_rules);
    }

    #[test]
    fn the_two_rule_sets_are_never_both_applied() {
        // An item is one or the other and never both.
        for category in [
            ItemCategory::ClusterJewel,
            ItemCategory::Flask,
            ItemCategory::Ring,
        ] {
            for rarity in [ItemRarity::Normal, ItemRarity::Rare, ItemRarity::Unique] {
                let got = tweaks(category, rarity, "x");

                assert!(
                    !(got.cluster_jewel_rules && got.flask_rules),
                    "{category:?} {rarity:?}"
                );
            }
        }
    }

    #[test]
    fn a_unique_hides_its_rune_filters() {
        // Filtering on them searches for a unique someone else socketed the
        // same way, which is a rarer item than the one being priced.
        assert!(tweaks(ItemCategory::BodyArmour, ItemRarity::Unique, "Kaom's Heart").hide_augments);
    }

    #[test]
    fn the_two_uniques_bought_for_their_runes_keep_them() {
        // Hiding their rune filters would hide the only thing that sets one
        // apart.
        for name in ["Morior Invictus", "Darkness Enthroned"] {
            assert!(
                !tweaks(ItemCategory::BodyArmour, ItemRarity::Unique, name).hide_augments,
                "{name}"
            );
        }
    }

    #[test]
    fn a_rare_keeps_its_rune_filters() {
        assert!(!tweaks(ItemCategory::BodyArmour, ItemRarity::Rare, "x").hide_augments);
    }

    #[test]
    fn an_item_with_an_empty_slot_offers_the_empty_modifier_filter() {
        let got = final_filter_tweaks(Some(ItemCategory::Ring), Some(ItemRarity::Rare), "x", true);

        assert!(got.show_empty_modifier);
    }

    #[test]
    fn a_full_item_does_not_offer_it() {
        assert!(!tweaks(ItemCategory::Ring, ItemRarity::Rare, "x").show_empty_modifier);
    }

    #[test]
    fn an_item_with_no_category_gets_no_rule_set() {
        let got = final_filter_tweaks(None, Some(ItemRarity::Rare), "x", false);

        assert_eq!(got, FinalTweaks::default());
    }
}
