const INVERT_BELOW: f64 = 0.01;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Price {
    pub amount: String,
    pub currency: String,
    pub inverted: Option<String>,
}

pub fn price(amount: f64, currency: &str) -> Price {
    if !amount.is_finite() || amount <= 0.0 {
        return Price {
            amount: "?".to_string(),
            currency: currency.to_string(),
            inverted: None,
        };
    }

    let inverted = (amount < INVERT_BELOW).then(|| {
        format!(
            "1 {currency} = {} of these",
            round_for_reading(1.0 / amount)
        )
    });

    Price {
        amount: round_for_reading(amount),
        currency: currency.to_string(),
        inverted,
    }
}

pub fn round_for_reading(value: f64) -> String {
    if !value.is_finite() {
        return "?".to_string();
    }

    if value >= 100.0 {
        return format!("{}", value.round() as i64);
    }

    let decimals = if value >= 10.0 {
        1
    } else if value >= 1.0 {
        2
    } else {
        let leading_zeroes = (-value.log10().floor()) as usize;

        (leading_zeroes + 2).min(10)
    };

    let text = format!("{value:.decimals$}");

    let trimmed = text.trim_end_matches('0').trim_end_matches('.');

    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_big_price_is_whole() {
        assert_eq!(round_for_reading(1387.42), "1387");
    }

    #[test]
    fn a_middling_price_keeps_one_decimal() {
        assert_eq!(round_for_reading(32.47), "32.5");
    }

    #[test]
    fn a_small_price_keeps_two_decimals() {
        assert_eq!(round_for_reading(1.234), "1.23");
    }

    #[test]
    fn a_trailing_zero_is_dropped() {
        assert_eq!(round_for_reading(1.5), "1.5");
        assert_eq!(round_for_reading(2.0), "2");
    }

    #[test]
    fn a_tiny_rate_keeps_its_significant_figures() {
        let got = round_for_reading(0.0007201155913700063);

        assert!(got.starts_with("0.0007"), "{got}");
        assert!(got.len() < 10, "still unreadable: {got}");
    }

    #[test]
    fn nineteen_digits_never_survive() {
        for value in [0.0007201155913700063, 1.0 / 3.0, 32.474747474747] {
            assert!(
                round_for_reading(value).len() <= 8,
                "{value} formatted as {}",
                round_for_reading(value)
            );
        }
    }

    #[test]
    fn a_tiny_rate_is_also_quoted_the_other_way_up() {
        let got = price(0.0007201155913700063, "mirror");

        let inverted = got.inverted.expect("a tiny rate is inverted");

        assert!(inverted.contains("1 mirror"), "{inverted}");
        assert!(inverted.contains("1389"), "{inverted}");
    }

    #[test]
    fn an_ordinary_rate_is_not_inverted() {
        assert_eq!(price(32.0, "divine").inverted, None);
    }

    #[test]
    fn the_line_is_at_one_hundredth() {
        assert_eq!(price(0.02, "divine").inverted, None);
        assert!(price(0.005, "divine").inverted.is_some());
    }

    #[test]
    fn a_price_of_nothing_says_so_rather_than_printing_zero() {
        assert_eq!(price(0.0, "divine").amount, "?");
    }

    #[test]
    fn an_infinite_rate_says_so_rather_than_printing_inf() {
        assert_eq!(price(f64::INFINITY, "divine").amount, "?");
        assert_eq!(price(f64::NAN, "divine").amount, "?");
    }

    #[test]
    fn a_negative_rate_is_refused() {
        assert_eq!(price(-5.0, "divine").amount, "?");
    }

    #[test]
    fn the_currency_is_carried_through_untouched() {
        assert_eq!(price(3.0, "exalted-orb").currency, "exalted-orb");
    }
}
