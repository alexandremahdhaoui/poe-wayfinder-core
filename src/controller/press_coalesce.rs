use std::time::Duration;

pub const GUARD: Duration = Duration::from_millis(300);

pub fn accept(since_last: Option<Duration>) -> bool {
    match since_last {
        None => true,
        Some(elapsed) => elapsed >= GUARD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_press_is_always_accepted() {
        assert!(accept(None));
    }

    #[test]
    fn the_second_report_of_one_press_is_dropped() {
        assert!(!accept(Some(Duration::from_millis(0))));
        assert!(!accept(Some(Duration::from_millis(100))));
        assert!(!accept(Some(Duration::from_millis(299))));
    }

    #[test]
    fn a_press_after_the_guard_is_a_new_press() {
        assert!(accept(Some(GUARD)));
        assert!(accept(Some(Duration::from_millis(500))));
        assert!(accept(Some(Duration::from_secs(10))));
    }

    #[test]
    fn the_guard_covers_several_frames() {
        assert!(GUARD >= Duration::from_millis(200));
    }

    #[test]
    fn the_guard_is_shorter_than_a_deliberate_second_press() {
        assert!(GUARD <= Duration::from_millis(500));
    }
}
