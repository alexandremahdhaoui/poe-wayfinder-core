//! Matching a stat line to a stat, and reading its roll.
//!
//! Ported from `tryParseTranslation` and `parseRoll` in
//! `renderer/src/parser/stat-translations.ts`.

use crate::adapter::data_adapter::{StatLookup, TradeStatLookup};
use crate::controller::stat_match::placeholder::{candidates, Candidate, NumMatch, StatString};
use crate::types::stat::{ParsedStat, StatHit, StatRoll};
use crate::util::number::roll_or_minmax_avg;

/// Match a stat line and read its roll.
///
/// Candidates are tried in order and the first one the data recognises wins.
/// That order is why a stat whose number belongs to its name matches before
/// the generic form.
pub fn try_parse_translation(stat: &StatString, data: &dyn StatLookup) -> Option<ParsedStat> {
    for candidate in candidates(&stat.string) {
        let Some(hit) = data.stat_by_matcher(&candidate.template) else {
            continue;
        };

        return Some(ParsedStat {
            reference: hit.stat.reference.clone(),
            matched: hit.matcher.string.clone(),
            roll: parse_roll(&hit, &candidate, stat),
        });
    }

    None
}

/// Match a stat line against the trade site's own list when our data misses.
///
/// Ported from `trySecondaryParseTranslation`.
///
/// # Why there is a second attempt at all
///
/// Our stat table is built from the game's own wordings. The trade site keeps
/// its own list and adds new stats there first, so a league's new modifier is
/// searchable days before our table knows the wording.
///
/// The fallback takes the line as written and asks the trade list directly. It
/// gives no roll, because a stat matched by its whole text has no placeholder
/// to read a number out of. A filter on its presence is still worth more than
/// dropping the modifier as unknown.
pub fn try_secondary_parse_translation(
    text: &str,
    data: &dyn TradeStatLookup,
) -> Option<ParsedStat> {
    if !data.trade_stat_exists(text) {
        return None;
    }

    Some(ParsedStat {
        // The line itself is the reference. There is no canonical form to map
        // to, which is the whole reason this path exists.
        reference: text.to_string(),
        matched: text.to_string(),
        roll: None,
    })
}

/// Read the roll a candidate placeheld.
///
/// Returns None when the candidate placeheld nothing and the matcher bakes no
/// value in, which means the stat has no roll to filter on.
pub fn parse_roll(hit: &StatHit, candidate: &Candidate, stat: &StatString) -> Option<StatRoll> {
    let mut values = candidate.values.clone();

    // The game prints some stats as a reduction with a positive number. Miss
    // this and the sign on the whole filter flips.
    if hit.matcher.negate {
        for v in &mut values {
            v.roll = -v.roll;

            if let Some((min, max)) = v.bounds {
                v.bounds = Some((-min, -max));
            }
        }
    }

    // A charge counter's floor is one use and the game does not say so.
    if hit.stat.reference == "# uses remaining" {
        if let Some(first) = values.first_mut() {
            let max = first.bounds.map_or(first.roll, |(_, max)| max);
            first.bounds = Some((1.0, max));
        }
    }

    let legacy = normalise_bounds(&mut values);

    // A matcher that bakes its value in has no placeholder to read, so the
    // baked value is the roll.
    if values.is_empty() {
        let baked = hit.matcher.value?;

        values.push(NumMatch {
            roll: baked,
            roll_str: baked.to_string(),
            decimal: false,
            bounds: Some((baked, baked)),
        });
    }

    if values.is_empty() {
        return None;
    }

    let rolls: Vec<f64> = values.iter().map(|v| v.roll).collect();
    let mins: Vec<f64> = values
        .iter()
        .map(|v| v.bounds.map_or(v.roll, |(min, _)| min))
        .collect();
    let maxes: Vec<f64> = values
        .iter()
        .map(|v| v.bounds.map_or(v.roll, |(_, max)| max))
        .collect();

    Some(StatRoll {
        value: roll_or_minmax_avg(&rolls)?,
        min: roll_or_minmax_avg(&mins)?,
        max: roll_or_minmax_avg(&maxes)?,
        unscalable: stat.unscalable,
        legacy,
        decimals: hit.stat.decimals || values.iter().any(|v| v.decimal),
        option: hit.matcher.value,
    })
}

