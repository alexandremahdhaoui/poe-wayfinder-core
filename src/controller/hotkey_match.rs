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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    pub code: u16,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    Ignore,
    Fire { binding: usize },
    Swallow,
}

impl Reaction {
    pub fn eats_the_key(self) -> bool {
        !matches!(self, Reaction::Ignore)
    }
}

#[derive(Debug, Default)]
pub struct Watcher {
    bindings: Vec<Binding>,
    swallowed: Option<u16>,
}

impl Watcher {
    pub fn new(bindings: Vec<Binding>) -> Self {
        Self {
            bindings: bindings
                .into_iter()
                .filter(|b| !is_modifier_code(b.code))
                .collect(),
            swallowed: None,
        }
    }

    pub fn react(&mut self, event: KeyEvent) -> Reaction {
        if is_modifier_code(event.code) {
            return Reaction::Ignore;
        }

        if event.down {
            let hit = self
                .bindings
                .iter()
                .position(|b| fires(event, b.code, b.modifiers));

            return match hit {
                Some(binding) => {
                    self.swallowed = Some(event.code);

                    Reaction::Fire { binding }
                }
                None => Reaction::Ignore,
            };
        }

        match self.swallowed == Some(event.code) {
            true => {
                self.swallowed = None;

                Reaction::Swallow
            }
            false => Reaction::Ignore,
        }
    }
}

#[cfg(test)]
mod watcher_tests {
    use super::*;

    const D: u16 = 0x44;
    const F: u16 = 0x46;
    const CTRL_CODE: u16 = 0xA2;

    fn mods(ctrl: bool, alt: bool, shift: bool) -> Modifiers {
        Modifiers {
            ctrl,
            alt,
            shift,
            meta: false,
        }
    }

    fn press(code: u16, m: Modifiers) -> KeyEvent {
        KeyEvent {
            code,
            down: true,
            modifiers: m,
        }
    }

    fn release(code: u16, m: Modifiers) -> KeyEvent {
        KeyEvent {
            code,
            down: false,
            modifiers: m,
        }
    }

    fn watcher() -> Watcher {
        Watcher::new(vec![
            Binding {
                code: D,
                modifiers: mods(true, false, false),
            },
            Binding {
                code: D,
                modifiers: mods(true, true, false),
            },
        ])
    }

    #[test]
    fn the_matching_press_fires_and_is_eaten_so_the_game_never_sees_it() {
        let mut w = watcher();

        let got = w.react(press(D, mods(true, false, false)));

        assert_eq!(got, Reaction::Fire { binding: 0 });
        assert!(got.eats_the_key(), "the game must not receive the key");
    }

    #[test]
    fn each_binding_is_reported_by_its_own_index() {
        let mut w = watcher();

        assert_eq!(
            w.react(press(D, mods(true, true, false))),
            Reaction::Fire { binding: 1 }
        );
    }

    #[test]
    fn the_release_of_a_swallowed_key_is_swallowed_too() {
        let mut w = watcher();

        w.react(press(D, mods(true, false, false)));

        assert_eq!(
            w.react(release(D, mods(true, false, false))),
            Reaction::Swallow
        );
    }

    #[test]
    fn a_release_with_no_swallowed_press_reaches_the_game() {
        let mut w = watcher();

        assert_eq!(
            w.react(release(D, mods(true, false, false))),
            Reaction::Ignore
        );
    }

    #[test]
    fn the_release_is_only_swallowed_once() {
        let mut w = watcher();

        w.react(press(D, mods(true, false, false)));
        w.react(release(D, mods(true, false, false)));

        assert_eq!(
            w.react(release(D, mods(true, false, false))),
            Reaction::Ignore,
            "a second release belongs to the game"
        );
    }

    #[test]
    fn eating_the_key_is_what_stops_the_character_walking_left_on_ctrl_d() {
        let mut w = watcher();

        assert!(
            w.react(press(D, mods(true, false, false))).eats_the_key(),
            "RegisterHotKey loses to whatever already owns the combination, so \
             this hook is the only thing that can stop the game seeing the key"
        );
    }

    #[test]
    fn the_bare_key_still_reaches_the_game() {
        let mut w = watcher();

        let got = w.react(press(D, Modifiers::default()));

        assert_eq!(got, Reaction::Ignore);
        assert!(!got.eats_the_key(), "walking with D must keep working");
    }

    #[test]
    fn a_key_nobody_asked_for_reaches_the_game() {
        let mut w = watcher();

        assert_eq!(
            w.react(press(F, mods(true, false, false))),
            Reaction::Ignore
        );
    }

    #[test]
    fn a_modifier_is_never_swallowed() {
        let mut w = watcher();

        assert_eq!(
            w.react(press(CTRL_CODE, mods(true, false, false))),
            Reaction::Ignore
        );
        assert_eq!(
            w.react(release(CTRL_CODE, Modifiers::default())),
            Reaction::Ignore
        );
    }

    #[test]
    fn a_binding_on_a_modifier_is_refused_rather_than_eating_every_ctrl() {
        let mut w = Watcher::new(vec![Binding {
            code: CTRL_CODE,
            modifiers: Modifiers::default(),
        }]);

        assert_eq!(
            w.react(press(CTRL_CODE, mods(true, false, false))),
            Reaction::Ignore,
            "a bare modifier binding would swallow every Ctrl the game needs"
        );
    }

    #[test]
    fn watching_nothing_leaves_every_key_alone() {
        let mut w = Watcher::new(Vec::new());

        assert_eq!(
            w.react(press(D, mods(true, false, false))),
            Reaction::Ignore
        );
    }
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
