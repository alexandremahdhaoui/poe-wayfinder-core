pub fn safe_zone(level: u32) -> u32 {
    3 + level / 16
}

pub fn effective_difference(character: u32, area: u32) -> u32 {
    let gap = character.abs_diff(area);

    gap.saturating_sub(safe_zone(character))
}

pub fn penalty(character: u32, area: u32) -> f64 {
    let over = effective_difference(character, area);

    if over == 0 {
        return 1.0;
    }

    let level = character.max(1) as f64;
    let ratio = (level + 5.0) / (level + 5.0 + f64::from(over).powf(2.5));

    ratio.powi(15).clamp(0.0, 1.0)
}

pub fn caption(character: u32, area: u32) -> String {
    let share = penalty(character, area);

    match share >= 1.0 {
        true => "full experience".to_string(),
        false => format!("{:.0}% experience", share * 100.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_safe_zone_widens_as_you_level() {
        assert!(safe_zone(70) > safe_zone(10));
        assert_eq!(safe_zone(0), 3);
    }

    #[test]
    fn an_area_inside_the_safe_zone_costs_nothing() {
        assert_eq!(penalty(70, 70), 1.0);
        assert_eq!(penalty(70, 68), 1.0);
        assert_eq!(caption(70, 70), "full experience");
    }

    #[test]
    fn the_penalty_is_the_same_whether_the_area_is_above_or_below() {
        assert_eq!(penalty(70, 80), penalty(70, 60));
    }

    #[test]
    fn a_bigger_gap_costs_more() {
        let near = penalty(70, 80);
        let far = penalty(70, 90);

        assert!(far < near, "{far} should be worse than {near}");
        assert!(far >= 0.0);
    }

    #[test]
    fn the_penalty_never_leaves_its_range() {
        for character in [1, 20, 50, 70, 100] {
            for area in [1, 20, 50, 70, 100] {
                let got = penalty(character, area);

                assert!(
                    (0.0..=1.0).contains(&got),
                    "{character} vs {area} gave {got}"
                );
            }
        }
    }

    #[test]
    fn the_difference_counts_only_what_is_outside_the_safe_zone() {
        assert_eq!(effective_difference(70, 70), 0);
        assert_eq!(effective_difference(70, 80), 10 - safe_zone(70));
    }

    #[test]
    fn a_reduced_share_is_reported_as_a_percentage() {
        let text = caption(70, 95);

        assert!(text.contains('%'), "{text}");
        assert!(!text.contains("full"));
    }
}
