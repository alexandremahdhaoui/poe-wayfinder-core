use crate::types::GameVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    Wiki,
    Poedb,
    CraftOfExile,
}

impl Site {
    pub fn as_str(self) -> &'static str {
        match self {
            Site::Wiki => "wiki",
            Site::Poedb => "poedb",
            Site::CraftOfExile => "craft of exile",
        }
    }
}

pub fn url(site: Site, game: GameVersion, reference_name: &str, raw_text: &str) -> Option<String> {
    match site {
        Site::Wiki => wiki(game, reference_name),
        Site::Poedb => poedb(game, reference_name),
        Site::CraftOfExile => craft_of_exile(game, raw_text),
    }
}

fn wiki(game: GameVersion, reference_name: &str) -> Option<String> {
    let name = reference_name.trim();

    if name.is_empty() {
        return None;
    }

    let host = match game {
        GameVersion::Poe1 => "www.poewiki.net",
        GameVersion::Poe2 => "www.poe2wiki.net",
    };

    Some(format!(
        "https://{host}/wiki/{}",
        encode(&name.replace(' ', "_"))
    ))
}

fn poedb(game: GameVersion, reference_name: &str) -> Option<String> {
    let name = reference_name.trim();

    if name.is_empty() {
        return None;
    }

    let host = match game {
        GameVersion::Poe1 => "poedb.tw/us",
        GameVersion::Poe2 => "poe2db.tw/us",
    };

    Some(format!(
        "https://{host}/{}",
        encode(&name.replace(' ', "_"))
    ))
}

fn craft_of_exile(game: GameVersion, raw_text: &str) -> Option<String> {
    if raw_text.trim().is_empty() {
        return None;
    }

    let which = match game {
        GameVersion::Poe1 => "poe1",
        GameVersion::Poe2 => "poe2",
    };

    Some(format!(
        "https://craftofexile.com/?game={which}&eimport={}",
        encode(raw_text)
    ))
}

pub fn similar_items(name: &str) -> Option<String> {
    let name = name.trim();

    match name.is_empty() {
        true => None,
        false => Some(format!("\"{name}\"")),
    }
}

pub fn same_priced(amount: f64, currency: &str) -> Option<String> {
    let currency = currency.trim();

    if currency.is_empty() || !amount.is_finite() || amount <= 0.0 {
        return None;
    }

    let amount = match amount.fract() == 0.0 {
        true => format!("{}", amount as i64),
        false => format!("{amount:.2}"),
    };

    Some(format!("\"{amount} {currency}\""))
}

fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM: &str = "Item Class: Rings\nRarity: Rare\nDoom Loop\nSapphire Ring\n";

    #[test]
    fn each_game_has_its_own_wiki() {
        let one = url(Site::Wiki, GameVersion::Poe1, "Sapphire Ring", ITEM).unwrap();
        let two = url(Site::Wiki, GameVersion::Poe2, "Sapphire Ring", ITEM).unwrap();

        assert!(one.contains("poewiki.net"), "{one}");
        assert!(two.contains("poe2wiki.net"), "{two}");
        assert_ne!(one, two);
    }

    #[test]
    fn each_game_has_its_own_poedb() {
        let one = url(Site::Poedb, GameVersion::Poe1, "Sapphire Ring", ITEM).unwrap();
        let two = url(Site::Poedb, GameVersion::Poe2, "Sapphire Ring", ITEM).unwrap();

        assert!(one.contains("poedb.tw"), "{one}");
        assert!(two.contains("poe2db.tw"), "{two}");
    }

    #[test]
    fn a_wiki_link_uses_underscores_the_way_a_wiki_page_is_named() {
        let got = url(Site::Wiki, GameVersion::Poe2, "Sapphire Ring", ITEM).unwrap();

        assert!(got.ends_with("/Sapphire_Ring"), "{got}");
    }

    #[test]
    fn craft_of_exile_carries_the_whole_item_text() {
        let got = url(Site::CraftOfExile, GameVersion::Poe2, "Sapphire Ring", ITEM).unwrap();

        assert!(got.contains("game=poe2"), "{got}");
        assert!(got.contains("eimport="), "{got}");
        assert!(
            got.contains("Doom%20Loop"),
            "the item text is encoded: {got}"
        );
    }

    #[test]
    fn a_newline_never_breaks_out_of_the_query_string() {
        let got = url(Site::CraftOfExile, GameVersion::Poe2, "x", "a\nb").unwrap();

        assert!(!got.contains('\n'));
        assert!(got.contains("%0A"), "{got}");
    }

    #[test]
    fn an_apostrophe_in_a_unique_name_is_encoded() {
        let got = url(Site::Wiki, GameVersion::Poe1, "Kaom's Heart", ITEM).unwrap();

        assert!(!got.contains('\''), "{got}");
        assert!(got.contains("Kaom%27s_Heart"), "{got}");
    }

    #[test]
    fn an_item_with_no_name_has_no_link_rather_than_a_broken_one() {
        assert!(url(Site::Wiki, GameVersion::Poe2, "", ITEM).is_none());
        assert!(url(Site::Poedb, GameVersion::Poe2, "   ", ITEM).is_none());
    }

    #[test]
    fn an_empty_clipboard_has_no_craft_of_exile_link() {
        assert!(url(Site::CraftOfExile, GameVersion::Poe2, "Ring", "").is_none());
    }

    #[test]
    fn a_similar_search_quotes_the_name_so_the_stash_matches_it_whole() {
        assert_eq!(
            similar_items("Sapphire Ring"),
            Some("\"Sapphire Ring\"".to_string())
        );
    }

    #[test]
    fn an_item_with_no_name_has_nothing_to_search_for() {
        assert_eq!(similar_items("  "), None);
    }

    #[test]
    fn a_same_priced_search_names_the_amount_and_the_currency() {
        assert_eq!(same_priced(4.0, "chaos"), Some("\"4 chaos\"".to_string()));
        assert_eq!(
            same_priced(4.5, "chaos"),
            Some("\"4.50 chaos\"".to_string())
        );
    }

    #[test]
    fn an_item_with_no_price_cannot_be_searched_by_price() {
        assert_eq!(same_priced(0.0, "chaos"), None);
        assert_eq!(same_priced(-1.0, "chaos"), None);
        assert_eq!(same_priced(f64::NAN, "chaos"), None);
        assert_eq!(same_priced(4.0, ""), None);
    }

    #[test]
    fn every_site_names_itself_for_the_log() {
        for site in [Site::Wiki, Site::Poedb, Site::CraftOfExile] {
            assert!(!site.as_str().is_empty());
        }
    }

    #[test]
    fn every_link_is_https() {
        for site in [Site::Wiki, Site::Poedb, Site::CraftOfExile] {
            let got = url(site, GameVersion::Poe2, "Sapphire Ring", ITEM).unwrap();

            assert!(got.starts_with("https://"), "{site:?}: {got}");
        }
    }
}
