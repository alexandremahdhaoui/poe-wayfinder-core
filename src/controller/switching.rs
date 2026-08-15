use crate::controller::game_detect;
use crate::controller::league_list::{self, LeagueFrom, PERMANENT};
use crate::types::GameVersion;

pub const AUTOMATIC_LEAGUE: &str = "current league (automatic)";
pub const AUTOMATIC_GAME: &str = "whichever game is in front";

pub const LIST_WAS_READ: &str = "from the trade site's own league list";
pub const LIST_WAS_NOT_READ: &str =
    "the trade site's league list was not read for this game, so only the leagues this run \
     already knows are offered";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LeagueChoice {
    #[default]
    Automatic,
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeagueOption {
    pub label: String,
    pub choice: LeagueChoice,
    pub selected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LeagueMenu {
    pub options: Vec<LeagueOption>,
    pub caption: &'static str,
    pub list_was_read: bool,
}

pub struct Known<'a> {
    pub fetched: &'a [String],
    pub remembered: Option<&'a str>,
    pub in_use: &'a str,
    pub chosen: &'a LeagueChoice,
}

pub fn league_menu(known: Known) -> LeagueMenu {
    let list_was_read = !known.fetched.is_empty();

    let names = match list_was_read {
        true => known.fetched.to_vec(),
        false => remaining_names(&known),
    };

    let mut options = vec![LeagueOption {
        label: AUTOMATIC_LEAGUE.to_string(),
        selected: *known.chosen == LeagueChoice::Automatic,
        choice: LeagueChoice::Automatic,
    }];

    for name in deduped(names) {
        options.push(LeagueOption {
            selected: *known.chosen == LeagueChoice::Named(name.clone()),
            label: name.clone(),
            choice: LeagueChoice::Named(name),
        });
    }

    LeagueMenu {
        options,
        caption: match list_was_read {
            true => LIST_WAS_READ,
            false => LIST_WAS_NOT_READ,
        },
        list_was_read,
    }
}

fn remaining_names(known: &Known) -> Vec<String> {
    let mut names = Vec::new();

    if let LeagueChoice::Named(name) = known.chosen {
        names.push(name.clone());
    }

    names.push(known.in_use.to_string());

    if let Some(remembered) = known.remembered {
        names.push(remembered.to_string());
    }

    names.extend(PERMANENT.iter().map(|name| (*name).to_string()));

    names
}

fn deduped(names: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    for name in names {
        let name = name.trim().to_string();

        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
    }

    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chosen {
    pub name: String,
    pub from: LeagueFrom,
}

pub fn league_now(chosen: &LeagueChoice, fetched: &[String], keep: &str) -> Chosen {
    if let LeagueChoice::Named(name) = chosen {
        return Chosen {
            name: name.clone(),
            from: LeagueFrom::Chosen,
        };
    }

    match league_list::current(fetched) {
        Some(name) => Chosen {
            name,
            from: LeagueFrom::TradeApi,
        },
        None => Chosen {
            name: keep.to_string(),
            from: LeagueFrom::LastRun,
        },
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GameChoice {
    #[default]
    Automatic,
    Pinned(GameVersion),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameOption {
    pub label: &'static str,
    pub choice: GameChoice,
    pub selected: bool,
}

pub fn game_menu(pinned: Option<GameVersion>) -> Vec<GameOption> {
    let chosen = match pinned {
        Some(game) => GameChoice::Pinned(game),
        None => GameChoice::Automatic,
    };

    [
        GameChoice::Automatic,
        GameChoice::Pinned(GameVersion::Poe1),
        GameChoice::Pinned(GameVersion::Poe2),
    ]
    .into_iter()
    .map(|choice| GameOption {
        label: match choice {
            GameChoice::Automatic => AUTOMATIC_GAME,
            GameChoice::Pinned(game) => game_detect::title_for(game),
        },
        selected: choice == chosen,
        choice,
    })
    .collect()
}

pub fn game_now(
    chosen: GameChoice,
    detected: Option<GameVersion>,
    keep: GameVersion,
) -> GameVersion {
    match chosen {
        GameChoice::Pinned(game) => game,
        GameChoice::Automatic => detected.unwrap_or(keep),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leagues(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    fn known<'a>(fetched: &'a [String], chosen: &'a LeagueChoice) -> Known<'a> {
        Known {
            fetched,
            remembered: None,
            in_use: "Rise of the Abyssal",
            chosen,
        }
    }

    fn labels(menu: &LeagueMenu) -> Vec<String> {
        menu.options.iter().map(|o| o.label.clone()).collect()
    }

    #[test]
    fn the_league_list_the_trade_site_answered_is_what_the_user_picks_from() {
        let fetched = leagues(&["Rise of the Abyssal", "HC Rise of the Abyssal", "Standard"]);

        let menu = league_menu(known(&fetched, &LeagueChoice::Automatic));

        assert_eq!(
            labels(&menu),
            leagues(&[
                AUTOMATIC_LEAGUE,
                "Rise of the Abyssal",
                "HC Rise of the Abyssal",
                "Standard"
            ])
        );
        assert!(menu.list_was_read);
        assert_eq!(menu.caption, LIST_WAS_READ);
    }

    #[test]
    fn automatic_is_always_the_first_choice_so_the_user_can_hand_the_league_back() {
        let fetched = leagues(&["Standard"]);

        let menu = league_menu(known(&fetched, &LeagueChoice::Named("Standard".into())));

        assert_eq!(menu.options[0].choice, LeagueChoice::Automatic);
        assert!(!menu.options[0].selected);
        assert!(menu.options[1].selected);
    }

    #[test]
    fn a_list_that_was_never_read_still_offers_what_this_run_knows_rather_than_nothing() {
        let menu = league_menu(Known {
            fetched: &[],
            remembered: Some("Mercenaries"),
            in_use: "Rise of the Abyssal",
            chosen: &LeagueChoice::Automatic,
        });

        assert!(!menu.list_was_read);
        assert_eq!(menu.caption, LIST_WAS_NOT_READ);
        assert_eq!(
            labels(&menu),
            leagues(&[
                AUTOMATIC_LEAGUE,
                "Rise of the Abyssal",
                "Mercenaries",
                "Standard",
                "Hardcore",
                "SSF Standard",
                "SSF Hardcore"
            ])
        );
    }

    #[test]
    fn a_league_already_chosen_by_hand_is_offered_even_when_no_list_was_read() {
        let chosen = LeagueChoice::Named("Private League (PL1234)".into());

        let menu = league_menu(Known {
            fetched: &[],
            remembered: None,
            in_use: "Standard",
            chosen: &chosen,
        });

        assert_eq!(menu.options[1].label, "Private League (PL1234)");
        assert!(menu.options[1].selected);
    }

    #[test]
    fn the_same_league_is_never_offered_twice() {
        let menu = league_menu(Known {
            fetched: &[],
            remembered: Some("Standard"),
            in_use: "Standard",
            chosen: &LeagueChoice::Named("Standard".into()),
        });

        assert_eq!(
            labels(&menu),
            leagues(&[
                AUTOMATIC_LEAGUE,
                "Standard",
                "Hardcore",
                "SSF Standard",
                "SSF Hardcore"
            ])
        );
    }

    #[test]
    fn a_league_this_run_has_no_name_for_is_not_offered_as_a_blank_row() {
        let menu = league_menu(Known {
            fetched: &[],
            remembered: None,
            in_use: "   ",
            chosen: &LeagueChoice::Automatic,
        });

        assert_eq!(menu.options[1].label, "Standard");
    }

    #[test]
    fn picking_a_league_by_hand_pins_it_and_says_the_user_chose_it() {
        let got = league_now(
            &LeagueChoice::Named("Standard".into()),
            &leagues(&["Rise of the Abyssal"]),
            "Rise of the Abyssal",
        );

        assert_eq!(got.name, "Standard");
        assert_eq!(got.from, LeagueFrom::Chosen);
    }

    #[test]
    fn picking_automatic_re_resolves_the_current_league_rather_than_keeping_the_pin() {
        let got = league_now(
            &LeagueChoice::Automatic,
            &leagues(&["Rise of the Abyssal", "Standard"]),
            "Standard",
        );

        assert_eq!(got.name, "Rise of the Abyssal");
        assert_eq!(got.from, LeagueFrom::TradeApi);
    }

    #[test]
    fn automatic_with_no_list_keeps_the_league_in_use_rather_than_emptying_it() {
        let got = league_now(&LeagueChoice::Automatic, &[], "Mercenaries");

        assert_eq!(got.name, "Mercenaries");
        assert_eq!(got.from, LeagueFrom::LastRun);
    }

    #[test]
    fn the_game_menu_offers_both_games_and_letting_the_foreground_window_decide() {
        let menu = game_menu(None);

        assert_eq!(
            menu.iter().map(|o| o.label).collect::<Vec<_>>(),
            vec![AUTOMATIC_GAME, "Path of Exile", "Path of Exile 2"]
        );
        assert!(menu[0].selected);
    }

    #[test]
    fn a_pinned_game_is_the_one_shown_as_selected() {
        let menu = game_menu(Some(GameVersion::Poe1));

        assert!(!menu[0].selected);
        assert!(menu[1].selected);
        assert!(!menu[2].selected);
    }

    #[test]
    fn pinning_a_game_ignores_whatever_window_is_in_front() {
        assert_eq!(
            game_now(
                GameChoice::Pinned(GameVersion::Poe1),
                Some(GameVersion::Poe2),
                GameVersion::Poe2
            ),
            GameVersion::Poe1
        );
    }

    #[test]
    fn automatic_follows_the_window_in_front() {
        assert_eq!(
            game_now(
                GameChoice::Automatic,
                Some(GameVersion::Poe1),
                GameVersion::Poe2
            ),
            GameVersion::Poe1
        );
    }

    #[test]
    fn automatic_with_no_game_running_keeps_the_one_already_watched() {
        assert_eq!(
            game_now(GameChoice::Automatic, None, GameVersion::Poe2),
            GameVersion::Poe2
        );
    }
}
