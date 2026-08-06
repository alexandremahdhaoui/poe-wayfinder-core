//! Filters that exclude rather than require.
//!
//! Ported from `statToNotFilter` and `noSourcePseudoToFilter` in Awakened PoE
//! Trade's `web/price-check/filters/pseudo/utils.ts` and
//! `item-property.ts`, plus the two rules that use them,
//! `applyFlaskHybridMod` and `valdoBadMods`.
//!
//! # Why an exclusion is not the same as a missing filter
//!
//! Leaving a modifier off the query means "I do not care". Excluding it means
//! "this must not be there". The two price very differently.
//!
//! A magic flask with increased charge recovery and no increased effect is
//! worth what it is worth because it has room for the effect modifier. Search
//! without the exclusion and every listing that already has both comes back,
//! which is a different and pricier item.
//!
//! A Valdo's map that does not send the player to the Void on death is worth
//! far more than one that does. That is a modifier the item does not have, and
//! no filter built from what an item has can express it.

use crate::types::item::{ItemRarity, ParsedItem};
use crate::types::query::{Range, StatFilter, StatGroup, StatGroupKind};

/// The stat a magic flask must not already carry.
pub const INCREASED_EFFECT: &str = "#% increased effect";

/// The stat that makes a magic flask worth excluding on.
pub const INCREASED_CHARGE_RECOVERY: &str = "#% increased Charge Recovery";

/// The stats a Valdo's map is worth much less for carrying.
///
/// One today. Kept as a list because the reference keeps it as one and a
/// league can add to it.
pub const VALDO_LETHAL_STATS: [&str; 1] = ["Players who Die in area are sent to the Void"];

/// A group that excludes everything in it.
///
/// The trade site takes exclusions as a group of type `not` rather than as a
/// flag on a filter, so an exclusion cannot be mixed into the `and` group.
pub fn not_group(ids: Vec<String>) -> Option<StatGroup> {
    if ids.is_empty() {
        return None;
    }

    Some(StatGroup {
        kind: StatGroupKind::Not,
        value: Range::default(),
        filters: ids
            .iter()
            .map(|id| StatFilter::range(id, Range::default()))
            .collect(),
        disabled: false,
    })
}

/// Whether a magic flask should exclude the increased effect modifier.
///
/// See <https://github.com/SnosMe/awakened-poe-trade/issues/758>, which is
/// where the reference's rule comes from.
///
/// Only for a magic flask that has charge recovery and does not already have
/// increased effect. A rare flask has both suffixes filled and the question
/// does not arise.
pub fn flask_excludes_increased_effect(item: &ParsedItem, references: &[String]) -> bool {
    if item.rarity != Some(ItemRarity::Magic) {
        return false;
    }

    let has = |wanted: &str| references.iter().any(|r| r == wanted);

    has(INCREASED_CHARGE_RECOVERY) && !has(INCREASED_EFFECT)
}

/// The lethal modifiers a Valdo's map should exclude.
///
/// Only the ones it does not already have. Excluding a modifier the item
/// carries would search for something that cannot exist.
pub fn valdo_bad_mods(item: &ParsedItem, references: &[String]) -> Vec<&'static str> {
    if item.map_completion_reward.is_none() {
        return Vec::new();
    }

    VALDO_LETHAL_STATS
        .into_iter()
        .filter(|lethal| !references.iter().any(|r| r == lethal))
        .collect()
}

