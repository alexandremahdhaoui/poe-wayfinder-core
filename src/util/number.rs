//! Reading numbers out of item text.
//!
//! The game decorates its numbers. `Quality: +20% (augmented)` holds the value
//! 20. `Stack Size: 2,448/40` holds 2448 and 40. Every stage that reads a
//! number goes through here so the decoration rules live in one place.

/// Read the leading number of a decorated string.
///
/// Mirrors JavaScript `parseInt`, which the reference relies on. It reads an
/// optional sign then digits and stops at the first character that is neither.
/// `+20% (augmented)` gives 20.
pub fn leading_int(text: &str) -> Option<i64> {
    let text = text.trim_start();
    let mut chars = text.char_indices();

    let mut end = 0;
    let mut seen_digit = false;

    if let Some((_, c)) = chars.clone().next() {
        if c == '+' || c == '-' {
            end = c.len_utf8();
            chars.next();
        }
    }

    for (i, c) in chars {
        if c.is_ascii_digit() {
            seen_digit = true;
            end = i + c.len_utf8();
        } else {
            break;
        }
    }

    if !seen_digit {
        return None;
    }

    text[..end].parse().ok()
}

/// Read the leading number of a decorated string, allowing a decimal point.
///
/// Mirrors JavaScript `parseFloat`. Attack speed and crit chance are printed
/// with decimals, so those stages need this and not `leading_int`.
pub fn leading_float(text: &str) -> Option<f64> {
    let text = text.trim_start();

    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;

    for (i, c) in text.char_indices() {
        let ok = match c {
            '+' | '-' if i == 0 => true,
            '.' if !seen_dot => {
                seen_dot = true;
                true
            }
            c if c.is_ascii_digit() => {
                seen_digit = true;
                true
            }
            _ => false,
        };

        if !ok {
            break;
        }

        end = i + c.len_utf8();
    }

    if !seen_digit {
        return None;
    }

    text[..end].parse().ok()
}

/// Read a number after stripping everything that is not a digit.
///
/// The game inserts a thousands separator that varies with the client locale,
/// so `2,448` and `2 448` both mean 2448. Any sign is lost, which is correct
/// here because every field that uses this is a count.
pub fn digits_only(text: &str) -> Option<u64> {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();

    if digits.is_empty() {
        return None;
    }

    digits.parse().ok()
}

/// Average a printed damage roll.
///
/// Ported from `getRollOrMinmaxAvg` in `stat-translations.ts`.
///
/// Four values average as four, two average as two, and anything else takes
/// the first. Damage prints as `min-max`, so the common case is two.
pub fn roll_or_minmax_avg(values: &[f64]) -> Option<f64> {
    match values.len() {
        0 => None,
        4 => Some((values[0] + values[1] + values[2] + values[3]) / 4.0),
        2 => Some((values[0] + values[1]) / 2.0),
        _ => Some(values[0]),
    }
}

/// Average one printed damage range such as `12-34`.
///
/// Every part is read with digits only, matching the reference, so a decorated
/// range still yields its numbers.
pub fn damage_range_avg(text: &str) -> Option<f64> {
    let parts: Vec<f64> = text
        .split('-')
        .filter_map(|p| digits_only(p).map(|v| v as f64))
        .collect();

    roll_or_minmax_avg(&parts)
}

