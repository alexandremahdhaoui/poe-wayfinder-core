use crate::types::GameVersion;

pub const POE1_TITLE: &str = "Path of Exile";
pub const POE2_TITLE: &str = "Path of Exile 2";

pub fn title_for(game: GameVersion) -> &'static str {
    match game {
        GameVersion::Poe1 => POE1_TITLE,
        GameVersion::Poe2 => POE2_TITLE,
    }
}

pub fn game_for_title(title: &str) -> Option<GameVersion> {
    match title.trim() {
        POE2_TITLE => Some(GameVersion::Poe2),
        POE1_TITLE => Some(GameVersion::Poe1),
        _ => None,
    }
}

pub fn detect(foreground: Option<&str>, titles: &[String]) -> Option<GameVersion> {
    if let Some(game) = foreground.and_then(game_for_title) {
        return Some(game);
    }

    let found: Vec<GameVersion> = titles.iter().filter_map(|t| game_for_title(t)).collect();

    found
        .iter()
        .copied()
        .find(|g| *g == GameVersion::Poe2)
        .or_else(|| found.first().copied())
}

pub fn detect_change(
    current: GameVersion,
    foreground: Option<&str>,
    titles: &[String],
) -> Option<GameVersion> {
    if let Some(game) = foreground.and_then(game_for_title) {
        return (game != current).then_some(game);
    }

    let open: Vec<GameVersion> = titles.iter().filter_map(|t| game_for_title(t)).collect();

    if open.contains(&current) || open.is_empty() {
        return None;
    }

    open.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn each_game_has_its_own_window_title() {
        assert_ne!(title_for(GameVersion::Poe1), title_for(GameVersion::Poe2));
    }

    #[test]
    fn a_title_round_trips_to_the_game_that_owns_it() {
        for game in [GameVersion::Poe1, GameVersion::Poe2] {
            assert_eq!(game_for_title(title_for(game)), Some(game), "{game:?}");
        }
    }

    #[test]
    fn poe2_is_not_read_as_poe1_even_though_its_title_contains_it() {
        assert_eq!(game_for_title("Path of Exile 2"), Some(GameVersion::Poe2));
    }

    #[test]
    fn only_the_exact_poe1_title_is_poe1() {
        for near in [
            "Path of Exile 2",
            "Path of Exile 2 - Settings",
            "Path of Exile Trade",
            "Path of Exiles",
            "path of exile",
            "PATH OF EXILE",
            "Awakened Path of Exile Trade",
        ] {
            assert_ne!(
                game_for_title(near),
                Some(GameVersion::Poe1),
                "{near:?} must not be read as PoE1"
            );
        }
    }

    #[test]
    fn surrounding_whitespace_does_not_hide_a_game() {
        assert_eq!(
            game_for_title("  Path of Exile 2 "),
            Some(GameVersion::Poe2)
        );
        assert_eq!(game_for_title("\tPath of Exile\n"), Some(GameVersion::Poe1));
    }

    #[test]
    fn a_window_that_is_not_a_game_is_no_game() {
        for other in ["", "Discord", "poe-wayfinder", "Firefox"] {
            assert_eq!(game_for_title(other), None, "{other:?}");
        }
    }

    #[test]
    fn nothing_open_means_no_game() {
        assert_eq!(detect(None, &[]), None);
        assert_eq!(detect(Some("Firefox"), &titles(&["Discord"])), None);
    }

    #[test]
    fn the_only_game_open_is_the_game() {
        assert_eq!(
            detect(Some("Firefox"), &titles(&["Discord", POE1_TITLE])),
            Some(GameVersion::Poe1)
        );
    }

    #[test]
    fn the_foreground_window_decides_when_both_games_are_open() {
        let both = titles(&[POE2_TITLE, POE1_TITLE]);

        assert_eq!(detect(Some(POE1_TITLE), &both), Some(GameVersion::Poe1));
        assert_eq!(detect(Some(POE2_TITLE), &both), Some(GameVersion::Poe2));
    }

    #[test]
    fn with_both_open_and_neither_in_front_poe2_wins() {
        assert_eq!(
            detect(Some("Firefox"), &titles(&[POE1_TITLE, POE2_TITLE])),
            Some(GameVersion::Poe2)
        );
        assert_eq!(
            detect(None, &titles(&[POE1_TITLE, POE2_TITLE])),
            Some(GameVersion::Poe2)
        );
    }

    #[test]
    fn alt_tabbing_to_something_that_is_not_a_game_changes_nothing() {
        let both = titles(&[POE1_TITLE, POE2_TITLE]);

        assert_eq!(
            detect_change(GameVersion::Poe1, Some("Firefox"), &both),
            None
        );
        assert_eq!(
            detect_change(GameVersion::Poe2, Some("Firefox"), &both),
            None
        );
        assert_eq!(detect_change(GameVersion::Poe1, None, &both), None);
    }

    #[test]
    fn bringing_the_other_game_forward_changes_the_game() {
        let both = titles(&[POE1_TITLE, POE2_TITLE]);

        assert_eq!(
            detect_change(GameVersion::Poe2, Some(POE1_TITLE), &both),
            Some(GameVersion::Poe1)
        );
    }

    #[test]
    fn bringing_the_game_it_already_follows_forward_changes_nothing() {
        let both = titles(&[POE1_TITLE, POE2_TITLE]);

        assert_eq!(
            detect_change(GameVersion::Poe1, Some(POE1_TITLE), &both),
            None
        );
    }

    #[test]
    fn closing_the_game_it_follows_moves_it_to_the_one_still_open() {
        assert_eq!(
            detect_change(GameVersion::Poe2, Some("Firefox"), &titles(&[POE1_TITLE])),
            Some(GameVersion::Poe1)
        );
    }

    #[test]
    fn closing_every_game_leaves_it_where_it_was_rather_than_flapping() {
        assert_eq!(
            detect_change(GameVersion::Poe1, Some("Firefox"), &titles(&["Discord"])),
            None
        );
        assert_eq!(detect_change(GameVersion::Poe1, None, &[]), None);
    }

    #[test]
    fn a_minimised_game_it_follows_is_not_swapped_for_the_other_one() {
        let both = titles(&[POE1_TITLE, POE2_TITLE]);

        assert_eq!(
            detect_change(GameVersion::Poe1, Some("Discord"), &both),
            None,
            "PoE2 being open must not steal the overlay while PoE1 is the game in play"
        );
    }

    #[test]
    fn a_foreground_window_that_is_not_a_game_does_not_hide_an_open_one() {
        assert_eq!(
            detect(Some("Discord"), &titles(&[POE2_TITLE])),
            Some(GameVersion::Poe2)
        );
    }
}
