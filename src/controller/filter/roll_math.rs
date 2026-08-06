//! Rounding a roll into a number a filter can be built from.
//!
//! Ported from `web/price-check/filters/util.ts`.
//!
//! # Why rounding is not cosmetic
//!
//! A filter is sent to the trade site as a number. Sending 30.000000000000004
//! where the item rolled 30 finds nothing, because the site compares against
//! the value it stored and that value is 30.
//!
//! So every bound a filter carries goes through here first. The direction
//! matters too: a lower bound rounds down and an upper bound rounds up, or the
//! filter excludes the very item it was built from.

/// How the bound is being moved.
///
/// A lower bound has to end at or below the true value and an upper bound at
/// or above it. Rounding both the same way makes one of them exclude the item
/// the filter came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    Down,
    Up,
}

impl Rounding {
    fn apply(self, value: f64) -> f64 {
        match self {
            Rounding::Down => value.floor(),
            Rounding::Up => value.ceil(),
        }
    }
}

/// Scale a value up by the decimal count, nudged past a float artefact.
///
/// The nudge has to happen before the multiply, not after. A roll of 0.29
/// times 100 is 28.999999999999996, and adding an epsilon to that changes
/// nothing because the gap between neighbouring floats up there is larger than
/// the epsilon. Nudging 0.29 first survives the multiply and the value floors
/// to 29 rather than 28.
fn scaled(value: f64, decimals: u32) -> f64 {
    (value + f64::EPSILON) * 10f64.powi(decimals as i32)
}

/// How many decimals to keep.
///
/// A whole number keeps none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// The roll is a whole number.
    Whole,
    /// The roll carries decimals, so the count depends on how large it is.
    Fractional,
    /// A fixed count, used where the caller knows the stat's own precision.
    Fixed(u32),
}

/// How many decimals a value of this size should keep.
///
/// Ported from `decimalPlaces`.
///
/// # Why the count depends on the value
///
/// A roll of 1.5 loses everything it means when rounded to 2. A roll of 47.3
/// loses nothing worth keeping when rounded to 47, and the extra decimal makes
/// the filter narrower than the buyer intends.
///
/// The 2.3 boundary is the reference's. Below it two decimals, above it one.
pub fn decimal_places(value: f64, precision: Precision) -> u32 {
    match precision {
        Precision::Fixed(count) => count,
        Precision::Whole => 0,
        // A large value has nothing worth keeping past the point.
        Precision::Fractional if value.abs() >= 10.0 => 0,
        Precision::Fractional if value.abs() < 2.3 => 2,
        Precision::Fractional => 1,
    }
}

/// Round a roll to the precision its stat carries.
///
/// Ported from `roundRoll`. Truncates rather than rounds, which is what the
/// game does. Rounding up here claims the item rolled higher than it did.
pub fn round_roll(value: f64, precision: Precision) -> f64 {
    let scale = 10f64.powi(decimal_places(value, precision) as i32);

    (value * scale).trunc() / scale
}

/// Move a roll by a percentage of itself.
///
/// Ported from `percentRoll`. Used to widen a filter around the roll in hand,
/// so a ring that rolled 30 life searches from 27 rather than from exactly 30.
///
/// The shift is by the absolute value, so a negative roll widens away from
/// zero rather than towards it. A resistance penalty of -20 with a ten percent
/// widening searches from -22, not from -18.
pub fn percent_roll(value: f64, percent: f64, rounding: Rounding, precision: Precision) -> f64 {
    let decimals = decimal_places(value, precision);
    let shifted = value + (value.abs() * percent) / 100.0;

    rounding.apply(scaled(shifted, decimals)) / 10f64.powi(decimals as i32)
}

/// Move a roll by a percentage of a separate amount.
///
/// Ported from `percentRollDelta`. Used where the widening should scale with
/// the stat's own range rather than with the roll. A stat that can roll from
/// 1 to 100 deserves a wider window at 50 than a stat that only rolls 49 to
/// 51, and scaling by the roll would give them the same window.
pub fn percent_roll_delta(
    value: f64,
    delta: f64,
    percent: f64,
    rounding: Rounding,
    precision: Precision,
) -> f64 {
    let decimals = decimal_places(value, precision);
    let shifted = value + (delta * percent) / 100.0;

    rounding.apply(scaled(shifted, decimals)) / 10f64.powi(decimals as i32)
}

/// What the user can edit on this item to preview a different one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorKind {
    /// Nothing to preview.
    None,
    /// A ring or amulet takes a catalyst, which scales its own modifiers.
    Catalyst,
    /// A weapon or armour piece takes a rune, which adds one.
    Augment,
}

