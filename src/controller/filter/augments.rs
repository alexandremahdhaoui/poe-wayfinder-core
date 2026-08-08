use crate::types::category::ItemCategory;
use crate::types::query::StatFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct AugmentEffect {
    pub reference: String,
    pub value: f64,
    pub categories: Vec<ItemCategory>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Augment {
    pub reference_name: String,
    pub name: String,
    pub effects: Vec<AugmentEffect>,
}

pub fn effect_for_category(augment: &Augment, category: ItemCategory) -> Option<&AugmentEffect> {
    augment
        .effects
        .iter()
        .find(|e| e.categories.contains(&category))
}

pub fn augment_name<'a>(augments: &'a [Augment], reference: &'a str) -> &'a str {
    augments
        .iter()
        .find(|a| a.reference_name == reference)
        .map_or(reference, |a| a.name.as_str())
}

pub fn preview_filters(
    augment: &Augment,
    category: ItemCategory,
    empty_sockets: u32,
    existing: &[StatFilter],
) -> Option<Vec<StatFilter>> {
    let effect = effect_for_category(augment, category)?;

    if empty_sockets == 0 {
        return None;
    }

    let granted = StatFilter::range(
        &effect.reference,
        crate::types::query::Range::at_least(effect.value * f64::from(empty_sockets)),
    );

    let mut out: Vec<StatFilter> = existing.to_vec();

    if let Some(existing) = out.iter_mut().find(|f| f.id == granted.id) {
        existing.range.min =
            Some(existing.range.min.unwrap_or(0.0) + effect.value * f64::from(empty_sockets));
    } else {
        out.push(granted);
    }

    Some(out)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FilterEdit {
    stored: Vec<StatFilter>,
}

impl FilterEdit {
    pub fn is_applied(&self) -> bool {
        !self.stored.is_empty()
    }

    pub fn apply(&mut self, filters: &mut Vec<StatFilter>, mut replacement: Vec<StatFilter>) {
        if self.is_applied() {
            return;
        }

        if replacement.is_empty() {
            return;
        }

        for filter in &mut replacement {
            if let Some(old) = filters.iter().find(|f| f.id == filter.id) {
                filter.disabled = old.disabled;
            }
        }

        self.stored = std::mem::take(filters);
        *filters = replacement;
    }

    pub fn remove(&mut self, filters: &mut Vec<StatFilter>) {
        if !self.is_applied() {
            return;
        }

        let mut restored = std::mem::take(&mut self.stored);

        for filter in &mut restored {
            if let Some(current) = filters.iter().find(|f| f.id == filter.id) {
                filter.disabled = current.disabled;
            }
        }

        *filters = restored;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::query::Range;

    fn effect(reference: &str, value: f64, categories: Vec<ItemCategory>) -> AugmentEffect {
        AugmentEffect {
            reference: reference.into(),
            value,
            categories,
        }
    }

    fn iron_rune() -> Augment {
        Augment {
            reference_name: "IronRune".into(),
            name: "Iron Rune".into(),
            effects: vec![
                effect(
                    "#% increased Physical Damage",
                    20.0,
                    vec![ItemCategory::Bow, ItemCategory::OneHandedAxe],
                ),
                effect(
                    "#% increased Armour",
                    20.0,
                    vec![ItemCategory::BodyArmour, ItemCategory::Helmet],
                ),
            ],
        }
    }

    fn filter(id: &str, disabled: bool) -> StatFilter {
        let mut f = StatFilter::range(id, Range::at_least(10.0));
        f.disabled = disabled;

        f
    }

    #[test]
    fn a_rune_grants_a_different_effect_per_base() {
        let rune = iron_rune();

        assert_eq!(
            effect_for_category(&rune, ItemCategory::Bow)
                .unwrap()
                .reference,
            "#% increased Physical Damage"
        );
        assert_eq!(
            effect_for_category(&rune, ItemCategory::BodyArmour)
                .unwrap()
                .reference,
            "#% increased Armour"
        );
    }

    #[test]
    fn a_rune_that_cannot_go_in_this_base_grants_nothing() {
        assert!(effect_for_category(&iron_rune(), ItemCategory::Ring).is_none());
    }

    #[test]
    fn a_rune_with_no_effects_grants_nothing() {
        let empty = Augment {
            reference_name: "x".into(),
            name: "x".into(),
            effects: Vec::new(),
        };

        assert!(effect_for_category(&empty, ItemCategory::Bow).is_none());
    }

    #[test]
    fn a_known_rune_shows_its_name() {
        assert_eq!(augment_name(&[iron_rune()], "IronRune"), "Iron Rune");
    }

    #[test]
    fn an_unknown_rune_shows_its_reference_rather_than_the_word_error() {
        assert_eq!(augment_name(&[iron_rune()], "MysteryRune"), "MysteryRune");
    }

    #[test]
    fn a_fresh_edit_is_not_applied() {
        assert!(!FilterEdit::default().is_applied());
    }

    #[test]
    fn applying_replaces_the_filters_and_records_the_edit() {
        let mut edit = FilterEdit::default();
        let mut filters = vec![filter("a", false)];

        edit.apply(&mut filters, vec![filter("b", false)]);

        assert!(edit.is_applied());
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].id, "b");
    }

    #[test]
    fn a_replacement_inherits_the_state_of_what_it_replaces() {
        let mut edit = FilterEdit::default();
        let mut filters = vec![filter("a", false)];

        edit.apply(&mut filters, vec![filter("a", true)]);

        assert!(!filters[0].disabled);
    }

    #[test]
    fn a_replacement_with_no_counterpart_keeps_its_own_state() {
        let mut edit = FilterEdit::default();
        let mut filters = vec![filter("a", false)];

        edit.apply(&mut filters, vec![filter("b", true)]);

        assert!(filters[0].disabled);
    }

    #[test]
    fn applying_twice_does_nothing_the_second_time() {
        let mut edit = FilterEdit::default();
        let mut filters = vec![filter("a", false)];

        edit.apply(&mut filters, vec![filter("b", false)]);
        edit.apply(&mut filters, vec![filter("c", false)]);

        assert_eq!(filters[0].id, "b");
    }

    #[test]
    fn applying_an_empty_replacement_does_nothing() {
        let mut edit = FilterEdit::default();
        let mut filters = vec![filter("a", false)];

        edit.apply(&mut filters, Vec::new());

        assert!(!edit.is_applied());
        assert_eq!(filters[0].id, "a");
    }

    #[test]
    fn removing_restores_exactly_what_was_there() {
        let mut edit = FilterEdit::default();
        let mut filters = vec![filter("a", false), filter("b", true)];

        edit.apply(&mut filters, vec![filter("c", false)]);
        edit.remove(&mut filters);

        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0].id, "a");
        assert_eq!(filters[1].id, "b");
        assert!(!edit.is_applied());
    }

    #[test]
    fn a_restored_filter_takes_back_the_state_the_user_left_it_in() {
        let mut edit = FilterEdit::default();
        let mut filters = vec![filter("a", false)];

        edit.apply(&mut filters, vec![filter("a", false)]);
        filters[0].disabled = true;
        edit.remove(&mut filters);

        assert!(filters[0].disabled);
    }

    #[test]
    fn removing_without_an_edit_does_nothing() {
        let mut edit = FilterEdit::default();
        let mut filters = vec![filter("a", false)];

        edit.remove(&mut filters);

        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].id, "a");
    }

    #[test]
    fn apply_and_remove_round_trip_exactly() {
        let original = vec![filter("a", false), filter("b", true), filter("c", false)];

        let mut edit = FilterEdit::default();
        let mut filters = original.clone();

        edit.apply(&mut filters, vec![filter("x", false)]);
        edit.remove(&mut filters);

        assert_eq!(filters, original);
    }

    #[test]
    fn a_rune_adds_its_stat_to_the_filters() {
        let got = preview_filters(&iron_rune(), ItemCategory::Bow, 1, &[])
            .expect("the rune fits the base");

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "#% increased Physical Damage");
        assert_eq!(got[0].range.min, Some(20.0));
    }

    #[test]
    fn the_value_multiplies_by_the_empty_socket_count() {
        let got = preview_filters(&iron_rune(), ItemCategory::Bow, 3, &[])
            .expect("the rune fits the base");

        assert_eq!(got[0].range.min, Some(60.0));
    }

    #[test]
    fn a_rune_raises_a_stat_the_item_already_has() {
        let existing = vec![StatFilter::range(
            "#% increased Physical Damage",
            Range::at_least(50.0),
        )];

        let got = preview_filters(&iron_rune(), ItemCategory::Bow, 1, &existing)
            .expect("the rune fits the base");

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].range.min, Some(70.0));
    }

    #[test]
    fn the_other_filters_are_carried_through() {
        let existing = vec![StatFilter::range(
            "+# to maximum Life",
            Range::at_least(50.0),
        )];

        let got = preview_filters(&iron_rune(), ItemCategory::Bow, 1, &existing)
            .expect("the rune fits the base");

        assert_eq!(got.len(), 2);
        assert_eq!(got[0].id, "+# to maximum Life");
    }

    #[test]
    fn a_rune_that_cannot_go_in_this_base_previews_nothing() {
        assert_eq!(
            preview_filters(&iron_rune(), ItemCategory::Ring, 1, &[]),
            None
        );
    }

    #[test]
    fn an_item_with_no_empty_socket_previews_nothing() {
        assert_eq!(
            preview_filters(&iron_rune(), ItemCategory::Bow, 0, &[]),
            None
        );
    }

    #[test]
    fn the_preview_round_trips_through_an_edit() {
        let original = vec![filter("a", false)];
        let mut filters = original.clone();

        let replacement = preview_filters(&iron_rune(), ItemCategory::Bow, 1, &filters)
            .expect("the rune fits the base");

        let mut edit = FilterEdit::default();
        edit.apply(&mut filters, replacement);
        edit.remove(&mut filters);

        assert_eq!(filters, original);
    }
}
