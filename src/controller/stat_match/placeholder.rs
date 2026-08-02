//! Turning a stat line into the templates the data files are keyed by.
//!
//! Ported from `_statPlaceholderGenerator` in
//! `renderer/src/parser/stat-translations.ts`.
//!
//! # The problem
//!
//! The game prints `+25 to maximum Life`. The data file is keyed by
//! `# to maximum Life`. Replacing every number with a hash looks like it would
//! be enough, and it is not.
//!
//! `Adds 5 to 12 Fire Damage to Attacks` has two numbers and both are rolls.
//! `+2 to Level of all Fire Skill Gems` has one number that is a roll.
//! `Grants Level 20 Purity of Fire` has one number that is part of the name.
//!
//! Nothing in the text says which is which. So the parser produces every
//! template the line could be, most placeheld first, and the first one the
//! data file recognises wins.
//!
//! # Advanced Item Description
//!
//! With it on the game also prints the roll's range inline.
//!
//! ```text
//! +25(20-30) to maximum Life
//! ```
//!
//! That is where the min and max on each match come from. Without it the
//! range is unknown and the client cannot tell a good roll from a bad one.

use crate::types::client_strings as cs;

/// One number found in a stat line.
#[derive(Debug, Clone, PartialEq)]
pub struct NumMatch {
    /// The value as printed.
    pub roll: f64,
    /// The exact characters, kept so a template can restore them verbatim.
    pub roll_str: String,
    /// Whether any part of the number or its range carried a decimal point.
    pub decimal: bool,
    /// The roll's possible range, when the game printed one.
    pub bounds: Option<(f64, f64)>,
}

/// One candidate template and the numbers it left as placeholders.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The stat text with some numbers replaced by `#`.
    pub template: String,
    /// The numbers this template treats as rolls, in order.
    pub values: Vec<NumMatch>,
}

/// Which indices keep their literal value, per number count.
///
/// Copied from `PLACEHOLDER_MAP`. Order is the search order, so the most
/// placeheld template is tried first. A template that keeps a literal only
/// wins when the data file has no more general form.
const PLACEHOLDER_MAP: [&[&[usize]]; 5] = [
    // 0 numbers.
    &[&[]],
    // 1 number.
    &[&[0], &[]],
    // 2 numbers.
    &[&[0, 1], &[0], &[1], &[]],
    // 3 numbers.
    &[&[0, 1, 2], &[1, 2], &[0, 2], &[0, 1], &[2], &[1], &[0], &[]],
    // 4 numbers.
    &[
        &[0, 1, 2, 3],
        &[1, 2, 3],
        &[0, 2, 3],
        &[0, 1, 3],
        &[0, 1, 2],
        &[2, 3],
        &[1, 3],
        &[1, 2],
        &[0, 3],
        &[0, 2],
        &[0, 1],
        &[3],
        &[2],
        &[1],
        &[0],
        &[],
    ],
];

/// A stat line and whether its value can be scaled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatString {
    pub string: String,
    /// The game marked this value as not scalable by quality or increases.
    pub unscalable: bool,
}

/// Split the unscalable marker off a stat line.
pub fn split_unscalable(text: &str) -> StatString {
    match text.strip_suffix(cs::UNSCALABLE_VALUE) {
        Some(rest) => StatString {
            string: rest.to_string(),
            unscalable: true,
        },
        None => StatString {
            string: text.to_string(),
            unscalable: false,
        },
    }
}

/// Whether a line is a reminder string rather than a stat.
///
/// The game wraps explanatory text in parentheses. It is not a stat and
/// matching it against the data file would waste every lookup on it.
pub fn opens_reminder(line: &str) -> bool {
    line.trim_start().starts_with(['(', '（'])
}

/// Whether a line closes a reminder string.
pub fn closes_reminder(line: &str) -> bool {
    line.trim_end().ends_with([')', '）'])
}

/// Every template a stat line could be, most placeheld first.
///
/// The last entry is always the line verbatim, so a stat whose text contains
/// no rolls at all still gets one chance to match.
pub fn candidates(stat: &str) -> Vec<Candidate> {
    // The game prints `()` when it has no range to give. Leaving it in makes
    // the template miss.
    let cleaned = stat.replace("()", "");

    let (with_placeholders, matches) = to_template(&cleaned);

    let mut out = Vec::new();

    if matches.len() < PLACEHOLDER_MAP.len() {
        for keep in PLACEHOLDER_MAP[matches.len()] {
            out.push(Candidate {
                template: restore(&with_placeholders, &matches, keep),
                values: matches
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !keep.contains(i))
                    .map(|(_, m)| m.clone())
                    .collect(),
            });
        }
    }

    // Fall back to the exact text. A line with five numbers gets only this.
    out.push(Candidate {
        template: stat.to_string(),
        values: Vec::new(),
    });

    out
}

