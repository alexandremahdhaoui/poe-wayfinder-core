use crate::types::ParsedItem;

pub const PROFILES: usize = 3;
pub const NO_DECISION: &str = "---";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Unset,
    Deadly,
    Warning,
    Good,
    Seen,
}

impl Verdict {
    pub fn from_char(c: char) -> Self {
        match c {
            'd' => Verdict::Deadly,
            'w' => Verdict::Warning,
            'g' => Verdict::Good,
            's' => Verdict::Seen,
            _ => Verdict::Unset,
        }
    }

    pub fn as_char(self) -> char {
        match self {
            Verdict::Deadly => 'd',
            Verdict::Warning => 'w',
            Verdict::Good => 'g',
            Verdict::Seen => 's',
            Verdict::Unset => '-',
        }
    }

    pub fn next(self) -> Self {
        match self {
            Verdict::Deadly => Verdict::Warning,
            Verdict::Warning => Verdict::Good,
            Verdict::Good => Verdict::Seen,
            _ => Verdict::Deadly,
        }
    }

    pub fn is_coloured(self) -> bool {
        !matches!(self, Verdict::Unset | Verdict::Seen)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Deadly => "deadly",
            Verdict::Warning => "warning",
            Verdict::Good => "good",
            Verdict::Seen => "seen",
            Verdict::Unset => "unset",
        }
    }
}

pub fn verdict_of(decisions: &str, profile: usize) -> Verdict {
    if profile == 0 || profile > PROFILES {
        return Verdict::Unset;
    }

    decisions
        .chars()
        .nth(profile - 1)
        .map(Verdict::from_char)
        .unwrap_or(Verdict::Unset)
}

