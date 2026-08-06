//! The bounds a stat filter carries, and how they flip.
//!
//! Ported from `shortRollToFilter`, `filterAdjustmentForNegate` and
//! `getMinMax`.
//!
//! # Why a filter carries four numbers rather than two
//!
//! The user sees a slider. It needs the roll the item actually has, the range
//! the slider starts at, and the range the slider may be dragged across. Those
//! are three different things and collapsing them loses the ability to widen a
//! search after seeing the first results.

use super::roll_math::{percent_roll, round_roll, Precision, Rounding};

/// A pair of bounds, either of which may be open.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Bounds {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl Bounds {
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }
}

/// Everything a filter knows about one roll.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterRoll {
    /// What the item rolled.
    pub value: f64,
    /// What is being sent right now.
    pub sent: Bounds,
    /// Where the slider starts.
    pub default: Bounds,
    /// How far the slider may be dragged.
    ///
    /// Absent when the stat has no known range, which is most of them.
    pub bounds: Option<Bounds>,
    /// The stat reads as a reduction but is stored as a negative increase.
    pub negated: bool,
    /// The trade site stores this stat with its sign flipped.
    pub trade_invert: bool,
    pub precision: Precision,
}

/// Build the roll a filter starts with.
///
/// Ported from `shortRollToFilter`.
///
/// # Why the default is clamped to the bounds
///
/// Widening by a percentage can push the default past what the stat can
/// actually roll. A filter asking for more than the maximum returns nothing,
/// and the user sees an empty result rather than a slider they can fix.
pub fn short_roll_to_filter(
    value: f64,
    min: f64,
    max: f64,
    percent: f64,
    show_bounds: bool,
) -> FilterRoll {
    let precision = Precision::Whole;

    let bounds = Bounds::new(
        percent_roll(min, 0.0, Rounding::Down, precision),
        percent_roll(max, 0.0, Rounding::Up, precision),
    );

    let mut default = Bounds::new(
        percent_roll(value, -percent, Rounding::Down, precision),
        percent_roll(value, percent, Rounding::Up, precision),
    );

    // A default past what the stat can roll returns nothing, and the user sees
    // an empty result rather than a slider they can fix.
    default.min = default.min.map(|v| v.max(bounds.min.unwrap_or(v)));
    default.max = default.max.map(|v| v.min(bounds.max.unwrap_or(v)));

    FilterRoll {
        value: round_roll(value, precision),
        // A higher roll is better here, so only the floor is sent. Sending
        // the ceiling too would exclude every listing better than this item.
        sent: Bounds {
            min: default.min,
            max: None,
        },
        default,
        // A range is only worth showing when there is one. A stat that rolls a
        // single value has a slider that cannot move.
        bounds: (show_bounds && min != max).then_some(bounds),
        negated: false,
        trade_invert: false,
        precision,
    }
}

/// Flip a roll's sign.
///
/// Ported from `filterAdjustmentForNegate`.
///
/// # Why a stat gets negated
///
/// "20% reduced Attribute Requirements" is stored on the trade site as -20 of
/// the increased version. A user reading the item sees 20 and a filter of 20
/// finds nothing.
///
/// The bounds swap ends as well as sign. A range of 10 to 30 negates to -30 to
/// -10, and keeping the ends in place would give a range whose floor is above
/// its ceiling, which matches nothing at all.
pub fn negate(roll: &mut FilterRoll) {
    roll.trade_invert = !roll.trade_invert;
    roll.negated = true;

    roll.value = -roll.value;
    roll.sent = swap_negated(roll.sent);
    roll.default = swap_negated(roll.default);
    roll.bounds = roll.bounds.map(swap_negated);
}

/// Negate a pair of bounds, swapping the ends.
fn swap_negated(bounds: Bounds) -> Bounds {
    Bounds {
        min: bounds.max.map(|v| -v),
        max: bounds.min.map(|v| -v),
    }
}

