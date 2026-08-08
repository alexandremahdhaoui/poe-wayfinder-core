use crate::controller::aggregate::StatTotal;
use crate::controller::calc::quality::{apply_scaling, strip_scaling, PropContributions, Roll};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropStats {
    pub flat: &'static [&'static str],
    pub increased: &'static [&'static str],
}

pub const ARMOUR: PropStats = PropStats {
    flat: &["# to Armour"],
    increased: &[
        "#% increased Armour",
        "#% increased Armour and Energy Shield",
        "#% increased Armour and Evasion",
        "#% increased Armour, Evasion and Energy Shield",
    ],
};

pub const EVASION: PropStats = PropStats {
    flat: &["# to Evasion Rating"],
    increased: &[
        "#% increased Evasion Rating",
        "#% increased Armour and Evasion",
        "#% increased Evasion and Energy Shield",
        "#% increased Armour, Evasion and Energy Shield",
    ],
};

pub const ENERGY_SHIELD: PropStats = PropStats {
    flat: &["# to maximum Energy Shield"],
    increased: &[
        "#% increased Energy Shield",
        "#% increased Armour and Energy Shield",
        "#% increased Evasion and Energy Shield",
        "#% increased Armour, Evasion and Energy Shield",
    ],
};

pub const PHYSICAL_DAMAGE: PropStats = PropStats {
    flat: &["Adds # to # Physical Damage"],
    increased: &["#% increased Physical Damage"],
};

pub const ATTACK_SPEED: PropStats = PropStats {
    flat: &[],
    increased: &["#% increased Attack Speed"],
};

pub fn contributions(stats: PropStats, totals: &[StatTotal]) -> PropContributions {
    let mut out = PropContributions::default();

    for total in totals {
        let target = if stats.flat.contains(&total.reference.as_str()) {
            &mut out.flat
        } else if stats.increased.contains(&total.reference.as_str()) {
            &mut out.increased
        } else {
            continue;
        };

        let Some(roll) = total.roll else {
            continue;
        };

        target.add(Roll {
            value: roll.value,
            min: roll.min,
            max: roll.max,
        });
    }

    out
}

pub fn base_value(
    printed: f64,
    stats: PropStats,
    totals: &[StatTotal],
    quality: f64,
    use_quality: bool,
) -> f64 {
    let contributions = contributions(stats, totals);
    let quality = if use_quality { quality } else { 0.0 };

    strip_scaling(printed, contributions.increased.value, quality) - contributions.flat.value
}

