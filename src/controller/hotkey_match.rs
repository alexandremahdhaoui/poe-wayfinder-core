//! Deciding whether a key event completes the price check combination.
//!
//! # Why this exists next to `RegisterHotKey`
//!
//! `RegisterHotKey` asks Windows to watch one combination and tell us. It is
//! the smallest possible ask and it sees nothing else, which is why it was
//! chosen first.
//!
//! It has two costs. Windows can decline to deliver it, and the delivery
//! cannot be tested: input injected with `SendInput` is flagged
//! `LLKHF_INJECTED` and the hotkey machinery ignores flagged input, so no test
//! can press the key.
//!
//! A low level keyboard hook has the opposite properties. It sees injected
//! input, which makes the whole path testable, and it is what Awakened PoE
//! Trade uses. The cost is that a hook sees every keystroke on the machine,
//! so what it does with them matters.
//!
//! **This decides, and it is the only thing that decides.** It is handed one
//! key event at a time and answers one question. It keeps no text, no history
//! and nothing that is not a modifier state, so there is no keystroke log to
//! leak even in principle.

/// The modifier keys, as a set of flags.
///
/// A struct rather than a bitmask, because a bitmask makes a swapped pair of
/// bits invisible and this is compared against user configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl Modifiers {
    /// Whether every modifier the combination wants is held.
    ///
    /// Exact, not a subset. `Ctrl+D` must not fire on `Ctrl+Shift+D`, because
    /// the user may well have bound that to something else, and a price check
    /// they did not ask for costs a request against the rate limit.
    pub fn matches(self, wanted: Modifiers) -> bool {
        self == wanted
    }
}

/// One key going down or coming up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// The Windows virtual key code.
    pub code: u16,
    /// Whether the key went down. An up event never fires a hotkey.
    pub down: bool,
    /// Which modifiers were held at that moment.
    pub modifiers: Modifiers,
}

/// Whether this event fires the combination.
///
/// # Why the key must be going down
///
/// Firing on the release would run the price check when the user lets go,
/// which feels like lag, and would fire a second time for a combination whose
/// modifier is released first.
pub fn fires(event: KeyEvent, wanted_code: u16, wanted: Modifiers) -> bool {
    event.down && event.code == wanted_code && event.modifiers.matches(wanted)
}

/// Whether a code is itself a modifier key.
///
/// A modifier going down must never be treated as the combination's key.
/// Without this, a hotkey of `Ctrl` alone would fire on every Ctrl press, and
/// worse, holding Ctrl in a combination would fire on the Ctrl rather than on
/// the letter.
pub fn is_modifier_code(code: u16) -> bool {
    matches!(
        code,
        // Ctrl, Shift, Alt in their generic and sided forms, and both Windows
        // keys. The generic codes arrive from some drivers and the sided ones
        // from others, so both have to be known.
        0x10 | 0xA0 | 0xA1     // Shift, LShift, RShift
            | 0x11 | 0xA2 | 0xA3 // Ctrl, LCtrl, RCtrl
            | 0x12 | 0xA4 | 0xA5 // Alt, LAlt, RAlt
            | 0x5B | 0x5C // LWin, RWin
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: u16 = 0x44;
    const C: u16 = 0x43;

    fn ctrl() -> Modifiers {
        Modifiers {
            ctrl: true,
            ..Modifiers::default()
        }
    }

    fn down(code: u16, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            code,
            down: true,
            modifiers,
        }
    }

    #[test]
    fn the_exact_combination_fires() {
        assert!(fires(down(D, ctrl()), D, ctrl()));
    }

    #[test]
    fn a_release_never_fires() {
        // Firing on the release feels like lag and can fire twice.
        let up = KeyEvent {
            down: false,
            ..down(D, ctrl())
        };

        assert!(!fires(up, D, ctrl()));
    }

    #[test]
    fn a_different_key_does_not_fire() {
        assert!(!fires(down(C, ctrl()), D, ctrl()));
    }

    #[test]
    fn a_missing_modifier_does_not_fire() {
        // Plain D is a movement key in game. Firing on it would price an item
        // every time the user walks.
        assert!(!fires(down(D, Modifiers::default()), D, ctrl()));
    }

    #[test]
    fn an_extra_modifier_does_not_fire() {
        // Ctrl+Shift+D may be bound to something else, and a price check the
        // user did not ask for costs a request against the rate limit.
        let extra = Modifiers {
            ctrl: true,
            shift: true,
            ..Modifiers::default()
        };

        assert!(!fires(down(D, extra), D, ctrl()));
    }

    #[test]
    fn the_wrong_modifier_does_not_fire() {
        let alt = Modifiers {
            alt: true,
            ..Modifiers::default()
        };

        assert!(!fires(down(D, alt), D, ctrl()));
    }

    #[test]
    fn a_combination_with_no_modifiers_fires_on_the_bare_key() {
        // F5 alone is a legitimate binding.
        assert!(fires(
            down(0x74, Modifiers::default()),
            0x74,
            Modifiers::default()
        ));
    }

    #[test]
    fn a_combination_with_no_modifiers_does_not_fire_while_one_is_held() {
        // Ctrl+F5 is a different thing from F5 and usually means reload.
        assert!(!fires(down(0x74, ctrl()), 0x74, Modifiers::default()));
    }

    #[test]
    fn every_modifier_can_be_required_together() {
        let all = Modifiers {
            ctrl: true,
            alt: true,
            shift: true,
            meta: true,
        };

        assert!(fires(down(D, all), D, all));
    }

    // -----------------------------------------------------------------
    // Modifier codes
    // -----------------------------------------------------------------

    #[test]
    fn both_the_generic_and_sided_modifier_codes_are_known() {
        // Some drivers send the generic code and some the sided one. Knowing
        // only one set means the other half fires the hotkey on the modifier
        // itself.
        for code in [
            0x10, 0xA0, 0xA1, // shift
            0x11, 0xA2, 0xA3, // ctrl
            0x12, 0xA4, 0xA5, // alt
            0x5B, 0x5C, // windows
        ] {
            assert!(is_modifier_code(code), "{code:#04X} is not known");
        }
    }

    #[test]
    fn an_ordinary_key_is_not_a_modifier() {
        for code in [D, C, 0x74, 0x20, 0x0D] {
            assert!(!is_modifier_code(code), "{code:#04X}");
        }
    }

    #[test]
    fn matching_modifiers_is_exact_and_not_a_subset() {
        // Stated on its own because it is the rule everything above depends
        // on, and a subset test reads as correct until a user binds both.
        let ctrl_only = ctrl();
        let ctrl_shift = Modifiers {
            ctrl: true,
            shift: true,
            ..Modifiers::default()
        };

        assert!(!ctrl_shift.matches(ctrl_only));
        assert!(!ctrl_only.matches(ctrl_shift));
        assert!(ctrl_only.matches(ctrl_only));
    }
}
