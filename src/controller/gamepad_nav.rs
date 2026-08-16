use crate::controller::gamepad_match as pad;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    FocusNext,
    FocusPrevious,
    Activate,
    Increase,
    Decrease,
    Close,
}

pub const STICK_THRESHOLD: f32 = 0.5;

pub fn stick_direction(x: f32, y: f32) -> u16 {
    if !x.is_finite() || !y.is_finite() {
        return 0;
    }

    if x.abs() < STICK_THRESHOLD && y.abs() < STICK_THRESHOLD {
        return 0;
    }

    match x.abs() >= y.abs() {
        true if x > 0.0 => pad::DPAD_RIGHT,
        true => pad::DPAD_LEFT,
        false if y > 0.0 => pad::DPAD_DOWN,
        false => pad::DPAD_UP,
    }
}

pub fn newly_pressed(before: u16, now: u16) -> u16 {
    now & !before
}

pub fn actions(pressed: u16) -> Vec<Nav> {
    let mut out = Vec::new();

    for (bit, action) in [
        (pad::DPAD_DOWN, Nav::FocusNext),
        (pad::DPAD_UP, Nav::FocusPrevious),
        (pad::DPAD_RIGHT, Nav::Increase),
        (pad::DPAD_LEFT, Nav::Decrease),
        (pad::A, Nav::Activate),
        (pad::B, Nav::Close),
    ] {
        if pressed & bit == bit {
            out.push(action);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_button_held_since_the_last_poll_is_not_pressed_again() {
        assert_eq!(newly_pressed(pad::A, pad::A), 0);
    }

    #[test]
    fn a_button_that_was_not_held_before_is_a_press() {
        assert_eq!(newly_pressed(0, pad::A), pad::A);
    }

    #[test]
    fn releasing_a_button_is_not_a_press() {
        assert_eq!(newly_pressed(pad::A, 0), 0);
    }

    #[test]
    fn one_button_going_down_beside_a_held_one_is_the_only_press() {
        assert_eq!(newly_pressed(pad::A, pad::A | pad::B), pad::B);
    }

    #[test]
    fn a_stick_at_rest_moves_nothing_so_a_drifting_pad_does_not_scroll_forever() {
        assert_eq!(stick_direction(0.0, 0.0), 0);
        assert_eq!(stick_direction(0.2, -0.3), 0);
    }

    #[test]
    fn a_stick_pushed_reads_as_the_direction_it_is_pushed() {
        assert_eq!(stick_direction(0.0, 0.9), pad::DPAD_DOWN);
        assert_eq!(stick_direction(0.0, -0.9), pad::DPAD_UP);
        assert_eq!(stick_direction(0.9, 0.0), pad::DPAD_RIGHT);
        assert_eq!(stick_direction(-0.9, 0.0), pad::DPAD_LEFT);
    }

    #[test]
    fn a_stick_pushed_at_an_angle_picks_one_direction_rather_than_both() {
        assert_eq!(stick_direction(0.9, 0.6), pad::DPAD_RIGHT);
        assert_eq!(stick_direction(0.6, 0.9), pad::DPAD_DOWN);
    }

    #[test]
    fn a_stick_reading_that_is_not_a_number_moves_nothing() {
        assert_eq!(stick_direction(f32::NAN, f32::NAN), 0);
        assert_eq!(stick_direction(f32::INFINITY, 0.0), 0);
    }

    #[test]
    fn a_stick_direction_navigates_exactly_as_the_dpad_does() {
        assert_eq!(
            actions(stick_direction(0.0, 0.9)),
            actions(pad::DPAD_DOWN),
            "the stick and the dpad must not need two code paths"
        );
    }

    #[test]
    fn the_dpad_moves_the_focus_the_way_it_points() {
        assert_eq!(actions(pad::DPAD_DOWN), vec![Nav::FocusNext]);
        assert_eq!(actions(pad::DPAD_UP), vec![Nav::FocusPrevious]);
    }

    #[test]
    fn left_and_right_change_the_row_rather_than_the_focus() {
        assert_eq!(actions(pad::DPAD_RIGHT), vec![Nav::Increase]);
        assert_eq!(actions(pad::DPAD_LEFT), vec![Nav::Decrease]);
    }

    #[test]
    fn cross_presses_what_the_focus_is_on() {
        assert_eq!(actions(pad::A), vec![Nav::Activate]);
    }

    #[test]
    fn circle_closes_the_panel_because_that_is_what_it_does_everywhere_else() {
        assert_eq!(actions(pad::B), vec![Nav::Close]);
    }

    #[test]
    fn a_pad_at_rest_asks_for_nothing() {
        assert!(actions(0).is_empty());
    }

    #[test]
    fn a_button_with_no_meaning_in_the_panel_is_ignored() {
        assert!(actions(pad::Y).is_empty());
        assert!(actions(pad::START).is_empty());
        assert!(actions(pad::LEFT_SHOULDER | pad::RIGHT_SHOULDER).is_empty());
    }

    #[test]
    fn two_buttons_at_once_ask_for_both_in_a_stable_order() {
        assert_eq!(
            actions(pad::A | pad::DPAD_DOWN),
            vec![Nav::FocusNext, Nav::Activate]
        );
    }

    #[test]
    fn the_chord_that_opens_the_panel_does_not_also_navigate_it() {
        let chord = pad::parse_chord("L1+R1+Triangle").expect("a chord").mask;

        assert!(
            actions(chord).is_empty(),
            "opening and closing must not move the focus on the way through"
        );
    }
}
