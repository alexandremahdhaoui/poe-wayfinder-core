pub const DPAD_UP: u16 = 0x0001;
pub const DPAD_DOWN: u16 = 0x0002;
pub const DPAD_LEFT: u16 = 0x0004;
pub const DPAD_RIGHT: u16 = 0x0008;
pub const START: u16 = 0x0010;
pub const BACK: u16 = 0x0020;
pub const LEFT_THUMB: u16 = 0x0040;
pub const RIGHT_THUMB: u16 = 0x0080;
pub const LEFT_SHOULDER: u16 = 0x0100;
pub const RIGHT_SHOULDER: u16 = 0x0200;
pub const LEFT_TRIGGER: u16 = 0x0400;
pub const RIGHT_TRIGGER: u16 = 0x0800;
pub const A: u16 = 0x1000;
pub const B: u16 = 0x2000;
pub const X: u16 = 0x4000;
pub const Y: u16 = 0x8000;

pub const NAMED_BUTTONS: &[(&str, u16)] = &[
    ("LB", LEFT_SHOULDER),
    ("RB", RIGHT_SHOULDER),
    ("LT", LEFT_TRIGGER),
    ("RT", RIGHT_TRIGGER),
    ("L3", LEFT_THUMB),
    ("R3", RIGHT_THUMB),
    ("BACK", BACK),
    ("START", START),
    ("UP", DPAD_UP),
    ("DOWN", DPAD_DOWN),
    ("LEFT", DPAD_LEFT),
    ("RIGHT", DPAD_RIGHT),
    ("A", A),
    ("B", B),
    ("X", X),
    ("Y", Y),
];

