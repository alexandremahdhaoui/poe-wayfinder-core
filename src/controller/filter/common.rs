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
}
