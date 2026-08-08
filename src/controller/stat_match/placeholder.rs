use crate::types::client_strings as cs;

#[derive(Debug, Clone, PartialEq)]
pub struct NumMatch {
    pub roll: f64,
    pub roll_str: String,
    pub decimal: bool,
    pub signed: bool,
    pub bounds: Option<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub template: String,
    pub values: Vec<NumMatch>,
}

const PLACEHOLDER_MAP: [&[&[usize]]; 5] = [
    &[&[]],
    &[&[0], &[]],
    &[&[0, 1], &[0], &[1], &[]],
    &[&[0, 1, 2], &[1, 2], &[0, 2], &[0, 1], &[2], &[1], &[0], &[]],
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatString {
    pub string: String,
    pub unscalable: bool,
}

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

pub fn opens_reminder(line: &str) -> bool {
    line.trim_start().starts_with(['(', '（'])
}

pub fn closes_reminder(line: &str) -> bool {
    line.trim_end().ends_with([')', '）'])
}

pub fn candidates(stat: &str) -> Vec<Candidate> {
    let cleaned = stat.replace("()", "");

    let (with_placeholders, matches) = to_template(&cleaned);

    let mut out = Vec::new();

    if matches.len() < PLACEHOLDER_MAP.len() {
        for keep in PLACEHOLDER_MAP[matches.len()] {
            let values: Vec<NumMatch> = matches
                .iter()
                .enumerate()
                .filter(|(i, _)| !keep.contains(i))
                .map(|(_, m)| m.clone())
                .collect();

            if values.iter().any(|m| m.signed) {
                out.push(Candidate {
                    template: restore_signed(&with_placeholders, &matches, keep),
                    values: values.clone(),
                });
            }

            out.push(Candidate {
                template: restore(&with_placeholders, &matches, keep),
                values,
            });
        }
    }

    out.push(Candidate {
        template: stat.to_string(),
        values: Vec::new(),
    });

    out
}

fn restore_signed(template: &str, matches: &[NumMatch], keep: &[usize]) -> String {
    let mut out = String::with_capacity(template.len() + matches.len());
    let mut idx = 0;

    for c in template.chars() {
        if c != '#' {
            out.push(c);

            continue;
        }

        if keep.contains(&idx) {
            out.push_str(&matches[idx].roll_str);
        } else {
            if matches[idx].signed {
                out.push('+');
            }

            out.push('#');
        }

        idx += 1;
    }

    out
}

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

fn to_template(stat: &str) -> (String, Vec<NumMatch>) {
    let chars: Vec<char> = stat.chars().collect();
    let mut out = String::with_capacity(stat.len());
    let mut matches = Vec::new();
    let mut i = 0;

    while i < chars.len() {
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

        out.push('#');

        if bounds.is_none() {
            if let Some((min_text, max_text)) = &bounds_text {
                out.push_str(&format!("({min_text}-{max_text})"));
            }
        }

        matches.push(NumMatch {
            signed: roll_str.starts_with('+'),
            roll,
            roll_str,
            decimal,
            bounds,
        });

        i = after_bounds;
    }

    (out, matches)
}

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

fn read_bounds(chars: &[char], start: usize) -> (Option<(String, String)>, usize) {
    if start >= chars.len() || chars[start] != '(' {
        return (None, start);
    }

    let mut i = start + 1;

    if i >= chars.len() {
        return (None, start);
    }

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

    #[test]
    fn an_inline_range_becomes_bounds() {
        let (t, m) = to_template("+25(20-30) to maximum Life");

        assert_eq!(t, "# to maximum Life");
        assert_eq!(m[0].roll, 25.0);
        assert_eq!(m[0].bounds, Some((20.0, 30.0)));
    }

    #[test]
    fn a_range_with_one_bound_reads_as_a_point() {
        let (_, m) = to_template("+25(25) to maximum Life");

        assert_eq!(m[0].bounds, Some((25.0, 25.0)));
    }

    #[test]
    fn a_negative_lower_bound_keeps_its_sign() {
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
        let (t, m) = to_template("+25(unknown) to maximum Life");

        assert_eq!(t, "#(unknown-unknown) to maximum Life");
        assert_eq!(m[0].bounds, None);
    }

    #[test]
    fn an_unterminated_range_falls_apart_into_separate_rolls() {
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
        let (_, m) = to_template("+25(20-30) to maximum Life");

        assert_eq!(m.len(), 1);
    }

    #[test]
    fn an_empty_range_marker_is_removed_before_matching() {
        assert!(
            templates("Passives in Radius of Wicked Ward() can be Allocated")
                .iter()
                .any(|t| t == "Passives in Radius of Wicked Ward can be Allocated")
        );
    }

    #[test]
    fn a_one_number_line_yields_both_readings_plus_the_verbatim_text() {
        let got = templates("+25 to maximum Life");

        assert_eq!(
            got,
            vec![
                "+25 to maximum Life",
                "+# to maximum Life",
                "# to maximum Life",
                "+25 to maximum Life",
            ]
        );
    }

    #[test]
    fn a_signed_roll_offers_the_signed_key_before_the_unsigned_one() {
        let got = templates("+2 to Level of all Arc Skills");

        let signed = got
            .iter()
            .position(|t| t == "+# to Level of all Arc Skills");
        let plain = got.iter().position(|t| t == "# to Level of all Arc Skills");

        assert!(signed.is_some(), "{got:?}");
        assert!(plain.is_some(), "{got:?}");
        assert!(
            signed < plain,
            "the signed key must be tried first: {got:?}"
        );
    }

    #[test]
    fn an_unsigned_roll_offers_no_signed_key() {
        let got = templates("25% Chance to Block Spell Damage");

        assert!(
            !got.iter().any(|t| t.contains('+')),
            "an unsigned line produced a signed key: {got:?}"
        );
    }

    #[test]
    fn a_negative_roll_offers_no_signed_key() {
        let got = templates("-15% to Fire Resistance");

        assert!(
            !got.iter().any(|t| t.contains("-#")),
            "a negative line produced a -# key: {got:?}"
        );
    }

    #[test]
    fn the_sign_is_recorded_on_the_number_it_belongs_to() {
        let (_, m) = to_template("Adds +5 to 12 Fire Damage");

        assert!(m[0].signed);
        assert!(!m[1].signed, "the unsigned second roll was marked signed");
    }

    #[test]
    fn a_kept_literal_keeps_its_printed_sign_in_the_signed_form() {
        let got = templates("+25 to maximum Life");

        assert!(!got.iter().any(|t| t.contains("++")), "{got:?}");
    }

    #[test]
    fn the_most_placeheld_template_is_not_first() {
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

        assert!(got[0].values.is_empty());

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
        let got = templates("1 2 3 4 5");

        assert_eq!(got, vec!["1 2 3 4 5"]);
    }

    #[test]
    fn a_line_with_no_numbers_yields_itself_twice() {
        let got = templates("Cannot be Frozen");

        assert_eq!(got, vec!["Cannot be Frozen", "Cannot be Frozen"]);
    }

    #[test]
    fn restoring_uses_the_printed_characters_and_not_the_parsed_value() {
        let got = templates("+25 to maximum Life");

        assert_eq!(got[0], "+25 to maximum Life");
    }

    #[test]
    fn the_unscalable_marker_is_split_off() {
        let line = format!("+25 to maximum Life{}", cs::UNSCALABLE_VALUE);

        let got = split_unscalable(&line);

        assert_eq!(got.string, "+25 to maximum Life");
        assert!(got.unscalable);
    }

    #[test]
    fn a_marker_written_with_a_hyphen_is_not_recognised() {
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
        assert!(opens_reminder("（reminder）"));
        assert!(closes_reminder("（reminder）"));
    }

    #[test]
    fn surrounding_whitespace_does_not_hide_a_reminder() {
        assert!(opens_reminder("  (reminder"));
        assert!(closes_reminder("reminder)  "));
    }
}
