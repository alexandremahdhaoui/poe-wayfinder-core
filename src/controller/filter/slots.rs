//! Modifier slot counting and the per category filter rules.
//!
//! Ported from `explicitModifierCount`, `itemBaseMaxModifiersOfType`,
//! `itemMaxModifiersBySlot`, `showHasEmptyModifier`, `applyClusterJewelRules`,
//! `applyFlaskRules`, `hideAllAugments` and `likelyFinishedItem` in
//! `create-stat-filters.ts` and `common.ts`.
//!
//! # Why an empty slot is worth searching for
//!
//! A rare with five modifiers and one open prefix is worth more than the same
//! rare with six, because the buyer can craft into the gap. The trade site has
//! a filter for it and it is invisible from the item text alone: you have to
//! count what is there and know what the base allows.

use crate::controller::parse::shared::modifiers::ParsedModifier;
use crate::types::category::ItemCategory;
use crate::types::item::{ItemRarity, ParsedItem};
use crate::types::modifier::{Generation, ModifierType};

/// How many prefixes and suffixes an item carries.
///
/// Ported from `explicitModifierCount`. Only the modifiers the trade site
/// counts as explicit. A rune or an implicit occupies no prefix or suffix
/// slot, so counting it would report a full item as full when it is not.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotCount {
    pub prefixes: usize,
    pub suffixes: usize,
}

impl SlotCount {
    /// Prefixes plus suffixes.
    pub fn total(self) -> usize {
        self.prefixes + self.suffixes
    }
}

/// Count the explicit modifiers by generation.
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
            // Without Advanced Item Description the game prints no generation,
            // so the modifier cannot be attributed to a slot. Guessing would
            // report an open slot that is not there.
            _ => {}
        }
    }

    out
}

/// How many modifiers of each kind a base can carry.
///
/// Ported from `itemBaseMaxModifiersOfType`. A jewel takes two of each and
/// most other rares take three.
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

/// Which slot an item has open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptySlot {
    Prefix,
    Suffix,
    /// One of each, or the caller does not care which.
    Either,
}

/// Whether the item has an open slot worth searching for.
///
/// Ported from `showHasEmptyModifier`.
///
/// Only a rare or a magic item that can still be crafted. A unique has no
/// slots to open, a normal item has no modifiers, and a corrupted item cannot
/// use the space.
///
/// Returns None when the counts are unknown, which is what happens without
/// Advanced Item Description. Reporting an open slot the item may not have
/// would send the buyer a filter that finds the wrong items.
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

    // Nothing attributed to a slot means the game printed no generations, so
    // the count says nothing.
    if counts.total() == 0 {
        return None;
    }

    let max = max_modifiers_of_type(item.category, item.rarity);

    let prefix_open = counts.prefixes < max;
    let suffix_open = counts.suffixes < max;

    match (prefix_open, suffix_open) {
        (true, true) => Some(EmptySlot::Either),
        (true, false) => Some(EmptySlot::Prefix),
        (false, true) => Some(EmptySlot::Suffix),
        (false, false) => None,
    }
}

/// Whether an item is finished and will not be crafted further.
///
/// Ported from `likelyFinishedItem`. A rare with every slot full is what a
/// buyer uses as it is, so its rolls are pinned rather than given room.
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

/// The bound a cluster jewel's passive count deserves.
///
/// Ported from `applyClusterJewelRules`.
///
/// Cluster jewels are priced by how many passives they add, and the good
/// numbers are not a range. Four is bought as "four or five". Five is bought
/// as exactly five. Three, six, ten, eleven and twelve are bought as "at
/// least", because those are the breakpoints notables sit on.
///
/// Two, eight and nine are bought as "at most", because past them the jewel
/// costs points without adding a notable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassiveBound {
    /// Exactly this many.
    Exact,
    /// This many or more.
    AtLeast,
    /// This many or fewer.
    AtMost,
    /// Between this and the given ceiling.
    UpTo(u32),
}

/// Which bound a passive count deserves.
pub fn passive_bound(count: u32) -> PassiveBound {
    match count {
        4 => PassiveBound::UpTo(5),
        5 => PassiveBound::Exact,
        3 | 6 | 10 | 11 | 12 => PassiveBound::AtLeast,
        _ => PassiveBound::AtMost,
    }
}

/// Whether a stat is a cluster jewel's socket count.
///
/// It is fixed by the base and filtering on it narrows nothing.
pub fn is_cluster_socket_stat(reference: &str) -> bool {
    reference == "# Added Passive Skills are Jewel Sockets"
}

/// Whether a flask enchant is worth showing.
///
/// Ported from `applyFlaskRules`. A harvest or instilling enchant is on almost
/// every flask and filtering on it returns nothing useful. An enkindling
/// orb's line is the exception, because it changes what the flask does.
pub fn flask_enchant_is_useful(references: &[String]) -> bool {
    references
        .iter()
        .any(|r| r == "Gains no Charges during Flask Effect")
}

/// Whether a modifier type should be hidden by default.
///
/// Ported from `hideAllAugments`. A rune's effect is a property of the rune
/// and not of the item, so a buyer searching for the item does not want it.
pub fn is_hidden_by_default(kind: ModifierType) -> bool {
    matches!(kind, ModifierType::Augment | ModifierType::AddedAugment)
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
        // The trade site indexes it as explicit and it occupies a slot.
        let got =
            explicit_modifier_count(&[modifier(ModifierType::Crafted, Some(Generation::Prefix))]);

        assert_eq!(got.prefixes, 1);
    }

    #[test]
    fn an_implicit_or_a_rune_occupies_no_slot() {
        // Counting one would report a full item as full when it is not.
        let got = explicit_modifier_count(&[
            modifier(ModifierType::Implicit, Some(Generation::Prefix)),
            modifier(ModifierType::Augment, Some(Generation::Suffix)),
        ]);

        assert_eq!(got.total(), 0);
    }

    #[test]
    fn a_modifier_with_no_generation_is_not_attributed() {
        // Without Advanced Item Description the game prints none, and guessing
        // would report an open slot that is not there.
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
        // It cannot use the space, so the filter would be a lie.
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
        // Without Advanced Item Description the count says nothing, and a
        // filter built on it would find the wrong items.
        assert_eq!(
            empty_slot(&rare(vec![modifier(ModifierType::Explicit, None)])),
            None
        );
    }

    #[test]
    fn a_full_rare_is_finished() {
        // A buyer uses it as it is, so its rolls are pinned rather than given
        // room.
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
        // The good numbers are not a range. These are the breakpoints notables
        // sit on.
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
        // It is fixed by the base and filtering on it narrows nothing.
        assert!(is_cluster_socket_stat(
            "# Added Passive Skills are Jewel Sockets"
        ));
        assert!(!is_cluster_socket_stat("Adds # Passive Skills"));
    }

    #[test]
    fn a_flask_enchant_is_only_useful_alongside_an_enkindling_line() {
        // Harvest and instilling enchants are on almost every flask.
        assert!(!flask_enchant_is_useful(&["# to Life".to_string()]));
        assert!(flask_enchant_is_useful(&[
            "Gains no Charges during Flask Effect".to_string()
        ]));
    }

    #[test]
    fn a_rune_effect_is_hidden_by_default() {
        // It belongs to the rune and not the item, so a buyer searching for
        // the item does not want it.
        assert!(is_hidden_by_default(ModifierType::Augment));
        assert!(is_hidden_by_default(ModifierType::AddedAugment));
        assert!(!is_hidden_by_default(ModifierType::Explicit));
        assert!(!is_hidden_by_default(ModifierType::Implicit));
    }
}
