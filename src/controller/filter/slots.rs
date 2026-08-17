use crate::controller::parse::shared::modifiers::ParsedModifier;
use crate::types::category::ItemCategory;
use crate::types::item::{ItemRarity, ParsedItem};
use crate::types::modifier::{Generation, ModifierType};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotCount {
    pub prefixes: usize,
    pub suffixes: usize,
}

impl SlotCount {
    pub fn total(self) -> usize {
        self.prefixes + self.suffixes
    }
}

pub fn explicit_modifier_count(modifiers: &[ParsedModifier]) -> SlotCount {
    use crate::controller::aggregate::is_explicit_family;

    let mut out = SlotCount::default();

    for modifier in modifiers {
        if !modifier.info.kind.is_some_and(is_explicit_family) {
            continue;
        }

        match modifier.info.generation {
            Some(Generation::Prefix) => out.prefixes += 1,
            Some(Generation::Suffix) => out.suffixes += 1,
            _ => {}
        }
    }

    out
}

pub fn max_modifiers_of_type(category: Option<ItemCategory>, rarity: Option<ItemRarity>) -> usize {
    match rarity {
        Some(ItemRarity::Normal) => 0,
        Some(ItemRarity::Magic) => 1,
        _ => match category {
            Some(
                ItemCategory::Jewel
                | ItemCategory::AbyssJewel
                | ItemCategory::Tablet
                | ItemCategory::Relic
                | ItemCategory::SanctumRelic,
            ) => 2,
            _ => 3,
        },
    }
}

pub const PREFIX_ALLOWED: &str = "# Prefix Modifier allowed";

pub const SUFFIX_ALLOWED: &str = "# Suffix Modifier allowed";

