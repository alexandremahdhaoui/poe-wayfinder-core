use super::roll_math::{percent_roll, round_roll, Precision, Rounding};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterRoll {
    pub value: f64,
    pub sent: Bounds,
    pub default: Bounds,
    pub bounds: Option<Bounds>,
    pub negated: bool,
    pub trade_invert: bool,
    pub precision: Precision,
}

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

    default.min = default.min.map(|v| v.max(bounds.min.unwrap_or(v)));
    default.max = default.max.map(|v| v.min(bounds.max.unwrap_or(v)));

    FilterRoll {
        value: round_roll(value, precision),
        sent: Bounds {
            min: default.min,
            max: None,
        },
        default,
        bounds: (show_bounds && min != max).then_some(bounds),
        negated: false,
        trade_invert: false,
        precision,
    }
}

pub fn negate(roll: &mut FilterRoll) {
    roll.trade_invert = !roll.trade_invert;
    roll.negated = true;

    roll.value = -roll.value;
    roll.sent = swap_negated(roll.sent);
    roll.default = swap_negated(roll.default);
    roll.bounds = roll.bounds.map(swap_negated);
}

fn swap_negated(bounds: Bounds) -> Bounds {
    Bounds {
        min: bounds.max.map(|v| -v),
        max: bounds.min.map(|v| -v),
    }
}

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
        let got = roll();

        assert_eq!(got.sent.min, Some(27.0));
        assert_eq!(got.sent.max, None);
    }

    #[test]
    fn the_default_is_clamped_to_what_the_stat_can_roll() {
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

    #[test]
    fn negating_flips_the_value() {
        let mut got = roll();

        negate(&mut got);

        assert_eq!(got.value, -30.0);
    }

    #[test]
    fn negating_swaps_the_ends_as_well_as_the_sign() {
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

    #[test]
    fn a_plain_roll_sends_its_bounds_as_they_are() {
        assert_eq!(for_trade(&roll()), roll().sent);
    }

    #[test]
    fn an_inverted_roll_sends_its_ends_swapped() {
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
        let mut got = roll();

        negate(&mut got);

        let sent = for_trade(&got);

        assert_eq!(sent.min, Some(27.0));
        assert_eq!(sent.max, None);
    }
}
