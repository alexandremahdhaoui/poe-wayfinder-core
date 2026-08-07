//! Whether the overlay can receive the hotkey at all.
//!
//! # The failure this exists for
//!
//! The overlay starts perfectly. It finds the game window, registers the
//! hotkey, reports that keyboard input works, and adds its tray icon. Then the
//! hotkey does nothing, forever, and the log stays silent because no press
//! ever arrives.
//!
//! The remedy is well attested even where the exact mechanism is not.
//!
//! Windows isolates processes by privilege, and a tool running below the
//! window that has focus can find its input silently withheld. What is
//! documented beyond doubt is the fix: Awakened PoE Trade hits the same
//! symptom and its answer is to run the tool at the same privilege as the
//! game.
//!
//! What is **not** claimed here is the precise kernel rule. Microsoft's own
//! notes on UIPI are about hooks rather than hotkeys, and at least one
//! reference states that hotkey combinations are not blocked. So this reports
//! a mismatch and the remedy, and does not assert a cause it cannot prove.
//!
//! Steam is the usual reason. After Steam updates itself it keeps running
//! elevated, and every game it launches inherits that.
//!
//! Awakened PoE Trade documents the same thing and its answer is the same:
//! run both at the same privilege level.
//!
//! # Why this is a whole module
//!
//! Because the diagnosis is the fix. Nothing in the tool can reach across the
//! privilege boundary, so the only useful thing it can do is say precisely
//! what is wrong and what to do about it, at startup, before the user presses
//! a key and sees nothing.

/// What the overlay can tell about the two privilege levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elevation {
    /// Running as administrator.
    Elevated,
    /// Running as an ordinary user.
    Normal,
    /// Could not be determined.
    ///
    /// Reported rather than assumed. Guessing wrong here produces a confident
    /// message pointing at the wrong problem.
    Unknown,
}

/// What the overlay should tell the user about the hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyOutlook {
    /// Nothing in the way.
    Fine,
    /// The game is elevated and the overlay is not. The hotkey cannot arrive.
    BlockedByGame,
    /// The overlay is elevated and the game is not.
    ///
    /// Works, but worth saying. The overlay can send input the game will not
    /// accept in the other direction, and the user usually did not mean to.
    OverlayElevated,
    /// Not enough is known to say.
    Unknown,
}

/// Compare the two privilege levels.
///
/// # Why unknown is not treated as normal
///
/// Reading another process's token can fail for reasons that have nothing to
/// do with elevation. Answering `Fine` on a failed read would hide the exact
/// problem this module exists to name.
pub fn hotkey_outlook(overlay: Elevation, game: Elevation) -> HotkeyOutlook {
    match (overlay, game) {
        (Elevation::Unknown, _) | (_, Elevation::Unknown) => HotkeyOutlook::Unknown,
        (Elevation::Normal, Elevation::Elevated) => HotkeyOutlook::BlockedByGame,
        (Elevation::Elevated, Elevation::Normal) => HotkeyOutlook::OverlayElevated,
        _ => HotkeyOutlook::Fine,
    }
}

/// What to tell the user, in one line they can act on.
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

/// Whether the outlook means the hotkey cannot work.
///
/// Separate from the advice because the caller logs a warning for this and an
/// informational line for the rest.
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
        // Matching levels work, whichever level it is.
        assert_eq!(
            hotkey_outlook(Elevation::Elevated, Elevation::Elevated),
            HotkeyOutlook::Fine
        );
    }

    #[test]
    fn an_elevated_game_blocks_an_ordinary_overlay() {
        // The whole reason this module exists. It is the mismatch the tool
        // community reports for this exact symptom, and the remedy is well
        // attested even though the precise kernel rule is not.
        assert_eq!(
            hotkey_outlook(Elevation::Normal, Elevation::Elevated),
            HotkeyOutlook::BlockedByGame
        );
    }

    #[test]
    fn an_elevated_overlay_over_an_ordinary_game_still_works() {
        // Whatever the restriction is, it bites in one direction only.
        assert_eq!(
            hotkey_outlook(Elevation::Elevated, Elevation::Normal),
            HotkeyOutlook::OverlayElevated
        );
    }

    #[test]
    fn an_unknown_level_on_either_side_is_unknown() {
        // Answering Fine on a failed read would hide the exact problem this
        // module exists to name.
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
        // A line printed on every healthy start is a line nobody reads when it
        // matters.
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
        // Steam is the usual cause and the user has no reason to suspect it,
        // because they never chose to run the game elevated.
        let text = advice(HotkeyOutlook::BlockedByGame).expect("advice");

        assert!(text.contains("Steam"), "{text}");
    }
}