/// Put literal values back at the indices in `keep`.
fn restore(template: &str, matches: &[NumMatch], keep: &[usize]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut idx = 0;

    for c in template.chars() {
        if c != '#' {
            out.push(c);

            continue;
        }

        if keep.contains(&idx) {
            out.push_str(&matches[idx].roll_str);
        } else {
            out.push('#');
        }

        idx += 1;
    }

    out
}

/// Replace every roll in a stat line with `#`.
///
/// Hand written rather than regex driven. The reference pattern relies on a
/// negative lookbehind, which Rust's regex crate cannot express, and a scanner
/// makes the "not preceded by a digit or a close paren" rule explicit anyway.
fn to_template(stat: &str) -> (String, Vec<NumMatch>) {
    let chars: Vec<char> = stat.chars().collect();
    let mut out = String::with_capacity(stat.len());
    let mut matches = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        // A number that follows a digit or a close paren is part of a range
        // the scanner already consumed, not a new roll.
        let follows_number = i > 0 && (chars[i - 1].is_ascii_digit() || chars[i - 1] == ')');

        if follows_number {
            out.push(chars[i]);
            i += 1;

            continue;
        }

        let Some((roll_str, after_number)) = read_number(&chars, i) else {
            out.push(chars[i]);
            i += 1;

            continue;
        };

        let (bounds_text, after_bounds) = read_bounds(&chars, after_number);

        let roll: f64 = roll_str.parse().unwrap_or(0.0);
        let mut decimal = roll_str.contains('.');

        let bounds = match &bounds_text {
            Some((min_text, max_text)) => {
                decimal = decimal || min_text.contains('.') || max_text.contains('.');

                match (min_text.parse::<f64>(), max_text.parse::<f64>()) {
                    (Ok(min), Ok(max)) => Some((min, max)),
                    _ => None,
                }
            }
            None => None,
        };

        // A range the scanner could not read stays in the template verbatim.
        // Dropping it would make the template match a different stat.
        out.push('#');

        if bounds.is_none() {
            if let Some((min_text, max_text)) = &bounds_text {
                out.push_str(&format!("({min_text}-{max_text})"));
            }
        }

        matches.push(NumMatch {
            roll,
            roll_str,
            decimal,
            bounds,
        });

        i = after_bounds;
    }

    (out, matches)
}

/// Read a signed number at `start`.
///
/// Returns the text and the index after it.
fn read_number(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    let mut text = String::new();

    if chars[i] == '+' || chars[i] == '-' {
        text.push(chars[i]);
        i += 1;
    }

    let digits_start = i;

    while i < chars.len() && chars[i].is_ascii_digit() {
        text.push(chars[i]);
        i += 1;
    }

    if i == digits_start {
        return None;
    }

    // A decimal part only counts when a digit follows the point.
    if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
        text.push('.');
        i += 1;

        while i < chars.len() && chars[i].is_ascii_digit() {
            text.push(chars[i]);
            i += 1;
        }
    }

    Some((text, i))
}

