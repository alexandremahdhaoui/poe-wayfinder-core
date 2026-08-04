//! Summing one stat across every modifier that grants it.
//!
//! Ported from `sumStatsByModType` and `statSourcesTotal` in
//! `renderer/src/parser/modifiers.ts`.
//!
//! # Why this exists
//!
//! An item can grant the same stat from three modifiers at once. A ring with
//! `+20 to maximum Life` as a prefix, `+15 to maximum Life` from a rune and
//! `+10 to maximum Life` as an implicit has 45 life, and that total is what a
//! buyer searches for.
//!
//! Searching on any one of the three finds almost nothing, because the split
//! across modifiers is random. This is also what every pseudo filter is built
//! on: total resistance is the sum of every resistance modifier on the item.
//!
//! # Which types merge
//!
//! Only the types the trade site treats as one namespace. An explicit and a
//! crafted modifier merge, because the trade site indexes both as explicit.
//! An implicit does not merge with an explicit, because they are different
//! filters and summing them would ask for a total no single item has.

use crate::controller::parse::shared::modifiers::ParsedModifier;
use crate::types::modifier::ModifierType;
use crate::types::stat::StatRoll;

/// How to combine several rolls of the same stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Combine {
    /// Add them. The normal case.
    #[default]
    Sum,
    /// Take the largest. Used where the game does not stack them.
    Max,
    /// Count how many there are rather than summing.
    Count,
}

/// One stat, totalled across every modifier that granted it.
#[derive(Debug, Clone, PartialEq)]
pub struct StatTotal {
    /// The canonical stat reference.
    pub reference: String,
    /// The namespace to query under.
    pub kind: ModifierType,
    /// The combined roll.
    pub roll: Option<StatRoll>,
    /// How many modifiers contributed.
    pub sources: usize,
}

/// Whether two modifier types share a trade namespace.
///
/// Ported from `typesCanBeGrouped`. The trade site indexes all of these as
/// explicit, so a life prefix and a crafted life suffix are one filter.
pub fn types_can_be_grouped(a: ModifierType, b: ModifierType) -> bool {
    if a == b {
        return true;
    }

    is_explicit_family(a) && is_explicit_family(b)
}

/// Whether a type is indexed as explicit on the trade site.
///
/// Ported from `EXPLICIT_MOD_TYPES`.
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

/// Total every stat across the modifiers that grant it.
///
/// The result is ordered by first appearance, so the UI shows stats in the
/// order the game printed them rather than in an order that changes between
/// runs.
pub fn sum_stats_by_type(modifiers: &[ParsedModifier]) -> Vec<StatTotal> {
    let mut out: Vec<StatTotal> = Vec::new();

    for modifier in modifiers {
        let Some(kind) = modifier.info.kind else {
            continue;
        };

        for stat in &modifier.stats {
            // Already totalled under a type this one groups with.
            if out
                .iter()
                .any(|t| t.reference == stat.reference && types_can_be_grouped(t.kind, kind))
            {
                continue;
            }

            let mut rolls = Vec::new();
            let mut sources = 0;
            // A fractured modifier in the group makes the whole total
            // fractured, because the trade site files it that way and it
            // cannot be removed.
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

/// Combine several rolls into one.
///
/// Ported from `statSourcesTotal`.
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

    // One roll is itself. Folding it would lose its flags.
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

        // Any contributing roll being legacy or unscalable makes the total so,
        // because the total inherits the constraint.
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
        // A ring with life on a prefix and life from a rune has the total,
        // and that total is what a buyer searches for. Searching on either
        // half finds almost nothing.
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
        // The trade site indexes both as explicit, so they are one filter.
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
        // They are different filters. Summing them asks for a total on one
        // filter that no single item has.
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
        // Runes have their own trade namespace.
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
        // The trade site files it that way and it cannot be removed.
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
        // An order that changes between runs makes the panel jump around.
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
        // It is a presence check and the query still needs the filter.
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

    // -----------------------------------------------------------------
    // Combining rolls
    // -----------------------------------------------------------------

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
        // A count filter of zero is a real thing to search for.
        let got = combine(&[], Combine::Count).unwrap();

        assert_eq!(got.value, 0.0);
    }

    #[test]
    fn summing_nothing_yields_nothing() {
        assert_eq!(combine(&[], Combine::Sum), None);
    }

    #[test]
    fn one_roll_keeps_its_own_flags() {
        // Folding a single roll through the accumulator would lose them.
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
        // The total inherits the constraint. An item with a legacy roll cannot
        // be crafted again whatever the other modifiers say.
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