pub fn total_value(
    base: f64,
    stats: PropStats,
    totals: &[StatTotal],
    quality: f64,
    use_quality: bool,
) -> f64 {
    let contributions = contributions(stats, totals);
    let quality = if use_quality { quality } else { 0.0 };

    apply_scaling(
        base + contributions.flat.value,
        contributions.increased.value,
        quality,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::modifier::ModifierType;
    use crate::types::stat::StatRoll;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn total(reference: &str, value: f64) -> StatTotal {
        StatTotal {
            reference: reference.to_string(),
            kind: ModifierType::Explicit,
            roll: Some(StatRoll {
                value,
                min: value,
                max: value,
                ..StatRoll::default()
            }),
            sources: 1,
        }
    }

    #[test]
    fn a_flat_modifier_lands_in_the_flat_total() {
        let got = contributions(ARMOUR, &[total("# to Armour", 80.0)]);

        assert_eq!(got.flat.value, 80.0);
        assert_eq!(got.increased.value, 0.0);
    }

    #[test]
    fn an_increase_modifier_lands_in_the_increased_total() {
        let got = contributions(ARMOUR, &[total("#% increased Armour", 50.0)]);

        assert_eq!(got.increased.value, 50.0);
        assert_eq!(got.flat.value, 0.0);
    }

    #[test]
    fn a_hybrid_modifier_feeds_both_properties() {
        let totals = [total("#% increased Armour and Evasion", 30.0)];

        assert_eq!(contributions(ARMOUR, &totals).increased.value, 30.0);
        assert_eq!(contributions(EVASION, &totals).increased.value, 30.0);
    }

    #[test]
    fn a_triple_hybrid_feeds_all_three_defences() {
        let totals = [total(
            "#% increased Armour, Evasion and Energy Shield",
            20.0,
        )];

        assert_eq!(contributions(ARMOUR, &totals).increased.value, 20.0);
        assert_eq!(contributions(EVASION, &totals).increased.value, 20.0);
        assert_eq!(contributions(ENERGY_SHIELD, &totals).increased.value, 20.0);
    }

    #[test]
    fn an_unrelated_modifier_contributes_nothing() {
        let got = contributions(ARMOUR, &[total("# to maximum Life", 80.0)]);

        assert_eq!(got.flat.value, 0.0);
        assert_eq!(got.increased.value, 0.0);
    }

    #[test]
    fn two_modifiers_of_one_kind_add_together() {
        let got = contributions(
            ARMOUR,
            &[
                total("#% increased Armour", 30.0),
                total("#% increased Armour and Evasion", 20.0),
            ],
        );

        assert_eq!(got.increased.value, 50.0);
    }

    #[test]
    fn attack_speed_has_no_flat_modifier() {
        assert!(ATTACK_SPEED.flat.is_empty());
    }

    #[test]
    fn an_unmodified_item_reports_its_printed_value_as_the_base() {
        assert!(close(base_value(500.0, ARMOUR, &[], 0.0, true), 500.0));
    }

    #[test]
    fn a_flat_modifier_is_taken_off_the_base() {
        let totals = [total("# to Armour", 80.0)];

        assert!(close(base_value(500.0, ARMOUR, &totals, 0.0, true), 420.0));
    }

    #[test]
    fn an_increase_modifier_is_divided_out_of_the_base() {
        let totals = [total("#% increased Armour", 50.0)];

        assert!(close(base_value(600.0, ARMOUR, &totals, 0.0, true), 400.0));
    }

    #[test]
    fn quality_is_divided_out_before_the_flat_modifier_is_subtracted() {
        let totals = [total("# to Armour", 100.0)];

        assert!(close(base_value(600.0, ARMOUR, &totals, 20.0, true), 400.0));
    }

    #[test]
    fn quality_is_ignored_where_it_does_not_apply() {
        let printed = 1.5;

        assert!(close(
            base_value(printed, ATTACK_SPEED, &[], 20.0, false),
            1.5
        ));
        assert!(!close(
            base_value(printed, ATTACK_SPEED, &[], 20.0, true),
            1.5
        ));
    }

    #[test]
    fn the_total_is_the_exact_inverse_of_the_base() {
        let totals = [
            total("# to Armour", 80.0),
            total("#% increased Armour", 45.0),
        ];

        let base = base_value(1000.0, ARMOUR, &totals, 17.0, true);
        let back = total_value(base, ARMOUR, &totals, 17.0, true);

        assert!(close(back, 1000.0));
    }

    #[test]
    fn the_inverse_holds_with_no_modifiers_at_all() {
        let base = base_value(500.0, ARMOUR, &[], 0.0, true);

        assert!(close(total_value(base, ARMOUR, &[], 0.0, true), 500.0));
    }

    #[test]
    fn the_inverse_holds_for_attack_speed_without_quality() {
        let totals = [total("#% increased Attack Speed", 25.0)];

        let base = base_value(1.5, ATTACK_SPEED, &totals, 20.0, false);
        let back = total_value(base, ATTACK_SPEED, &totals, 20.0, false);

        assert!(close(back, 1.5));
    }

    #[test]
    fn adding_a_modifier_raises_the_total() {
        let base = 400.0;

        let without = total_value(base, ARMOUR, &[], 0.0, true);
        let with = total_value(base, ARMOUR, &[total("# to Armour", 100.0)], 0.0, true);

        assert!(with > without);
        assert!(close(with - without, 100.0));
    }

    #[test]
    fn physical_damage_uses_its_own_modifiers() {
        let totals = [
            total("Adds # to # Physical Damage", 20.0),
            total("#% increased Physical Damage", 100.0),
        ];

        assert!(close(
            base_value(100.0, PHYSICAL_DAMAGE, &totals, 0.0, true),
            30.0
        ));
    }

    #[test]
    fn a_stat_with_no_roll_contributes_nothing() {
        let totals = [StatTotal {
            reference: "# to Armour".to_string(),
            kind: ModifierType::Explicit,
            roll: None,
            sources: 1,
        }];

        assert_eq!(contributions(ARMOUR, &totals).flat.value, 0.0);
    }

    #[test]
    fn the_bounds_are_carried_through_alongside_the_value() {
        let totals = [StatTotal {
            reference: "# to Armour".to_string(),
            kind: ModifierType::Explicit,
            roll: Some(StatRoll {
                value: 80.0,
                min: 60.0,
                max: 100.0,
                ..StatRoll::default()
            }),
            sources: 1,
        }];

        let got = contributions(ARMOUR, &totals);

        assert_eq!(got.flat.min, 60.0);
        assert_eq!(got.flat.max, 100.0);
    }

    #[test]
    fn every_property_table_names_at_least_one_modifier() {
        for stats in [
            ARMOUR,
            EVASION,
            ENERGY_SHIELD,
            PHYSICAL_DAMAGE,
            ATTACK_SPEED,
        ] {
            assert!(!stats.flat.is_empty() || !stats.increased.is_empty());
        }
    }

    #[test]
    fn no_modifier_is_both_flat_and_increased_for_one_property() {
        for stats in [
            ARMOUR,
            EVASION,
            ENERGY_SHIELD,
            PHYSICAL_DAMAGE,
            ATTACK_SPEED,
        ] {
            for flat in stats.flat {
                assert!(!stats.increased.contains(flat), "{flat}");
            }
        }
    }
}
