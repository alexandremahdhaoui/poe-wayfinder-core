use crate::controller::parse::shared::modifiers::ParsedModifier;
use crate::types::modifier::ModifierType;
use crate::types::stat::StatRoll;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Combine {
    #[default]
    Sum,
    Max,
    Count,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatTotal {
    pub reference: String,
    pub kind: ModifierType,
    pub roll: Option<StatRoll>,
    pub sources: usize,
}

pub fn types_can_be_grouped(a: ModifierType, b: ModifierType) -> bool {
    if a == b {
        return true;
    }

    is_explicit_family(a) && is_explicit_family(b)
}

pub fn is_explicit_family(kind: ModifierType) -> bool {
    matches!(
        kind,
        ModifierType::Explicit
            | ModifierType::Fractured
            | ModifierType::Veiled
            | ModifierType::Desecrated
            | ModifierType::Crafted
            | ModifierType::Sanctum
    )
}

pub fn sum_stats_by_type(modifiers: &[ParsedModifier]) -> Vec<StatTotal> {
    let mut out: Vec<StatTotal> = Vec::new();

    for modifier in modifiers {
        let Some(kind) = modifier.info.kind else {
            continue;
        };

        for stat in &modifier.stats {
            if out
                .iter()
                .any(|t| t.reference == stat.reference && types_can_be_grouped(t.kind, kind))
            {
                continue;
            }

            let mut rolls = Vec::new();
            let mut sources = 0;
            let mut has_fractured = false;

            for other in modifiers {
                let Some(other_kind) = other.info.kind else {
                    continue;
                };

                if !types_can_be_grouped(other_kind, kind) {
                    continue;
                }

                for other_stat in &other.stats {
                    if other_stat.reference != stat.reference {
                        continue;
                    }

                    if other_kind == ModifierType::Fractured {
                        has_fractured = true;
                    }

                    sources += 1;

                    if let Some(roll) = other_stat.roll {
                        rolls.push(roll);
                    }
                }
            }

            out.push(StatTotal {
                reference: stat.reference.clone(),
                kind: if has_fractured {
                    ModifierType::Fractured
                } else {
                    kind
                },
                roll: combine(&rolls, Combine::Sum),
                sources,
            });
        }
    }

    out
}

pub fn combine(rolls: &[StatRoll], mode: Combine) -> Option<StatRoll> {
    if mode == Combine::Count {
        let count = rolls.len() as f64;

        return Some(StatRoll {
            value: count,
            min: count,
            max: count,
            option: rolls.first().and_then(|r| r.option),
            ..StatRoll::default()
        });
    }

    if rolls.is_empty() {
        return None;
    }

    if rolls.len() == 1 {
        return Some(rolls[0]);
    }

    let mut out = StatRoll::default();

    for roll in rolls {
        match mode {
            Combine::Sum => {
                out.value += roll.value;
                out.min += roll.min;
                out.max += roll.max;
            }
            Combine::Max => {
                out.value = out.value.max(roll.value);
                out.min = out.min.max(roll.min);
                out.max = out.max.max(roll.max);
            }
            Combine::Count => unreachable!("handled above"),
        }

        out.legacy |= roll.legacy;
        out.unscalable |= roll.unscalable;
        out.decimals |= roll.decimals;
        out.option = roll.option;
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::modifier::ModifierInfo;
    use crate::types::stat::ParsedStat;

    fn roll(value: f64) -> Option<StatRoll> {
        Some(StatRoll {
            value,
            min: value,
            max: value,
            ..StatRoll::default()
        })
    }

    fn modifier(kind: ModifierType, stats: Vec<(&str, Option<StatRoll>)>) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(kind),
                ..ModifierInfo::default()
            },
            stats: stats
                .into_iter()
                .map(|(reference, roll)| ParsedStat {
                    reference: reference.into(),
                    matched: reference.into(),
                    roll,
                })
                .collect(),
        }
    }

    #[test]
    fn the_same_stat_from_two_modifiers_is_summed() {
        let mods = vec![
            modifier(
                ModifierType::Explicit,
                vec![("# to maximum Life", roll(20.0))],
            ),
            modifier(
                ModifierType::Explicit,
                vec![("# to maximum Life", roll(15.0))],
            ),
        ];

        let totals = sum_stats_by_type(&mods);

        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].roll.unwrap().value, 35.0);
        assert_eq!(totals[0].sources, 2);
    }

    #[test]
    fn an_explicit_and_a_crafted_modifier_merge() {
        let mods = vec![
            modifier(
                ModifierType::Explicit,
                vec![("# to maximum Life", roll(20.0))],
            ),
            modifier(
                ModifierType::Crafted,
                vec![("# to maximum Life", roll(15.0))],
            ),
        ];

        let totals = sum_stats_by_type(&mods);

        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].roll.unwrap().value, 35.0);
    }

    #[test]
    fn an_implicit_does_not_merge_with_an_explicit() {
        let mods = vec![
            modifier(
                ModifierType::Explicit,
                vec![("# to maximum Life", roll(20.0))],
            ),
            modifier(
                ModifierType::Implicit,
                vec![("# to maximum Life", roll(10.0))],
            ),
        ];

        let totals = sum_stats_by_type(&mods);

        assert_eq!(totals.len(), 2);
        assert_eq!(totals[0].roll.unwrap().value, 20.0);
        assert_eq!(totals[1].roll.unwrap().value, 10.0);
    }

    #[test]
    fn a_rune_does_not_merge_with_an_explicit() {
        let mods = vec![
            modifier(
                ModifierType::Explicit,
                vec![("# to maximum Life", roll(20.0))],
            ),
            modifier(
                ModifierType::Augment,
                vec![("# to maximum Life", roll(15.0))],
            ),
        ];

        assert_eq!(sum_stats_by_type(&mods).len(), 2);
    }

    #[test]
    fn every_explicit_family_type_groups_with_every_other() {
        let family = [
            ModifierType::Explicit,
            ModifierType::Fractured,
            ModifierType::Veiled,
            ModifierType::Desecrated,
            ModifierType::Crafted,
            ModifierType::Sanctum,
        ];

        for a in family {
            for b in family {
                assert!(types_can_be_grouped(a, b), "{a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn a_type_outside_the_family_groups_only_with_itself() {
        for kind in [
            ModifierType::Implicit,
            ModifierType::Enchant,
            ModifierType::Augment,
            ModifierType::Pseudo,
        ] {
            assert!(types_can_be_grouped(kind, kind), "{kind:?}");
            assert!(
                !types_can_be_grouped(kind, ModifierType::Explicit),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_fractured_contributor_makes_the_whole_total_fractured() {
        let mods = vec![
            modifier(
                ModifierType::Explicit,
                vec![("# to maximum Life", roll(20.0))],
            ),
            modifier(
                ModifierType::Fractured,
                vec![("# to maximum Life", roll(15.0))],
            ),
        ];

        let totals = sum_stats_by_type(&mods);

        assert_eq!(totals[0].kind, ModifierType::Fractured);
    }

    #[test]
    fn different_stats_stay_separate() {
        let mods = vec![modifier(
            ModifierType::Explicit,
            vec![
                ("# to maximum Life", roll(20.0)),
                ("# to Dexterity", roll(30.0)),
            ],
        )];

        let totals = sum_stats_by_type(&mods);

        assert_eq!(totals.len(), 2);
    }

    #[test]
    fn the_order_the_game_printed_them_in_is_kept() {
        let mods = vec![modifier(
            ModifierType::Explicit,
            vec![
                ("# to Dexterity", roll(30.0)),
                ("# to maximum Life", roll(20.0)),
            ],
        )];

        let totals = sum_stats_by_type(&mods);

        assert_eq!(totals[0].reference, "# to Dexterity");
        assert_eq!(totals[1].reference, "# to maximum Life");
    }

    #[test]
    fn a_stat_with_no_roll_still_gets_a_total_entry() {
        let mods = vec![modifier(
            ModifierType::Explicit,
            vec![("Cannot be Frozen", None)],
        )];

        let totals = sum_stats_by_type(&mods);

        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].roll, None);
        assert_eq!(totals[0].sources, 1);
    }

    #[test]
    fn a_modifier_with_no_type_contributes_nothing() {
        let mods = vec![ParsedModifier {
            info: ModifierInfo::default(),
            stats: vec![ParsedStat {
                reference: "# to maximum Life".into(),
                matched: "# to maximum Life".into(),
                roll: roll(20.0),
            }],
        }];

        assert!(sum_stats_by_type(&mods).is_empty());
    }

    #[test]
    fn no_modifiers_yields_no_totals() {
        assert!(sum_stats_by_type(&[]).is_empty());
    }

    #[test]
    fn three_modifiers_of_one_stat_all_contribute() {
        let mods = vec![
            modifier(
                ModifierType::Explicit,
                vec![("# to maximum Life", roll(20.0))],
            ),
            modifier(
                ModifierType::Crafted,
                vec![("# to maximum Life", roll(15.0))],
            ),
            modifier(
                ModifierType::Fractured,
                vec![("# to maximum Life", roll(10.0))],
            ),
        ];

        let totals = sum_stats_by_type(&mods);

        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].roll.unwrap().value, 45.0);
        assert_eq!(totals[0].sources, 3);
    }

    #[test]
    fn summing_adds_every_bound() {
        let got = combine(
            &[
                StatRoll {
                    value: 20.0,
                    min: 15.0,
                    max: 25.0,
                    ..StatRoll::default()
                },
                StatRoll {
                    value: 10.0,
                    min: 5.0,
                    max: 15.0,
                    ..StatRoll::default()
                },
            ],
            Combine::Sum,
        )
        .unwrap();

        assert_eq!(got.value, 30.0);
        assert_eq!(got.min, 20.0);
        assert_eq!(got.max, 40.0);
    }

    #[test]
    fn taking_the_max_picks_the_largest() {
        let got = combine(
            &[
                StatRoll {
                    value: 20.0,
                    ..StatRoll::default()
                },
                StatRoll {
                    value: 35.0,
                    ..StatRoll::default()
                },
            ],
            Combine::Max,
        )
        .unwrap();

        assert_eq!(got.value, 35.0);
    }

    #[test]
    fn counting_reports_how_many_there_were() {
        let got = combine(
            &[
                StatRoll::default(),
                StatRoll::default(),
                StatRoll::default(),
            ],
            Combine::Count,
        )
        .unwrap();

        assert_eq!(got.value, 3.0);
        assert_eq!(got.min, 3.0);
        assert_eq!(got.max, 3.0);
    }

    #[test]
    fn counting_nothing_reports_zero_rather_than_nothing() {
        let got = combine(&[], Combine::Count).unwrap();

        assert_eq!(got.value, 0.0);
    }

    #[test]
    fn summing_nothing_yields_nothing() {
        assert_eq!(combine(&[], Combine::Sum), None);
    }

    #[test]
    fn one_roll_keeps_its_own_flags() {
        let only = StatRoll {
            value: 20.0,
            legacy: true,
            unscalable: true,
            decimals: true,
            option: Some(3.0),
            ..StatRoll::default()
        };

        assert_eq!(combine(&[only], Combine::Sum), Some(only));
    }

    #[test]
    fn one_legacy_contributor_makes_the_total_legacy() {
        let got = combine(
            &[
                StatRoll {
                    value: 20.0,
                    ..StatRoll::default()
                },
                StatRoll {
                    value: 10.0,
                    legacy: true,
                    ..StatRoll::default()
                },
            ],
            Combine::Sum,
        )
        .unwrap();

        assert!(got.legacy);
    }

    #[test]
    fn one_unscalable_contributor_makes_the_total_unscalable() {
        let got = combine(
            &[
                StatRoll {
                    value: 20.0,
                    ..StatRoll::default()
                },
                StatRoll {
                    value: 10.0,
                    unscalable: true,
                    ..StatRoll::default()
                },
            ],
            Combine::Sum,
        )
        .unwrap();

        assert!(got.unscalable);
    }
}
