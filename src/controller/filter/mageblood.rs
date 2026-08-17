use crate::controller::filter::stat_filters::FilterSource;
use crate::types::item::ParsedItem;
use crate::types::query::{Range, StatFilter, StatGroup, StatGroupKind, TradeQuery};

#[derive(Debug, Clone, PartialEq)]
pub struct CountFilter {
    pub at_least: u32,
    pub floors: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Duplicates {
    Plain,
    AtStrength(u32),
    Counted(CountFilter),
}

pub const MAX_LEGACY: u32 = 4;

pub fn duplicates_filter(known: u32, duplicates: u32, active: bool) -> Option<Duplicates> {
    if known + duplicates > MAX_LEGACY {
        return None;
    }

    if !active || duplicates == 0 {
        return Some(Duplicates::Plain);
    }

    if known + duplicates == MAX_LEGACY {
        return Some(match duplicates {
            3 => Duplicates::AtStrength(4),
            2 => Duplicates::Counted(CountFilter {
                at_least: 4,
                floors: vec![1, 2, 3],
            }),
            1 => Duplicates::Counted(CountFilter {
                at_least: 7,
                floors: vec![1, 2],
            }),
            _ => Duplicates::AtStrength(1),
        });
    }

    Some(Duplicates::Counted(CountFilter {
        at_least: 1,
        floors: vec![duplicates + 1],
    }))
}

pub const LEGACY_PREFIX: &str = "Legacy of ";

pub fn apply_legacy_rules(item: &ParsedItem, sources: &[FilterSource], query: &mut TradeQuery) {
    if item.info.reference_name != "Mageblood" {
        return;
    }

    let printed = item
        .modifiers
        .iter()
        .flat_map(|modifier| &modifier.stats)
        .filter(|stat| stat.reference.starts_with(LEGACY_PREFIX))
        .count() as u32;

    if printed == 0 {
        return;
    }

    let legacies: Vec<String> = sources
        .iter()
        .filter(|source| source.text.starts_with(LEGACY_PREFIX))
        .map(|source| source.id.clone())
        .collect();

    if legacies.is_empty() {
        return;
    }

    let known = legacies.len() as u32;
    let duplicates = printed.saturating_sub(known);

    match duplicates_filter(known, duplicates, true) {
        None | Some(Duplicates::Plain) => {}
        Some(Duplicates::AtStrength(at_least)) => {
            raise_each(query, &legacies, f64::from(at_least));
        }
        Some(Duplicates::Counted(count)) => {
            let mut group = StatGroup {
                kind: StatGroupKind::Count,
                value: Range::at_least(f64::from(count.at_least)),
                filters: Vec::new(),
                disabled: false,
            };

            for id in &legacies {
                for floor in &count.floors {
                    group
                        .filters
                        .push(StatFilter::range(id, Range::at_least(f64::from(*floor))));
                }
            }

            query.stats.push(group);
        }
    }
}

fn raise_each(query: &mut TradeQuery, legacies: &[String], at_least: f64) {
    for group in &mut query.stats {
        for filter in &mut group.filters {
            if legacies.contains(&filter.id) {
                filter.range = Range::at_least(at_least);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_belt_with_no_duplicates_sends_its_readable_modifiers() {
        assert_eq!(duplicates_filter(4, 0, true), Some(Duplicates::Plain));
        assert_eq!(duplicates_filter(1, 0, true), Some(Duplicates::Plain));
    }

    #[test]
    fn switching_the_duplicate_filter_off_sends_the_readable_modifiers() {
        assert_eq!(duplicates_filter(2, 2, false), Some(Duplicates::Plain));
    }

    #[test]
    fn one_modifier_filling_all_four_slots_is_asked_for_at_full_strength() {
        assert_eq!(
            duplicates_filter(1, 3, true),
            Some(Duplicates::AtStrength(4))
        );
    }

    #[test]
    fn two_duplicates_alongside_two_readable_becomes_a_count_filter() {
        assert_eq!(
            duplicates_filter(2, 2, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 4,
                floors: vec![1, 2, 3],
            }))
        );
    }

    #[test]
    fn one_duplicate_alongside_three_readable_becomes_a_count_filter() {
        assert_eq!(
            duplicates_filter(3, 1, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 7,
                floors: vec![1, 2],
            }))
        );
    }