/// The memory strands filter for an accessory.
///
/// Ported from `filterMemoryStrands`. The count is printed on its own line and
/// is not a modifier, so it never reaches the query any other way.
///
/// Off below sixty. A low strand count is not what anybody searches for, and
/// requiring it excludes every better item.
pub fn memory_strands_filter(item: &ParsedItem) -> Option<StatFilter> {
    let strands = item.memory_strands?;

    let mut filter = StatFilter::range(
        "item.memory_strands",
        Range {
            min: Some(strands as f64),
            max: None,
        },
    );

    filter.disabled = strands < 60;

    Some(filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::category::ItemCategory;

    fn flask(rarity: ItemRarity) -> ParsedItem {
        ParsedItem {
            rarity: Some(rarity),
            category: Some(ItemCategory::Flask),
            ..ParsedItem::default()
        }
    }

    fn refs(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    // -----------------------------------------------------------------
    // Flask hybrid modifier
    // -----------------------------------------------------------------

    #[test]
    fn a_magic_flask_with_charge_recovery_excludes_increased_effect() {
        // It is worth what it is worth because it has room for the effect
        // modifier. Without the exclusion every listing that already has both
        // comes back, which is a pricier item.
        let got = flask_excludes_increased_effect(
            &flask(ItemRarity::Magic),
            &refs(&[INCREASED_CHARGE_RECOVERY]),
        );

        assert!(got);
    }

    #[test]
    fn a_flask_that_already_has_the_effect_excludes_nothing() {
        let got = flask_excludes_increased_effect(
            &flask(ItemRarity::Magic),
            &refs(&[INCREASED_CHARGE_RECOVERY, INCREASED_EFFECT]),
        );

        assert!(!got);
    }

    #[test]
    fn a_rare_flask_excludes_nothing() {
        // Both suffixes are filled and the question does not arise.
        let got = flask_excludes_increased_effect(
            &flask(ItemRarity::Rare),
            &refs(&[INCREASED_CHARGE_RECOVERY]),
        );

        assert!(!got);
    }

    #[test]
    fn a_flask_without_charge_recovery_excludes_nothing() {
        let got = flask_excludes_increased_effect(&flask(ItemRarity::Magic), &refs(&[]));

        assert!(!got);
    }

    // -----------------------------------------------------------------
    // Valdo lethal modifiers
    // -----------------------------------------------------------------

    fn valdo() -> ParsedItem {
        ParsedItem {
            category: Some(ItemCategory::Map),
            map_completion_reward: Some("Headhunter".into()),
            ..ParsedItem::default()
        }
    }

    #[test]
    fn a_valdo_map_excludes_the_lethal_modifier_it_does_not_have() {
        // A map that sends the player to the Void on death is worth far less,
        // and no filter built from what an item has can say so.
        let got = valdo_bad_mods(&valdo(), &refs(&[]));

        assert_eq!(got, VALDO_LETHAL_STATS.to_vec());
    }

    #[test]
    fn a_valdo_map_that_has_the_lethal_modifier_excludes_nothing() {
        // Excluding a modifier the item carries searches for something that
        // cannot exist.
        let got = valdo_bad_mods(&valdo(), &refs(&VALDO_LETHAL_STATS));

        assert!(got.is_empty());
    }

    #[test]
    fn an_ordinary_map_excludes_nothing() {
        let mut item = valdo();
        item.map_completion_reward = None;

        assert!(valdo_bad_mods(&item, &refs(&[])).is_empty());
    }

    // -----------------------------------------------------------------
    // Not groups
    // -----------------------------------------------------------------

    #[test]
    fn a_not_group_excludes_every_id_in_it() {
        let got = not_group(vec!["explicit.a".into(), "explicit.b".into()]).unwrap();

        assert_eq!(got.kind, StatGroupKind::Not);
        assert_eq!(got.filters.len(), 2);
    }

    #[test]
    fn an_empty_not_group_is_no_group() {
        // A `not` group with nothing in it excludes nothing and still costs a
        // round trip through the site's query planner.
        assert_eq!(not_group(Vec::new()), None);
    }

    #[test]
    fn a_not_group_is_enabled() {
        // An exclusion the user has to switch on is an exclusion nobody uses.
        let got = not_group(vec!["explicit.a".into()]).unwrap();

        assert!(!got.disabled);
        assert!(!got.filters[0].disabled);
    }

    // -----------------------------------------------------------------
    // Memory strands
    // -----------------------------------------------------------------

    fn amulet(strands: Option<u32>) -> ParsedItem {
        ParsedItem {
            category: Some(ItemCategory::Amulet),
            memory_strands: strands,
            ..ParsedItem::default()
        }
    }

    #[test]
    fn an_accessory_with_strands_gets_a_filter() {
        let got = memory_strands_filter(&amulet(Some(80))).unwrap();

        assert_eq!(got.id, "item.memory_strands");
        assert_eq!(got.range.min, Some(80.0));
    }

    #[test]
    fn a_high_strand_count_is_on_by_default() {
        assert!(!memory_strands_filter(&amulet(Some(80))).unwrap().disabled);
    }

    #[test]
    fn a_low_strand_count_starts_off() {
        // Nobody searches for a low count, and requiring it excludes every
        // better item.
        assert!(memory_strands_filter(&amulet(Some(20))).unwrap().disabled);
    }

    #[test]
    fn sixty_is_the_line() {
        assert!(memory_strands_filter(&amulet(Some(59))).unwrap().disabled);
        assert!(!memory_strands_filter(&amulet(Some(60))).unwrap().disabled);
    }

    #[test]
    fn an_accessory_with_no_strands_gets_no_filter() {
        assert_eq!(memory_strands_filter(&amulet(None)), None);
    }
}
