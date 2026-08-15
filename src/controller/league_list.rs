use crate::controller::background::is_private_league;

pub const PERMANENT: [&str; 4] = ["Standard", "Hardcore", "SSF Standard", "SSF Hardcore"];

pub const FALLBACK: &str = "Standard";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LeagueFrom {
    Configured,
    Chosen,
    TradeApi,
    LastRun,
    GameLog,
    #[default]
    Fallback,
}

impl LeagueFrom {
    pub fn as_str(self) -> &'static str {
        match self {
            LeagueFrom::Configured => "configured",
            LeagueFrom::Chosen => "chosen by hand",
            LeagueFrom::TradeApi => "trade api",
            LeagueFrom::LastRun => "last run",
            LeagueFrom::GameLog => "read from the game",
            LeagueFrom::Fallback => "fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct League {
    pub name: String,
    pub from: LeagueFrom,
}

pub struct Sources<'a> {
    pub configured: &'a str,
    pub chosen: Option<String>,
    pub fetched: Option<String>,
    pub remembered: Option<String>,
}

pub fn resolve(sources: Sources) -> League {
    let ordered = [
        (LeagueFrom::Configured, Some(sources.configured.to_string())),
        (LeagueFrom::Chosen, sources.chosen),
        (LeagueFrom::TradeApi, sources.fetched),
        (LeagueFrom::LastRun, sources.remembered),
    ];

    for (from, candidate) in ordered {
        let Some(name) = candidate else {
            continue;
        };

        if !name.trim().is_empty() {
            return League {
                name: name.trim().to_string(),
                from,
            };
        }
    }

    League {
        name: FALLBACK.to_string(),
        from: LeagueFrom::Fallback,
    }
}

pub fn parse(body: &str) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };

    let Some(entries) = root.get("result").and_then(|r| r.as_array()) else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|entry| entry.get("id").and_then(|id| id.as_str()))
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn current(leagues: &[String]) -> Option<String> {
    leagues
        .iter()
        .find(|league| is_temporary(league))
        .or_else(|| leagues.first())
        .cloned()
}

fn is_temporary(league: &str) -> bool {
    !PERMANENT.contains(&league)
        && !is_private_league(league)
        && !is_hardcore(league)
        && !is_ruthless(league)
        && !is_solo_self_found(league)
}

fn is_hardcore(league: &str) -> bool {
    league.starts_with("HC ") || league.starts_with("Hardcore ")
}

fn is_ruthless(league: &str) -> bool {
    league.contains("Ruthless")
}