/// Which editor the overlay offers for an item.
///
/// Ported from `getItemEditorType`.
///
/// A unique gets nothing. Its modifiers are fixed, so previewing a change
/// shows an item that cannot exist.
pub fn editor_kind(
    category: Option<crate::types::category::ItemCategory>,
    rarity: Option<crate::types::item::ItemRarity>,
) -> EditorKind {
    use crate::types::item::ItemRarity;

    let Some(category) = category else {
        return EditorKind::None;
    };

    if category.is_ring_or_amulet() {
        return EditorKind::Catalyst;
    }

    if !(category.is_martial_weapon() || category.is_armour()) {
        return EditorKind::None;
    }

    match rarity {
        // A unique's modifiers are fixed, so previewing a change shows an item
        // that cannot exist.
        Some(ItemRarity::Unique) | None => EditorKind::None,
        Some(_) => EditorKind::Augment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::category::ItemCategory;
    use crate::types::item::ItemRarity;

    // -----------------------------------------------------------------
    // Decimal counts
    // -----------------------------------------------------------------

    #[test]
    fn a_whole_roll_keeps_no_decimals() {
        assert_eq!(decimal_places(1.5, Precision::Whole), 0);
    }

    #[test]
    fn a_fixed_count_is_taken_as_given() {
        assert_eq!(decimal_places(1.5, Precision::Fixed(3)), 3);
        assert_eq!(decimal_places(999.0, Precision::Fixed(1)), 1);
    }

    #[test]
    fn a_large_roll_keeps_no_decimals() {
        // Nothing worth keeping past the point, and the extra decimal makes
        // the filter narrower than the buyer intends.
        assert_eq!(decimal_places(47.3, Precision::Fractional), 0);
        assert_eq!(decimal_places(10.0, Precision::Fractional), 0);
    }

    #[test]
    fn a_small_roll_keeps_two_decimals() {
        // A roll of 1.5 loses everything it means when rounded to 2.
        assert_eq!(decimal_places(1.5, Precision::Fractional), 2);
    }

    #[test]
    fn a_middling_roll_keeps_one_decimal() {
        assert_eq!(decimal_places(5.0, Precision::Fractional), 1);
        assert_eq!(decimal_places(2.3, Precision::Fractional), 1);
    }

    #[test]
    fn the_count_ignores_the_sign() {
        // A resistance penalty deserves the same precision as a bonus.
        for value in [1.5f64, 5.0, 47.3] {
            assert_eq!(
                decimal_places(value, Precision::Fractional),
                decimal_places(-value, Precision::Fractional),
                "{value}"
            );
        }
    }

    // -----------------------------------------------------------------
    // Rounding a roll
    // -----------------------------------------------------------------

    #[test]
    fn a_whole_roll_rounds_to_itself() {
        assert_eq!(round_roll(30.0, Precision::Whole), 30.0);
    }

    #[test]
    fn rounding_truncates_rather_than_rounds() {
        // Rounding up claims the item rolled higher than it did.
        assert_eq!(round_roll(30.9, Precision::Whole), 30.0);
        assert_eq!(round_roll(1.999, Precision::Fractional), 1.99);
    }

    #[test]
    fn a_negative_roll_truncates_towards_zero() {
        assert_eq!(round_roll(-30.9, Precision::Whole), -30.0);
    }

    #[test]
    fn rounding_is_idempotent() {
        // A bound that shifted every time it passed through would drift.
        for value in [30.9f64, 1.999, -4.44, 0.0] {
            let once = round_roll(value, Precision::Fractional);

            assert_eq!(round_roll(once, Precision::Fractional), once, "{value}");
        }
    }

    #[test]
    fn zero_rounds_to_zero() {
        assert_eq!(round_roll(0.0, Precision::Fractional), 0.0);
    }

    // -----------------------------------------------------------------
    // Widening by a percentage
    // -----------------------------------------------------------------

    #[test]
    fn a_lower_bound_widens_downwards() {
        // A ring that rolled 30 life searches from 27 rather than exactly 30.
        assert_eq!(
            percent_roll(30.0, -10.0, Rounding::Down, Precision::Whole),
            27.0
        );
    }

    #[test]
    fn an_upper_bound_widens_upwards() {
        assert_eq!(
            percent_roll(30.0, 10.0, Rounding::Up, Precision::Whole),
            33.0
        );
    }

    #[test]
    fn a_negative_roll_widens_away_from_zero() {
        // A resistance penalty of -20 widened by ten percent searches from
        // -22, not from -18.
        assert_eq!(
            percent_roll(-20.0, 10.0, Rounding::Up, Precision::Whole),
            -18.0
        );
        assert_eq!(
            percent_roll(-20.0, -10.0, Rounding::Down, Precision::Whole),
            -22.0
        );
    }

    #[test]
    fn a_zero_shift_leaves_the_roll_where_it_is() {
        assert_eq!(
            percent_roll(30.0, 0.0, Rounding::Down, Precision::Whole),
            30.0
        );
        assert_eq!(
            percent_roll(30.0, 0.0, Rounding::Up, Precision::Whole),
            30.0
        );
    }

    #[test]
    fn a_lower_bound_never_ends_above_the_roll() {
        // Otherwise the filter excludes the very item it was built from.
        for value in [1.0f64, 5.5, 30.0, 47.3, 100.0] {
            let bound = percent_roll(value, -10.0, Rounding::Down, Precision::Whole);

            assert!(bound <= value, "{value} gave {bound}");
        }
    }

    #[test]
    fn an_upper_bound_never_ends_below_the_roll() {
        for value in [1.0f64, 5.5, 30.0, 47.3, 100.0] {
            let bound = percent_roll(value, 10.0, Rounding::Up, Precision::Whole);

            assert!(bound >= value, "{value} gave {bound}");
        }
    }

    #[test]
    fn a_float_artefact_does_not_cost_the_roll_a_point() {
        // 0.29 times 100 is 28.999999999999996, which floors to 28 and turns
        // the filter into 0.28. The item then fails its own filter.
        assert_eq!(
            percent_roll(0.29, 0.0, Rounding::Down, Precision::Fixed(2)),
            0.29
        );
    }

    #[test]
    fn a_fractional_roll_keeps_its_decimals_when_widened() {
        assert_eq!(
            percent_roll(1.5, -10.0, Rounding::Down, Precision::Fractional),
            1.35
        );
    }

    // -----------------------------------------------------------------
    // Widening by a separate amount
    // -----------------------------------------------------------------

    #[test]
    fn a_delta_shift_scales_with_the_amount_given() {
        // A stat that can roll from 1 to 100 deserves a wider window at 50
        // than a stat that only rolls 49 to 51.
        let wide = percent_roll_delta(50.0, 99.0, -10.0, Rounding::Down, Precision::Whole);
        let narrow = percent_roll_delta(50.0, 2.0, -10.0, Rounding::Down, Precision::Whole);

        assert_eq!(wide, 40.0);
        assert_eq!(narrow, 49.0);
    }

    #[test]
    fn a_delta_shift_of_zero_leaves_the_roll_alone() {
        assert_eq!(
            percent_roll_delta(50.0, 0.0, -10.0, Rounding::Down, Precision::Whole),
            50.0
        );
    }

    #[test]
    fn a_delta_shift_ignores_the_sign_of_the_roll() {
        // Unlike percent_roll, the amount is given outright, so a negative
        // roll shifts by the same amount a positive one does.
        assert_eq!(
            percent_roll_delta(-50.0, 100.0, 10.0, Rounding::Up, Precision::Whole),
            -40.0
        );
    }

    // -----------------------------------------------------------------
    // Editors
    // -----------------------------------------------------------------

    #[test]
    fn a_ring_or_amulet_takes_a_catalyst() {
        for category in [ItemCategory::Ring, ItemCategory::Amulet] {
            assert_eq!(
                editor_kind(Some(category), Some(ItemRarity::Rare)),
                EditorKind::Catalyst,
                "{category:?}"
            );
        }
    }

    #[test]
    fn a_weapon_or_armour_piece_takes_a_rune() {
        for category in [ItemCategory::Bow, ItemCategory::BodyArmour] {
            assert_eq!(
                editor_kind(Some(category), Some(ItemRarity::Rare)),
                EditorKind::Augment,
                "{category:?}"
            );
        }
    }

    #[test]
    fn a_unique_gets_no_editor() {
        // Its modifiers are fixed, so previewing a change shows an item that
        // cannot exist.
        assert_eq!(
            editor_kind(Some(ItemCategory::Bow), Some(ItemRarity::Unique)),
            EditorKind::None
        );
    }

    #[test]
    fn a_unique_ring_still_takes_a_catalyst() {
        // A catalyst scales what is already there, so it applies to a unique.
        assert_eq!(
            editor_kind(Some(ItemCategory::Ring), Some(ItemRarity::Unique)),
            EditorKind::Catalyst
        );
    }

    #[test]
    fn an_item_with_no_category_gets_no_editor() {
        assert_eq!(editor_kind(None, Some(ItemRarity::Rare)), EditorKind::None);
    }

    #[test]
    fn a_weapon_with_no_rarity_gets_no_editor() {
        // Without a rarity there is no way to rule out a unique.
        assert_eq!(editor_kind(Some(ItemCategory::Bow), None), EditorKind::None);
    }

    #[test]
    fn something_that_takes_neither_gets_no_editor() {
        assert_eq!(
            editor_kind(Some(ItemCategory::Flask), Some(ItemRarity::Magic)),
            EditorKind::None
        );
        assert_eq!(
            editor_kind(Some(ItemCategory::Map), Some(ItemRarity::Normal)),
            EditorKind::None
        );
    }
}
