use crate::controller::gamepad_match::{describe_for, parse_chord, PadFamily};
use crate::controller::overlay::event_to_hotkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Keyboard,
    Pad,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub row: Row,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capture {
    listening: Option<Row>,
}

impl Capture {
    pub fn listen(&mut self, row: Row) {
        self.listening = Some(row);
    }

    pub fn stop(&mut self) {
        self.listening = None;
    }

    pub fn listening_to(&self) -> Option<Row> {
        self.listening
    }

    pub fn from_key(&mut self, key: &str, ctrl: bool, shift: bool, alt: bool) -> Option<Binding> {
        if self.listening != Some(Row::Keyboard) {
            return None;
        }

        let text = event_to_hotkey(key, ctrl, shift, alt);

        if matches!(text.as_str(), "Ctrl" | "Shift" | "Alt") {
            return None;
        }

        self.listening = None;

        Some(Binding {
            row: Row::Keyboard,
            text,
        })
    }

    pub fn from_pad(&mut self, held: u16, family: PadFamily) -> Option<Binding> {
        if self.listening != Some(Row::Pad) || held == 0 {
            return None;
        }

        self.listening = None;

        Some(Binding {
            row: Row::Pad,
            text: describe_for(family, held),
        })
    }
}

pub fn shown(row: Row, text: &str, family: PadFamily) -> String {
    match row {
        Row::Keyboard => text.to_string(),
        Row::Pad => match parse_chord(text) {
            Some(chord) => describe_for(family, chord.mask),
            None => "not set".to_string(),
        },
    }
}

pub fn caption(row: Row) -> &'static str {
    match row {
        Row::Keyboard => "Keyboard",
        Row::Pad => "Pad",
    }
}

pub fn prompt(listening: Option<Row>, row: Row) -> &'static str {
    match listening == Some(row) {
        true => "press it now",
        false => "press to bind",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::controller::gamepad_match as pad;

    #[test]
    fn nothing_is_captured_until_a_row_is_listening() {
        let mut capture = Capture::default();

        assert_eq!(capture.from_key("D", true, false, false), None);
        assert_eq!(capture.from_pad(pad::A, PadFamily::Xbox), None);
    }

    #[test]
    fn a_key_press_becomes_the_keyboard_binding() {
        let mut capture = Capture::default();

        capture.listen(Row::Keyboard);

        let got = capture.from_key("D", true, false, false).expect("a binding");

        assert_eq!(got.row, Row::Keyboard);
        assert_eq!(got.text, "Ctrl + D");
    }

    #[test]
    fn a_bare_modifier_is_not_a_binding_so_the_capture_keeps_listening() {
        let mut capture = Capture::default();

        capture.listen(Row::Keyboard);

        assert_eq!(capture.from_key("Ctrl", true, false, false), None);
        assert_eq!(capture.listening_to(), Some(Row::Keyboard));
    }

    #[test]
    fn a_pad_chord_becomes_the_pad_binding_in_the_names_of_that_pad() {
        let mut capture = Capture::default();
        let held = pad::LEFT_SHOULDER | pad::RIGHT_SHOULDER | pad::Y;

        capture.listen(Row::Pad);

        let got = capture
            .from_pad(held, PadFamily::PlayStation)
            .expect("a binding");

        assert_eq!(got.row, Row::Pad);
        assert_eq!(got.text, "L1+R1+TRIANGLE");
    }

    #[test]
    fn a_pad_at_rest_is_not_a_binding() {
        let mut capture = Capture::default();

        capture.listen(Row::Pad);

        assert_eq!(capture.from_pad(0, PadFamily::Xbox), None);
        assert_eq!(capture.listening_to(), Some(Row::Pad));
    }

    #[test]
    fn one_row_listening_never_captures_the_other_kind() {
        let mut capture = Capture::default();

        capture.listen(Row::Keyboard);

        assert_eq!(capture.from_pad(pad::A, PadFamily::Xbox), None);

        capture.listen(Row::Pad);

        assert_eq!(capture.from_key("D", false, false, false), None);
    }

    #[test]
    fn capturing_stops_listening_so_the_next_press_reaches_the_game() {
        let mut capture = Capture::default();

        capture.listen(Row::Keyboard);
        capture.from_key("D", true, false, false);

        assert_eq!(capture.listening_to(), None);
    }

    #[test]
    fn a_capture_can_be_given_up_on() {
        let mut capture = Capture::default();

        capture.listen(Row::Pad);
        capture.stop();

        assert_eq!(capture.listening_to(), None);
    }

    #[test]
    fn the_same_pad_binding_reads_in_the_names_of_the_pad_in_hand() {
        assert_eq!(shown(Row::Pad, "LB+RB+Y", PadFamily::Xbox), "LB+RB+Y");
        assert_eq!(
            shown(Row::Pad, "LB+RB+Y", PadFamily::PlayStation),
            "L1+R1+TRIANGLE"
        );
    }

    #[test]
    fn a_keyboard_binding_is_shown_as_written() {
        assert_eq!(
            shown(Row::Keyboard, "Ctrl+D", PadFamily::PlayStation),
            "Ctrl+D"
        );
    }

    #[test]
    fn a_pad_binding_nobody_set_says_so_rather_than_showing_nothing() {
        assert_eq!(shown(Row::Pad, "", PadFamily::Xbox), "not set");
    }

    #[test]
    fn the_prompt_says_which_row_is_waiting() {
        assert_eq!(prompt(Some(Row::Pad), Row::Pad), "press it now");
        assert_eq!(prompt(Some(Row::Pad), Row::Keyboard), "press to bind");
        assert_eq!(prompt(None, Row::Keyboard), "press to bind");
    }

    #[test]
    fn every_row_has_a_caption() {
        for row in [Row::Keyboard, Row::Pad] {
            assert!(!caption(row).is_empty());
        }
    }
}
