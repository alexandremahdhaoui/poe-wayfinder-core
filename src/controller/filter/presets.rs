use crate::types::category::ItemCategory;
use crate::types::item::ParsedItem;
use crate::types::query::{Range, TypeFilters};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Gem,
    UncutGem,
    Trials,
    ByName,
    Modifiers,
}

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

pub fn uses_modifiers(preset: Preset) -> bool {
    matches!(preset, Preset::Modifiers | Preset::Trials)
}

pub fn uses_item_properties(preset: Preset) -> bool {
    preset == Preset::Modifiers
}

pub fn apply_gem_filters(item: &ParsedItem, out: &mut TypeFilters) {
    if let Some(quality) = item.quality {
        out.quality = Range::at_least(f64::from(quality));
    }
}

pub fn gem_level_filter(item: &ParsedItem, preset: Preset) -> Option<Range> {
    if !matches!(preset, Preset::Gem | Preset::UncutGem) {
        return None;
    }

    let level = item.gem_level?;

    if preset == Preset::UncutGem {
        return Some(Range::exactly(f64::from(level)));
    }

    Some(Range::at_least(f64::from(level)))
}

pub fn ascendancy_area_level_filter(item: &ParsedItem, preset: Preset) -> Option<Range> {
    use crate::controller::filter::brackets::{
        area_level_by_ascendancy_points, ascendancy_points_by_area_level,
    };

    if preset != Preset::Trials {
        return None;
    }

    let area_level = item.area_level?;
    let name = item.info.reference_name.as_str();
    let points = ascendancy_points_by_area_level(name, area_level);

    if points == 0 {
        return None;
    }

    let floor = area_level_by_ascendancy_points(name, points);

    Some(Range::at_least(f64::from(floor)))
}

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
    fn an_ultimatum_is_searched_at_the_area_level_that_grants_the_same_points() {
        let mut it = item(None);

        it.info.reference_name = "Inscribed Ultimatum".to_string();
        it.area_level = Some(80);

        let got = ascendancy_area_level_filter(&it, Preset::Trials).expect("a range");

        assert_eq!(
            got.min,
            Some(75.0),
            "level 80 grants three points, and 75 is the lowest level that does"
        );
    }

    #[test]
    fn a_barya_has_its_own_table() {
        let mut it = item(None);

        it.info.reference_name = "Djinn Barya".to_string();
        it.area_level = Some(50);

        let got = ascendancy_area_level_filter(&it, Preset::Trials).expect("a range");

        assert_eq!(got.min, Some(45.0), "fifty grants two points, from 45 up");
    }

    #[test]
    fn an_item_that_is_not_a_trial_gets_no_area_level_filter() {
        let mut it = item(None);

        it.area_level = Some(80);

        assert_eq!(
            ascendancy_area_level_filter(&it, Preset::Modifiers),
            None
        );
    }

    #[test]
    fn a_trial_with_no_area_level_printed_gets_no_filter() {
        let mut it = item(None);

        it.info.reference_name = "Inscribed Ultimatum".to_string();

        assert_eq!(ascendancy_area_level_filter(&it, Preset::Trials), None);
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