/// The floor and ceiling to send to the trade site.
///
/// Ported from `getMinMax`. An inverted stat sends its ends swapped, because
/// the site stores the sign flipped and a floor there is a ceiling here.
pub fn for_trade(roll: &FilterRoll) -> Bounds {
    if !roll.trade_invert {
        return roll.sent;
    }

    Bounds {
        min: roll.sent.max.map(|v| -v),
        max: roll.sent.min.map(|v| -v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roll() -> FilterRoll {
        short_roll_to_filter(30.0, 10.0, 50.0, 10.0, true)
    }

    #[test]
    fn the_value_is_what_the_item_rolled() {
        assert_eq!(roll().value, 30.0);
    }

    #[test]
    fn the_default_widens_around_the_roll() {
        let got = roll();

        assert_eq!(got.default.min, Some(27.0));
        assert_eq!(got.default.max, Some(33.0));
    }

    #[test]
    fn only_the_floor_is_sent() {
        // Sending the ceiling too would exclude every listing better than this
        // item, which is the opposite of what a buyer wants.
        let got = roll();

        assert_eq!(got.sent.min, Some(27.0));
        assert_eq!(got.sent.max, None);
    }

    #[test]
    fn the_default_is_clamped_to_what_the_stat_can_roll() {
        // A filter asking for more than the maximum returns nothing, and the
        // user sees an empty result rather than a slider they can fix.
        let got = short_roll_to_filter(50.0, 10.0, 50.0, 50.0, true);

        assert_eq!(got.default.max, Some(50.0));
    }

    #[test]
    fn the_default_floor_is_clamped_too() {
        let got = short_roll_to_filter(10.0, 10.0, 50.0, 50.0, true);

        assert_eq!(got.default.min, Some(10.0));
    }

    #[test]
    fn a_stat_that_rolls_a_single_value_shows_no_slider() {
        // The slider could not move.
        assert_eq!(
            short_roll_to_filter(30.0, 30.0, 30.0, 10.0, true).bounds,
            None
        );
    }

    #[test]
    fn a_stat_whose_range_is_not_worth_showing_has_no_bounds() {
        assert_eq!(
            short_roll_to_filter(30.0, 10.0, 50.0, 10.0, false).bounds,
            None
        );
    }

    #[test]
    fn the_bounds_are_what_the_stat_can_roll() {
        assert_eq!(roll().bounds, Some(Bounds::new(10.0, 50.0)));
    }

    #[test]
    fn a_fresh_roll_is_neither_negated_nor_inverted() {
        let got = roll();

        assert!(!got.negated);
        assert!(!got.trade_invert);
    }

    // -----------------------------------------------------------------
    // Negation
    // -----------------------------------------------------------------

    #[test]
    fn negating_flips_the_value() {
        // A user reading "20% reduced" sees 20, and a filter of 20 finds
        // nothing because the site stores -20.
        let mut got = roll();

        negate(&mut got);

        assert_eq!(got.value, -30.0);
    }

    #[test]
    fn negating_swaps_the_ends_as_well_as_the_sign() {
        // Keeping the ends in place gives a range whose floor is above its
        // ceiling, which matches nothing at all.
        let mut got = roll();

        negate(&mut got);

        assert_eq!(got.default, Bounds::new(-33.0, -27.0));
        assert_eq!(got.bounds, Some(Bounds::new(-50.0, -10.0)));
    }

    #[test]
    fn negating_turns_an_open_floor_into_an_open_ceiling() {
        let mut got = roll();

        negate(&mut got);

        assert_eq!(got.sent.min, None);
        assert_eq!(got.sent.max, Some(-27.0));
    }

    #[test]
    fn negating_marks_the_roll() {
        let mut got = roll();

        negate(&mut got);

        assert!(got.negated);
        assert!(got.trade_invert);
    }

    #[test]
    fn negating_twice_returns_the_original_numbers() {
        // A user toggling the option twice must land back where they started.
        let original = roll();
        let mut got = original;

        negate(&mut got);
        negate(&mut got);

        assert_eq!(got.value, original.value);
        assert_eq!(got.default, original.default);
        assert_eq!(got.bounds, original.bounds);
        assert_eq!(got.sent, original.sent);
        assert!(!got.trade_invert);
    }

    #[test]
    fn a_roll_with_no_bounds_negates_without_inventing_any() {
        let mut got = short_roll_to_filter(30.0, 10.0, 50.0, 10.0, false);

        negate(&mut got);

        assert_eq!(got.bounds, None);
    }

    // -----------------------------------------------------------------
    // What reaches the request
    // -----------------------------------------------------------------

    #[test]
    fn a_plain_roll_sends_its_bounds_as_they_are() {
        assert_eq!(for_trade(&roll()), roll().sent);
    }

    #[test]
    fn an_inverted_roll_sends_its_ends_swapped() {
        // The site stores the sign flipped, so a floor there is a ceiling here.
        let mut got = roll();
        got.trade_invert = true;

        assert_eq!(
            for_trade(&got),
            Bounds {
                min: None,
                max: Some(-27.0)
            }
        );
    }

    #[test]
    fn an_inverted_two_ended_roll_swaps_both() {
        let mut got = roll();
        got.trade_invert = true;
        got.sent = Bounds::new(27.0, 33.0);

        assert_eq!(for_trade(&got), Bounds::new(-33.0, -27.0));
    }

    #[test]
    fn a_negated_roll_reaches_the_request_the_right_way_round() {
        // The whole point. A negated filter that sends its floor as a floor
        // asks for at least -27, which every item satisfies.
        let mut got = roll();

        negate(&mut got);

        let sent = for_trade(&got);

        assert_eq!(sent.min, Some(27.0));
        assert_eq!(sent.max, None);
    }
}