    #[test]
    fn a_belt_with_an_empty_slot_asks_only_for_a_floor() {
        assert_eq!(
            duplicates_filter(1, 1, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 1,
                floors: vec![2],
            }))
        );
        assert_eq!(
            duplicates_filter(1, 2, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 1,
                floors: vec![3],
            }))
        );
    }

    #[test]
    fn a_belt_that_cannot_exist_reports_nothing() {
        assert_eq!(duplicates_filter(3, 3, true), None);
        assert_eq!(duplicates_filter(5, 0, true), None);
    }

    #[test]
    fn the_boundary_at_four_is_allowed() {
        assert!(duplicates_filter(4, 0, true).is_some());
        assert!(duplicates_filter(0, 4, true).is_some());
        assert!(duplicates_filter(4, 1, true).is_none());
    }

    #[test]
    fn a_belt_with_nothing_on_it_sends_nothing_special() {
        assert_eq!(duplicates_filter(0, 0, true), Some(Duplicates::Plain));
    }

    #[test]
    fn every_count_filter_asks_for_at_least_one() {
        for known in 0..=MAX_LEGACY {
            for duplicates in 0..=MAX_LEGACY {
                let Some(Duplicates::Counted(count)) = duplicates_filter(known, duplicates, true)
                else {
                    continue;
                };

                assert!(count.at_least >= 1, "{known} known {duplicates} dup");
                assert!(!count.floors.is_empty(), "{known} known {duplicates} dup");
            }
        }
    }

    #[test]
    fn every_floor_is_within_what_a_belt_can_carry() {
        for known in 0..=MAX_LEGACY {
            for duplicates in 0..=MAX_LEGACY {
                let Some(Duplicates::Counted(count)) = duplicates_filter(known, duplicates, true)
                else {
                    continue;
                };

                for floor in count.floors {
                    assert!(
                        (1..=MAX_LEGACY).contains(&floor),
                        "{known} known {duplicates} dup gave floor {floor}"
                    );
                }
            }
        }
    }

    #[test]
    fn no_valid_combination_is_left_without_an_answer() {
        for known in 0..=MAX_LEGACY {
            for duplicates in 0..=(MAX_LEGACY - known) {
                assert!(
                    duplicates_filter(known, duplicates, true).is_some(),
                    "{known} known {duplicates} dup"
                );
            }
        }
    }

    fn belt(legacies: &[&str]) -> ParsedItem {
        use crate::controller::parse::shared::modifiers::ParsedModifier;
        use crate::types::item::BaseInfo;
        use crate::types::modifier::{ModifierInfo, ModifierType};
        use crate::types::stat::ParsedStat;

        ParsedItem {
            info: BaseInfo {
                reference_name: "Mageblood".into(),
                ..BaseInfo::default()
            },
            modifiers: legacies
                .iter()
                .map(|reference| ParsedModifier {
                    info: ModifierInfo {
                        kind: Some(ModifierType::Explicit),
                        ..ModifierInfo::default()
                    },
                    stats: vec![ParsedStat {
                        reference: (*reference).to_string(),
                        matched: (*reference).to_string(),
                        roll: None,
                    }],
                })
                .collect(),
            ..ParsedItem::default()
        }
    }

    fn sources_for(ids: &[(&str, &str)]) -> Vec<FilterSource> {
        ids.iter()
            .map(|(id, text)| FilterSource {
                id: (*id).to_string(),
                text: (*text).to_string(),
                roll: None,
                tier: None,
                contributors: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn a_belt_that_is_not_a_mageblood_is_left_alone() {
        let mut item = belt(&["Legacy of Fury"]);
        item.info.reference_name = "Leather Belt".into();

        let mut query = TradeQuery::default();

        apply_legacy_rules(
            &item,
            &sources_for(&[("explicit.stat_1", "Legacy of Fury")]),
            &mut query,
        );

        assert!(query.stats.is_empty());
    }

    #[test]
    fn four_readable_legacies_send_no_extra_group() {
        let item = belt(&[
            "Legacy of Fury",
            "Legacy of Spite",
            "Legacy of Guile",
            "Legacy of Cunning",
        ]);
        let mut query = TradeQuery::default();

        apply_legacy_rules(
            &item,
            &sources_for(&[
                ("explicit.stat_1", "Legacy of Fury"),
                ("explicit.stat_2", "Legacy of Spite"),
                ("explicit.stat_3", "Legacy of Guile"),
                ("explicit.stat_4", "Legacy of Cunning"),
            ]),
            &mut query,
        );

        assert!(query.stats.is_empty());
    }

    #[test]
    fn one_readable_legacy_and_three_hidden_asks_for_it_at_full_strength() {
        let item = belt(&[
            "Legacy of Fury",
            "Legacy of Fury",
            "Legacy of Fury",
            "Legacy of Fury",
        ]);
        let mut query = TradeQuery::default();

        query.stats.push(StatGroup::all(vec![StatFilter::range(
            "explicit.stat_1",
            Range::at_least(1.0),
        )]));

        apply_legacy_rules(
            &item,
            &sources_for(&[("explicit.stat_1", "Legacy of Fury")]),
            &mut query,
        );

        assert_eq!(query.stats[0].filters[0].range.min, Some(4.0));
    }

    #[test]
    fn two_readable_legacies_and_two_hidden_send_a_count_group() {
        let item = belt(&[
            "Legacy of Fury",
            "Legacy of Fury",
            "Legacy of Spite",
            "Legacy of Spite",
        ]);
        let mut query = TradeQuery::default();

        apply_legacy_rules(
            &item,
            &sources_for(&[
                ("explicit.stat_1", "Legacy of Fury"),
                ("explicit.stat_2", "Legacy of Spite"),
            ]),
            &mut query,
        );

        let group = query.stats.last().expect("a count group");

        assert_eq!(group.kind, StatGroupKind::Count);
        assert_eq!(group.value.min, Some(4.0));
        assert_eq!(group.filters.len(), 6);
    }

    #[test]
    fn a_mageblood_with_no_legacy_lines_is_left_alone() {
        let item = belt(&[]);
        let mut query = TradeQuery::default();

        apply_legacy_rules(&item, &[], &mut query);

        assert!(query.stats.is_empty());
    }
}
