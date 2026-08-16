use crate::controller::gamepad_match::parse_chord;

pub const PRESS_POLLS: usize = 2;
pub const RELEASE_POLLS: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    polls: Vec<u16>,
}

impl Script {
    pub fn at(&self, poll: usize) -> Option<u16> {
        self.polls.get(poll).copied()
    }

    pub fn polls(&self) -> usize {
        self.polls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.polls.is_empty()
    }
}

pub fn parse(text: &str) -> Result<Script, String> {
    let mut polls = Vec::new();

    for (number, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();

        if line.is_empty() {
            continue;
        }

        let mut words = line.split_whitespace();
        let verb = words.next().unwrap_or("");

        match verb {
            "wait" => {
                let count = number_of(words.next(), number)?;

                polls.extend(std::iter::repeat_n(0, count));
            }
            "press" => {
                let mask = chord_of(words.next(), number)?;

                polls.extend(std::iter::repeat_n(mask, PRESS_POLLS));
                polls.extend(std::iter::repeat_n(0, RELEASE_POLLS));
            }
            "hold" => {
                let mask = chord_of(words.next(), number)?;
                let count = number_of(words.next(), number)?;

                polls.extend(std::iter::repeat_n(mask, count));
                polls.extend(std::iter::repeat_n(0, RELEASE_POLLS));
            }
            other => {
                return Err(format!(
                    "line {}: {other:?} is not wait, press or hold",
                    number + 1
                ))
            }
        }
    }

    Ok(Script { polls })
}

fn chord_of(word: Option<&str>, line: usize) -> Result<u16, String> {
    let Some(word) = word else {
        return Err(format!("line {}: no buttons named", line + 1));
    };

    parse_chord(word).map(|chord| chord.mask).ok_or_else(|| {
        format!(
            "line {}: {word:?} names no button this build knows",
            line + 1
        )
    })
}

fn number_of(word: Option<&str>, line: usize) -> Result<usize, String> {
    let Some(word) = word else {
        return Err(format!("line {}: no number of polls given", line + 1));
    };

    word.parse()
        .map_err(|_| format!("line {}: {word:?} is not a number of polls", line + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::controller::gamepad_match as pad;

    #[test]
    fn a_press_is_held_for_a_few_polls_then_let_go() {
        let script = parse("press CROSS").expect("a script");

        assert_eq!(script.at(0), Some(pad::A));
        assert_eq!(script.at(PRESS_POLLS - 1), Some(pad::A));
        assert_eq!(script.at(PRESS_POLLS), Some(0));
    }

    #[test]
    fn a_press_is_let_go_so_the_next_one_is_a_new_press() {
        let script = parse("press CROSS\npress CROSS").expect("a script");
        let mut edges = 0;
        let mut before = 0;

        for poll in 0..script.polls() {
            let now = script.at(poll).expect("a poll");

            if now != 0 && before == 0 {
                edges += 1;
            }

            before = now;
        }

        assert_eq!(edges, 2, "two presses must read as two presses");
    }

    #[test]
    fn a_hold_lasts_as_long_as_it_says() {
        let script = parse("hold DOWN 7").expect("a script");

        for poll in 0..7 {
            assert_eq!(script.at(poll), Some(pad::DPAD_DOWN), "poll {poll}");
        }

        assert_eq!(script.at(7), Some(0));
    }

    #[test]
    fn waiting_reads_as_a_pad_at_rest() {
        let script = parse("wait 3").expect("a script");

        assert_eq!(script.polls(), 3);
        assert!((0..3).all(|poll| script.at(poll) == Some(0)));
    }

    #[test]
    fn a_chord_of_several_buttons_is_one_mask() {
        let script = parse("press L1+R1+Triangle").expect("a script");
        let wanted = parse_chord("L1+R1+Triangle").expect("a chord").mask;

        assert_eq!(script.at(0), Some(wanted));
    }

    #[test]
    fn the_script_runs_out_rather_than_repeating_forever() {
        let script = parse("press CROSS").expect("a script");

        assert_eq!(script.at(script.polls()), None);
    }

    #[test]
    fn blank_lines_and_comments_are_skipped() {
        let script = parse("# open it\n\n  \npress CROSS  # then let go\n").expect("a script");

        assert_eq!(script.polls(), PRESS_POLLS + RELEASE_POLLS);
    }

    #[test]
    fn a_button_this_build_does_not_know_is_refused_with_its_line() {
        let error = parse("press GUIDE").expect_err("an error");

        assert!(error.contains("line 1"), "{error}");
        assert!(error.contains("GUIDE"), "{error}");
    }

    #[test]
    fn a_verb_this_build_does_not_know_is_refused_rather_than_ignored() {
        let error = parse("press CROSS\nwiggle DOWN").expect_err("an error");

        assert!(error.contains("line 2"), "{error}");
    }

    #[test]
    fn a_hold_with_no_number_is_refused() {
        assert!(parse("hold DOWN").is_err());
    }

    #[test]
    fn an_empty_script_holds_nothing_rather_than_failing() {
        let script = parse("").expect("a script");

        assert!(script.is_empty());
    }
}
