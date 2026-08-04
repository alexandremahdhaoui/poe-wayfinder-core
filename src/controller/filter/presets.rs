//! Ready made filter sets for the searches people actually run.
//!
//! Ported from `create-presets.ts` and the gem and trial halves of
//! `create-item-filters.ts`.
//!
//! # Why presets exist
//!
//! The default query is built from the item in hand. That is right for a rare,
//! where the modifiers are the item. It is wrong for a gem, where the level
//! and quality are the whole price and the modifiers do not exist.
//!
//! A preset says which handful of filters matter for a kind of item, so the
//! user does not switch six things off before searching.

use crate::types::category::ItemCategory;
use crate::types::item::ParsedItem;
use crate::types::query::{Range, TypeFilters};

/// A named set of filters for one kind of item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// A skill or support gem. Level and quality are the price.
    Gem,
    /// An uncut gem. Only its level matters.
    UncutGem,
    /// A trial item. The area level and the trial count are the price.
    Trials,
    /// Currency, cards and anything else priced purely by name.
    ByName,
    /// The normal case: an item priced by its modifiers.
    Modifiers,
}

/// Which preset an item calls for.
///
/// Ported from `createPresets`. The order matters: a meta gem is a gem before
/// it is anything else, and a trial item is a trial before it is currency.
pub fn preset_for(item: &ParsedItem) -> Preset {
    let name = item.info.reference_name.as_str();

    if name == "Djinn Barya" || name == "Inscribed Ultimatum" {
        return Preset::Trials;
    }

    match item.category {
        Some(ItemCategory::UncutGem) => Preset::UncutGem,
        Some(c) if c.is_gem() => Preset::Gem,
        Some(ItemCategory::Currency)
        | Some(ItemCategory::DivinationCard)
        | Some(ItemCategory::CapturedBeast) => Preset::ByName,
        _ => Preset::Modifiers,
    }
}

/// Whether the item's modifiers should reach the query.
///
/// A gem has none and currency has none. Sending a stat group for either is an
/// empty group, which matches everything and only slows the search.
pub fn uses_modifiers(preset: Preset) -> bool {
    matches!(preset, Preset::Modifiers | Preset::Trials)
}

/// Whether the item's own numbers should reach the query.
///
/// Only a modifier priced item has damage or defences worth filtering on.
pub fn uses_item_properties(preset: Preset) -> bool {
    preset == Preset::Modifiers
}

/// Apply the gem filters.
///
/// Ported from `createGemFilters`. Level and quality are the whole price of a
/// gem, and both are floors because a higher one is strictly better.
pub fn apply_gem_filters(item: &ParsedItem, out: &mut TypeFilters) {
    if let Some(quality) = item.quality {
        out.quality = Range::at_least(f64::from(quality));
    }
}

/// The gem level filter.
///
/// A separate function because gem level sits under the misc filters and
/// quality under the type filters, which is the API's shape and not ours.
pub fn gem_level_filter(item: &ParsedItem, preset: Preset) -> Option<Range> {
    if !matches!(preset, Preset::Gem | Preset::UncutGem) {
        return None;
    }

    let level = item.gem_level?;

    // An uncut gem is bought for exactly the level it is, because that is what
    // it cuts into. A cut gem is bought for at least its level.
    if preset == Preset::UncutGem {
        return Some(Range::exactly(f64::from(level)));
    }

    Some(Range::at_least(f64::from(level)))
}

