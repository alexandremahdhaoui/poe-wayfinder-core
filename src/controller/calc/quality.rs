pub fn strip_scaling(total: f64, increased_pct: f64, more_pct: f64) -> f64 {
    total / (1.0 + more_pct / 100.0) / (1.0 + increased_pct / 100.0)
}

pub fn apply_scaling(flat: f64, increased_pct: f64, more_pct: f64) -> f64 {
    flat * (1.0 + increased_pct / 100.0) * (1.0 + more_pct / 100.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Roll {
    pub value: f64,
    pub min: f64,
    pub max: f64,
}

impl Roll {
    pub fn fixed(value: f64) -> Self {
        Self {
            value,
            min: value,
            max: value,
        }
    }

    pub fn zero() -> Self {
        Self::fixed(0.0)
    }

    pub fn add(&mut self, other: Roll) {
        self.value += other.value;
        self.min += other.min;
        self.max += other.max;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropContributions {
    pub flat: Roll,
    pub increased: Roll,
}

impl Default for PropContributions {
    fn default() -> Self {
        Self {
            flat: Roll::zero(),
            increased: Roll::zero(),
        }
    }
}

pub const TRADE_QUALITY: f64 = 20.0;

pub fn prop_at_20_quality(
    total: f64,
    contributions: PropContributions,
    quality: f64,
    is_modifiable: bool,
) -> Roll {
    let base =
        strip_scaling(total, contributions.increased.value, quality) - contributions.flat.value;

    let target_quality = if is_modifiable {
        quality.max(TRADE_QUALITY)
    } else {
        quality
    };

    Roll {
        value: apply_scaling(
            base + contributions.flat.value,
            contributions.increased.value,
            target_quality,
        ),
        min: apply_scaling(
            base + contributions.flat.min,
            contributions.increased.min,
            target_quality,
        ),
        max: apply_scaling(
            base + contributions.flat.max,
            contributions.increased.max,
            target_quality,
        ),
    }
}

pub fn prop_bounds(total: f64, contributions: PropContributions) -> Roll {
    let base = strip_scaling(total, contributions.increased.value, 0.0) - contributions.flat.value;

    Roll {
        value: apply_scaling(
            base + contributions.flat.value,
            contributions.increased.value,
            0.0,
        ),
        min: apply_scaling(
            base + contributions.flat.min,
            contributions.increased.min,
            0.0,
        ),
        max: apply_scaling(
            base + contributions.flat.max,
            contributions.increased.max,
            0.0,
        ),
    }
}

pub fn prop_percentile(
    total: f64,
    bounds: (f64, f64),
    contributions: PropContributions,
    quality: f64,
) -> f64 {
    let roll =
        strip_scaling(total, contributions.increased.value, quality) - contributions.flat.value;

    let (min, max) = bounds;

    if (max - min).abs() < f64::EPSILON {
        return 100.0;
    }

    let result = (((roll - min) / (max - min)) * 100.0).round();

    result.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn stripping_undoes_an_increase() {
        assert!(close(strip_scaling(150.0, 50.0, 0.0), 100.0));
    }

    #[test]
    fn stripping_undoes_quality() {
        assert!(close(strip_scaling(120.0, 0.0, 20.0), 100.0));
    }

    #[test]
    fn stripping_undoes_both_multipliers() {
        assert!(close(strip_scaling(180.0, 50.0, 20.0), 100.0));
    }

    #[test]
    fn applying_and_stripping_are_exact_inverses() {
        let base = 137.0;

        let printed = apply_scaling(base, 63.0, 17.0);

        assert!(close(strip_scaling(printed, 63.0, 17.0), base));
    }

    #[test]
    fn no_scaling_changes_nothing_in_either_direction() {
        assert!(close(strip_scaling(100.0, 0.0, 0.0), 100.0));
        assert!(close(apply_scaling(100.0, 0.0, 0.0), 100.0));
    }

    #[test]
    fn a_roll_adds_every_field() {
        let mut r = Roll {
            value: 1.0,
            min: 2.0,
            max: 3.0,
        };

        r.add(Roll {
            value: 10.0,
            min: 20.0,
            max: 30.0,
        });

        assert_eq!(
            r,
            Roll {
                value: 11.0,
                min: 22.0,
                max: 33.0
            }
        );
    }

    #[test]
    fn a_fixed_roll_cannot_vary() {
        let r = Roll::fixed(5.0);

        assert_eq!(r.min, 5.0);
        assert_eq!(r.max, 5.0);
    }

    #[test]
    fn an_unmodified_base_at_zero_quality_reaches_twenty() {
        let got = prop_at_20_quality(100.0, PropContributions::default(), 0.0, true);

        assert!(close(got.value, 120.0));
    }

    #[test]
    fn an_item_already_above_twenty_quality_keeps_its_own_quality() {
        let got = prop_at_20_quality(128.0, PropContributions::default(), 28.0, true);

        assert!(close(got.value, 128.0));
    }

    #[test]
    fn a_corrupted_item_is_valued_at_the_quality_it_has() {
        let got = prop_at_20_quality(100.0, PropContributions::default(), 0.0, false);

        assert!(close(got.value, 100.0));
    }

    #[test]
    fn an_increase_modifier_is_unwound_and_reapplied() {
        let contributions = PropContributions {
            flat: Roll::zero(),
            increased: Roll::fixed(50.0),
        };

        let got = prop_at_20_quality(150.0, contributions, 0.0, true);

        assert!(close(got.value, 180.0));
    }

    #[test]
    fn a_flat_modifier_is_held_out_of_the_base() {
        let contributions = PropContributions {
            flat: Roll::fixed(80.0),
            increased: Roll::zero(),
        };

        let got = prop_at_20_quality(180.0, contributions, 0.0, true);

        assert!(close(got.value, 216.0));
    }

    #[test]
    fn the_roll_range_follows_the_modifier_range() {
        let contributions = PropContributions {
            flat: Roll {
                value: 80.0,
                min: 60.0,
                max: 100.0,
            },
            increased: Roll::zero(),
        };

        let got = prop_at_20_quality(180.0, contributions, 0.0, true);

        assert!(close(got.min, (100.0 + 60.0) * 1.2));
        assert!(close(got.max, (100.0 + 100.0) * 1.2));
        assert!(got.min < got.value && got.value < got.max);
    }

    #[test]
    fn a_roll_at_the_bottom_of_its_range_is_percentile_zero() {
        let got = prop_percentile(100.0, (100.0, 200.0), PropContributions::default(), 0.0);

        assert_eq!(got, 0.0);
    }

    #[test]
    fn a_roll_at_the_top_of_its_range_is_percentile_one_hundred() {
        let got = prop_percentile(200.0, (100.0, 200.0), PropContributions::default(), 0.0);

        assert_eq!(got, 100.0);
    }

    #[test]
    fn a_roll_in_the_middle_is_percentile_fifty() {
        let got = prop_percentile(150.0, (100.0, 200.0), PropContributions::default(), 0.0);

        assert_eq!(got, 50.0);
    }

    #[test]
    fn quality_is_removed_before_the_percentile_is_taken() {
        let got = prop_percentile(120.0, (100.0, 200.0), PropContributions::default(), 20.0);

        assert_eq!(got, 0.0);
    }

    #[test]
    fn a_roll_beyond_its_range_is_clamped_rather_than_reported_raw() {
        let above = prop_percentile(300.0, (100.0, 200.0), PropContributions::default(), 0.0);
        let below = prop_percentile(50.0, (100.0, 200.0), PropContributions::default(), 0.0);

        assert_eq!(above, 100.0);
        assert_eq!(below, 0.0);
    }

    #[test]
    fn a_zero_width_range_reports_the_top() {
        let got = prop_percentile(100.0, (100.0, 100.0), PropContributions::default(), 0.0);

        assert_eq!(got, 100.0);
    }

    #[test]
    fn the_percentile_is_a_whole_number() {
        let got = prop_percentile(133.0, (100.0, 200.0), PropContributions::default(), 0.0);

        assert_eq!(got, got.round());
    }

    #[test]
    fn a_property_with_no_modifiers_reports_itself() {
        let got = prop_bounds(1.5, PropContributions::default());

        assert_eq!(got.value, 1.5);
        assert_eq!(got.min, 1.5);
        assert_eq!(got.max, 1.5);
    }

    #[test]
    fn quality_never_touches_these_bounds() {
        let plain = prop_bounds(1.5, PropContributions::default());

        assert_eq!(plain.value, 1.5);
    }

    #[test]
    fn an_increase_is_backed_out_of_the_printed_total() {
        let got = prop_bounds(
            150.0,
            PropContributions {
                flat: Roll::zero(),
                increased: Roll {
                    value: 50.0,
                    min: 50.0,
                    max: 50.0,
                },
            },
        );

        assert_eq!(got.value, 150.0);
    }

    #[test]
    fn the_bounds_move_with_the_modifier_range() {
        let got = prop_bounds(
            150.0,
            PropContributions {
                flat: Roll::zero(),
                increased: Roll {
                    value: 50.0,
                    min: 40.0,
                    max: 60.0,
                },
            },
        );

        assert!(got.min < got.value, "{got:?}");
        assert!(got.max > got.value, "{got:?}");
    }

    #[test]
    fn a_flat_addition_is_backed_out_too() {
        let got = prop_bounds(
            180.0,
            PropContributions {
                flat: Roll {
                    value: 80.0,
                    min: 80.0,
                    max: 80.0,
                },
                increased: Roll::zero(),
            },
        );

        assert_eq!(got.value, 180.0);
    }

    #[test]
    fn a_zero_total_stays_zero() {
        assert_eq!(prop_bounds(0.0, PropContributions::default()).value, 0.0);
    }
}
