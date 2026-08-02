//! Normalising a rolled property back to its base.
//!
//! Ported from `renderer/src/parser/calc-q20.ts`.
//!
//! # Why this exists
//!
//! The game prints a finished number. A body armour showing 800 armour has a
//! base value scaled by its quality and by every increase modifier on it.
//! Two identical bases show wildly different numbers.
//!
//! Searching on the printed number therefore finds almost nothing. The trade
//! site expects the value at 20 quality, so the client has to unwind the
//! scaling and reapply it at a fixed quality.
//!
//! # The two directions
//!
//! `strip_scaling` removes the multipliers. `apply_scaling` puts them back.
//! They are exact inverses, which is what makes a round trip safe.

/// Remove increase and quality scaling from a printed total.
///
/// Ported from `calcFlat`.
pub fn strip_scaling(total: f64, increased_pct: f64, more_pct: f64) -> f64 {
    total / (1.0 + more_pct / 100.0) / (1.0 + increased_pct / 100.0)
}

/// Apply increase and quality scaling to a base value.
///
/// Ported from `calcIncreased`.
pub fn apply_scaling(flat: f64, increased_pct: f64, more_pct: f64) -> f64 {
    flat * (1.0 + increased_pct / 100.0) * (1.0 + more_pct / 100.0)
}

/// One roll and the range it could have landed in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Roll {
    pub value: f64,
    pub min: f64,
    pub max: f64,
}

impl Roll {
    /// A roll that cannot vary.
    pub fn fixed(value: f64) -> Self {
        Self {
            value,
            min: value,
            max: value,
        }
    }

    /// A roll of zero.
    pub fn zero() -> Self {
        Self::fixed(0.0)
    }

    /// Add another roll into this one.
    pub fn add(&mut self, other: Roll) {
        self.value += other.value;
        self.min += other.min;
        self.max += other.max;
    }
}

/// What the item's modifiers contribute to one property.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropContributions {
    /// Flat additions, such as `+80 to Armour`.
    pub flat: Roll,
    /// Percentage increases, such as `+50% increased Armour`.
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

/// The quality the trade site normalises to.
pub const TRADE_QUALITY: f64 = 20.0;

/// Restate a printed property at 20 quality.
///
/// Ported from `propAt20Quality`.
///
/// `quality` is the item's current quality. `is_modifiable` says whether the
/// item can still be improved. An item that can be taken to 20 quality is
/// valued as if it were, because the buyer will do that. An item that cannot,
/// such as a corrupted one, is valued at the quality it has.
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

/// Where a rolled property sits inside its possible range, 0 to 100.
///
/// Ported from `calcPropPercentile`. Clamped, because a base whose range our
/// data has wrong would otherwise report a percentile outside the scale and
/// the UI would show a nonsense number rather than a wrong one.
pub fn prop_percentile(
    total: f64,
    bounds: (f64, f64),
    contributions: PropContributions,
    quality: f64,
) -> f64 {
    let roll =
        strip_scaling(total, contributions.increased.value, quality) - contributions.flat.value;

    let (min, max) = bounds;

    // A zero width range has no percentile. Reporting the top is the kind
    // answer: the roll is by definition the best possible.
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
        // 100 base with 50% increased prints 150.
        assert!(close(strip_scaling(150.0, 50.0, 0.0), 100.0));
    }

    #[test]
    fn stripping_undoes_quality() {
        assert!(close(strip_scaling(120.0, 0.0, 20.0), 100.0));
    }

    #[test]
    fn stripping_undoes_both_multipliers() {
        // 100 * 1.5 * 1.2 = 180.
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
        // 100 base armour, no modifiers, 0 quality. At 20 quality it is 120.
        let got = prop_at_20_quality(100.0, PropContributions::default(), 0.0, true);

        assert!(close(got.value, 120.0));
    }

    #[test]
    fn an_item_already_above_twenty_quality_keeps_its_own_quality() {
        // A 28 quality item is worth more than a 20 quality one. Normalising
        // down would understate it.
        let got = prop_at_20_quality(128.0, PropContributions::default(), 28.0, true);

        assert!(close(got.value, 128.0));
    }

    #[test]
    fn a_corrupted_item_is_valued_at_the_quality_it_has() {
        // It cannot be improved, so pretending it will be to 20 overstates it.
        let got = prop_at_20_quality(100.0, PropContributions::default(), 0.0, false);

        assert!(close(got.value, 100.0));
    }

    #[test]
    fn an_increase_modifier_is_unwound_and_reapplied() {
        // 100 base, 50% increased, 0 quality, so the game prints 150.
        // At 20 quality that becomes 100 * 1.5 * 1.2 = 180.
        let contributions = PropContributions {
            flat: Roll::zero(),
            increased: Roll::fixed(50.0),
        };

        let got = prop_at_20_quality(150.0, contributions, 0.0, true);

        assert!(close(got.value, 180.0));
    }

    #[test]
    fn a_flat_modifier_is_held_out_of_the_base() {
        // 100 base plus 80 flat armour, no increase, 0 quality prints 180.
        // The base is 100, so at 20 quality the total is 180 * 1.2 = 216.
        let contributions = PropContributions {
            flat: Roll::fixed(80.0),
            increased: Roll::zero(),
        };

        let got = prop_at_20_quality(180.0, contributions, 0.0, true);

        assert!(close(got.value, 216.0));
    }

    #[test]
    fn the_roll_range_follows_the_modifier_range() {
        // A flat modifier that could have rolled 60 to 100 and landed on 80.
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
        // 120 printed at 20 quality is 100 base, which is the bottom of the
        // range. Skipping the unwind would report 20 percent instead of zero.
        let got = prop_percentile(120.0, (100.0, 200.0), PropContributions::default(), 20.0);

        assert_eq!(got, 0.0);
    }

    #[test]
    fn a_roll_beyond_its_range_is_clamped_rather_than_reported_raw() {
        // Our data can be older than the game. A percentile of 140 on a scale
        // of 100 is a nonsense number in the UI.
        let above = prop_percentile(300.0, (100.0, 200.0), PropContributions::default(), 0.0);
        let below = prop_percentile(50.0, (100.0, 200.0), PropContributions::default(), 0.0);

        assert_eq!(above, 100.0);
        assert_eq!(below, 0.0);
    }

    #[test]
    fn a_zero_width_range_reports_the_top() {
        // Dividing by the width would give infinity or a NaN, and both render
        // as garbage. A roll that can only be one value is the best possible.
        let got = prop_percentile(100.0, (100.0, 100.0), PropContributions::default(), 0.0);

        assert_eq!(got, 100.0);
    }

    #[test]
    fn the_percentile_is_a_whole_number() {
        let got = prop_percentile(133.0, (100.0, 200.0), PropContributions::default(), 0.0);

        assert_eq!(got, got.round());
    }
}
