#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elevation {
    Elevated,
    Normal,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyOutlook {
    Fine,
    BlockedByGame,
    OverlayElevated,
    Unknown,
}

pub fn hotkey_outlook(overlay: Elevation, game: Elevation) -> HotkeyOutlook {
    match (overlay, game) {
        (Elevation::Unknown, _) | (_, Elevation::Unknown) => HotkeyOutlook::Unknown,
        (Elevation::Normal, Elevation::Elevated) => HotkeyOutlook::BlockedByGame,
        (Elevation::Elevated, Elevation::Normal) => HotkeyOutlook::OverlayElevated,
        _ => HotkeyOutlook::Fine,
    }
}

pub fn advice(outlook: HotkeyOutlook) -> Option<&'static str> {
    match outlook {
        HotkeyOutlook::Fine => None,

        HotkeyOutlook::BlockedByGame => Some(
            "The game runs as administrator and this does not. That mismatch is the usual \
             reason a price check hotkey registers and then never fires. Close this, right \
             click it and choose Run as administrator. Steam is the usual cause: once it \
             updates itself it stays elevated and every game it launches inherits that. \
             To check it is the cause, press the hotkey with the game minimised: if it works \
             there and not in game, this is why.",
        ),

        HotkeyOutlook::OverlayElevated => Some(
            "This is running as administrator and the game is not. The hotkey will work. \
             Running both the same way is tidier.",
        ),

        HotkeyOutlook::Unknown => Some(
            "Could not tell whether the game runs as administrator. If the hotkey does nothing, \
             try running this as administrator, and try pressing it with the game minimised to \
             see whether the game is what stops it.",
        ),
    }
}

pub fn is_blocking(outlook: HotkeyOutlook) -> bool {
    outlook == HotkeyOutlook::BlockedByGame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_ordinary_processes_are_fine() {
        assert_eq!(
            hotkey_outlook(Elevation::Normal, Elevation::Normal),
            HotkeyOutlook::Fine
        );
    }

    #[test]
    fn two_elevated_processes_are_fine() {
        assert_eq!(
            hotkey_outlook(Elevation::Elevated, Elevation::Elevated),
            HotkeyOutlook::Fine
        );
    }

    #[test]
    fn an_elevated_game_blocks_an_ordinary_overlay() {
        assert_eq!(
            hotkey_outlook(Elevation::Normal, Elevation::Elevated),
            HotkeyOutlook::BlockedByGame
        );
    }

    #[test]
    fn an_elevated_overlay_over_an_ordinary_game_still_works() {
        assert_eq!(
            hotkey_outlook(Elevation::Elevated, Elevation::Normal),
            HotkeyOutlook::OverlayElevated
        );
    }

    #[test]
    fn an_unknown_level_on_either_side_is_unknown() {
        for pair in [
            (Elevation::Unknown, Elevation::Elevated),
            (Elevation::Unknown, Elevation::Normal),
            (Elevation::Normal, Elevation::Unknown),
            (Elevation::Elevated, Elevation::Unknown),
            (Elevation::Unknown, Elevation::Unknown),
        ] {
            assert_eq!(
                hotkey_outlook(pair.0, pair.1),
                HotkeyOutlook::Unknown,
                "{pair:?}"
            );
        }
    }

    #[test]
    fn only_the_blocked_case_is_blocking() {
        assert!(is_blocking(HotkeyOutlook::BlockedByGame));

        for outlook in [
            HotkeyOutlook::Fine,
            HotkeyOutlook::OverlayElevated,
            HotkeyOutlook::Unknown,
        ] {
            assert!(!is_blocking(outlook), "{outlook:?}");
        }
    }

    #[test]
    fn a_working_setup_says_nothing() {
        assert_eq!(advice(HotkeyOutlook::Fine), None);
    }

    #[test]
    fn every_other_outlook_says_something_actionable() {
        for outlook in [
            HotkeyOutlook::BlockedByGame,
            HotkeyOutlook::OverlayElevated,
            HotkeyOutlook::Unknown,
        ] {
            let text = advice(outlook).expect("advice");

            assert!(
                text.contains("administrator"),
                "{outlook:?} does not name the fix: {text}"
            );
        }
    }

    #[test]
    fn the_blocking_advice_names_steam() {
        let text = advice(HotkeyOutlook::BlockedByGame).expect("advice");

        assert!(text.contains("Steam"), "{text}");
    }
}
