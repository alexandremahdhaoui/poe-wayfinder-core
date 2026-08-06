//! Which wording of a stat to show for a given roll.
//!
//! Ported from `translateStatWithRoll`.
//!
//! # Why a stat has more than one wording
//!
//! `10% reduced Attack Speed` and `10% increased Attack Speed` are the same
//! stat with the same trade id. The game prints whichever reads correctly for
//! the sign, and a panel showing the wrong one tells the user their item has a
//! bonus where it has a penalty.
//!
//! Some wordings bake their value into the text. `Adds 1 Passive Skill` has no
//! placeholder because the value is always one, and it must be preferred over
//! the generic form when the roll matches.

use crate::types::stat::{StatMatcher, StatRoll};

/// Pick the wording to show.
///
/// Returns nothing when the stat has no wordings at all, which means the data
/// file is malformed. Inventing text would put a made up stat in front of the
/// user.
pub fn wording_for(matchers: &[StatMatcher], roll: Option<StatRoll>) -> Option<&StatMatcher> {
    let Some(roll) = roll else {
        // With no roll there is nothing to match a baked value against, so the
        // generic form is the only honest choice.
        return generic(matchers).or_else(|| matchers.first());
    };

    // A wording that bakes this exact value in reads better than the generic
    // one. "Adds 1 Passive Skill" beats "Adds # Passive Skills".
    let wanted = roll.option.unwrap_or(roll.value);

    if let Some(exact) = matchers.iter().find(|m| m.value == Some(wanted)) {
        return Some(exact);
    }

    // A range that straddles zero has no single correct sign, so the positive
    // wording is used and the sign shows in the number.
    let same_sign = roll.min.signum() == roll.max.signum();

    let by_sign = if same_sign {
        matchers
            .iter()
            .find(|m| m.value.is_none() && m.negate == (roll.value < 0.0))
    } else {
        matchers.iter().find(|m| m.value.is_none() && !m.negate)
    };

    by_sign
        .or_else(|| generic(matchers))
        .or_else(|| matchers.first())
}

/// The first wording that takes any roll.
fn generic(matchers: &[StatMatcher]) -> Option<&StatMatcher> {
    matchers.iter().find(|m| m.value.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(string: &str, negate: bool, value: Option<f64>) -> StatMatcher {
        StatMatcher {
            string: string.to_string(),
            negate,
            value,
            ..StatMatcher::default()
        }
    }

    fn attack_speed() -> Vec<StatMatcher> {
        vec![
            matcher("#% increased Attack Speed", false, None),
            matcher("#% reduced Attack Speed", true, None),
        ]
    }

    fn roll(value: f64, min: f64, max: f64) -> StatRoll {
        StatRoll {
            value,
            min,
            max,
            ..StatRoll::default()
        }
    }

    #[test]
    fn a_positive_roll_reads_as_increased() {
        let matchers = attack_speed();
        let got = wording_for(&matchers, Some(roll(10.0, 5.0, 15.0))).expect("a wording");

        assert_eq!(got.string, "#% increased Attack Speed");
    }

    #[test]
    fn a_negative_roll_reads_as_reduced() {
        // Showing the increased wording tells the user their item has a bonus
        // where it has a penalty.
        let matchers = attack_speed();
        let got = wording_for(&matchers, Some(roll(-10.0, -15.0, -5.0))).expect("a wording");

        assert_eq!(got.string, "#% reduced Attack Speed");
    }

    #[test]
    fn a_range_that_straddles_zero_reads_as_increased() {
        // There is no single correct sign, so the sign shows in the number.
        let matchers = attack_speed();
        let got = wording_for(&matchers, Some(roll(-5.0, -10.0, 10.0))).expect("a wording");

        assert_eq!(got.string, "#% increased Attack Speed");
    }

    #[test]
    fn a_wording_that_bakes_the_value_in_wins() {
        // "Adds 1 Passive Skill" beats "Adds # Passive Skills".
        let matchers = vec![
            matcher("Adds # Passive Skills", false, None),
            matcher("Adds 1 Passive Skill", false, Some(1.0)),
        ];

        let got = wording_for(&matchers, Some(roll(1.0, 1.0, 1.0))).expect("a wording");

        assert_eq!(got.string, "Adds 1 Passive Skill");
    }

    #[test]
    fn a_baked_value_that_does_not_match_is_not_used() {
        let matchers = vec![
            matcher("Adds # Passive Skills", false, None),
            matcher("Adds 1 Passive Skill", false, Some(1.0)),
        ];

        let got = wording_for(&matchers, Some(roll(3.0, 3.0, 3.0))).expect("a wording");

        assert_eq!(got.string, "Adds # Passive Skills");
    }

    #[test]
    fn a_fixed_option_is_matched_before_the_value() {
        // A stat whose roll is an option carries the option in a field of its
        // own, and matching on the value would pick the wrong wording.
        let matchers = vec![
            matcher("Something #", false, None),
            matcher("Something specific", false, Some(7.0)),
        ];

        let mut r = roll(1.0, 1.0, 1.0);
        r.option = Some(7.0);

        assert_eq!(
            wording_for(&matchers, Some(r)).expect("a wording").string,
            "Something specific"
        );
    }

    #[test]
    fn a_stat_with_no_roll_takes_the_generic_wording() {
        // With no roll there is nothing to match a baked value against.
        let matchers = vec![
            matcher("Adds 1 Passive Skill", false, Some(1.0)),
            matcher("Adds # Passive Skills", false, None),
        ];

        assert_eq!(
            wording_for(&matchers, None).expect("a wording").string,
            "Adds # Passive Skills"
        );
    }

    #[test]
    fn a_stat_with_only_baked_wordings_falls_back_to_the_first() {
        // Showing nothing would drop the modifier from the panel entirely.
        let matchers = vec![matcher("Cannot be Frozen", false, Some(1.0))];

        assert_eq!(
            wording_for(&matchers, None).expect("a wording").string,
            "Cannot be Frozen"
        );
    }

    #[test]
    fn a_roll_with_no_matching_sign_falls_back_rather_than_vanishing() {
        // A stat that only ships an increased wording still has to show a
        // negative roll.
        let matchers = vec![matcher("#% increased Attack Speed", false, None)];

        assert_eq!(
            wording_for(&matchers, Some(roll(-10.0, -15.0, -5.0)))
                .expect("a wording")
                .string,
            "#% increased Attack Speed"
        );
    }

    #[test]
    fn a_stat_with_no_wordings_at_all_reports_nothing() {
        // That means the data file is malformed. Inventing text would put a
        // made up stat in front of the user.
        assert!(wording_for(&[], Some(roll(10.0, 5.0, 15.0))).is_none());
        assert!(wording_for(&[], None).is_none());
    }

    #[test]
    fn a_zero_roll_reads_as_increased() {
        // Zero is not negative, and "0% reduced" reads as a bug.
        let matchers = attack_speed();
        let got = wording_for(&matchers, Some(roll(0.0, 0.0, 0.0))).expect("a wording");

        assert_eq!(got.string, "#% increased Attack Speed");
    }
}