/// Read a `(min-max)` or `(min)` range at `start`.
///
/// Returns the two halves and the index after the range. A `(min)` with no
/// max reads as `min-min`, matching the reference, because a legacy roll
/// prints only one bound.
fn read_bounds(chars: &[char], start: usize) -> (Option<(String, String)>, usize) {
    if start >= chars.len() || chars[start] != '(' {
        return (None, start);
    }

    let mut i = start + 1;

    if i >= chars.len() {
        return (None, start);
    }

    // The first character is taken unconditionally so a negative bound keeps
    // its sign. Everything after it stops at a hyphen or a close paren.
    let mut min = String::new();
    min.push(chars[i]);
    i += 1;

    while i < chars.len() && chars[i] != ')' && chars[i] != '-' {
        min.push(chars[i]);
        i += 1;
    }

    let mut max = None;

    if i < chars.len() && chars[i] == '-' {
        i += 1;
        let mut text = String::new();

        while i < chars.len() && chars[i] != ')' {
            text.push(chars[i]);
            i += 1;
        }

        max = Some(text);
    }

    // An unterminated range is not a range.
    if i >= chars.len() || chars[i] != ')' {
        return (None, start);
    }

    let max = max.unwrap_or_else(|| min.clone());

    (Some((min, max)), i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn templates(stat: &str) -> Vec<String> {
        candidates(stat).into_iter().map(|c| c.template).collect()
    }

    // -----------------------------------------------------------------
    // Reading numbers
    // -----------------------------------------------------------------

    #[test]
    fn one_roll_becomes_a_placeholder() {
        let (t, m) = to_template("+25 to maximum Life");

        assert_eq!(t, "# to maximum Life");
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].roll, 25.0);
        assert_eq!(m[0].roll_str, "+25");
    }

    #[test]
    fn a_negative_roll_keeps_its_sign() {
        let (t, m) = to_template("-15% to Fire Resistance");

        assert_eq!(t, "#% to Fire Resistance");
        assert_eq!(m[0].roll, -15.0);
    }

    #[test]
    fn two_rolls_become_two_placeholders() {
        let (t, m) = to_template("Adds 5 to 12 Fire Damage");

        assert_eq!(t, "Adds # to # Fire Damage");
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].roll, 5.0);
        assert_eq!(m[1].roll, 12.0);
    }

    #[test]
    fn a_decimal_roll_is_flagged() {
        let (_, m) = to_template("1.5% of Damage Leeched");

        assert_eq!(m[0].roll, 1.5);
        assert!(m[0].decimal);
    }

    #[test]
    fn a_whole_roll_is_not_flagged_as_decimal() {
        let (_, m) = to_template("+25 to maximum Life");

        assert!(!m[0].decimal);
    }

    #[test]
    fn a_trailing_dot_is_not_read_as_a_decimal_point() {
        // "Regenerate 5 Life per second." ends in a full stop.
        let (t, m) = to_template("Regenerate 5 Life.");

        assert_eq!(t, "Regenerate # Life.");
        assert_eq!(m[0].roll, 5.0);
    }

    #[test]
    fn a_line_with_no_numbers_yields_no_matches() {
        let (t, m) = to_template("Cannot be Frozen");

        assert_eq!(t, "Cannot be Frozen");
        assert!(m.is_empty());
    }

    // -----------------------------------------------------------------
    // Reading inline ranges
    // -----------------------------------------------------------------

    #[test]
    fn an_inline_range_becomes_bounds() {
        let (t, m) = to_template("+25(20-30) to maximum Life");

        assert_eq!(t, "# to maximum Life");
        assert_eq!(m[0].roll, 25.0);
        assert_eq!(m[0].bounds, Some((20.0, 30.0)));
    }

    #[test]
    fn a_range_with_one_bound_reads_as_a_point() {
        // A legacy roll prints only one bound.
        let (_, m) = to_template("+25(25) to maximum Life");

        assert_eq!(m[0].bounds, Some((25.0, 25.0)));
    }

    #[test]
    fn a_negative_lower_bound_keeps_its_sign() {
        // The first character of the bound is taken unconditionally, which is
        // the only reason a leading minus survives.
        let (_, m) = to_template("+5(-10-20)% to Resistance");

        assert_eq!(m[0].bounds, Some((-10.0, 20.0)));
    }

    #[test]
    fn a_decimal_bound_flags_the_match_as_decimal() {
        let (_, m) = to_template("1.5(1.0-2.0)% Leeched");

        assert!(m[0].decimal);
        assert_eq!(m[0].bounds, Some((1.0, 2.0)));
    }

    #[test]
    fn a_range_the_scanner_cannot_read_stays_in_the_template() {
        // Dropping it would make the template match a different stat.
        let (t, m) = to_template("+25(unknown) to maximum Life");

        assert_eq!(t, "#(unknown-unknown) to maximum Life");
        assert_eq!(m[0].bounds, None);
    }

    #[test]
    fn an_unterminated_range_falls_apart_into_separate_rolls() {
        // Faithful to the reference. Its regex needs a closing paren for the
        // range group, so the numbers inside an unterminated one are scanned
        // again as ordinary rolls. The result is a template that matches
        // nothing, which is the right outcome for malformed text.
        let (t, m) = to_template("+25(20-30 to maximum Life");

        assert_eq!(t, "#(#-# to maximum Life");
        assert_eq!(m.len(), 3);
        assert!(m.iter().all(|x| x.bounds.is_none()));
    }

    #[test]
    fn each_of_two_rolls_gets_its_own_range() {
        let (t, m) = to_template("Adds 5(3-7) to 12(10-15) Fire Damage");

        assert_eq!(t, "Adds # to # Fire Damage");
        assert_eq!(m[0].bounds, Some((3.0, 7.0)));
        assert_eq!(m[1].bounds, Some((10.0, 15.0)));
    }

    #[test]
    fn a_number_inside_a_range_is_not_read_as_a_new_roll() {
        // Without the preceding character check, 20 and 30 inside the range
        // would each become their own placeholder.
        let (_, m) = to_template("+25(20-30) to maximum Life");

        assert_eq!(m.len(), 1);
    }

    #[test]
    fn an_empty_range_marker_is_removed_before_matching() {
        // The game prints () when it has no advanced description to give.
        assert!(
            templates("Passives in Radius of Wicked Ward() can be Allocated")
                .iter()
                .any(|t| t == "Passives in Radius of Wicked Ward can be Allocated")
        );
    }

    // -----------------------------------------------------------------
    // Candidate generation
    // -----------------------------------------------------------------

    #[test]
    fn a_one_number_line_yields_both_readings_plus_the_verbatim_text() {
        let got = templates("+25 to maximum Life");

        assert_eq!(
            got,
            vec![
                // The number is part of the name.
                "+25 to maximum Life",
                // The number is a roll.
                "# to maximum Life",
                // Verbatim fallback.
                "+25 to maximum Life",
            ]
        );
    }

    #[test]
    fn the_most_placeheld_template_is_not_first() {
        // The map keeps literals first, so a stat whose number is part of its
        // name matches before the generic form. Reversing this would match
        // "Grants Level # Purity of Fire" as a roll on every gem.
        let got = templates("Grants Level 20 Purity of Fire");

        assert_eq!(got[0], "Grants Level 20 Purity of Fire");
        assert_eq!(got[1], "Grants Level # Purity of Fire");
    }

    #[test]
    fn a_two_number_line_yields_four_readings_plus_the_fallback() {
        let got = templates("Adds 5 to 12 Fire Damage");

        assert_eq!(got.len(), 5);
        assert_eq!(got[0], "Adds 5 to 12 Fire Damage");
        assert_eq!(got[3], "Adds # to # Fire Damage");
    }

    #[test]
    fn a_candidate_carries_only_the_numbers_it_placeheld() {
        let got = candidates("Adds 5 to 12 Fire Damage");

        // Both literal, so no rolls.
        assert!(got[0].values.is_empty());

        // Both placeheld, so both are rolls.
        assert_eq!(got[3].values.len(), 2);
        assert_eq!(got[3].values[0].roll, 5.0);
        assert_eq!(got[3].values[1].roll, 12.0);
    }

    #[test]
    fn a_partly_placeheld_candidate_carries_only_the_placeheld_number() {
        let got = candidates("Adds 5 to 12 Fire Damage");

        let one = got
            .iter()
            .find(|c| c.template == "Adds 5 to # Fire Damage")
            .unwrap();

        assert_eq!(one.values.len(), 1);
        assert_eq!(one.values[0].roll, 12.0);
    }

    #[test]
    fn a_four_number_line_yields_sixteen_readings_plus_the_fallback() {
        let got = templates("1 2 3 4");

        assert_eq!(got.len(), 17);
    }

    #[test]
    fn a_five_number_line_yields_only_the_verbatim_text() {
        // The map stops at four. Generating 32 templates for a line that long
        // costs more than it finds.
        let got = templates("1 2 3 4 5");

        assert_eq!(got, vec!["1 2 3 4 5"]);
    }

    #[test]
    fn a_line_with_no_numbers_yields_itself_twice() {
        // Once from the zero number map entry and once from the fallback.
        let got = templates("Cannot be Frozen");

        assert_eq!(got, vec!["Cannot be Frozen", "Cannot be Frozen"]);
    }

    #[test]
    fn restoring_uses_the_printed_characters_and_not_the_parsed_value() {
        // "+25" must come back as "+25" and not as "25". The data file keys on
        // the exact text.
        let got = templates("+25 to maximum Life");

        assert_eq!(got[0], "+25 to maximum Life");
    }

    // -----------------------------------------------------------------
    // Line level helpers
    // -----------------------------------------------------------------

    #[test]
    fn the_unscalable_marker_is_split_off() {
        let line = format!("+25 to maximum Life{}", cs::UNSCALABLE_VALUE);

        let got = split_unscalable(&line);

        assert_eq!(got.string, "+25 to maximum Life");
        assert!(got.unscalable);
    }

    #[test]
    fn a_marker_written_with_a_hyphen_is_not_recognised() {
        // The game prints an em dash. A hyphen here would leave the marker in
        // the stat text and the lookup would miss.
        let got = split_unscalable("+25 to maximum Life - Unscalable Value");

        assert!(!got.unscalable);
    }

    #[test]
    fn a_line_without_the_marker_is_scalable() {
        let got = split_unscalable("+25 to maximum Life");

        assert_eq!(got.string, "+25 to maximum Life");
        assert!(!got.unscalable);
    }

    #[test]
    fn a_reminder_string_is_recognised_by_its_parentheses() {
        assert!(opens_reminder("(Fire Resistance is capped at 75%)"));
        assert!(closes_reminder("(Fire Resistance is capped at 75%)"));
        assert!(!opens_reminder("+25 to maximum Life"));
    }

    #[test]
    fn a_full_width_parenthesis_also_marks_a_reminder() {
        // The game uses them in some fonts even in English text.
        assert!(opens_reminder("（reminder）"));
        assert!(closes_reminder("（reminder）"));
    }

    #[test]
    fn surrounding_whitespace_does_not_hide_a_reminder() {
        assert!(opens_reminder("  (reminder"));
        assert!(closes_reminder("reminder)  "));
    }
}
