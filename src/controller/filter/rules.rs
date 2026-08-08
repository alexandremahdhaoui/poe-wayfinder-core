use crate::types::stat::{StatBetter, StatRoll};

pub fn goodness(roll: StatRoll, better: StatBetter) -> Option<f64> {
    if better == StatBetter::NotComparable {
        return None;
    }

    if (roll.max - roll.min).abs() < f64::EPSILON {
        return Some(1.0);
    }

    let raw = (roll.value - roll.min) / (roll.max - roll.min);

    let scaled = if better == StatBetter::NegativeRoll {
        1.0 - raw
    } else {
        raw
    };

    Some(scaled.clamp(0.0, 1.0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hidden {
    ConstantRoll,
    UniqueFixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFacts {
    pub is_unique: bool,
    pub is_modifiable: bool,
    pub is_finished_magic: bool,
}

pub fn hidden_reason(reference: &str, roll: Option<StatRoll>, facts: ItemFacts) -> Option<Hidden> {
    if !facts.is_unique {
        return None;
    }

    if reference == "# uses remaining" {
        return None;
    }

    let Some(roll) = roll else {
        return Some(Hidden::ConstantRoll);
    };

    if (roll.max - roll.min).abs() < f64::EPSILON {
        return Some(Hidden::UniqueFixed);
    }

    None
}

pub fn tolerance_for(facts: ItemFacts) -> f64 {
    if facts.is_finished_magic || !facts.is_modifiable {
        return 0.0;
    }

    0.1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillEnds {
    Min,
    Max,
    Both,
}

pub fn fill_ends(better: StatBetter) -> FillEnds {
    match better {
        StatBetter::PositiveRoll => FillEnds::Min,
        StatBetter::NegativeRoll => FillEnds::Max,
        StatBetter::NotComparable => FillEnds::Both,
    }
}

pub fn should_enable(
    roll: Option<StatRoll>,
    better: StatBetter,
    hidden: Option<Hidden>,
    above: f64,
) -> bool {
    if hidden.is_some() {
        return false;
    }

    let Some(roll) = roll else {
        return true;
    };

    let Some(goodness) = goodness(roll, better) else {
        return true;
    };

    goodness >= above
}

pub const GOOD_ROLL_THRESHOLD: f64 = 0.8;

#[cfg(test)]
mod tests {
    use super::*;

    fn roll(value: f64, min: f64, max: f64) -> StatRoll {
        StatRoll {
            value,
            min,
            max,
            ..StatRoll::default()
        }
    }

    fn rare() -> ItemFacts {
        ItemFacts {
            is_unique: false,
            is_modifiable: true,
            is_finished_magic: false,
        }
    }

    fn unique() -> ItemFacts {
        ItemFacts {
            is_unique: true,
            is_modifiable: false,
            is_finished_magic: false,
        }
    }

    #[test]
    fn a_roll_at_the_top_of_its_range_is_perfect() {
        assert_eq!(
            goodness(roll(100.0, 50.0, 100.0), StatBetter::PositiveRoll),
            Some(1.0)
        );
    }

    #[test]
    fn a_roll_at_the_bottom_of_its_range_is_worthless() {
        assert_eq!(
            goodness(roll(50.0, 50.0, 100.0), StatBetter::PositiveRoll),
            Some(0.0)
        );
    }

    #[test]
    fn a_roll_in_the_middle_is_half() {
        assert_eq!(
            goodness(roll(75.0, 50.0, 100.0), StatBetter::PositiveRoll),
            Some(0.5)
        );
    }

    #[test]
    fn the_scale_flips_for_a_stat_that_is_better_low() {
        assert_eq!(
            goodness(roll(50.0, 50.0, 100.0), StatBetter::NegativeRoll),
            Some(1.0)
        );
        assert_eq!(
            goodness(roll(100.0, 50.0, 100.0), StatBetter::NegativeRoll),
            Some(0.0)
        );
    }

    #[test]
    fn a_roll_that_cannot_vary_is_perfect() {
        assert_eq!(
            goodness(roll(50.0, 50.0, 50.0), StatBetter::PositiveRoll),
            Some(1.0)
        );
    }

    #[test]
    fn a_stat_with_no_direction_has_no_goodness() {
        assert_eq!(
            goodness(roll(50.0, 0.0, 100.0), StatBetter::NotComparable),
            None
        );
    }

    #[test]
    fn a_legacy_roll_above_its_range_is_clamped_to_perfect() {
        assert_eq!(
            goodness(roll(150.0, 50.0, 100.0), StatBetter::PositiveRoll),
            Some(1.0)
        );
    }

    #[test]
    fn a_legacy_roll_below_its_range_is_clamped_to_worthless() {
        assert_eq!(
            goodness(roll(10.0, 50.0, 100.0), StatBetter::PositiveRoll),
            Some(0.0)
        );
    }

    #[test]
    fn a_rare_hides_nothing() {
        assert_eq!(
            hidden_reason("# to maximum Life", Some(roll(50.0, 50.0, 50.0)), rare()),
            None
        );
    }

    #[test]
    fn a_uniques_fixed_stat_is_hidden() {
        assert_eq!(
            hidden_reason("# to maximum Life", Some(roll(50.0, 50.0, 50.0)), unique()),
            Some(Hidden::UniqueFixed)
        );
    }

    #[test]
    fn a_uniques_varying_stat_is_shown() {
        assert_eq!(
            hidden_reason("# to maximum Life", Some(roll(75.0, 50.0, 100.0)), unique()),
            None
        );
    }

    #[test]
    fn a_uniques_stat_with_no_roll_is_hidden() {
        assert_eq!(
            hidden_reason("Cannot be Frozen", None, unique()),
            Some(Hidden::ConstantRoll)
        );
    }

    #[test]
    fn the_charge_counter_is_shown_on_a_unique() {
        assert_eq!(
            hidden_reason("# uses remaining", Some(roll(3.0, 3.0, 3.0)), unique()),
            None
        );
    }

    #[test]
    fn a_craftable_item_gets_room() {
        assert_eq!(tolerance_for(rare()), 0.1);
    }

    #[test]
    fn a_finished_magic_item_is_pinned_exactly() {
        let facts = ItemFacts {
            is_finished_magic: true,
            ..rare()
        };

        assert_eq!(tolerance_for(facts), 0.0);
    }

    #[test]
    fn an_uncraftable_item_is_pinned_exactly() {
        assert_eq!(tolerance_for(unique()), 0.0);
    }

    #[test]
    fn each_direction_fills_the_helpful_end() {
        assert_eq!(fill_ends(StatBetter::PositiveRoll), FillEnds::Min);
        assert_eq!(fill_ends(StatBetter::NegativeRoll), FillEnds::Max);
        assert_eq!(fill_ends(StatBetter::NotComparable), FillEnds::Both);
    }

    #[test]
    fn a_good_roll_is_enabled() {
        assert!(should_enable(
            Some(roll(95.0, 50.0, 100.0)),
            StatBetter::PositiveRoll,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_bad_roll_is_left_off() {
        assert!(!should_enable(
            Some(roll(55.0, 50.0, 100.0)),
            StatBetter::PositiveRoll,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_roll_exactly_at_the_threshold_is_enabled() {
        assert!(should_enable(
            Some(roll(90.0, 50.0, 100.0)),
            StatBetter::PositiveRoll,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_hidden_filter_is_never_enabled() {
        assert!(!should_enable(
            Some(roll(100.0, 50.0, 100.0)),
            StatBetter::PositiveRoll,
            Some(Hidden::UniqueFixed),
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_presence_check_is_enabled() {
        assert!(should_enable(
            None,
            StatBetter::PositiveRoll,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_stat_with_no_direction_is_enabled() {
        assert!(should_enable(
            Some(roll(50.0, 0.0, 100.0)),
            StatBetter::NotComparable,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_fixed_roll_on_a_rare_is_enabled() {
        assert!(should_enable(
            Some(roll(50.0, 50.0, 50.0)),
            StatBetter::PositiveRoll,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_good_low_roll_is_enabled_for_a_stat_that_is_better_low() {
        assert!(should_enable(
            Some(roll(55.0, 50.0, 100.0)),
            StatBetter::NegativeRoll,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn the_threshold_is_high_enough_to_mean_something() {
        let threshold = std::hint::black_box(GOOD_ROLL_THRESHOLD);

        assert!((0.7..1.0).contains(&threshold));
    }

    #[test]
    fn a_threshold_of_zero_enables_everything() {
        assert!(should_enable(
            Some(roll(50.0, 50.0, 100.0)),
            StatBetter::PositiveRoll,
            None,
            0.0
        ));
    }
}
