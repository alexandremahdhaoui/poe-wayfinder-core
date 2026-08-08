use crate::types::category::ItemCategory;
use crate::types::query::StatFilter;

pub fn max_useful_item_level(category: Option<ItemCategory>) -> u32 {
    match category {
        Some(ItemCategory::Wand | ItemCategory::Staff) => 81,
        Some(ItemCategory::Relic) => 80,
        Some(ItemCategory::Tablet | ItemCategory::Jewel | ItemCategory::Map) => 1,
        _ => 82,
    }
}

pub fn enable_all(filters: &mut [StatFilter]) {
    for filter in filters.iter_mut() {
        if !filter.hidden {
            filter.disabled = false;
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FinalTweaks {
    pub cluster_jewel_rules: bool,
    pub flask_rules: bool,
    pub hide_augments: bool,
    pub show_empty_modifier: bool,
}

const AUGMENT_UNIQUES: &[&str] = &["Morior Invictus", "Darkness Enthroned"];

pub fn final_filter_tweaks(
    category: Option<ItemCategory>,
    rarity: Option<crate::types::item::ItemRarity>,
    reference_name: &str,
    has_empty_slot: bool,
) -> FinalTweaks {
    use crate::types::item::ItemRarity;

    let unique = rarity == Some(ItemRarity::Unique);

    FinalTweaks {
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
        assert_eq!(max_useful_item_level(None), 82);
    }

    #[test]
    fn a_wand_and_a_staff_cap_at_eighty_one() {
        assert_eq!(max_useful_item_level(Some(ItemCategory::Wand)), 81);
        assert_eq!(max_useful_item_level(Some(ItemCategory::Staff)), 81);
    }

    #[test]
    fn a_relic_caps_at_eighty() {
        assert_eq!(max_useful_item_level(Some(ItemCategory::Relic)), 80);
    }

    #[test]
    fn a_base_whose_level_says_nothing_caps_at_one() {
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

    use crate::types::item::ItemRarity;

    fn tweaks(category: ItemCategory, rarity: ItemRarity, name: &str) -> FinalTweaks {
        final_filter_tweaks(Some(category), Some(rarity), name, false)
    }

    #[test]
    fn a_rare_cluster_jewel_gets_the_cluster_rules() {
        assert!(tweaks(ItemCategory::ClusterJewel, ItemRarity::Rare, "x").cluster_jewel_rules);
    }

    #[test]
    fn a_unique_cluster_jewel_does_not() {
        assert!(!tweaks(ItemCategory::ClusterJewel, ItemRarity::Unique, "x").cluster_jewel_rules);
    }

    #[test]
    fn a_flask_gets_the_flask_rules() {
        assert!(tweaks(ItemCategory::Flask, ItemRarity::Magic, "x").flask_rules);
    }

    #[test]
    fn the_two_rule_sets_are_never_both_applied() {
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
        assert!(tweaks(ItemCategory::BodyArmour, ItemRarity::Unique, "Kaom's Heart").hide_augments);
    }

    #[test]
    fn the_two_uniques_bought_for_their_runes_keep_them() {
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
