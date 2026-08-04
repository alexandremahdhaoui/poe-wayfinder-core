//! Deciding which filters are worth switching on and which are worth showing.
//!
//! Ported from `create-stat-filters.ts`.
//!
//! # Why this layer exists
//!
//! Every filter reaching the query is not the same as the query being usable.
//! A rare has six modifiers, every one gets a filter, and a query requiring
//! all six finds the one listing that is this exact item.
//!
//! So the filters arrive disabled and this decides which ones to switch on. A
//! modifier that rolled near the top of its range is why the item is worth
//! anything, and a modifier that rolled badly is noise.
//!
//! # Why some filters are hidden entirely
//!
//! A unique's stats are fixed. Filtering on them narrows nothing and clutters
//! the panel with rows nobody will ever click.

use crate::types::stat::{StatBetter, StatRoll};

/// How good a roll is, from 0 for the worst possible to 1 for the best.
///
/// Ported from the goodness calculation in `calculatedStatToFilter`.
///
/// Returns None when the stat has no direction, because a granted skill or an
/// allocated passive has no better or worse. Guessing one would enable filters
/// on the wrong things.
pub fn goodness(roll: StatRoll, better: StatBetter) -> Option<f64> {
    if better == StatBetter::NotComparable {
        return None;
    }

    // A roll that could only be one value is by definition the best possible.
    // Dividing by the width here would give infinity or a NaN.
    if (roll.max - roll.min).abs() < f64::EPSILON {
        return Some(1.0);
    }

    let raw = (roll.value - roll.min) / (roll.max - roll.min);

    let scaled = if better == StatBetter::NegativeRoll {
        1.0 - raw
    } else {
        raw
    };

    // A legacy roll sits outside its stated range, so the fraction can leave
    // the scale. Clamping keeps the threshold meaningful.
    Some(scaled.clamp(0.0, 1.0))
}

/// Why a filter is not shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hidden {
    /// The roll cannot vary, so filtering on it narrows nothing.
    ConstantRoll,
    /// A unique's fixed stat.
    UniqueFixed,
}

/// What the item is, as far as these rules care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFacts {
    pub is_unique: bool,
    /// The item can still be crafted.
    pub is_modifiable: bool,
    /// A magic item that cannot be improved.
    pub is_finished_magic: bool,
}

/// Whether a filter should be hidden.
///
/// Ported from `hideNotVariableStat`. A unique's stats are fixed, so filtering
/// on them narrows nothing and fills the panel with rows nobody clicks.
///
/// The charge counter is the exception. It varies on a unique and people do
/// search for it.
pub fn hidden_reason(reference: &str, roll: Option<StatRoll>, facts: ItemFacts) -> Option<Hidden> {
    if !facts.is_unique {
        return None;
    }

    // Varies on a unique and people do search for it.
    if reference == "# uses remaining" {
        return None;
    }

    let Some(roll) = roll else {
        // No roll at all on a unique means a fixed line.
        return Some(Hidden::ConstantRoll);
    };

    // A roll whose range is a single point cannot vary either.
    if (roll.max - roll.min).abs() < f64::EPSILON {
        return Some(Hidden::UniqueFixed);
    }

    None
}

/// How much of a roll's range to give away when filling in a bound.
///
/// Ported from the percent table in `calculatedStatToFilter`. A finished magic
/// item cannot be improved, so its roll is exactly what a buyer gets and the
/// filter can be pinned. Everything else gets room.
pub fn tolerance_for(facts: ItemFacts) -> f64 {
    if facts.is_finished_magic || !facts.is_modifiable {
        return 0.0;
    }

    0.1
}

/// Which end of a roll to constrain, given its direction.
///
/// Ported from `filterFillMinMax`. A stat with no direction gets both ends,
/// because the only meaningful search is for the value it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillEnds {
    /// Set a floor.
    Min,
    /// Set a ceiling.
    Max,
    /// Set both.
    Both,
}

/// Which ends to fill for a direction.
pub fn fill_ends(better: StatBetter) -> FillEnds {
    match better {
        StatBetter::PositiveRoll => FillEnds::Min,
        StatBetter::NegativeRoll => FillEnds::Max,
        StatBetter::NotComparable => FillEnds::Both,
    }
}