/// Sum a comma separated list of damage ranges.
///
/// Elemental damage prints as `12-34, 56-78`, one range per element.
pub fn damage_list_sum(text: &str) -> Option<f64> {
    let mut total = 0.0;
    let mut any = false;

    for part in text.split(", ") {
        if let Some(v) = damage_range_avg(part) {
            total += v;
            any = true;
        }
    }

    any.then_some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_int_reads_a_plain_number() {
        assert_eq!(leading_int("84"), Some(84));
    }

    #[test]
    fn leading_int_stops_at_the_decoration() {
        assert_eq!(leading_int("+20% (augmented)"), Some(20));
        assert_eq!(leading_int("20 (Max)"), Some(20));
    }

    #[test]
    fn leading_int_keeps_a_negative_sign() {
        assert_eq!(leading_int("-15%"), Some(-15));
    }

    #[test]
    fn leading_int_rejects_text_with_no_leading_digit() {
        for s in ["", "augmented", "+", "-", " (Max)"] {
            assert_eq!(leading_int(s), None, "{s:?} parsed");
        }
    }

    #[test]
    fn leading_int_ignores_leading_whitespace() {
        assert_eq!(leading_int("  84"), Some(84));
    }

    #[test]
    fn leading_int_stops_at_a_decimal_point() {
        // parseInt semantics. 1.35 attacks per second reads as 1 here, which
        // is why attack speed uses leading_float instead.
        assert_eq!(leading_int("1.35"), Some(1));
    }

    #[test]
    fn leading_float_keeps_the_decimals() {
        assert_eq!(leading_float("1.35"), Some(1.35));
        assert_eq!(leading_float("6.50%"), Some(6.5));
    }

    #[test]
    fn leading_float_handles_a_sign() {
        assert_eq!(leading_float("-1.5"), Some(-1.5));
        assert_eq!(leading_float("+1.5"), Some(1.5));
    }

    #[test]
    fn leading_float_stops_at_a_second_decimal_point() {
        assert_eq!(leading_float("1.2.3"), Some(1.2));
    }

    #[test]
    fn leading_float_rejects_text_with_no_digit() {
        for s in ["", ".", "+", "abc"] {
            assert_eq!(leading_float(s), None, "{s:?} parsed");
        }
    }

    #[test]
    fn digits_only_drops_a_thousands_separator() {
        // The separator varies with the client locale, so no single character
        // can be special cased.
        assert_eq!(digits_only("2,448"), Some(2448));
        assert_eq!(digits_only("2 448"), Some(2448));
        assert_eq!(digits_only("2.448"), Some(2448));
    }

    #[test]
    fn digits_only_drops_a_sign() {
        // Every caller counts something, so a sign is never meaningful.
        assert_eq!(digits_only("-15"), Some(15));
    }

    #[test]
    fn digits_only_rejects_text_with_no_digits() {
        assert_eq!(digits_only("none"), None);
        assert_eq!(digits_only(""), None);
    }

    #[test]
    fn a_two_value_roll_averages_the_pair() {
        assert_eq!(roll_or_minmax_avg(&[10.0, 20.0]), Some(15.0));
    }

    #[test]
    fn a_four_value_roll_averages_all_four() {
        assert_eq!(roll_or_minmax_avg(&[1.0, 2.0, 3.0, 4.0]), Some(2.5));
    }

    #[test]
    fn a_three_value_roll_takes_the_first() {
        // Odd and faithful. The reference falls through to values[0] for any
        // length that is not 2 or 4.
        assert_eq!(roll_or_minmax_avg(&[7.0, 8.0, 9.0]), Some(7.0));
    }

    #[test]
    fn a_one_value_roll_takes_the_only_value() {
        assert_eq!(roll_or_minmax_avg(&[7.0]), Some(7.0));
    }

    #[test]
    fn an_empty_roll_yields_nothing() {
        assert_eq!(roll_or_minmax_avg(&[]), None);
    }

    #[test]
    fn a_damage_range_averages_its_ends() {
        assert_eq!(damage_range_avg("12-34"), Some(23.0));
    }

    #[test]
    fn a_single_damage_value_is_taken_as_is() {
        assert_eq!(damage_range_avg("50"), Some(50.0));
    }

    #[test]
    fn a_damage_range_with_a_separator_still_parses() {
        assert_eq!(damage_range_avg("1,000-2,000"), Some(1500.0));
    }

    #[test]
    fn a_damage_list_sums_every_element() {
        // Elemental damage prints one range per element.
        assert_eq!(damage_list_sum("12-34, 56-78"), Some(23.0 + 67.0));
    }

    #[test]
    fn a_single_element_list_is_just_that_element() {
        assert_eq!(damage_list_sum("12-34"), Some(23.0));
    }

    #[test]
    fn a_damage_list_with_no_numbers_yields_nothing() {
        assert_eq!(damage_list_sum("none"), None);
    }
}