pub const PLAYSTATION_BUTTONS: &[(&str, u16)] = &[
    ("L1", LEFT_SHOULDER),
    ("R1", RIGHT_SHOULDER),
    ("L2", LEFT_TRIGGER),
    ("R2", RIGHT_TRIGGER),
    ("L3", LEFT_THUMB),
    ("R3", RIGHT_THUMB),
    ("CREATE", BACK),
    ("SHARE", BACK),
    ("OPTIONS", START),
    ("UP", DPAD_UP),
    ("DOWN", DPAD_DOWN),
    ("LEFT", DPAD_LEFT),
    ("RIGHT", DPAD_RIGHT),
    ("CROSS", A),
    ("CIRCLE", B),
    ("SQUARE", X),
    ("TRIANGLE", Y),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PadFamily {
    #[default]
    Xbox,
    PlayStation,
}

impl PadFamily {
    pub fn buttons(self) -> &'static [(&'static str, u16)] {
        match self {
            PadFamily::Xbox => NAMED_BUTTONS,
            PadFamily::PlayStation => PLAYSTATION_BUTTONS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub mask: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    Ignore,
    Fire { binding: usize },
}

pub fn holds(state: u16, wanted: u16) -> bool {
    wanted != 0 && state & wanted == wanted
}

pub fn best_match(bindings: &[Chord], state: u16) -> Option<usize> {
    bindings
        .iter()
        .enumerate()
        .filter(|(_, chord)| holds(state, chord.mask))
        .max_by_key(|(_, chord)| chord.mask.count_ones())
        .map(|(index, _)| index)
}

pub fn parse_chord(text: &str) -> Option<Chord> {
    let mut mask = 0u16;

    for part in text.split('+') {
        let wanted = part.trim();

        if wanted.is_empty() {
            return None;
        }

        let found = NAMED_BUTTONS
            .iter()
            .chain(PLAYSTATION_BUTTONS.iter())
            .find(|(name, _)| name.eq_ignore_ascii_case(wanted))?;

        mask |= found.1;
    }

    match mask {
        0 => None,
        mask => Some(Chord { mask }),
    }
}

pub fn describe_for(family: PadFamily, mask: u16) -> String {
    let mut named: Vec<&str> = Vec::new();
    let mut printed = 0u16;

    for (name, bit) in family.buttons() {
        if mask & bit == *bit && printed & bit == 0 {
            printed |= bit;
            named.push(name);
        }
    }

    match named.is_empty() {
        true => "nothing".to_string(),
        false => named.join("+"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControllerStatus {
    pub chord: Option<Chord>,
    pub connected: bool,
    pub family: PadFamily,
}

pub fn controller_caption(status: &ControllerStatus) -> String {
    let Some(chord) = status.chord else {
        return "off. Set a chord to price check from a pad.".to_string();
    };

    let named = describe_for(status.family, chord.mask);

    match status.connected {
        true => format!("{named}, pad connected"),
        false => format!("{named}, no pad connected"),
    }
}

#[derive(Debug, Default)]
pub struct Watcher {
    bindings: Vec<Chord>,
    firing: Option<usize>,
}

impl Watcher {
    pub fn new(bindings: Vec<Chord>) -> Self {
        Self {
            bindings: bindings.into_iter().filter(|c| c.mask != 0).collect(),
            firing: None,
        }
    }


    pub fn react(&mut self, state: u16) -> Reaction {
        let Some(binding) = best_match(&self.bindings, state) else {
            self.firing = None;

            return Reaction::Ignore;
        };

        if self.firing.is_some() {
            return Reaction::Ignore;
        }

        self.firing = Some(binding);

        Reaction::Fire { binding }
    }
}

#[cfg(test)]
mod chord_tests {
    use super::*;

    #[test]
    fn a_chord_is_held_when_every_button_in_it_is_down() {
        assert!(holds(LEFT_SHOULDER | RIGHT_SHOULDER | Y, LEFT_SHOULDER | Y));
    }

    #[test]
    fn a_chord_missing_one_of_its_buttons_is_not_held() {
        assert!(!holds(LEFT_SHOULDER, LEFT_SHOULDER | Y));
    }

    #[test]
    fn an_empty_chord_is_never_held_so_a_pad_at_rest_fires_nothing() {
        assert!(!holds(0, 0));
        assert!(!holds(A, 0));
    }

    #[test]
    fn extra_buttons_beyond_the_chord_still_hold_it() {
        assert!(holds(LEFT_SHOULDER | A | B, LEFT_SHOULDER));
    }

    #[test]
    fn every_button_name_reads_back_as_the_bit_it_named() {
        for (name, bit) in NAMED_BUTTONS.iter().chain(PLAYSTATION_BUTTONS.iter()) {
            assert_eq!(parse_chord(name), Some(Chord { mask: *bit }), "{name}");
        }
    }

    #[test]
    fn a_playstation_name_and_its_xbox_name_are_the_same_chord() {
        assert_eq!(parse_chord("L1+R1+Triangle"), parse_chord("LB+RB+Y"));
        assert_eq!(parse_chord("Cross"), parse_chord("A"));
        assert_eq!(parse_chord("Circle"), parse_chord("B"));
        assert_eq!(parse_chord("Options"), parse_chord("START"));
    }

    #[test]
    fn the_xbox_x_is_where_square_is_and_not_where_the_letter_suggests() {
        assert_eq!(parse_chord("Square"), parse_chord("X"));
        assert_eq!(parse_chord("Triangle"), parse_chord("Y"));
        assert_ne!(parse_chord("Cross"), parse_chord("X"));
    }

    #[test]
    fn create_and_share_are_the_same_button_under_two_names() {
        assert_eq!(parse_chord("Create"), parse_chord("Share"));
        assert_eq!(parse_chord("Create"), parse_chord("BACK"));
    }

    #[test]
    fn the_triggers_have_a_bit_of_their_own_and_do_not_collide() {
        let every: Vec<u16> = NAMED_BUTTONS.iter().map(|(_, bit)| *bit).collect();

        assert_eq!(parse_chord("LT"), parse_chord("L2"));
        assert_eq!(parse_chord("RT"), parse_chord("R2"));
        assert_eq!(every.iter().filter(|bit| **bit == LEFT_TRIGGER).count(), 1);
        assert_eq!(every.iter().filter(|bit| **bit == RIGHT_TRIGGER).count(), 1);
    }

    #[test]
    fn button_names_are_read_whatever_case_they_are_written_in() {
        assert_eq!(parse_chord("lb+rb+y"), parse_chord("LB+RB+Y"));
    }

    #[test]
    fn spaces_around_a_button_name_are_ignored() {
        assert_eq!(parse_chord(" LB + Y "), parse_chord("LB+Y"));
    }

    #[test]
    fn a_chord_is_the_union_of_its_buttons() {
        assert_eq!(
            parse_chord("LB+RB+Y"),
            Some(Chord {
                mask: LEFT_SHOULDER | RIGHT_SHOULDER | Y
            })
        );
    }

    #[test]
    fn a_button_this_build_does_not_know_leaves_the_chord_unset() {
        assert_eq!(parse_chord("LB+GUIDE"), None);
    }

    #[test]
    fn an_empty_binding_is_off_rather_than_a_chord_that_fires_on_nothing() {
        assert_eq!(parse_chord(""), None);
        assert_eq!(parse_chord("   "), None);
        assert_eq!(parse_chord("LB+"), None);
    }

    #[test]
    fn a_chord_is_described_in_the_order_a_player_reads_it() {
        let held = LEFT_SHOULDER | RIGHT_SHOULDER | Y;

        assert_eq!(describe_for(PadFamily::Xbox, held), "LB+RB+Y");
    }

    #[test]
    fn the_same_chord_is_described_in_the_names_on_the_pad_in_hand() {
        let held = LEFT_SHOULDER | RIGHT_SHOULDER | Y;

        assert_eq!(describe_for(PadFamily::PlayStation, held), "L1+R1+TRIANGLE");
    }

    #[test]
    fn a_button_with_two_playstation_names_is_printed_once() {
        assert_eq!(describe_for(PadFamily::PlayStation, BACK), "CREATE");
    }

    #[test]
    fn a_pad_at_rest_is_described_as_nothing_rather_than_an_empty_line() {
        assert_eq!(describe_for(PadFamily::Xbox, 0), "nothing");
        assert_eq!(describe_for(PadFamily::PlayStation, 0), "nothing");
    }

    #[test]
    fn what_is_parsed_is_what_is_described() {
        let chord = parse_chord("BACK+L3").expect("a chord");

        assert_eq!(describe_for(PadFamily::Xbox, chord.mask), "L3+BACK");
    }
}

#[cfg(test)]
mod caption_tests {
    use super::*;

    #[test]
    fn no_chord_reads_as_off_and_says_how_to_turn_it_on() {
        let caption = controller_caption(&ControllerStatus::default());

        assert!(caption.starts_with("off"), "{caption}");
        assert!(caption.contains("chord"), "{caption}");
    }

    #[test]
    fn a_configured_chord_is_named_so_the_player_reads_what_fires_it() {
        let status = ControllerStatus {
            chord: parse_chord("LB+RB+Y"),
            connected: true,
            family: PadFamily::Xbox,
        };

        assert_eq!(controller_caption(&status), "LB+RB+Y, pad connected");
    }

    #[test]
    fn a_chord_with_no_pad_says_so_rather_than_looking_ready() {
        let status = ControllerStatus {
            chord: parse_chord("LB+RB+Y"),
            connected: false,
            family: PadFamily::Xbox,
        };

        assert_eq!(controller_caption(&status), "LB+RB+Y, no pad connected");
    }

    #[test]
    fn a_playstation_pad_reads_its_own_button_names_back() {
        let status = ControllerStatus {
            chord: parse_chord("LB+RB+Y"),
            connected: true,
            family: PadFamily::PlayStation,
        };

        assert_eq!(
            controller_caption(&status),
            "L1+R1+TRIANGLE, pad connected",
            "a chord written in xbox names must still read as the pad in hand"
        );
    }
}

#[cfg(test)]
mod best_match_tests {
    use super::*;

    fn bindings() -> Vec<Chord> {
        vec![
            Chord {
                mask: LEFT_SHOULDER | RIGHT_SHOULDER,
            },
            Chord {
                mask: LEFT_SHOULDER | RIGHT_SHOULDER | Y,
            },
        ]
    }

    #[test]
    fn nothing_held_matches_nothing() {
        assert_eq!(best_match(&bindings(), 0), None);
    }

    #[test]
    fn the_chord_with_the_most_buttons_wins_so_a_superset_does_not_fire_its_subset() {
        let held = LEFT_SHOULDER | RIGHT_SHOULDER | Y;

        assert_eq!(best_match(&bindings(), held), Some(1));
    }

    #[test]
    fn the_shorter_chord_still_matches_when_the_longer_one_is_not_held() {
        let held = LEFT_SHOULDER | RIGHT_SHOULDER;

        assert_eq!(best_match(&bindings(), held), Some(0));
    }

    #[test]
    fn a_binding_nobody_configured_matches_nothing() {
        assert_eq!(best_match(&[], A | B), None);
    }
}

#[cfg(test)]
mod watcher_tests {
    use super::*;

    const CHECK: u16 = LEFT_SHOULDER | RIGHT_SHOULDER | Y;

    fn watcher() -> Watcher {
        Watcher::new(vec![
            Chord { mask: CHECK },
            Chord {
                mask: LEFT_SHOULDER | RIGHT_SHOULDER,
            },
        ])
    }

    #[test]
    fn holding_the_chord_fires_the_binding_it_belongs_to() {
        let mut w = watcher();

        assert_eq!(w.react(CHECK), Reaction::Fire { binding: 0 });
    }

    #[test]
    fn holding_the_chord_for_many_polls_fires_once_and_not_every_poll() {
        let mut w = watcher();

        w.react(CHECK);

        for _ in 0..20 {
            assert_eq!(w.react(CHECK), Reaction::Ignore);
        }
    }

    #[test]
    fn letting_go_and_pressing_again_fires_again() {
        let mut w = watcher();

        w.react(CHECK);
        w.react(0);

        assert_eq!(w.react(CHECK), Reaction::Fire { binding: 0 });
    }

    #[test]
    fn rolling_off_one_button_of_a_held_chord_does_not_fire_the_shorter_chord_underneath() {
        let mut w = watcher();

        w.react(CHECK);

        assert_eq!(
            w.react(LEFT_SHOULDER | RIGHT_SHOULDER),
            Reaction::Ignore,
            "a player releasing Y before the shoulders would price the item twice"
        );
    }

    #[test]
    fn a_pad_at_rest_fires_nothing() {
        let mut w = watcher();

        assert_eq!(w.react(0), Reaction::Ignore);
    }

    #[test]
    fn a_button_outside_every_chord_fires_nothing() {
        let mut w = watcher();

        assert_eq!(w.react(A), Reaction::Ignore);
        assert_eq!(w.react(DPAD_UP | B), Reaction::Ignore);
    }

    #[test]
    fn an_empty_chord_is_refused_rather_than_firing_on_every_poll() {
        let mut w = Watcher::new(vec![Chord { mask: 0 }]);

        assert_eq!(w.react(0), Reaction::Ignore);
        assert_eq!(w.react(A), Reaction::Ignore, "an empty chord fires never");
    }

    #[test]
    fn watching_nothing_leaves_every_button_alone() {
        let mut w = Watcher::new(Vec::new());

        assert_eq!(w.react(CHECK), Reaction::Ignore);
    }

    #[test]
    fn each_chord_is_reported_by_its_own_index() {
        let mut w = watcher();

        assert_eq!(
            w.react(LEFT_SHOULDER | RIGHT_SHOULDER),
            Reaction::Fire { binding: 1 }
        );
    }

    #[test]
    fn a_pad_that_disconnects_mid_chord_leaves_the_watcher_ready_for_the_next_press() {
        let mut w = watcher();

        w.react(CHECK);
        w.react(0);

        assert_eq!(w.react(CHECK), Reaction::Fire { binding: 0 });
    }
}
