pub const ITEM_LEVEL_BRACKETS: &[u32] = &[1, 50, 68, 75, 84];

pub const AREA_LEVEL_BRACKETS: &[u32] = &[1, 68, 73, 75, 78, 80];

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

pub fn ceil_to_bracket(value: u32, brackets: &[u32]) -> u32 {
    for &bracket in brackets {
        if bracket >= value {
            return bracket;
        }
    }

    brackets.last().copied().unwrap_or(value)
}

fn ascendancy_table(reference_name: &str) -> Option<&'static [(u32, u32)]> {
    match reference_name {
        "Inscribed Ultimatum" => Some(&[(1, 1), (2, 60), (3, 75)]),
        "Djinn Barya" => Some(&[(1, 1), (2, 45), (3, 60), (4, 75)]),
        _ => None,
    }
}

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
        assert_eq!(floor_to_bracket(83, ITEM_LEVEL_BRACKETS), 75);
        assert_eq!(floor_to_bracket(70, ITEM_LEVEL_BRACKETS), 68);
    }

    #[test]
    fn a_value_above_every_bracket_takes_the_highest() {
        assert_eq!(floor_to_bracket(100, ITEM_LEVEL_BRACKETS), 84);
    }

    #[test]
    fn a_value_below_the_first_bracket_takes_the_first() {
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
        for brackets in [ITEM_LEVEL_BRACKETS, AREA_LEVEL_BRACKETS] {
            assert!(brackets.windows(2).all(|w| w[0] < w[1]), "{brackets:?}");
        }
    }

    #[test]
    fn each_trial_base_grants_the_points_its_area_level_earns() {
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
        assert_eq!(area_level_by_ascendancy_points("Djinn Barya", 3), 60);
        assert_eq!(
            area_level_by_ascendancy_points("Inscribed Ultimatum", 3),
            75
        );
    }

    #[test]
    fn a_point_count_the_base_cannot_reach_reports_zero() {
        assert_eq!(area_level_by_ascendancy_points("Inscribed Ultimatum", 4), 0);
    }

    #[test]
    fn the_two_lookups_are_consistent() {
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
