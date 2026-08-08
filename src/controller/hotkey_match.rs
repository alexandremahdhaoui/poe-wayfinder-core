#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl Modifiers {
    pub fn matches(self, wanted: Modifiers) -> bool {
        self == wanted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: u16,
    pub down: bool,
    pub modifiers: Modifiers,
}

pub fn fires(event: KeyEvent, wanted_code: u16, wanted: Modifiers) -> bool {
    event.down && event.code == wanted_code && event.modifiers.matches(wanted)
}

pub fn is_modifier_code(code: u16) -> bool {
    matches!(
        code,
        0x10 | 0xA0 | 0xA1 | 0x11 | 0xA2 | 0xA3 | 0x12 | 0xA4 | 0xA5 | 0x5B | 0x5C
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
        assert!(!fires(down(D, Modifiers::default()), D, ctrl()));
    }

    #[test]
    fn an_extra_modifier_does_not_fire() {
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
        assert!(fires(
            down(0x74, Modifiers::default()),
            0x74,
            Modifiers::default()
        ));
    }

    #[test]
    fn a_combination_with_no_modifiers_does_not_fire_while_one_is_held() {
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

    #[test]
    fn both_the_generic_and_sided_modifier_codes_are_known() {
        for code in [
            0x10, 0xA0, 0xA1, 0x11, 0xA2, 0xA3, 0x12, 0xA4, 0xA5, 0x5B, 0x5C,
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
