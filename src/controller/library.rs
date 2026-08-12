pub const MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub struct Logged {
    pub name: String,
    pub amount: Option<f64>,
    pub currency: String,
    pub listings: u64,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Library {
    entries: Vec<Logged>,
}

impl Library {
    pub fn record(&mut self, entry: Logged) {
        if entry.name.trim().is_empty() {
            return;
        }

        self.entries.insert(0, entry);
        self.entries.truncate(MAX_ENTRIES);
    }

    pub fn entries(&self) -> &[Logged] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn total_in(&self, currency: &str) -> f64 {
        self.entries
            .iter()
            .filter(|e| e.currency == currency)
            .filter_map(|e| e.amount)
            .sum()
    }

    pub fn currencies(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();

        for entry in &self.entries {
            if entry.amount.is_some() && !out.contains(&entry.currency) {
                out.push(entry.currency.clone());
            }
        }

        out
    }
}

pub fn header() -> Vec<String> {
    ["name", "amount", "currency", "listings"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

pub fn row(entry: &Logged) -> Vec<String> {
    vec![
        entry.name.clone(),
        entry.amount.map(trim).unwrap_or_default(),
        entry.currency.clone(),
        entry.listings.to_string(),
    ]
}

pub fn csv_field(value: &str) -> String {
    let needs_quotes = value.contains([',', '"', '\n', '\r']);

    match needs_quotes {
        true => format!("\"{}\"", value.replace('"', "\"\"")),
        false => value.to_string(),
    }
}

pub fn csv_line(values: &[String]) -> String {
    values
        .iter()
        .map(|v| csv_field(v))
        .collect::<Vec<String>>()
        .join(",")
}

pub fn to_csv(entries: &[Logged]) -> String {
    let mut out = csv_line(&header());

    for entry in entries {
        out.push('\n');
        out.push_str(&csv_line(&row(entry)));
    }

    out.push('\n');
    out
}

pub fn caption(entry: &Logged) -> String {
    match entry.amount {
        Some(amount) => format!("{}, {} {}", entry.name, trim(amount), entry.currency),
        None => format!("{}, no price", entry.name),
    }
}

fn trim(amount: f64) -> String {
    match amount.fract() == 0.0 {
        true => format!("{}", amount as i64),
        false => format!("{amount:.2}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logged(name: &str, amount: Option<f64>) -> Logged {
        Logged {
            name: name.to_string(),
            amount,
            currency: "chaos".to_string(),
            listings: 12,
            at_ms: 0,
        }
    }

    #[test]
    fn the_newest_item_is_first_because_that_is_the_one_just_priced() {
        let mut lib = Library::default();

        lib.record(logged("Sapphire Ring", Some(1.0)));
        lib.record(logged("Kaom's Heart", Some(20.0)));

        assert_eq!(lib.entries()[0].name, "Kaom's Heart");
    }

    #[test]
    fn an_item_with_no_name_is_not_logged() {
        let mut lib = Library::default();

        lib.record(logged("   ", Some(1.0)));

        assert!(lib.is_empty());
    }

    #[test]
    fn the_log_stops_growing_at_the_limit() {
        let mut lib = Library::default();

        for i in 0..(MAX_ENTRIES + 50) {
            lib.record(logged(&format!("item {i}"), Some(1.0)));
        }

        assert_eq!(lib.len(), MAX_ENTRIES);
        assert_eq!(
            lib.entries()[0].name,
            format!("item {}", MAX_ENTRIES + 49),
            "the newest survives, the oldest falls off"
        );
    }

    #[test]
    fn a_total_adds_only_the_currency_asked_for() {
        let mut lib = Library::default();

        lib.record(logged("a", Some(2.0)));
        lib.record(Logged {
            currency: "divine".to_string(),
            ..logged("b", Some(5.0))
        });

        assert_eq!(lib.total_in("chaos"), 2.0);
        assert_eq!(lib.total_in("divine"), 5.0);
    }

    #[test]
    fn an_item_with_no_price_adds_nothing_to_the_total() {
        let mut lib = Library::default();

        lib.record(logged("a", None));
        lib.record(logged("b", Some(3.0)));

        assert_eq!(lib.total_in("chaos"), 3.0);
    }

    #[test]
    fn the_currencies_seen_are_listed_once_each() {
        let mut lib = Library::default();

        lib.record(logged("a", Some(1.0)));
        lib.record(logged("b", Some(1.0)));

        assert_eq!(lib.currencies(), vec!["chaos".to_string()]);
    }

    #[test]
    fn a_priced_item_reads_as_a_name_and_a_price() {
        assert_eq!(caption(&logged("Ring", Some(4.0))), "Ring, 4 chaos");
        assert_eq!(caption(&logged("Ring", Some(4.5))), "Ring, 4.50 chaos");
    }

    #[test]
    fn an_unpriced_item_says_so_rather_than_showing_a_zero() {
        assert_eq!(caption(&logged("Ring", None)), "Ring, no price");
    }

    #[test]
    fn the_export_starts_with_a_header_row() {
        let csv = to_csv(&[]);

        assert_eq!(csv.lines().next(), Some("name,amount,currency,listings"));
    }

    #[test]
    fn every_logged_item_becomes_one_row() {
        let csv = to_csv(&[logged("a", Some(2.0)), logged("b", None)]);

        assert_eq!(csv.lines().count(), 3, "{csv}");
        assert!(csv.contains("a,2,chaos,12"));
    }

    #[test]
    fn a_comma_in_a_name_is_quoted_so_the_columns_survive() {
        assert_eq!(
            csv_field("Kaom's Heart, mirrored"),
            "\"Kaom's Heart, mirrored\""
        );
    }

    #[test]
    fn a_quote_in_a_name_is_doubled_the_way_csv_expects() {
        assert_eq!(csv_field("the \"good\" one"), "\"the \"\"good\"\" one\"");
    }

    #[test]
    fn a_newline_in_a_value_cannot_break_the_row() {
        let quoted = csv_field("a\nb");

        assert!(quoted.starts_with('"') && quoted.ends_with('"'), "{quoted}");
    }

    #[test]
    fn a_plain_value_is_not_quoted_for_no_reason() {
        assert_eq!(csv_field("Sapphire Ring"), "Sapphire Ring");
    }

    #[test]
    fn an_item_with_no_price_leaves_the_amount_empty_rather_than_zero() {
        let csv = to_csv(&[logged("a", None)]);

        assert!(csv.contains("a,,chaos,12"), "{csv}");
    }

    #[test]
    fn clearing_empties_it() {
        let mut lib = Library::default();

        lib.record(logged("a", Some(1.0)));
        lib.clear();

        assert!(lib.is_empty());
    }
}