pub fn set_verdict(decisions: &str, profile: usize, verdict: Verdict) -> String {
    if profile == 0 || profile > PROFILES {
        return decisions.to_string();
    }

    let mut chars: Vec<char> = decisions.chars().collect();

    chars.resize(PROFILES, '-');
    chars[profile - 1] = verdict.as_char();

    chars.into_iter().collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concern {
    pub text: String,
    pub verdict: Verdict,
}

pub fn is_outdated(decisions: &str, profile: usize, still_known: bool) -> bool {
    verdict_of(decisions, profile).is_coloured() && !still_known
}

pub fn review(item: &ParsedItem, decisions: &[(String, String)], profile: usize) -> Vec<Concern> {
    let mut out = Vec::new();

    for modifier in &item.modifiers {
        for stat in &modifier.stats {
            let text = stat.matched.trim();
            let template = stat.reference.trim();

            if text.is_empty() {
                continue;
            }

            let verdict = decisions
                .iter()
                .find(|(matcher, _)| matcher == template)
                .map(|(_, set)| verdict_of(set, profile))
                .unwrap_or(Verdict::Unset);

            out.push(Concern {
                text: text.to_string(),
                verdict,
            });
        }
    }

    out
}

pub fn worst(concerns: &[Concern]) -> Verdict {
    if concerns.iter().any(|c| c.verdict == Verdict::Deadly) {
        return Verdict::Deadly;
    }

    if concerns.iter().any(|c| c.verdict == Verdict::Warning) {
        return Verdict::Warning;
    }

    Verdict::Unset
}

pub fn headline(concerns: &[Concern]) -> String {
    match worst(concerns) {
        Verdict::Deadly => "Do not run this map".to_string(),
        Verdict::Warning => "This map needs care".to_string(),
        _ => format!("{} mods, none marked", concerns.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_mod_is_shown_as_the_game_printed_it_not_as_a_template() {
        let item = ParsedItem {
            modifiers: vec![crate::controller::parse::shared::modifiers::ParsedModifier {
                info: crate::types::modifier::ModifierInfo::default(),
                stats: vec![crate::types::stat::ParsedStat {
                    reference: "#% increased Monster Damage".to_string(),
                    matched: "35% increased Monster Damage".to_string(),
                    roll: None,
                }],
            }],
            ..ParsedItem::default()
        };

        let got = review(&item, &[], 0);

        assert_eq!(got[0].text, "35% increased Monster Damage");
        assert!(!got[0].text.contains('#'), "nobody wants to read a template");
    }

    #[test]
    fn a_verdict_round_trips_through_its_letter() {
        for verdict in [
            Verdict::Deadly,
            Verdict::Warning,
            Verdict::Good,
            Verdict::Seen,
            Verdict::Unset,
        ] {
            assert_eq!(Verdict::from_char(verdict.as_char()), verdict);
        }
    }

    #[test]
    fn cycling_a_verdict_goes_deadly_warning_good_seen_and_back() {
        assert_eq!(Verdict::Unset.next(), Verdict::Deadly);
        assert_eq!(Verdict::Deadly.next(), Verdict::Warning);
        assert_eq!(Verdict::Warning.next(), Verdict::Good);
        assert_eq!(Verdict::Good.next(), Verdict::Seen);
        assert_eq!(Verdict::Seen.next(), Verdict::Deadly);
    }

    #[test]
    fn only_a_real_judgement_is_coloured() {
        assert!(Verdict::Deadly.is_coloured());
        assert!(Verdict::Warning.is_coloured());
        assert!(Verdict::Good.is_coloured());

        assert!(!Verdict::Unset.is_coloured());
        assert!(
            !Verdict::Seen.is_coloured(),
            "seen means read and dismissed"
        );
    }

    #[test]
    fn each_profile_keeps_its_own_verdict() {
        let set = set_verdict(NO_DECISION, 2, Verdict::Deadly);

        assert_eq!(verdict_of(&set, 1), Verdict::Unset);
        assert_eq!(verdict_of(&set, 2), Verdict::Deadly);
        assert_eq!(verdict_of(&set, 3), Verdict::Unset);
    }

    #[test]
    fn three_profiles_fit_and_a_fourth_is_refused() {
        let set = set_verdict(NO_DECISION, 4, Verdict::Deadly);

        assert_eq!(set, NO_DECISION, "there are only three profiles");
        assert_eq!(verdict_of(NO_DECISION, 0), Verdict::Unset);
    }

    #[test]
    fn a_short_or_empty_decision_set_still_reads() {
        assert_eq!(verdict_of("", 1), Verdict::Unset);
        assert_eq!(verdict_of("d", 1), Verdict::Deadly);
        assert_eq!(verdict_of("d", 3), Verdict::Unset);
    }

    #[test]
    fn setting_a_verdict_on_a_short_set_pads_it_out() {
        assert_eq!(set_verdict("", 3, Verdict::Good).len(), PROFILES);
    }

    #[test]
    fn a_mod_that_was_marked_and_no_longer_exists_is_outdated() {
        let marked = set_verdict(NO_DECISION, 1, Verdict::Deadly);

        assert!(is_outdated(&marked, 1, false), "the game dropped the mod");
        assert!(
            !is_outdated(&marked, 1, true),
            "the mod is still in the data"
        );
    }

    #[test]
    fn a_mod_that_was_never_marked_is_not_outdated_just_because_it_is_gone() {
        assert!(!is_outdated(NO_DECISION, 1, false));
    }

    #[test]
    fn a_mod_only_marked_seen_is_not_worth_reporting_as_outdated() {
        let seen = set_verdict(NO_DECISION, 1, Verdict::Seen);

        assert!(!is_outdated(&seen, 1, false));
    }

    #[test]
    fn the_worst_verdict_wins_because_it_is_what_kills_you() {
        let concerns = vec![
            Concern {
                text: "a".into(),
                verdict: Verdict::Good,
            },
            Concern {
                text: "b".into(),
                verdict: Verdict::Deadly,
            },
            Concern {
                text: "c".into(),
                verdict: Verdict::Warning,
            },
        ];

        assert_eq!(worst(&concerns), Verdict::Deadly);
        assert!(headline(&concerns).contains("Do not run"));
    }

    #[test]
    fn a_warning_is_reported_when_nothing_is_deadly() {
        let concerns = vec![Concern {
            text: "a".into(),
            verdict: Verdict::Warning,
        }];

        assert_eq!(worst(&concerns), Verdict::Warning);
        assert!(headline(&concerns).contains("care"));
    }

    #[test]
    fn a_map_with_nothing_marked_says_how_many_mods_it_has() {
        let concerns = vec![Concern {
            text: "a".into(),
            verdict: Verdict::Unset,
        }];

        assert_eq!(worst(&concerns), Verdict::Unset);
        assert!(headline(&concerns).contains('1'));
    }

    #[test]
    fn an_item_with_no_mods_reviews_to_nothing() {
        assert!(review(&ParsedItem::default(), &[], 1).is_empty());
    }
}
