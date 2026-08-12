pub const SHOWN: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub name: String,
    pub exact: bool,
}

pub fn search(needle: &str, names: &[String]) -> Vec<Hit> {
    let needle = needle.trim().to_lowercase();

    if needle.len() < 2 {
        return Vec::new();
    }

    let mut out: Vec<Hit> = names
        .iter()
        .filter_map(|name| {
            let lower = name.to_lowercase();

            if !lower.contains(&needle) {
                return None;
            }

            Some(Hit {
                exact: lower == needle,
                name: name.clone(),
            })
        })
        .collect();

    out.sort_by(|a, b| {
        b.exact
            .cmp(&a.exact)
            .then_with(|| a.name.len().cmp(&b.name.len()))
            .then_with(|| a.name.cmp(&b.name))
    });

    out.dedup_by(|a, b| a.name == b.name);
    out.truncate(SHOWN);

    out
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Starred {
    names: Vec<String>,
}

impl Starred {
    pub fn star(&mut self, name: &str) {
        if name.trim().is_empty() || self.names.iter().any(|n| n == name) {
            return;
        }

        self.names.push(name.to_string());
    }

    pub fn toggle_star(&mut self, name: &str) -> bool {
        match self.names.iter().position(|n| n == name) {
            Some(at) => {
                self.names.remove(at);

                false
            }
            None => {
                self.star(name);

                true
            }
        }
    }

    pub fn starred(&self) -> &[String] {
        &self.names
    }

    pub fn is_starred(&self, name: &str) -> bool {
        self.names.iter().any(|n| n == name)
    }

    pub fn clear_stars(&mut self) {
        self.names.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        [
            "Sapphire Ring",
            "Two-Stone Ring",
            "Ring",
            "Gold Ring",
            "Chaos Orb",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
    }

    #[test]
    fn starring_a_name_keeps_it_and_starring_it_twice_does_not_duplicate() {
        let mut starred = Starred::default();

        starred.star("Sapphire Ring");
        starred.star("Sapphire Ring");

        assert_eq!(starred.starred(), ["Sapphire Ring"]);
        assert!(starred.is_starred("Sapphire Ring"));
    }

    #[test]
    fn clicking_a_starred_name_unstars_it() {
        let mut starred = Starred::default();

        assert!(starred.toggle_star("Ring"), "the first click stars it");
        assert!(!starred.toggle_star("Ring"), "the second click clears it");
        assert!(starred.starred().is_empty());
    }

    #[test]
    fn a_blank_name_is_never_starred() {
        let mut starred = Starred::default();

        starred.star("   ");

        assert!(starred.starred().is_empty());
    }

    #[test]
    fn the_stars_can_all_be_cleared_at_once() {
        let mut starred = Starred::default();

        starred.star("a");
        starred.star("b");
        starred.clear_stars();

        assert!(starred.starred().is_empty());
    }

    #[test]
    fn a_search_finds_every_name_that_contains_it() {
        let got = search("ring", &names());

        assert_eq!(got.len(), 4);
        assert!(got.iter().all(|h| h.name.to_lowercase().contains("ring")));
    }

    #[test]
    fn an_exact_name_comes_first() {
        let got = search("ring", &names());

        assert_eq!(got[0].name, "Ring");
        assert!(got[0].exact);
    }

    #[test]
    fn shorter_names_come_before_longer_ones() {
        let got = search("ring", &names());
        let positions: Vec<usize> = got.iter().map(|h| h.name.len()).collect();

        let mut sorted = positions.clone();
        sorted.sort_unstable();

        assert_eq!(positions, sorted, "{got:?}");
    }

    #[test]
    fn the_search_ignores_case() {
        assert_eq!(search("SAPPHIRE", &names()).len(), 1);
    }

    #[test]
    fn one_letter_is_too_broad_to_search_on() {
        assert!(search("r", &names()).is_empty());
        assert!(search("", &names()).is_empty());
        assert!(search("  ", &names()).is_empty());
    }

    #[test]
    fn a_name_nobody_has_finds_nothing() {
        assert!(search("mirror of kalandra", &names()).is_empty());
    }

    #[test]
    fn the_list_is_capped_so_the_panel_stays_readable() {
        let many: Vec<String> = (0..100).map(|i| format!("Ring {i}")).collect();

        assert_eq!(search("ring", &many).len(), SHOWN);
    }

    #[test]
    fn the_same_name_twice_is_shown_once() {
        let twice = vec!["Gold Ring".to_string(), "Gold Ring".to_string()];

        assert_eq!(search("gold", &twice).len(), 1);
    }
}