fn is_solo_self_found(league: &str) -> bool {
    league.starts_with("SSF ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"{"result":[
        {"id":"Runes of Aldur","realm":"poe2","text":"Runes of Aldur"},
        {"id":"HC Runes of Aldur","realm":"poe2","text":"HC Runes of Aldur"},
        {"id":"Standard","realm":"poe2","text":"Standard"},
        {"id":"Hardcore","realm":"poe2","text":"Hardcore"}
    ]}"#;

    fn leagues(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    #[test]
    fn the_league_ids_are_read_in_the_order_the_trade_site_gives_them() {
        assert_eq!(
            parse(BODY),
            leagues(&[
                "Runes of Aldur",
                "HC Runes of Aldur",
                "Standard",
                "Hardcore"
            ])
        );
    }

    #[test]
    fn the_current_league_is_the_temporary_one_rather_than_standard() {
        assert_eq!(
            current(&parse(BODY)),
            Some("Runes of Aldur".to_string()),
            "searching Standard prices a league item against dead listings"
        );
    }

    #[test]
    fn a_hardcore_temporary_league_is_not_mistaken_for_the_softcore_one() {
        assert_eq!(
            current(&leagues(&["HC Runes of Aldur", "Runes of Aldur"])),
            Some("Runes of Aldur".to_string())
        );
    }

    #[test]
    fn a_private_league_is_never_chosen_on_its_own() {
        assert_eq!(
            current(&leagues(&["Runes of Aldur (PL12345)", "Runes of Aldur"])),
            Some("Runes of Aldur".to_string())
        );
    }

    #[test]
    fn a_ruthless_league_is_never_chosen_because_its_prices_are_a_different_economy() {
        assert_eq!(
            current(&leagues(&["Ruthless", "Runes of Aldur"])),
            Some("Runes of Aldur".to_string())
        );
        assert_eq!(
            current(&leagues(&[
                "Runes of Aldur Ruthless",
                "HC Ruthless",
                "Runes of Aldur"
            ])),
            Some("Runes of Aldur".to_string())
        );
    }

    #[test]
    fn a_solo_self_found_temporary_league_is_never_chosen_over_the_trade_league() {
        assert_eq!(
            current(&leagues(&["SSF Runes of Aldur", "Runes of Aldur"])),
            Some("Runes of Aldur".to_string())
        );
    }

    #[test]
    fn when_only_permanent_leagues_exist_the_first_one_is_used() {
        assert_eq!(
            current(&leagues(&["Standard", "Hardcore"])),
            Some("Standard".to_string())
        );
    }

    #[test]
    fn nothing_at_all_yields_no_league_rather_than_an_empty_one() {
        assert_eq!(current(&[]), None);
    }

    #[test]
    fn a_body_that_is_not_the_league_list_yields_nothing() {
        assert!(parse("not json").is_empty());
        assert!(parse("{}").is_empty());
        assert!(parse(r#"{"result":{}}"#).is_empty());
    }

    #[test]
    fn an_entry_with_no_id_is_skipped_rather_than_read_as_blank() {
        assert_eq!(
            parse(r#"{"result":[{"realm":"poe2"},{"id":"  "},{"id":"Runes of Aldur"}]}"#),
            leagues(&["Runes of Aldur"])
        );
    }

    fn sources() -> Sources<'static> {
        Sources {
            configured: "",
            chosen: None,
            fetched: None,
            remembered: None,
        }
    }

    #[test]
    fn a_named_league_wins_over_anything_detected() {
        let got = resolve(Sources {
            configured: "Standard",
            fetched: Some("HC".into()),
            remembered: Some("Runes of Aldur".into()),
            ..sources()
        });

        assert_eq!(got.name, "Standard");
        assert_eq!(got.from, LeagueFrom::Configured);
    }

    #[test]
    fn a_league_the_user_pinned_by_hand_survives_the_restart_that_would_re_resolve_it() {
        let got = resolve(Sources {
            chosen: Some("Standard".into()),
            fetched: Some("Runes of Aldur".into()),
            remembered: Some("Runes of Aldur".into()),
            ..sources()
        });

        assert_eq!(got.name, "Standard");
        assert_eq!(got.from, LeagueFrom::Chosen);
    }

    #[test]
    fn an_unset_league_takes_the_one_the_trade_site_reports() {
        let got = resolve(Sources {
            fetched: Some("Runes of Aldur".into()),
            remembered: Some("Old League".into()),
            ..sources()
        });

        assert_eq!(got.name, "Runes of Aldur");
        assert_eq!(got.from, LeagueFrom::TradeApi);
    }

    #[test]
    fn a_league_lookup_that_failed_falls_back_to_the_last_run() {
        let got = resolve(Sources {
            remembered: Some("Runes of Aldur".into()),
            ..sources()
        });

        assert_eq!(got.name, "Runes of Aldur");
        assert_eq!(got.from, LeagueFrom::LastRun);
    }

    #[test]
    fn knowing_nothing_at_all_searches_standard_rather_than_no_league() {
        for spelling in ["", "   "] {
            let got = resolve(Sources {
                configured: spelling,
                ..sources()
            });

            assert_eq!(got.name, FALLBACK);
            assert_eq!(got.from, LeagueFrom::Fallback);
        }
    }

    #[test]
    fn every_source_a_league_can_come_from_has_a_name_a_log_reader_can_read() {
        let every = [
            LeagueFrom::Configured,
            LeagueFrom::Chosen,
            LeagueFrom::TradeApi,
            LeagueFrom::LastRun,
            LeagueFrom::GameLog,
            LeagueFrom::Fallback,
        ];

        for (i, from) in every.iter().enumerate() {
            assert!(!from.as_str().is_empty(), "{from:?}");

            for other in &every[i + 1..] {
                assert_ne!(from.as_str(), other.as_str(), "{from:?} reads as {other:?}");
            }
        }
    }
}
