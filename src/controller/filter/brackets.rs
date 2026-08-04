//! Rounding a value to the bracket the trade site indexes.
//!
//! Ported from `ceilToBracket`, `floorToBracket`, `areaLevelByAscendancyPoints`
//! and `ascendancyPointsByAreaLevel` in `create-item-filters.ts`.
//!
//! # Why brackets instead of the value
//!
//! An item level of 83 and one of 84 are the same item to a buyer, but a
//! filter pinned to 84 excludes every 83. The game's own drop tables change at
//! a handful of levels, and those are the only places a filter should sit.
//!
//! Rounding to the bracket below turns "at least 84" into "at least 84", and
//! "at least 83" into "at least 75", which is what a buyer actually means.

/// The item level brackets the trade site indexes.
///
/// These are where the game's drop tables change. Anything between them is the
/// same item to a buyer.
pub const ITEM_LEVEL_BRACKETS: &[u32] = &[1, 50, 68, 75, 84];

/// The area level brackets for a map or logbook.
pub const AREA_LEVEL_BRACKETS: &[u32] = &[1, 68, 73, 75, 78, 80];

/// Round down to the nearest bracket.
///
/// Ported from `floorToBracket`. A value below the first bracket returns the
/// first, because a filter of zero matches everything and narrows nothing.
pub fn floor_to_bracket(value: u32, brackets: &[u32]) -> u32 {
    let mut best = brackets.first().copied().unwrap_or(value);

    for &bracket in brackets {
        if bracket <= value {
            best = bracket;
        } else {
            break;
        }
    }

    best
}

/// Round up to the nearest bracket.
///
/// Ported from `ceilToBracket`. Used where a lower bound would widen the
/// search past what the buyer asked for.
pub fn ceil_to_bracket(value: u32, brackets: &[u32]) -> u32 {
    for &bracket in brackets {
        if bracket >= value {
            return bracket;
        }
    }

    brackets.last().copied().unwrap_or(value)
}

/// The area levels at which a trial base grants another ascendancy point.
///
/// Ported from `ASCENDANCY_POINTS`. This is what a trial item is bought for:
/// nobody wants a specific area level, they want the point count it grants.
fn ascendancy_table(reference_name: &str) -> Option<&'static [(u32, u32)]> {
    match reference_name {
        "Inscribed Ultimatum" => Some(&[(1, 1), (2, 60), (3, 75)]),
        "Djinn Barya" => Some(&[(1, 1), (2, 45), (3, 60), (4, 75)]),
        _ => None,
    }
}

/// How many ascendancy points an area level grants.
///
/// Ported from `ascendancyPointsByAreaLevel`. Returns zero for a base that
/// grants none, which is every base but the two trial items.
pub fn ascendancy_points_by_area_level(reference_name: &str, area_level: u32) -> u32 {
    let Some(table) = ascendancy_table(reference_name) else {
        return 0;
    };

    let mut best = 0;

    for &(points, required) in table {
        if area_level >= required {
            best = points;
        }
    }

    best
}

