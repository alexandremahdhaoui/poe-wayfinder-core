use std::time::Duration;

use crate::controller::gamepad_match as pad;

pub const FIRST_REPEAT: Duration = Duration::from_millis(400);
pub const THEN_EVERY: Duration = Duration::from_millis(110);

pub const DIRECTIONS: u16 = pad::DPAD_UP | pad::DPAD_DOWN | pad::DPAD_LEFT | pad::DPAD_RIGHT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Column {
    #[default]
    Line,
    Min,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Focus {
    pub row: usize,
    pub column: Column,
    pub editing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadEdit {
    Toggle,
    AdjustMin(f64),
    AdjustMax(f64),
    Search,
    Close,
}

#[derive(Debug, Default)]
pub struct AutoRepeat {
    held: u16,
    waited: Duration,
    fired_once: bool,
}

impl AutoRepeat {
    pub fn tick(&mut self, held: u16, since_last: Duration) -> u16 {
        let held = held & DIRECTIONS;

        if held != self.held {
            self.held = held;
            self.waited = Duration::ZERO;
            self.fired_once = false;

            return 0;
        }

        if held == 0 {
            return 0;
        }

        self.waited += since_last;

        let due = match self.fired_once {
            true => THEN_EVERY,
            false => FIRST_REPEAT,
        };

        if self.waited < due {
            return 0;
        }

        self.waited = Duration::ZERO;
        self.fired_once = true;

        held
    }
}

pub fn react(focus: &mut Focus, pressed: u16, rows: usize) -> Vec<PadEdit> {
    if rows == 0 {
        return Vec::new();
    }

    focus.row = focus.row.min(rows - 1);

    match focus.editing {
        true => edit(focus, pressed),
        false => walk(focus, pressed, rows),
    }
}

fn walk(focus: &mut Focus, pressed: u16, rows: usize) -> Vec<PadEdit> {
    let mut out = Vec::new();

    if pressed & pad::DPAD_DOWN != 0 {
        focus.row = (focus.row + 1).min(rows - 1);
        focus.column = Column::Line;
    }

    if pressed & pad::DPAD_UP != 0 {
        focus.row = focus.row.saturating_sub(1);
        focus.column = Column::Line;
    }

    if pressed & pad::DPAD_RIGHT != 0 {
        focus.column = match focus.column {
            Column::Line => Column::Min,
            Column::Min | Column::Max => Column::Max,
        };
    }

    if pressed & pad::DPAD_LEFT != 0 {
        focus.column = match focus.column {
            Column::Max => Column::Min,
            Column::Min | Column::Line => Column::Line,
        };
    }

    if pressed & pad::A != 0 {
        match focus.column {
            Column::Line => out.push(PadEdit::Toggle),
            Column::Min | Column::Max => focus.editing = true,
        }
    }

    if pressed & pad::B != 0 {
        out.push(PadEdit::Close);
    }

    if pressed & pad::RIGHT_SHOULDER != 0 {
        out.push(PadEdit::Search);
    }

    out
}

fn edit(focus: &mut Focus, pressed: u16) -> Vec<PadEdit> {
    if pressed & (pad::A | pad::B) != 0 {
        focus.editing = false;

        return Vec::new();
    }

    let steps = [
        (pad::DPAD_RIGHT, 1.0),
        (pad::DPAD_LEFT, -1.0),
        (pad::DPAD_UP, 10.0),
        (pad::DPAD_DOWN, -10.0),
        (pad::RIGHT_TRIGGER, 100.0),
        (pad::LEFT_TRIGGER, -100.0),
    ];

    let mut out = Vec::new();

    for (bit, delta) in steps {
        if pressed & bit == 0 {
            continue;
        }

        out.push(match focus.column {
            Column::Max => PadEdit::AdjustMax(delta),
            Column::Min | Column::Line => PadEdit::AdjustMin(delta),
        });
    }

    out
}

pub fn hints(editing: bool) -> &'static str {
    match editing {
        true => "L/R  1     U/D  10     L2/R2  100     Cross or Circle  done",
        false => "D-pad or stick  move     Cross  toggle or edit     R1  search     Circle  close",
    }
}

#[cfg(test)]
mod walk_tests {
    use super::*;

    fn at(row: usize, column: Column) -> Focus {
        Focus {
            row,
            column,
            editing: false,
        }
    }

    #[test]
    fn down_and_up_walk_the_lines() {
        let mut focus = Focus::default();

        react(&mut focus, pad::DPAD_DOWN, 4);
        assert_eq!(focus.row, 1);

        react(&mut focus, pad::DPAD_DOWN, 4);
        assert_eq!(focus.row, 2);

        react(&mut focus, pad::DPAD_UP, 4);
        assert_eq!(focus.row, 1);
    }

    #[test]
    fn walking_stops_at_the_ends_rather_than_wrapping() {
        let mut focus = Focus::default();

        react(&mut focus, pad::DPAD_UP, 3);
        assert_eq!(focus.row, 0, "up from the first line stays");

        focus.row = 2;
        react(&mut focus, pad::DPAD_DOWN, 3);
        assert_eq!(focus.row, 2, "down from the last line stays");
    }

    #[test]
    fn walking_down_never_lands_on_a_value_box() {
        let mut focus = at(0, Column::Max);

        react(&mut focus, pad::DPAD_DOWN, 3);

        assert_eq!(focus.column, Column::Line, "a new line starts at the line");
    }

    #[test]
    fn right_reaches_the_min_then_the_max_and_stops() {
        let mut focus = Focus::default();

        react(&mut focus, pad::DPAD_RIGHT, 2);
        assert_eq!(focus.column, Column::Min);

        react(&mut focus, pad::DPAD_RIGHT, 2);
        assert_eq!(focus.column, Column::Max);

        react(&mut focus, pad::DPAD_RIGHT, 2);
        assert_eq!(focus.column, Column::Max);
    }

    #[test]
    fn left_walks_back_to_the_line() {
        let mut focus = at(0, Column::Max);

        react(&mut focus, pad::DPAD_LEFT, 2);
        assert_eq!(focus.column, Column::Min);

        react(&mut focus, pad::DPAD_LEFT, 2);
        assert_eq!(focus.column, Column::Line);

        react(&mut focus, pad::DPAD_LEFT, 2);
        assert_eq!(focus.column, Column::Line);
    }

    #[test]
    fn cross_on_a_line_switches_the_filter_rather_than_editing_it() {
        let mut focus = Focus::default();

        assert_eq!(react(&mut focus, pad::A, 2), vec![PadEdit::Toggle]);
        assert!(!focus.editing);
    }

    #[test]
    fn cross_on_a_value_starts_editing_it_and_changes_nothing_yet() {
        let mut focus = at(1, Column::Min);

        assert!(react(&mut focus, pad::A, 3).is_empty());
        assert!(focus.editing);
    }

    #[test]
    fn r1_searches_from_anywhere() {
        let mut focus = at(2, Column::Max);

        assert_eq!(
            react(&mut focus, pad::RIGHT_SHOULDER, 4),
            vec![PadEdit::Search]
        );
    }

    #[test]
    fn circle_closes_from_anywhere_that_is_not_editing() {
        let mut focus = at(1, Column::Min);

        assert_eq!(react(&mut focus, pad::B, 3), vec![PadEdit::Close]);
    }

    #[test]
    fn a_panel_with_no_rows_does_nothing_at_all() {
        let mut focus = Focus::default();

        assert!(react(&mut focus, pad::DPAD_DOWN, 0).is_empty());
        assert!(react(&mut focus, pad::A, 0).is_empty());
    }

    #[test]
    fn a_focus_past_the_end_is_pulled_back_when_the_rows_shrink() {
        let mut focus = at(9, Column::Line);

        react(&mut focus, 0, 3);

        assert_eq!(focus.row, 2);
    }
}

#[cfg(test)]
mod edit_tests {
    use super::*;

    fn editing(column: Column) -> Focus {
        Focus {
            row: 0,
            column,
            editing: true,
        }
    }

    #[test]
    fn left_and_right_step_the_value_by_one() {
        let mut focus = editing(Column::Min);

        assert_eq!(
            react(&mut focus, pad::DPAD_RIGHT, 2),
            vec![PadEdit::AdjustMin(1.0)]
        );
        assert_eq!(
            react(&mut focus, pad::DPAD_LEFT, 2),
            vec![PadEdit::AdjustMin(-1.0)]
        );
    }

    #[test]
    fn up_and_down_step_the_value_by_ten() {
        let mut focus = editing(Column::Min);

        assert_eq!(
            react(&mut focus, pad::DPAD_UP, 2),
            vec![PadEdit::AdjustMin(10.0)]
        );
        assert_eq!(
            react(&mut focus, pad::DPAD_DOWN, 2),
            vec![PadEdit::AdjustMin(-10.0)]
        );
    }

    #[test]
    fn the_triggers_step_the_value_by_a_hundred() {
        let mut focus = editing(Column::Min);

        assert_eq!(
            react(&mut focus, pad::RIGHT_TRIGGER, 2),
            vec![PadEdit::AdjustMin(100.0)]
        );
        assert_eq!(
            react(&mut focus, pad::LEFT_TRIGGER, 2),
            vec![PadEdit::AdjustMin(-100.0)]
        );
    }

    #[test]
    fn the_max_box_is_edited_when_the_max_box_is_the_one_selected() {
        let mut focus = editing(Column::Max);

        assert_eq!(
            react(&mut focus, pad::DPAD_RIGHT, 2),
            vec![PadEdit::AdjustMax(1.0)]
        );
    }

    #[test]
    fn editing_does_not_walk_to_another_line() {
        let mut focus = editing(Column::Min);

        react(&mut focus, pad::DPAD_DOWN, 4);

        assert_eq!(focus.row, 0, "down changes the value, not the line");
    }

    #[test]
    fn cross_stops_editing_and_keeps_what_was_set() {
        let mut focus = editing(Column::Min);

        assert!(react(&mut focus, pad::A, 2).is_empty());
        assert!(!focus.editing);
    }

    #[test]
    fn circle_stops_editing_rather_than_closing_the_panel() {
        let mut focus = editing(Column::Min);

        assert_eq!(
            react(&mut focus, pad::B, 2),
            Vec::new(),
            "circle out of a value box must not also shut the panel"
        );
        assert!(!focus.editing);
    }

    #[test]
    fn r1_does_not_search_while_a_value_is_being_edited() {
        let mut focus = editing(Column::Min);

        assert!(react(&mut focus, pad::RIGHT_SHOULDER, 2).is_empty());
    }
}

#[cfg(test)]
mod repeat_tests {
    use super::*;

    const FRAME: Duration = Duration::from_millis(100);

    #[test]
    fn a_direction_just_pressed_does_not_repeat_yet() {
        let mut repeat = AutoRepeat::default();

        assert_eq!(repeat.tick(pad::DPAD_DOWN, FRAME), 0);
    }

    #[test]
    fn holding_a_direction_repeats_after_the_first_wait() {
        let mut repeat = AutoRepeat::default();

        repeat.tick(pad::DPAD_DOWN, FRAME);

        let mut fired = 0;

        for _ in 0..5 {
            if repeat.tick(pad::DPAD_DOWN, FRAME) != 0 {
                fired += 1;
            }
        }

        assert!(fired >= 1, "a held direction must keep moving");
    }

    #[test]
    fn the_repeat_speeds_up_after_the_first_one() {
        let mut repeat = AutoRepeat::default();

        repeat.tick(pad::DPAD_DOWN, FIRST_REPEAT);
        assert_ne!(repeat.tick(pad::DPAD_DOWN, FIRST_REPEAT), 0);

        assert_ne!(
            repeat.tick(pad::DPAD_DOWN, THEN_EVERY),
            0,
            "the second wait is the short one"
        );
    }

    #[test]
    fn letting_go_stops_the_repeat() {
        let mut repeat = AutoRepeat::default();

        repeat.tick(pad::DPAD_DOWN, FIRST_REPEAT);
        repeat.tick(pad::DPAD_DOWN, FIRST_REPEAT);

        assert_eq!(repeat.tick(0, FRAME), 0);
        assert_eq!(repeat.tick(0, FRAME), 0);
    }

    #[test]
    fn changing_direction_starts_the_wait_again() {
        let mut repeat = AutoRepeat::default();

        repeat.tick(pad::DPAD_DOWN, FIRST_REPEAT);
        repeat.tick(pad::DPAD_DOWN, FIRST_REPEAT);

        assert_eq!(repeat.tick(pad::DPAD_UP, FRAME), 0);
    }

    #[test]
    fn a_button_that_is_not_a_direction_never_repeats() {
        let mut repeat = AutoRepeat::default();

        repeat.tick(pad::A, FIRST_REPEAT);

        assert_eq!(repeat.tick(pad::A, FIRST_REPEAT), 0);
    }

    #[test]
    fn every_hint_names_the_buttons_it_is_about() {
        assert!(hints(false).contains("R1"));
        assert!(hints(false).contains("Cross"));
        assert!(hints(true).contains("L2/R2"));
    }
}