/// Whether a filter should start switched on.
///
/// Ported from `enableGoodRolledFilters`.
///
/// A modifier that rolled near the top of its range is why the item is worth
/// anything. A modifier that rolled badly is noise and enabling it excludes
/// every item that rolled it worse, which is most of them.
///
/// A stat with no direction is enabled, because its presence is the point.
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
        // A presence check. Its being there is the whole filter.
        return true;
    };

    let Some(goodness) = goodness(roll, better) else {
        // No direction means no way to judge it, and its presence is the
        // point.
        return true;
    };

    goodness >= above
}

/// The fraction of its range a roll must reach to be switched on.
///
/// Ported from the reference's own default. Eighty percent is high enough that
/// only the modifiers that make the item interesting are enabled.
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

    // -----------------------------------------------------------------
    // Goodness
    // -----------------------------------------------------------------

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
        // A low attribute requirement is a good roll. Not flipping would
        // enable filters on exactly the wrong items.
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
        // Dividing by the width would give infinity or a NaN.
        assert_eq!(
            goodness(roll(50.0, 50.0, 50.0), StatBetter::PositiveRoll),
            Some(1.0)
        );
    }

    #[test]
    fn a_stat_with_no_direction_has_no_goodness() {
        // A granted skill has no better or worse, and guessing one would
        // enable filters on the wrong things.
        assert_eq!(
            goodness(roll(50.0, 0.0, 100.0), StatBetter::NotComparable),
            None
        );
    }

    #[test]
    fn a_legacy_roll_above_its_range_is_clamped_to_perfect() {
        // It sits outside its stated range, so the raw fraction leaves the
        // scale and the threshold stops meaning anything.
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

    // -----------------------------------------------------------------
    // Hiding
    // -----------------------------------------------------------------

    #[test]
    fn a_rare_hides_nothing() {
        assert_eq!(
            hidden_reason("# to maximum Life", Some(roll(50.0, 50.0, 50.0)), rare()),
            None
        );
    }

    #[test]
    fn a_uniques_fixed_stat_is_hidden() {
        // Filtering on it narrows nothing and fills the panel with rows
        // nobody clicks.
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
        // It varies on a unique and people do search for it.
        assert_eq!(
            hidden_reason("# uses remaining", Some(roll(3.0, 3.0, 3.0)), unique()),
            None
        );
    }

    // -----------------------------------------------------------------
    // Tolerance
    // -----------------------------------------------------------------

    #[test]
    fn a_craftable_item_gets_room() {
        assert_eq!(tolerance_for(rare()), 0.1);
    }

    #[test]
    fn a_finished_magic_item_is_pinned_exactly() {
        // It cannot be improved, so its roll is exactly what a buyer gets.
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

    // -----------------------------------------------------------------
    // Which ends to fill
    // -----------------------------------------------------------------

    #[test]
    fn each_direction_fills_the_helpful_end() {
        assert_eq!(fill_ends(StatBetter::PositiveRoll), FillEnds::Min);
        assert_eq!(fill_ends(StatBetter::NegativeRoll), FillEnds::Max);
        assert_eq!(fill_ends(StatBetter::NotComparable), FillEnds::Both);
    }

    // -----------------------------------------------------------------
    // Enabling
    // -----------------------------------------------------------------

    #[test]
    fn a_good_roll_is_enabled() {
        // It is why the item is worth anything.
        assert!(should_enable(
            Some(roll(95.0, 50.0, 100.0)),
            StatBetter::PositiveRoll,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_bad_roll_is_left_off() {
        // Enabling it excludes every item that rolled it worse, which is most
        // of them.
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
        // Its being there is the whole filter.
        assert!(should_enable(
            None,
            StatBetter::PositiveRoll,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_stat_with_no_direction_is_enabled() {
        // No way to judge it, and its presence is the point.
        assert!(should_enable(
            Some(roll(50.0, 0.0, 100.0)),
            StatBetter::NotComparable,
            None,
            GOOD_ROLL_THRESHOLD
        ));
    }

    #[test]
    fn a_fixed_roll_on_a_rare_is_enabled() {
        // It could only be that value, so it is a perfect roll.
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
        // Too low and every modifier is enabled, which is the same as having
        // no rule at all. Too high and none are, which is the same again.
        //
        // Read through a variable so this is a real check rather than a
        // constant the compiler folds away.
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
