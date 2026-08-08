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

pub fn digits_only(text: &str) -> Option<u64> {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();

    if digits.is_empty() {
        return None;
    }

    digits.parse().ok()
}

pub fn roll_or_minmax_avg(values: &[f64]) -> Option<f64> {
    match values.len() {
        0 => None,
        4 => Some((values[0] + values[1] + values[2] + values[3]) / 4.0),
        2 => Some((values[0] + values[1]) / 2.0),
        _ => Some(values[0]),
    }
}

pub fn damage_range_avg(text: &str) -> Option<f64> {
    let parts: Vec<f64> = text
        .split('-')
        .filter_map(|p| digits_only(p).map(|v| v as f64))
        .collect();

    roll_or_minmax_avg(&parts)
}

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
        assert_eq!(digits_only("2,448"), Some(2448));
        assert_eq!(digits_only("2 448"), Some(2448));
        assert_eq!(digits_only("2.448"), Some(2448));
    }

    #[test]
    fn digits_only_drops_a_sign() {
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
