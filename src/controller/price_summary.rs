use std::collections::BTreeMap;

use crate::controller::money::round_for_reading;

#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub amount: f64,
    pub currency: String,
    pub account: String,
    pub online: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    pub low: f64,
    pub median: f64,
    pub high: f64,
    pub currency: String,
    pub counted: usize,
    pub outliers: usize,
}

const OUTLIER_FACTOR: f64 = 4.0;
const MIN_FOR_OUTLIER_TRIM: usize = 5;

pub fn estimate_from(quotes: &[Quote]) -> Option<Estimate> {
    let currency = most_common_currency(quotes)?;

    let mut amounts: Vec<f64> = quotes
        .iter()
        .filter(|q| q.currency == currency)
        .map(|q| q.amount)
        .filter(|a| a.is_finite() && *a > 0.0)
        .collect();

    if amounts.is_empty() {
        return None;
    }

    amounts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let before = amounts.len();
    let kept = without_outliers(amounts);

    Some(Estimate {
        low: kept[0],
        median: median(&kept),
        high: kept[kept.len() - 1],
        currency,
        counted: kept.len(),
        outliers: before - kept.len(),
    })
}

fn without_outliers(sorted: Vec<f64>) -> Vec<f64> {
    if sorted.len() < MIN_FOR_OUTLIER_TRIM {
        return sorted;
    }

    let floor = median(&sorted) * OUTLIER_FACTOR;

    let kept: Vec<f64> = sorted.iter().copied().filter(|a| *a <= floor).collect();

    match kept.is_empty() {
        true => sorted,
        false => kept,
    }
}

fn median(sorted: &[f64]) -> f64 {
    let mid = sorted.len() / 2;

    match sorted.len() % 2 {
        1 => sorted[mid],
        _ => (sorted[mid - 1] + sorted[mid]) / 2.0,
    }
}

fn most_common_currency(quotes: &[Quote]) -> Option<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();

    for quote in quotes {
        if quote.amount <= 0.0 || !quote.amount.is_finite() {
            continue;
        }

        *counts.entry(quote.currency.as_str()).or_default() += 1;
    }

    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(c, _)| c.to_string())
}

pub fn price_spread(estimate: &Estimate) -> String {
    if estimate.low >= estimate.high {
        return format!(
            "{} {}",
            round_for_reading(estimate.median),
            estimate.currency
        );
    }

    format!(
        "{} to {} {}",
        round_for_reading(estimate.low),
        round_for_reading(estimate.high),
        estimate.currency
    )
}

pub fn price_headline(estimate: &Estimate) -> String {
    format!(
        "~{} {}",
        round_for_reading(estimate.median),
        estimate.currency
    )
}

pub fn stack_value(estimate: &Estimate, stack: u32) -> Option<String> {
    if stack <= 1 {
        return None;
    }

    Some(format!(
        "{} {} for the stack of {stack}",
        round_for_reading(estimate.median * f64::from(stack)),
        estimate.currency
    ))
}

pub fn online_count(quotes: &[Quote]) -> usize {
    quotes.iter().filter(|q| q.online).count()
}