/// The trial count filter.
///
/// Ported from `createTrialsFilters`. Fewer trials is better, because each one
/// is a room the buyer has to survive.
pub fn trials_filter(item: &ParsedItem, preset: Preset) -> Option<Range> {
    if preset != Preset::Trials {
        return None;
    }

    let count = item.trials?.number_of_trials?;

    Some(Range::at_most(f64::from(count)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::item::{BaseInfo, Trials};

    fn item(category: Option<ItemCategory>) -> ParsedItem {
        ParsedItem {
            category,
            ..ParsedItem::default()
        }
    }

    fn named(name: &str) -> ParsedItem {
        ParsedItem {
            info: BaseInfo {
                reference_name: name.into(),
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        }
    }

    #[test]
    fn a_rare_is_priced_by_its_modifiers() {
        assert_eq!(
            preset_for(&item(Some(ItemCategory::Ring))),
            Preset::Modifiers
        );
    }

    #[test]
    fn a_gem_gets_the_gem_preset() {
        for c in [
            ItemCategory::Gem,
            ItemCategory::SupportGem,
            ItemCategory::MetaGem,
        ] {
            assert_eq!(preset_for(&item(Some(c))), Preset::Gem, "{c:?}");
        }
    }

    #[test]
    fn an_uncut_gem_gets_its_own_preset() {
        // It is bought for the level it cuts into, not for what it does.
        assert_eq!(
            preset_for(&item(Some(ItemCategory::UncutGem))),
            Preset::UncutGem
        );
    }

    #[test]
    fn currency_and_cards_are_priced_by_name() {
        for c in [
            ItemCategory::Currency,
            ItemCategory::DivinationCard,
            ItemCategory::CapturedBeast,
        ] {
            assert_eq!(preset_for(&item(Some(c))), Preset::ByName, "{c:?}");
        }
    }

    #[test]
    fn a_trial_item_is_a_trial_before_it_is_anything_else() {
        // Order matters. Both of these would otherwise fall through to the
        // modifier preset and lose their trial count.
        for name in ["Djinn Barya", "Inscribed Ultimatum"] {
            assert_eq!(preset_for(&named(name)), Preset::Trials, "{name}");
        }
    }

    #[test]
    fn an_item_with_no_category_is_priced_by_its_modifiers() {
        assert_eq!(preset_for(&item(None)), Preset::Modifiers);
    }

    #[test]
    fn a_gem_sends_no_modifier_group() {
        // It has none, and an empty group matches everything and only slows
        // the search.
        assert!(!uses_modifiers(Preset::Gem));
        assert!(!uses_modifiers(Preset::UncutGem));
        assert!(!uses_modifiers(Preset::ByName));
    }

    #[test]
    fn a_modifier_priced_item_sends_its_modifiers() {
        assert!(uses_modifiers(Preset::Modifiers));
    }

    #[test]
    fn a_trial_item_sends_its_modifiers_too() {
        // An Inscribed Ultimatum carries real modifiers alongside its trial
        // count.
        assert!(uses_modifiers(Preset::Trials));
    }

    #[test]
    fn only_a_modifier_priced_item_sends_its_own_numbers() {
        assert!(uses_item_properties(Preset::Modifiers));

        for preset in [
            Preset::Gem,
            Preset::UncutGem,
            Preset::ByName,
            Preset::Trials,
        ] {
            assert!(!uses_item_properties(preset), "{preset:?}");
        }
    }

    #[test]
    fn a_gems_quality_becomes_a_floor() {
        // A higher quality is strictly better, so a ceiling would exclude
        // every gem worth buying.
        let gem = ParsedItem {
            quality: Some(20),
            ..item(Some(ItemCategory::Gem))
        };

        let mut out = TypeFilters::default();
        apply_gem_filters(&gem, &mut out);

        assert_eq!(out.quality.min, Some(20.0));
        assert_eq!(out.quality.max, None);
    }

    #[test]
    fn a_gem_with_no_quality_gets_no_quality_filter() {
        let mut out = TypeFilters::default();
        apply_gem_filters(&item(Some(ItemCategory::Gem)), &mut out);

        assert!(out.quality.is_empty());
    }

    #[test]
    fn a_cut_gems_level_becomes_a_floor() {
        let gem = ParsedItem {
            gem_level: Some(20),
            ..item(Some(ItemCategory::Gem))
        };

        let got = gem_level_filter(&gem, Preset::Gem).unwrap();

        assert_eq!(got.min, Some(20.0));
        assert_eq!(got.max, None);
    }

    #[test]
    fn an_uncut_gems_level_is_pinned_exactly() {
        // It is bought for the level it cuts into. A floor would return every
        // higher one, which costs more and is a different purchase.
        let gem = ParsedItem {
            gem_level: Some(15),
            ..item(Some(ItemCategory::UncutGem))
        };

        let got = gem_level_filter(&gem, Preset::UncutGem).unwrap();

        assert_eq!(got.min, Some(15.0));
        assert_eq!(got.max, Some(15.0));
    }

    #[test]
    fn a_non_gem_gets_no_level_filter() {
        let ring = ParsedItem {
            gem_level: Some(20),
            ..item(Some(ItemCategory::Ring))
        };

        assert_eq!(gem_level_filter(&ring, Preset::Modifiers), None);
    }

    #[test]
    fn a_gem_with_no_level_gets_no_level_filter() {
        assert_eq!(
            gem_level_filter(&item(Some(ItemCategory::Gem)), Preset::Gem),
            None
        );
    }

    #[test]
    fn fewer_trials_is_better_so_the_filter_is_a_ceiling() {
        // Each trial is a room the buyer has to survive. A floor would return
        // every worse one.
        let barya = ParsedItem {
            trials: Some(Trials {
                number_of_trials: Some(4),
                ultimatum_hint: None,
            }),
            ..named("Djinn Barya")
        };

        let got = trials_filter(&barya, Preset::Trials).unwrap();

        assert_eq!(got.max, Some(4.0));
        assert_eq!(got.min, None);
    }

    #[test]
    fn a_non_trial_item_gets_no_trial_filter() {
        let ring = ParsedItem {
            trials: Some(Trials {
                number_of_trials: Some(4),
                ultimatum_hint: None,
            }),
            ..item(Some(ItemCategory::Ring))
        };

        assert_eq!(trials_filter(&ring, Preset::Modifiers), None);
    }

    #[test]
    fn a_trial_item_with_no_count_gets_no_trial_filter() {
        assert_eq!(trials_filter(&named("Djinn Barya"), Preset::Trials), None);
    }
}
