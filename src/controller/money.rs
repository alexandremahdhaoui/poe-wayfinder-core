//! Turning a price into something a person reads.
//!
//! # Why this is not just a format string
//!
//! A price check on a Divine Orb came back as
//! `0.0007201155913700063 mirror`. Every digit of that is true and none of it
//! is usable. A player reading it has to count zeroes to find out whether
//! their orb is worth anything.
//!
//! Two things are wrong with it, and they are separate. The float carries
//! nineteen digits when three carry all the information. And a rate below one
//! is quoted upside down: nobody in the game says a Divine costs
//! 0.0007 Mirror, they say a Mirror costs about 1400 Divine.
//!
//! # What this does not do
//!
//! It does not convert between currencies or look up a rate. The number comes
//! from real listings and is passed through; only its presentation changes.

/// Below this a rate is easier to read the other way up.
///
/// One hundredth. Above it, "0.05 divine" reads fine. Below it the leading
/// zeroes start doing the work and the inverse is what a trader would say.
const INVERT_BELOW: f64 = 0.01;

/// A price, ready to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Price {
    /// The number, already rounded.
    pub amount: String,
    /// What the number is counted in.
    pub currency: String,
    /// The same rate the other way up, when that reads better.
    ///
    /// None when the rate is comfortable as it stands.
    pub inverted: Option<String>,
}

/// Format a rate for a person.
///
/// `amount` is how much of `currency` one of the item costs.
pub fn price(amount: f64, currency: &str) -> Price {
    // Not a number at all. Better to say so than to print NaN, which reads as
    // a bug in the tool rather than as missing data.
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

/// Round to about three significant figures and drop trailing zeroes.
///
/// Three because that is what a price is known to. The listings themselves
/// disagree by more than a tenth of a percent, so a fourth digit is noise
/// dressed as precision.
pub fn round_for_reading(value: f64) -> String {
    if !value.is_finite() {
        return "?".to_string();
    }

    // A large price is whole. Nobody quotes 1387.4 divine.
    if value >= 100.0 {
        return format!("{}", value.round() as i64);
    }

    let decimals = if value >= 10.0 {
        1
    } else if value >= 1.0 {
        2
    } else {
        // Below one, keep three significant figures however small it gets, so
        // 0.0007 does not round away to nothing.
        let leading_zeroes = (-value.log10().floor()) as usize;

        (leading_zeroes + 2).min(10)
    };

    let text = format!("{value:.decimals$}");

    // 1.50 reads worse than 1.5, and 2.00 worse than 2.
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
        // Nobody quotes 1387.4 divine.
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
        // 1.50 reads worse than 1.5.
        assert_eq!(round_for_reading(1.5), "1.5");
        assert_eq!(round_for_reading(2.0), "2");
    }

    #[test]
    fn a_tiny_rate_keeps_its_significant_figures() {
        // The bug this file exists for. Rounding to two decimals here gives
        // "0.00", which says the item is worthless.
        let got = round_for_reading(0.0007201155913700063);

        assert!(got.starts_with("0.0007"), "{got}");
        assert!(got.len() < 10, "still unreadable: {got}");
    }

    #[test]
    fn nineteen_digits_never_survive() {
        // The actual output that started this: 0.0007201155913700063.
        for value in [0.0007201155913700063, 1.0 / 3.0, 32.474747474747] {
            assert!(
                round_for_reading(value).len() <= 8,
                "{value} formatted as {}",
                round_for_reading(value)
            );
        }
    }

    // -----------------------------------------------------------------
    // The inverse
    // -----------------------------------------------------------------

    #[test]
    fn a_tiny_rate_is_also_quoted_the_other_way_up() {
        // Nobody says a Divine costs 0.0007 Mirror. They say a Mirror costs
        // about fourteen hundred Divine.
        let got = price(0.0007201155913700063, "mirror");

        let inverted = got.inverted.expect("a tiny rate is inverted");

        assert!(inverted.contains("1 mirror"), "{inverted}");
        assert!(inverted.contains("1389"), "{inverted}");
    }

    #[test]
    fn an_ordinary_rate_is_not_inverted() {
        // "32 divine" needs no help.
        assert_eq!(price(32.0, "divine").inverted, None);
    }

    #[test]
    fn the_line_is_at_one_hundredth() {
        assert_eq!(price(0.02, "divine").inverted, None);
        assert!(price(0.005, "divine").inverted.is_some());
    }

    // -----------------------------------------------------------------
    // Nothing that is not a number reaches the panel
    // -----------------------------------------------------------------

    #[test]
    fn a_price_of_nothing_says_so_rather_than_printing_zero() {
        // Zero divine reads as free, which is never what the data meant.
        assert_eq!(price(0.0, "divine").amount, "?");
    }

    #[test]
    fn an_infinite_rate_says_so_rather_than_printing_inf() {
        // Dividing by a zero stock produces this, and "inf divine" reads as a
        // bug in the tool rather than as missing data.
        assert_eq!(price(f64::INFINITY, "divine").amount, "?");
        assert_eq!(price(f64::NAN, "divine").amount, "?");
    }

    #[test]
    fn a_negative_rate_is_refused() {
        // No listing is priced below nothing. One that parses that way is a
        // reading error, not a bargain.
        assert_eq!(price(-5.0, "divine").amount, "?");
    }

    #[test]
    fn the_currency_is_carried_through_untouched() {
        // It is an id from the trade site, not a word to prettify. Changing it
        // would break the link back to the listing.
        assert_eq!(price(3.0, "exalted-orb").currency, "exalted-orb");
    }
}
