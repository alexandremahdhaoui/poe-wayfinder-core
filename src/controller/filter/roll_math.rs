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

fn scaled(value: f64, decimals: u32) -> f64 {
    (value + f64::EPSILON) * 10f64.powi(decimals as i32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    Whole,
    Fractional,
    Fixed(u32),
}

pub fn decimal_places(value: f64, precision: Precision) -> u32 {
    match precision {
        Precision::Fixed(count) => count,
        Precision::Whole => 0,
        Precision::Fractional if value.abs() >= 10.0 => 0,
        Precision::Fractional if value.abs() < 2.3 => 2,
        Precision::Fractional => 1,
    }
}

pub fn round_roll(value: f64, precision: Precision) -> f64 {
    let scale = 10f64.powi(decimal_places(value, precision) as i32);

    (value * scale).trunc() / scale
}

pub fn percent_roll(value: f64, percent: f64, rounding: Rounding, precision: Precision) -> f64 {
    let decimals = decimal_places(value, precision);
    let shifted = value + (value.abs() * percent) / 100.0;

    rounding.apply(scaled(shifted, decimals)) / 10f64.powi(decimals as i32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorKind {
    None,
    Catalyst,
    Augment,
}

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
        Some(ItemRarity::Unique) | None => EditorKind::None,
        Some(_) => EditorKind::Augment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::category::ItemCategory;
    use crate::types::item::ItemRarity;

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
        assert_eq!(decimal_places(47.3, Precision::Fractional), 0);
        assert_eq!(decimal_places(10.0, Precision::Fractional), 0);
    }

    #[test]
    fn a_small_roll_keeps_two_decimals() {
        assert_eq!(decimal_places(1.5, Precision::Fractional), 2);
    }

    #[test]
    fn a_middling_roll_keeps_one_decimal() {
        assert_eq!(decimal_places(5.0, Precision::Fractional), 1);
        assert_eq!(decimal_places(2.3, Precision::Fractional), 1);
    }

    #[test]
    fn the_count_ignores_the_sign() {
        for value in [1.5f64, 5.0, 47.3] {
            assert_eq!(
                decimal_places(value, Precision::Fractional),
                decimal_places(-value, Precision::Fractional),
                "{value}"
            );
        }
    }

    #[test]
    fn a_whole_roll_rounds_to_itself() {
        assert_eq!(round_roll(30.0, Precision::Whole), 30.0);
    }

    #[test]
    fn rounding_truncates_rather_than_rounds() {
        assert_eq!(round_roll(30.9, Precision::Whole), 30.0);
        assert_eq!(round_roll(1.999, Precision::Fractional), 1.99);
    }

    #[test]
    fn a_negative_roll_truncates_towards_zero() {
        assert_eq!(round_roll(-30.9, Precision::Whole), -30.0);
    }

    #[test]
    fn rounding_is_idempotent() {
        for value in [30.9f64, 1.999, -4.44, 0.0] {
            let once = round_roll(value, Precision::Fractional);

            assert_eq!(round_roll(once, Precision::Fractional), once, "{value}");
        }
    }

    #[test]
    fn zero_rounds_to_zero() {
        assert_eq!(round_roll(0.0, Precision::Fractional), 0.0);
    }

    #[test]
    fn a_lower_bound_widens_downwards() {
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
        assert_eq!(
            editor_kind(Some(ItemCategory::Bow), Some(ItemRarity::Unique)),
            EditorKind::None
        );
    }

    #[test]
    fn a_unique_ring_still_takes_a_catalyst() {
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