/// The lowest area level that grants this many points.
///
/// Ported from `areaLevelByAscendancyPoints`. This is the filter a buyer
/// wants: the cheapest item that still grants the points they need.
///
/// Returns zero for a base that grants no points, and for a point count higher
/// than the base can reach.
pub fn area_level_by_ascendancy_points(reference_name: &str, points: u32) -> u32 {
    let Some(table) = ascendancy_table(reference_name) else {
        return 0;
    };

    table
        .iter()
        .find(|(p, _)| *p == points)
        .map_or(0, |(_, level)| *level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_on_a_bracket_stays_there() {
        assert_eq!(floor_to_bracket(84, ITEM_LEVEL_BRACKETS), 84);
        assert_eq!(floor_to_bracket(68, ITEM_LEVEL_BRACKETS), 68);
    }

    #[test]
    fn a_value_between_brackets_rounds_down() {
        // An item level of 83 and one of 84 are the same item to a buyer, but
        // a filter pinned to 83 excludes nothing useful and narrows nothing.
        assert_eq!(floor_to_bracket(83, ITEM_LEVEL_BRACKETS), 75);
        assert_eq!(floor_to_bracket(70, ITEM_LEVEL_BRACKETS), 68);
    }

    #[test]
    fn a_value_above_every_bracket_takes_the_highest() {
        assert_eq!(floor_to_bracket(100, ITEM_LEVEL_BRACKETS), 84);
    }

    #[test]
    fn a_value_below_the_first_bracket_takes_the_first() {
        // A filter of zero matches everything and narrows nothing.
        assert_eq!(floor_to_bracket(0, ITEM_LEVEL_BRACKETS), 1);
    }

    #[test]
    fn rounding_up_finds_the_next_bracket() {
        assert_eq!(ceil_to_bracket(70, ITEM_LEVEL_BRACKETS), 75);
        assert_eq!(ceil_to_bracket(84, ITEM_LEVEL_BRACKETS), 84);
    }

    #[test]
    fn rounding_up_past_every_bracket_takes_the_highest() {
        assert_eq!(ceil_to_bracket(100, ITEM_LEVEL_BRACKETS), 84);
    }

    #[test]
    fn rounding_up_is_never_below_rounding_down() {
        for value in 0..=100 {
            assert!(
                ceil_to_bracket(value, ITEM_LEVEL_BRACKETS)
                    >= floor_to_bracket(value, ITEM_LEVEL_BRACKETS),
                "{value}"
            );
        }
    }

    #[test]
    fn an_empty_bracket_list_returns_the_value_unchanged() {
        assert_eq!(floor_to_bracket(42, &[]), 42);
        assert_eq!(ceil_to_bracket(42, &[]), 42);
    }

    #[test]
    fn the_area_level_brackets_are_ascending() {
        // The rounding walks them in order and stops at the first miss, so an
        // unsorted list would round to the wrong bracket.
        for brackets in [ITEM_LEVEL_BRACKETS, AREA_LEVEL_BRACKETS] {
            assert!(brackets.windows(2).all(|w| w[0] < w[1]), "{brackets:?}");
        }
    }

    #[test]
    fn each_trial_base_grants_the_points_its_area_level_earns() {
        // A trial item is bought for its point count and nothing else.
        assert_eq!(ascendancy_points_by_area_level("Djinn Barya", 45), 2);
        assert_eq!(ascendancy_points_by_area_level("Djinn Barya", 60), 3);
        assert_eq!(ascendancy_points_by_area_level("Djinn Barya", 75), 4);

        assert_eq!(
            ascendancy_points_by_area_level("Inscribed Ultimatum", 60),
            2
        );
        assert_eq!(
            ascendancy_points_by_area_level("Inscribed Ultimatum", 75),
            3
        );
    }

    #[test]
    fn an_area_level_between_thresholds_grants_the_lower_count() {
        assert_eq!(ascendancy_points_by_area_level("Djinn Barya", 59), 2);
    }

    #[test]
    fn an_area_level_above_every_threshold_grants_the_most() {
        assert_eq!(ascendancy_points_by_area_level("Djinn Barya", 100), 4);
    }

    #[test]
    fn a_base_that_grants_no_points_reports_zero() {
        assert_eq!(ascendancy_points_by_area_level("Sapphire Ring", 80), 0);
        assert_eq!(area_level_by_ascendancy_points("Sapphire Ring", 3), 0);
    }

    #[test]
    fn the_cheapest_item_granting_a_point_count_is_found() {
        // This is the filter a buyer wants: the cheapest item that still
        // grants the points they need.
        assert_eq!(area_level_by_ascendancy_points("Djinn Barya", 3), 60);
        assert_eq!(
            area_level_by_ascendancy_points("Inscribed Ultimatum", 3),
            75
        );
    }

    #[test]
    fn a_point_count_the_base_cannot_reach_reports_zero() {
        // An Inscribed Ultimatum tops out at three.
        assert_eq!(area_level_by_ascendancy_points("Inscribed Ultimatum", 4), 0);
    }

    #[test]
    fn the_two_lookups_are_consistent() {
        // Reading the level for a point count and reading the count back must
        // agree, or the filter asks for something the item does not grant.
        for base in ["Djinn Barya", "Inscribed Ultimatum"] {
            for points in 1..=4 {
                let level = area_level_by_ascendancy_points(base, points);

                if level == 0 {
                    continue;
                }

                assert_eq!(
                    ascendancy_points_by_area_level(base, level),
                    points,
                    "{base} at {points}"
                );
            }
        }
    }
}