pub fn max_modifiers_by_slot(
    category: Option<ItemCategory>,
    rarity: Option<ItemRarity>,
    stats: &[(String, f64)],
) -> SlotCount {
    let base = max_modifiers_of_type(category, rarity) as f64;

    let mut prefixes = base;
    let mut suffixes = base;

    for (reference, roll) in stats {
        if reference == PREFIX_ALLOWED {
            prefixes += roll;
        } else if reference == SUFFIX_ALLOWED {
            suffixes += roll;
        }
    }

    SlotCount {
        prefixes: prefixes.max(0.0) as usize,
        suffixes: suffixes.max(0.0) as usize,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptySlot {
    Prefix,
    Suffix,
    Either,
}

impl EmptySlot {
    pub fn trade_option(self) -> f64 {
        match self {
            EmptySlot::Either => 0.0,
            EmptySlot::Prefix => 1.0,
            EmptySlot::Suffix => 2.0,
        }
    }
}

pub fn empty_slot(item: &ParsedItem) -> Option<EmptySlot> {
    if !item.is_modifiable() || item.category == Some(ItemCategory::Map) {
        return None;
    }

    if !matches!(
        item.rarity,
        Some(ItemRarity::Rare) | Some(ItemRarity::Magic)
    ) {
        return None;
    }

    let counts = explicit_modifier_count(&item.modifiers);

    if counts.total() == 0 {
        return None;
    }

    let allowances: Vec<(String, f64)> = item
        .modifiers
        .iter()
        .flat_map(|m| &m.stats)
        .filter(|s| s.reference == PREFIX_ALLOWED || s.reference == SUFFIX_ALLOWED)
        .map(|s| {
            (
                s.reference.clone(),
                s.roll.map(|r| r.value).unwrap_or_default(),
            )
        })
        .collect();

    let max = max_modifiers_by_slot(item.category, item.rarity, &allowances);

    let prefix_open = counts.prefixes < max.prefixes;
    let suffix_open = counts.suffixes < max.suffixes;

    match (prefix_open, suffix_open) {
        (true, true) => Some(EmptySlot::Either),
        (true, false) => Some(EmptySlot::Prefix),
        (false, true) => Some(EmptySlot::Suffix),
        (false, false) => None,
    }
}

pub fn likely_finished(item: &ParsedItem) -> bool {
    if !item.is_modifiable() {
        return true;
    }

    let counts = explicit_modifier_count(&item.modifiers);

    if counts.total() == 0 {
        return false;
    }

    let max = max_modifiers_of_type(item.category, item.rarity);

    counts.prefixes >= max && counts.suffixes >= max
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassiveBound {
    Exact,
    AtLeast,
    AtMost,
    UpTo(u32),
}

pub fn passive_bound(count: u32) -> PassiveBound {
    match count {
        4 => PassiveBound::UpTo(5),
        5 => PassiveBound::Exact,
        3 | 6 | 10 | 11 | 12 => PassiveBound::AtLeast,
        _ => PassiveBound::AtMost,
    }
}

pub const ADDS_PASSIVES: &str = "Adds # Passive Skills";

pub const ENKINDLING: &str = "Gains no Charges during Flask Effect";

pub fn is_cluster_socket_stat(reference: &str) -> bool {
    reference == "# Added Passive Skills are Jewel Sockets"
}

pub fn flask_enchant_is_useful(references: &[String]) -> bool {
    references
        .iter()
        .any(|r| r == "Gains no Charges during Flask Effect")
}

pub fn is_hidden_by_default(kind: ModifierType) -> bool {
    matches!(kind, ModifierType::Augment | ModifierType::AddedAugment)
}

#[cfg(test)]
mod slot_budget_tests {
    use super::*;
    use crate::controller::parse::shared::modifiers::ParsedModifier;
    use crate::types::modifier::{Generation, ModifierInfo};
    use crate::types::stat::{ParsedStat, StatRoll};

    fn modifier(generation: Generation, reference: &str, roll: Option<f64>) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(ModifierType::Explicit),
                generation: Some(generation),
                ..ModifierInfo::default()
            },
            stats: vec![ParsedStat {
                reference: reference.into(),
                matched: reference.into(),
                roll: roll.map(|value| StatRoll {
                    value,
                    min: value,
                    max: value,
                    ..StatRoll::default()
                }),
            }],
        }
    }

    fn rare_with(modifiers: Vec<ParsedModifier>) -> ParsedItem {
        ParsedItem {
            rarity: Some(ItemRarity::Rare),
            category: Some(ItemCategory::Ring),
            modifiers,
            info: crate::types::item::BaseInfo {
                craftable: true,
                ..crate::types::item::BaseInfo::default()
            },
            ..ParsedItem::default()
        }
    }

    #[test]
    fn a_rare_with_three_prefixes_has_no_prefix_room() {
        let item = rare_with(vec![
            modifier(Generation::Prefix, "# to maximum Life", Some(79.0)),
            modifier(Generation::Prefix, "# to maximum Mana", Some(61.0)),
            modifier(Generation::Prefix, "# to Armour", Some(50.0)),
            modifier(Generation::Suffix, "#% to Fire Resistance", Some(45.0)),
        ]);

        assert_eq!(empty_slot(&item), Some(EmptySlot::Suffix));
    }

    #[test]
    fn a_crafted_prefix_allowance_opens_a_fourth_slot() {
        let item = rare_with(vec![
            modifier(Generation::Prefix, "# to maximum Life", Some(79.0)),
            modifier(Generation::Prefix, "# to maximum Mana", Some(61.0)),
            modifier(Generation::Prefix, "# to Armour", Some(50.0)),
            modifier(Generation::Suffix, PREFIX_ALLOWED, Some(1.0)),
            modifier(Generation::Suffix, "#% to Fire Resistance", Some(45.0)),
            modifier(Generation::Suffix, "#% to Cold Resistance", Some(45.0)),
        ]);

        assert_eq!(empty_slot(&item), Some(EmptySlot::Prefix));
    }

    #[test]
    fn a_negative_allowance_closes_a_slot_that_looked_open() {
        let item = rare_with(vec![
            modifier(Generation::Prefix, "# to maximum Life", Some(79.0)),
            modifier(Generation::Prefix, PREFIX_ALLOWED, Some(-1.0)),
            modifier(Generation::Suffix, "#% to Fire Resistance", Some(45.0)),
            modifier(Generation::Suffix, "#% to Cold Resistance", Some(45.0)),
            modifier(Generation::Suffix, "#% to Lightning Resistance", Some(45.0)),
        ]);

        assert_eq!(empty_slot(&item), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::item::BaseInfo;
    use crate::types::modifier::ModifierInfo;

    fn modifier(kind: ModifierType, generation: Option<Generation>) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(kind),
                generation,
                ..ModifierInfo::default()
            },
            stats: Vec::new(),
        }
    }

    fn rare(modifiers: Vec<ParsedModifier>) -> ParsedItem {
        ParsedItem {
            rarity: Some(ItemRarity::Rare),
            category: Some(ItemCategory::Ring),
            modifiers,
            info: BaseInfo {
                craftable: true,
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        }
    }

    #[test]
    fn prefixes_and_suffixes_are_counted_separately() {
        let got = explicit_modifier_count(&[
            modifier(ModifierType::Explicit, Some(Generation::Prefix)),
            modifier(ModifierType::Explicit, Some(Generation::Suffix)),
            modifier(ModifierType::Explicit, Some(Generation::Suffix)),
        ]);

        assert_eq!(got.prefixes, 1);
        assert_eq!(got.suffixes, 2);
        assert_eq!(got.total(), 3);
    }

    #[test]
    fn a_crafted_modifier_counts_toward_the_slots() {
        let got =
            explicit_modifier_count(&[modifier(ModifierType::Crafted, Some(Generation::Prefix))]);

        assert_eq!(got.prefixes, 1);
    }

    #[test]
    fn an_implicit_or_a_rune_occupies_no_slot() {
        let got = explicit_modifier_count(&[
            modifier(ModifierType::Implicit, Some(Generation::Prefix)),
            modifier(ModifierType::Augment, Some(Generation::Suffix)),
        ]);

        assert_eq!(got.total(), 0);
    }

    #[test]
    fn a_modifier_with_no_generation_is_not_attributed() {
        let got = explicit_modifier_count(&[modifier(ModifierType::Explicit, None)]);

        assert_eq!(got.total(), 0);
    }

    #[test]
    fn a_rare_takes_three_of_each() {
        assert_eq!(
            max_modifiers_of_type(Some(ItemCategory::Ring), Some(ItemRarity::Rare)),
            3
        );
    }

    #[test]
    fn a_jewel_takes_two_of_each() {
        for c in [
            ItemCategory::Jewel,
            ItemCategory::AbyssJewel,
            ItemCategory::Tablet,
            ItemCategory::Relic,
            ItemCategory::SanctumRelic,
        ] {
            assert_eq!(
                max_modifiers_of_type(Some(c), Some(ItemRarity::Rare)),
                2,
                "{c:?}"
            );
        }
    }

    #[test]
    fn a_magic_item_takes_one_of_each_and_a_normal_none() {
        assert_eq!(
            max_modifiers_of_type(Some(ItemCategory::Ring), Some(ItemRarity::Magic)),
            1
        );
        assert_eq!(
            max_modifiers_of_type(Some(ItemCategory::Ring), Some(ItemRarity::Normal)),
            0
        );
    }

    #[test]
    fn an_item_with_room_in_both_slots_reports_either() {
        let item = rare(vec![
            modifier(ModifierType::Explicit, Some(Generation::Prefix)),
            modifier(ModifierType::Explicit, Some(Generation::Suffix)),
        ]);

        assert_eq!(empty_slot(&item), Some(EmptySlot::Either));
    }

    #[test]
    fn an_item_with_full_prefixes_reports_only_a_suffix() {
        let item = rare(vec![
            modifier(ModifierType::Explicit, Some(Generation::Prefix)),
            modifier(ModifierType::Explicit, Some(Generation::Prefix)),
            modifier(ModifierType::Explicit, Some(Generation::Prefix)),
            modifier(ModifierType::Explicit, Some(Generation::Suffix)),
        ]);

        assert_eq!(empty_slot(&item), Some(EmptySlot::Suffix));
    }

    #[test]
    fn a_full_item_reports_no_open_slot() {
        let mut modifiers = Vec::new();

        for _ in 0..3 {
            modifiers.push(modifier(ModifierType::Explicit, Some(Generation::Prefix)));
            modifiers.push(modifier(ModifierType::Explicit, Some(Generation::Suffix)));
        }

        assert_eq!(empty_slot(&rare(modifiers)), None);
    }

    #[test]
    fn a_corrupted_item_reports_no_open_slot() {
        let item = ParsedItem {
            is_corrupted: true,
            ..rare(vec![modifier(
                ModifierType::Explicit,
                Some(Generation::Prefix),
            )])
        };

        assert_eq!(empty_slot(&item), None);
    }

    #[test]
    fn a_unique_reports_no_open_slot() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            ..rare(vec![modifier(
                ModifierType::Explicit,
                Some(Generation::Prefix),
            )])
        };

        assert_eq!(empty_slot(&item), None);
    }

    #[test]
    fn a_map_reports_no_open_slot() {
        let item = ParsedItem {
            category: Some(ItemCategory::Map),
            ..rare(vec![modifier(
                ModifierType::Explicit,
                Some(Generation::Prefix),
            )])
        };

        assert_eq!(empty_slot(&item), None);
    }

    #[test]
    fn an_item_with_no_attributed_modifiers_reports_nothing() {
        assert_eq!(
            empty_slot(&rare(vec![modifier(ModifierType::Explicit, None)])),
            None
        );
    }

    #[test]
    fn a_full_rare_is_finished() {
        let mut modifiers = Vec::new();

        for _ in 0..3 {
            modifiers.push(modifier(ModifierType::Explicit, Some(Generation::Prefix)));
            modifiers.push(modifier(ModifierType::Explicit, Some(Generation::Suffix)));
        }

        assert!(likely_finished(&rare(modifiers)));
    }

    #[test]
    fn a_rare_with_room_is_not_finished() {
        assert!(!likely_finished(&rare(vec![modifier(
            ModifierType::Explicit,
            Some(Generation::Prefix)
        )])));
    }

    #[test]
    fn a_corrupted_item_is_finished_whatever_it_carries() {
        let item = ParsedItem {
            is_corrupted: true,
            ..rare(vec![modifier(
                ModifierType::Explicit,
                Some(Generation::Prefix),
            )])
        };

        assert!(likely_finished(&item));
    }

    #[test]
    fn each_cluster_passive_count_gets_the_bound_the_market_uses() {
        assert_eq!(passive_bound(4), PassiveBound::UpTo(5));
        assert_eq!(passive_bound(5), PassiveBound::Exact);

        for n in [3, 6, 10, 11, 12] {
            assert_eq!(passive_bound(n), PassiveBound::AtLeast, "{n}");
        }

        for n in [2, 8, 9] {
            assert_eq!(passive_bound(n), PassiveBound::AtMost, "{n}");
        }
    }

    #[test]
    fn a_cluster_socket_stat_is_recognised() {
        assert!(is_cluster_socket_stat(
            "# Added Passive Skills are Jewel Sockets"
        ));
        assert!(!is_cluster_socket_stat("Adds # Passive Skills"));
    }

    #[test]
    fn a_flask_enchant_is_only_useful_alongside_an_enkindling_line() {
        assert!(!flask_enchant_is_useful(&["# to Life".to_string()]));
        assert!(flask_enchant_is_useful(&[
            "Gains no Charges during Flask Effect".to_string()
        ]));
    }

    #[test]
    fn a_rune_effect_is_hidden_by_default() {
        assert!(is_hidden_by_default(ModifierType::Augment));
        assert!(is_hidden_by_default(ModifierType::AddedAugment));
        assert!(!is_hidden_by_default(ModifierType::Explicit));
        assert!(!is_hidden_by_default(ModifierType::Implicit));
    }
}
