#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stopwatch {
    started_at: Option<u64>,
    banked_ms: u64,
}

impl Stopwatch {
    pub fn is_running(self) -> bool {
        self.started_at.is_some()
    }

    pub fn start(&mut self, now_ms: u64) {
        if self.started_at.is_none() {
            self.started_at = Some(now_ms);
        }
    }

    pub fn stop(&mut self, now_ms: u64) {
        if let Some(at) = self.started_at.take() {
            self.banked_ms += now_ms.saturating_sub(at);
        }
    }

    pub fn toggle(&mut self, now_ms: u64) -> bool {
        match self.is_running() {
            true => {
                self.stop(now_ms);

                false
            }
            false => {
                self.start(now_ms);

                true
            }
        }
    }

    pub fn reset(&mut self) {
        self.started_at = None;
        self.banked_ms = 0;
    }

    pub fn elapsed_ms(self, now_ms: u64) -> u64 {
        let running = self
            .started_at
            .map(|at| now_ms.saturating_sub(at))
            .unwrap_or(0);

        self.banked_ms + running
    }

    pub fn reading(self, now_ms: u64) -> String {
        clock(self.elapsed_ms(now_ms))
    }
}

pub fn clock(total_ms: u64) -> String {
    let seconds = total_ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;

    match hours {
        0 => format!("{:02}:{:02}", minutes % 60, seconds % 60),
        _ => format!("{}:{:02}:{:02}", hours, minutes % 60, seconds % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_stopwatch_reads_zero_and_is_not_running() {
        let w = Stopwatch::default();

        assert!(!w.is_running());
        assert_eq!(w.reading(0), "00:00");
    }

    #[test]
    fn it_counts_while_it_runs() {
        let mut w = Stopwatch::default();

        w.start(1_000);

        assert!(w.is_running());
        assert_eq!(w.elapsed_ms(6_000), 5_000);
    }

    #[test]
    fn stopping_banks_the_time_and_it_stops_counting() {
        let mut w = Stopwatch::default();

        w.start(0);
        w.stop(5_000);

        assert_eq!(
            w.elapsed_ms(60_000),
            5_000,
            "a stopped watch does not drift"
        );
    }

    #[test]
    fn starting_again_adds_to_what_was_banked() {
        let mut w = Stopwatch::default();

        w.start(0);
        w.stop(5_000);
        w.start(10_000);

        assert_eq!(w.elapsed_ms(12_000), 7_000);
    }

    #[test]
    fn starting_a_running_watch_does_not_restart_it() {
        let mut w = Stopwatch::default();

        w.start(1_000);
        w.start(9_000);

        assert_eq!(w.elapsed_ms(11_000), 10_000);
    }

    #[test]
    fn toggle_reports_whether_it_is_now_running() {
        let mut w = Stopwatch::default();

        assert!(w.toggle(0));
        assert!(!w.toggle(1_000));
        assert_eq!(w.elapsed_ms(9_000), 1_000);
    }

    #[test]
    fn reset_clears_everything() {
        let mut w = Stopwatch::default();

        w.start(0);
        w.stop(5_000);
        w.reset();

        assert_eq!(w.elapsed_ms(9_000), 0);
        assert!(!w.is_running());
    }

    #[test]
    fn a_clock_that_goes_backwards_does_not_underflow() {
        let mut w = Stopwatch::default();

        w.start(10_000);

        assert_eq!(
            w.elapsed_ms(5_000),
            0,
            "a backwards clock reads zero, not a huge number"
        );
    }

    #[test]
    fn the_reading_grows_a_field_only_when_it_needs_one() {
        assert_eq!(clock(0), "00:00");
        assert_eq!(clock(65_000), "01:05");
        assert_eq!(clock(3_600_000), "1:00:00");
        assert_eq!(clock(3_725_000), "1:02:05");
    }
}
