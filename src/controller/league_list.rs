use crate::controller::background::is_private_league;

pub const PERMANENT: [&str; 4] = ["Standard", "Hardcore", "SSF Standard", "SSF Hardcore"];

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
    !PERMANENT.contains(&league) && !is_private_league(league) && !is_hardcore(league)
}

fn is_hardcore(league: &str) -> bool {
    league.starts_with("HC ") || league.starts_with("Hardcore ")
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
}