pub fn queue_note(estimated_millis: u64, clean_millis: u64) -> Option<String> {
    let waited = crate::controller::bulk::longest_queue_wait(&[(estimated_millis, clean_millis)])?;

    Some(format!(
        "the rate limiter held the search for {:.1}s",
        waited as f64 / 1000.0
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote(amount: f64, currency: &str) -> Quote {
        Quote {
            amount,
            currency: currency.to_string(),
            account: "seller".to_string(),
            online: true,
        }
    }

    fn divines(amounts: &[f64]) -> Vec<Quote> {
        amounts.iter().map(|a| quote(*a, "divine")).collect()
    }

    #[test]
    fn no_listings_produce_no_estimate() {
        assert_eq!(estimate_from(&[]), None);
    }

    #[test]
    fn one_listing_is_its_own_estimate() {
        let got = estimate_from(&divines(&[5.0])).expect("an estimate");

        assert_eq!(got.median, 5.0);
        assert_eq!(got.low, 5.0);
        assert_eq!(got.high, 5.0);
        assert_eq!(got.counted, 1);
    }

    #[test]
    fn an_odd_number_of_listings_takes_the_middle_one() {
        let got = estimate_from(&divines(&[1.0, 5.0, 9.0])).expect("an estimate");

        assert_eq!(got.median, 5.0);
    }

    #[test]
    fn an_even_number_of_listings_averages_the_two_in_the_middle() {
        let got = estimate_from(&divines(&[1.0, 4.0, 6.0, 9.0])).expect("an estimate");

        assert_eq!(got.median, 5.0);
    }

    #[test]
    fn the_listings_do_not_have_to_arrive_sorted() {
        let got = estimate_from(&divines(&[9.0, 1.0, 5.0])).expect("an estimate");

        assert_eq!(got.median, 5.0);
        assert_eq!(got.low, 1.0);
        assert_eq!(got.high, 9.0);
    }

    #[test]
    fn the_currency_most_sellers_use_is_the_one_reported() {
        let mut quotes = divines(&[5.0, 6.0, 7.0]);
        quotes.push(quote(1000.0, "chaos"));

        let got = estimate_from(&quotes).expect("an estimate");

        assert_eq!(got.currency, "divine");
    }

    #[test]
    fn prices_in_another_currency_do_not_pollute_the_estimate() {
        let mut quotes = divines(&[5.0, 6.0, 7.0]);
        quotes.push(quote(1000.0, "chaos"));

        assert_eq!(estimate_from(&quotes).expect("an estimate").high, 7.0);
    }

    #[test]
    fn a_price_far_above_the_rest_is_left_out_of_the_range() {
        let got = estimate_from(&divines(&[5.0, 5.0, 6.0, 6.0, 7.0, 900.0])).expect("an estimate");

        assert_eq!(got.high, 7.0);
        assert_eq!(got.outliers, 1);
    }

    #[test]
    fn a_short_list_is_left_alone_because_there_is_nothing_to_compare_against() {
        let got = estimate_from(&divines(&[5.0, 900.0])).expect("an estimate");

        assert_eq!(got.high, 900.0);
        assert_eq!(got.outliers, 0);
    }

    #[test]
    fn a_listing_priced_at_zero_is_not_a_price() {
        let mut quotes = divines(&[5.0, 6.0]);
        quotes.push(quote(0.0, "divine"));

        assert_eq!(estimate_from(&quotes).expect("an estimate").counted, 2);
    }

    #[test]
    fn a_listing_with_no_price_at_all_does_not_become_an_estimate() {
        assert_eq!(estimate_from(&[quote(0.0, "divine")]), None);
    }

    #[test]
    fn trimming_never_throws_away_every_price() {
        let got = estimate_from(&divines(&[1.0, 1.0, 1.0, 1.0, 1.0])).expect("an estimate");

        assert_eq!(got.counted, 5);
    }

    #[test]
    fn a_spread_reads_as_a_range() {
        let got = estimate_from(&divines(&[1.0, 5.0, 9.0])).expect("an estimate");

        assert_eq!(price_spread(&got), "1 to 9 divine");
    }

    #[test]
    fn a_spread_of_one_price_reads_as_that_price() {
        let got = estimate_from(&divines(&[5.0])).expect("an estimate");

        assert_eq!(price_spread(&got), "5 divine");
    }

    #[test]
    fn the_headline_is_the_middle_price() {
        let got = estimate_from(&divines(&[1.0, 5.0, 9.0])).expect("an estimate");

        assert_eq!(price_headline(&got), "~5 divine");
    }

    #[test]
    fn a_stack_is_priced_by_multiplying_the_unit() {
        let got = estimate_from(&divines(&[2.0])).expect("an estimate");

        assert_eq!(
            stack_value(&got, 20).as_deref(),
            Some("40 divine for the stack of 20")
        );
    }

    #[test]
    fn a_single_item_has_no_stack_value_worth_saying() {
        let got = estimate_from(&divines(&[2.0])).expect("an estimate");

        assert_eq!(stack_value(&got, 1), None);
    }

    #[test]
    fn the_number_of_online_sellers_is_counted() {
        let quotes = vec![
            Quote {
                online: false,
                ..quote(1.0, "divine")
            },
            quote(2.0, "divine"),
        ];

        assert_eq!(online_count(&quotes), 1);
    }
    #[test]
    fn a_search_that_waited_says_so() {
        assert_eq!(
            queue_note(4000, 0).as_deref(),
            Some("the rate limiter held the search for 4.0s")
        );
    }

    #[test]
    fn a_search_that_did_not_wait_says_nothing() {
        assert_eq!(queue_note(0, 0), None);
    }

    #[test]
    fn a_wait_too_short_to_notice_is_not_worth_saying() {
        assert_eq!(queue_note(200, 0), None);
    }
}