/// Put every bound the right way round and widen it to fit the roll.
///
/// Returns whether any roll fell outside its stated range, which makes the
/// item legacy.
fn normalise_bounds(values: &mut [NumMatch]) -> bool {
    let mut legacy = false;

    for v in values.iter_mut() {
        let Some((mut min, mut max)) = v.bounds else {
            continue;
        };

        // Negating a range swaps its ends. So does a legacy modifier that
        // shares its translation with a newer one. Neither is a legacy roll on
        // its own, so this swap does not set the flag.
        if min > max {
            std::mem::swap(&mut min, &mut max);
        }

        // A roll outside its stated range means the item predates a balance
        // change. It cannot be crafted again, so the UI has to say so.
        if v.roll > max {
            max = v.roll;
            legacy = true;
        }

        if v.roll < min {
            min = v.roll;
            legacy = true;
        }

        v.bounds = Some((min, max));
    }

    legacy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::stat::{Stat, StatMatcher};

    /// A stat table backed by a fixed list.
    ///
    /// Hand written rather than mocked. The trait has one method and a fake
    /// that holds real stats reads better than an expectation script.
    struct FakeStats {
        stats: Vec<Stat>,
    }

    impl StatLookup for FakeStats {
        fn stat_by_matcher(&self, template: &str) -> Option<StatHit<'_>> {
            for stat in &self.stats {
                for matcher in &stat.matchers {
                    if matcher.string == template {
                        return Some(StatHit { stat, matcher });
                    }
                }
            }

            None
        }
    }

    fn stat(reference: &str, matchers: Vec<StatMatcher>) -> Stat {
        Stat {
            reference: reference.into(),
            matchers,
            ..Stat::default()
        }
    }

    fn plain(s: &str) -> StatMatcher {
        StatMatcher {
            string: s.into(),
            ..StatMatcher::default()
        }
    }

    fn line(s: &str) -> StatString {
        StatString {
            string: s.into(),
            unscalable: false,
        }
    }

    fn life_table() -> FakeStats {
        FakeStats {
            stats: vec![stat("# to maximum Life", vec![plain("# to maximum Life")])],
        }
    }

    #[test]
    fn a_stat_line_matches_and_reports_its_roll() {
        let got = try_parse_translation(&line("+25 to maximum Life"), &life_table()).unwrap();

        assert_eq!(got.reference, "# to maximum Life");
        assert_eq!(got.roll.unwrap().value, 25.0);
    }

    #[test]
    fn an_unknown_stat_line_matches_nothing() {
        let got = try_parse_translation(&line("+25 to maximum Sanity"), &life_table());

        assert!(got.is_none());
    }

    #[test]
    fn an_inline_range_becomes_the_roll_bounds() {
        let got =
            try_parse_translation(&line("+25(20-30) to maximum Life"), &life_table()).unwrap();

        let roll = got.roll.unwrap();
        assert_eq!(roll.value, 25.0);
        assert_eq!(roll.min, 20.0);
        assert_eq!(roll.max, 30.0);
        assert!(!roll.legacy);
    }

    #[test]
    fn a_roll_with_no_range_reports_itself_as_both_bounds() {
        let got = try_parse_translation(&line("+25 to maximum Life"), &life_table()).unwrap();

        let roll = got.roll.unwrap();
        assert_eq!(roll.min, 25.0);
        assert_eq!(roll.max, 25.0);
    }

    #[test]
    fn a_roll_above_its_stated_range_is_legacy() {
        // The item predates a balance change. It cannot be crafted again and
        // is usually worth more, so the UI has to say so.
        let got =
            try_parse_translation(&line("+40(20-30) to maximum Life"), &life_table()).unwrap();

        let roll = got.roll.unwrap();
        assert!(roll.legacy);
        assert_eq!(roll.max, 40.0);
    }

    #[test]
    fn a_roll_below_its_stated_range_is_legacy() {
        let got =
            try_parse_translation(&line("+10(20-30) to maximum Life"), &life_table()).unwrap();

        let roll = got.roll.unwrap();
        assert!(roll.legacy);
        assert_eq!(roll.min, 10.0);
    }

    #[test]
    fn a_backwards_range_is_swapped_and_is_not_legacy() {
        // A legacy modifier can share a translation with a newer one and print
        // its bounds the other way round. That is not a legacy roll.
        let got =
            try_parse_translation(&line("+25(30-20) to maximum Life"), &life_table()).unwrap();

        let roll = got.roll.unwrap();
        assert_eq!(roll.min, 20.0);
        assert_eq!(roll.max, 30.0);
        assert!(!roll.legacy);
    }

    #[test]
    fn a_negating_matcher_flips_the_sign() {
        // "10% reduced Attack Speed" is the same stat as -10% increased. Miss
        // this and the filter asks for the opposite of what the item has.
        let table = FakeStats {
            stats: vec![stat(
                "#% increased Attack Speed",
                vec![StatMatcher {
                    string: "#% reduced Attack Speed".into(),
                    negate: true,
                    ..StatMatcher::default()
                }],
            )],
        };

        let got = try_parse_translation(&line("10% reduced Attack Speed"), &table).unwrap();

        assert_eq!(got.roll.unwrap().value, -10.0);
    }

    #[test]
    fn a_negating_matcher_flips_the_bounds_too() {
        let table = FakeStats {
            stats: vec![stat(
                "#% increased Attack Speed",
                vec![StatMatcher {
                    string: "#% reduced Attack Speed".into(),
                    negate: true,
                    ..StatMatcher::default()
                }],
            )],
        };

        let got = try_parse_translation(&line("10(5-15)% reduced Attack Speed"), &table).unwrap();

        let roll = got.roll.unwrap();
        assert_eq!(roll.value, -10.0);
        // Negating swaps the ends, and normalising puts them back in order.
        assert_eq!(roll.min, -15.0);
        assert_eq!(roll.max, -5.0);
        assert!(!roll.legacy);
    }

    #[test]
    fn a_matcher_that_bakes_its_value_in_reports_that_value() {
        // "Adds 1 Passive Skill" carries no placeholder because the value is
        // always one. Reporting no roll would drop the filter entirely.
        let table = FakeStats {
            stats: vec![stat(
                "Adds # Passive Skills",
                vec![StatMatcher {
                    string: "Adds 1 Passive Skill".into(),
                    value: Some(1.0),
                    ..StatMatcher::default()
                }],
            )],
        };

        let got = try_parse_translation(&line("Adds 1 Passive Skill"), &table).unwrap();

        let roll = got.roll.unwrap();
        assert_eq!(roll.value, 1.0);
        assert_eq!(roll.option, Some(1.0));
    }

    #[test]
    fn a_stat_with_no_roll_and_no_baked_value_reports_no_roll() {
        let table = FakeStats {
            stats: vec![stat("Cannot be Frozen", vec![plain("Cannot be Frozen")])],
        };

        let got = try_parse_translation(&line("Cannot be Frozen"), &table).unwrap();

        assert_eq!(got.reference, "Cannot be Frozen");
        assert!(got.roll.is_none());
    }

    #[test]
    fn a_two_roll_stat_averages_its_values() {
        let table = FakeStats {
            stats: vec![stat(
                "Adds # to # Fire Damage",
                vec![plain("Adds # to # Fire Damage")],
            )],
        };

        let got = try_parse_translation(&line("Adds 5 to 15 Fire Damage"), &table).unwrap();

        assert_eq!(got.roll.unwrap().value, 10.0);
    }

    #[test]
    fn a_two_roll_stat_averages_each_bound_separately() {
        let table = FakeStats {
            stats: vec![stat(
                "Adds # to # Fire Damage",
                vec![plain("Adds # to # Fire Damage")],
            )],
        };

        let got =
            try_parse_translation(&line("Adds 5(3-7) to 15(10-20) Fire Damage"), &table).unwrap();

        let roll = got.roll.unwrap();
        assert_eq!(roll.value, 10.0);
        assert_eq!(roll.min, 6.5);
        assert_eq!(roll.max, 13.5);
    }

    #[test]
    fn a_literal_match_beats_the_placeheld_one() {
        // Both forms are in the table. The literal has to win or every gem
        // reads its granted skill level as a roll.
        let table = FakeStats {
            stats: vec![
                stat(
                    "Grants Level 20 Purity of Fire",
                    vec![plain("Grants Level 20 Purity of Fire")],
                ),
                stat(
                    "Grants Level # Purity of Fire",
                    vec![plain("Grants Level # Purity of Fire")],
                ),
            ],
        };

        let got = try_parse_translation(&line("Grants Level 20 Purity of Fire"), &table).unwrap();

        assert_eq!(got.reference, "Grants Level 20 Purity of Fire");
        assert!(got.roll.is_none());
    }

    #[test]
    fn the_placeheld_form_wins_when_the_literal_is_absent() {
        let table = FakeStats {
            stats: vec![stat(
                "Grants Level # Purity of Fire",
                vec![plain("Grants Level # Purity of Fire")],
            )],
        };

        let got = try_parse_translation(&line("Grants Level 20 Purity of Fire"), &table).unwrap();

        assert_eq!(got.reference, "Grants Level # Purity of Fire");
        assert_eq!(got.roll.unwrap().value, 20.0);
    }

    #[test]
    fn a_charge_counter_gets_a_floor_of_one_use() {
        // The game prints no lower bound. Without the floor the range reads as
        // a point and the item looks unusable.
        let table = FakeStats {
            stats: vec![stat("# uses remaining", vec![plain("# uses remaining")])],
        };

        let got = try_parse_translation(&line("3 uses remaining"), &table).unwrap();

        let roll = got.roll.unwrap();
        assert_eq!(roll.min, 1.0);
        assert_eq!(roll.max, 3.0);
    }

    #[test]
    fn an_unscalable_line_carries_the_flag_through() {
        let stat_line = StatString {
            string: "+25 to maximum Life".into(),
            unscalable: true,
        };

        let got = try_parse_translation(&stat_line, &life_table()).unwrap();

        assert!(got.roll.unwrap().unscalable);
    }

    #[test]
    fn a_decimal_roll_sets_the_decimal_flag() {
        let table = FakeStats {
            stats: vec![stat(
                "#% of Damage Leeched",
                vec![plain("#% of Damage Leeched")],
            )],
        };

        let got = try_parse_translation(&line("1.5% of Damage Leeched"), &table).unwrap();

        assert!(got.roll.unwrap().decimals);
    }

    #[test]
    fn a_stat_marked_decimal_in_the_data_sets_the_flag_on_a_whole_roll() {
        // The trade site expects two decimal places for these whether or not
        // the game printed any.
        let table = FakeStats {
            stats: vec![Stat {
                reference: "#% of Damage Leeched".into(),
                decimals: true,
                matchers: vec![plain("#% of Damage Leeched")],
                ..Stat::default()
            }],
        };

        let got = try_parse_translation(&line("2% of Damage Leeched"), &table).unwrap();

        assert!(got.roll.unwrap().decimals);
    }

    #[test]
    fn the_matched_literal_form_is_reported_alongside_the_reference() {
        // The UI shows what the item says. The query uses the reference.
        let table = FakeStats {
            stats: vec![stat(
                "#% increased Attack Speed",
                vec![StatMatcher {
                    string: "#% reduced Attack Speed".into(),
                    negate: true,
                    ..StatMatcher::default()
                }],
            )],
        };

        let got = try_parse_translation(&line("10% reduced Attack Speed"), &table).unwrap();

        assert_eq!(got.reference, "#% increased Attack Speed");
        assert_eq!(got.matched, "#% reduced Attack Speed");
    }

    // -----------------------------------------------------------------
    // The trade list fallback
    // -----------------------------------------------------------------

    struct FakeTradeStats {
        known: Vec<&'static str>,
    }

    impl TradeStatLookup for FakeTradeStats {
        fn trade_stat_exists(&self, text: &str) -> bool {
            self.known.contains(&text)
        }
    }

    #[test]
    fn a_stat_the_trade_site_knows_is_matched_by_its_whole_text() {
        // A league's new modifier is searchable days before our table knows
        // its wording.
        let data = FakeTradeStats {
            known: vec!["+15% to Brand New Resistance"],
        };

        let got = try_secondary_parse_translation("+15% to Brand New Resistance", &data)
            .expect("a known stat matches");

        assert_eq!(got.reference, "+15% to Brand New Resistance");
        assert_eq!(got.matched, "+15% to Brand New Resistance");
    }

    #[test]
    fn the_fallback_gives_no_roll() {
        // A stat matched by its whole text has no placeholder to read a number
        // out of, and inventing one would send a filter for a value the item
        // may not have.
        let data = FakeTradeStats {
            known: vec!["+15% to Brand New Resistance"],
        };

        let got = try_secondary_parse_translation("+15% to Brand New Resistance", &data)
            .expect("a known stat matches");

        assert_eq!(got.roll, None);
    }

    #[test]
    fn a_stat_neither_list_knows_is_left_unmatched() {
        let data = FakeTradeStats { known: vec![] };

        assert_eq!(try_secondary_parse_translation("Mystery line", &data), None);
    }

    #[test]
    fn the_match_is_exact() {
        // A near match is a different stat with a different trade id, and the
        // filter would search for something the item does not have.
        let data = FakeTradeStats {
            known: vec!["+15% to Brand New Resistance"],
        };

        assert_eq!(
            try_secondary_parse_translation("+16% to Brand New Resistance", &data),
            None
        );
    }

    #[test]
    fn an_empty_line_matches_nothing() {
        let data = FakeTradeStats {
            known: vec!["+15% to Brand New Resistance"],
        };

        assert_eq!(try_secondary_parse_translation("", &data), None);
    }
}
