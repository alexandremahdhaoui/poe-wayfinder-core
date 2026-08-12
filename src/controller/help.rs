pub const TIPS: &[&str] = &[
    "Hold the hotkey's modifier to keep the panel open while you read it.",
    "Right click a flag to search for items that do not have it.",
    "A locked check takes focus, so you can type in the filters.",
    "Turn a filter off rather than deleting it: it keeps its range for later.",
    "Currency goes through the exchange, so it shows offers rather than listings.",
    "The price comes from the listings themselves, not from a third party.",
];

pub fn tip(seed: u64) -> &'static str {
    TIPS[(seed as usize) % TIPS.len()]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub keys: String,
    pub does: String,
}

pub fn entries(bindings: &[(String, String)]) -> Vec<Entry> {
    bindings
        .iter()
        .filter(|(keys, does)| !keys.trim().is_empty() && !does.trim().is_empty())
        .map(|(keys, does)| Entry {
            keys: keys.trim().to_string(),
            does: does.trim().to_string(),
        })
        .collect()
}

pub fn widest_key(entries: &[Entry]) -> usize {
    entries
        .iter()
        .map(|e| e.keys.chars().count())
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some() -> Vec<(String, String)> {
        vec![
            ("Ctrl+D".into(), "price check".into()),
            ("Ctrl+Alt+D".into(), "price check, stays open".into()),
            ("".into(), "not bound".into()),
            ("F5".into(), "  ".into()),
        ]
    }

    #[test]
    fn a_tip_is_picked_for_any_seed_without_going_out_of_range() {
        for seed in [0u64, 1, 5, 6, 999, u64::MAX] {
            assert!(!tip(seed).trim().is_empty(), "seed {seed}");
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_tip() {
        assert_eq!(tip(3), tip(3));
    }

    #[test]
    fn different_seeds_reach_different_tips() {
        assert_ne!(tip(0), tip(1));
    }

    #[test]
    fn every_tip_is_a_sentence() {
        for tip in TIPS {
            assert!(tip.ends_with('.'), "{tip}");
            assert!(tip.len() > 20, "{tip}");
        }
    }

    #[test]
    fn only_bindings_that_do_something_are_listed() {
        let got = entries(&some());

        assert_eq!(got.len(), 2);
        assert_eq!(got[0].keys, "Ctrl+D");
    }

    #[test]
    fn an_unbound_action_is_not_shown_as_a_blank_row() {
        let got = entries(&some());

        assert!(got.iter().all(|e| !e.keys.is_empty()));
        assert!(got.iter().all(|e| !e.does.is_empty()));
    }

    #[test]
    fn the_widest_key_is_known_so_the_column_can_line_up() {
        assert_eq!(widest_key(&entries(&some())), "Ctrl+Alt+D".chars().count());
    }

    #[test]
    fn nothing_bound_is_an_empty_list_rather_than_a_panic() {
        assert!(entries(&[]).is_empty());
        assert_eq!(widest_key(&[]), 0);
    }
}
