#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn distance_to(self, other: Point) -> f64 {
        let dx = f64::from(other.x - self.x);
        let dy = f64::from(other.y - self.y);

        dx.hypot(dy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn contains(self, point: Point) -> bool {
        point.x > self.x
            && point.x < self.x + self.width
            && point.y > self.y
            && point.y < self.y + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldKey {
    Ctrl,
    Alt,
    Shift,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Closed,
    Loading,
    Watching,
    Interactive,
}

impl Phase {
    pub fn is_on_screen(self) -> bool {
        !matches!(self, Phase::Closed)
    }

    pub fn takes_input(self) -> bool {
        matches!(self, Phase::Interactive)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Input {
    pub pointer: Point,
    pub hold_down: bool,
    pub alt_alone: bool,
    pub clicked: bool,
    pub elapsed_ms: u64,
}

pub const CLOSE_THRESHOLD: f64 = 38.0;

pub const ALT_HIDE_MS_INTERACTIVE: u64 = 85;
pub const ALT_HIDE_MS_WATCHING: u64 = 275;

#[derive(Debug, Clone, Copy)]
pub struct Lifecycle {
    phase: Phase,
    origin: Point,
    panel: Rect,
    hold: HoldKey,
    alt_held_ms: u64,
    alt_hidden: bool,
}

impl Lifecycle {
    pub fn new(hold: HoldKey) -> Self {
        Self {
            phase: Phase::Closed,
            origin: Point { x: 0, y: 0 },
            panel: Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            hold,
            alt_held_ms: 0,
            alt_hidden: false,
        }
    }

    pub fn phase(self) -> Phase {
        self.phase
    }

    pub fn is_drawn(self) -> bool {
        self.phase.is_on_screen() && !self.alt_hidden
    }

    pub fn takes_input(self) -> bool {
        self.phase.takes_input() && !self.alt_hidden
    }

    pub fn begin(&mut self, pointer: Point) {
        self.phase = Phase::Loading;
        self.origin = pointer;
        self.alt_held_ms = 0;
        self.alt_hidden = false;
    }

    pub fn ready(&mut self, panel: Rect) {
        self.panel = panel;

        if self.phase == Phase::Loading {
            self.phase = Phase::Watching;
        }
    }

    pub fn dismiss(&mut self) {
        self.phase = Phase::Closed;
        self.alt_hidden = false;
        self.alt_held_ms = 0;
    }

    pub fn tick(&mut self, input: Input) {
        self.track_alt(input);

        if self.phase == Phase::Closed {
            return;
        }

        if self.alt_hidden {
            return;
        }

        if self.phase == Phase::Loading {
            if self.drifted(input) {
                self.phase = Phase::Closed;
            }

            return;
        }

        let inside = self.panel.contains(input.pointer);

        if inside {
            self.phase = Phase::Interactive;

            return;
        }

        if input.clicked {
            self.phase = Phase::Closed;

            return;
        }

        match self.phase {
            Phase::Interactive => self.phase = Phase::Watching,

            Phase::Watching => {
                if self.drifted(input) {
                    self.phase = Phase::Closed;
                }
            }

            Phase::Closed | Phase::Loading => {}
        }
    }

    fn drifted(&self, input: Input) -> bool {
        if input.hold_down && self.hold != HoldKey::None {
            return false;
        }

        self.origin.distance_to(input.pointer) > CLOSE_THRESHOLD
    }

    fn track_alt(&mut self, input: Input) {
        if !input.alt_alone {
            if self.alt_hidden {
                self.origin = input.pointer;
            }

            self.alt_held_ms = 0;
            self.alt_hidden = false;

            return;
        }

        self.alt_held_ms = self.alt_held_ms.saturating_add(input.elapsed_ms);

        let wait = if self.phase == Phase::Interactive {
            ALT_HIDE_MS_INTERACTIVE
        } else {
            ALT_HIDE_MS_WATCHING
        };

        if self.alt_held_ms >= wait {
            self.alt_hidden = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PANEL: Rect = Rect {
        x: 1000,
        y: 100,
        width: 400,
        height: 600,
    };

    const ORIGIN: Point = Point { x: 500, y: 500 };

    fn quiet(pointer: Point) -> Input {
        Input {
            pointer,
            hold_down: false,
            alt_alone: false,
            clicked: false,
            elapsed_ms: 16,
        }
    }

    fn watching() -> Lifecycle {
        let mut life = Lifecycle::new(HoldKey::Ctrl);
        life.begin(ORIGIN);
        life.ready(PANEL);

        life
    }

    #[test]
    fn nothing_is_drawn_before_a_price_check() {
        let life = Lifecycle::new(HoldKey::Ctrl);

        assert_eq!(life.phase(), Phase::Closed);
        assert!(!life.is_drawn());
    }

    #[test]
    fn a_check_opens_it_without_taking_clicks() {
        let life = watching();

        assert_eq!(life.phase(), Phase::Watching);
        assert!(life.is_drawn());
        assert!(!life.takes_input());
    }

    #[test]
    fn moving_the_mouse_away_closes_it() {
        let mut life = watching();

        life.tick(quiet(Point { x: 900, y: 500 }));

        assert_eq!(life.phase(), Phase::Closed);
    }

    #[test]
    fn a_small_movement_does_not_close_it() {
        let mut life = watching();

        life.tick(quiet(Point { x: 510, y: 505 }));

        assert_eq!(life.phase(), Phase::Watching);
    }

    #[test]
    fn holding_the_hotkey_modifier_keeps_it_open() {
        let mut life = watching();

        life.tick(Input {
            hold_down: true,
            ..quiet(Point { x: 2000, y: 900 })
        });

        assert_eq!(life.phase(), Phase::Watching);
    }

    #[test]
    fn a_hotkey_with_no_modifier_cannot_be_held_open() {
        let mut life = Lifecycle::new(HoldKey::None);
        life.begin(ORIGIN);
        life.ready(PANEL);

        life.tick(Input {
            hold_down: true,
            ..quiet(Point { x: 2000, y: 900 })
        });

        assert_eq!(life.phase(), Phase::Closed);
    }

    #[test]
    fn reaching_the_panel_makes_it_interactive() {
        let mut life = watching();

        life.tick(quiet(Point { x: 1200, y: 400 }));

        assert_eq!(life.phase(), Phase::Interactive);
        assert!(life.takes_input());
    }

    #[test]
    fn reaching_the_panel_beats_the_distance_rule() {
        let mut life = watching();

        assert!(ORIGIN.distance_to(Point { x: 1200, y: 400 }) > CLOSE_THRESHOLD);

        life.tick(quiet(Point { x: 1200, y: 400 }));

        assert_eq!(life.phase(), Phase::Interactive);
    }

    #[test]
    fn leaving_the_panel_gives_the_game_its_clicks_back() {
        let mut life = watching();
        life.tick(quiet(Point { x: 1200, y: 400 }));

        life.tick(quiet(Point { x: 800, y: 400 }));

        assert_eq!(life.phase(), Phase::Watching);
        assert!(!life.takes_input());
        assert!(life.is_drawn(), "leaving must not close it outright");
    }

    #[test]
    fn a_click_outside_closes_it() {
        let mut life = watching();

        life.tick(Input {
            clicked: true,
            ..quiet(Point { x: 505, y: 505 })
        });

        assert_eq!(life.phase(), Phase::Closed);
    }

    #[test]
    fn a_click_inside_does_not_close_it() {
        let mut life = watching();

        life.tick(Input {
            clicked: true,
            ..quiet(Point { x: 1200, y: 400 })
        });

        assert_eq!(life.phase(), Phase::Interactive);
    }

    #[test]
    fn escape_closes_it_from_any_phase() {
        for pointer in [Point { x: 1200, y: 400 }, Point { x: 505, y: 505 }] {
            let mut life = watching();
            life.tick(quiet(pointer));

            life.dismiss();

            assert_eq!(life.phase(), Phase::Closed);
        }
    }

    #[test]
    fn holding_alt_alone_hides_the_overlay() {
        let mut life = watching();

        life.tick(Input {
            alt_alone: true,
            elapsed_ms: ALT_HIDE_MS_WATCHING,
            ..quiet(ORIGIN)
        });

        assert!(!life.is_drawn());
    }

    #[test]
    fn a_quick_alt_tap_does_not_hide_it() {
        let mut life = watching();

        life.tick(Input {
            alt_alone: true,
            elapsed_ms: 50,
            ..quiet(ORIGIN)
        });

        assert!(life.is_drawn());
    }

    #[test]
    fn an_interactive_panel_hides_sooner() {
        let mut life = watching();
        life.tick(quiet(Point { x: 1200, y: 400 }));

        assert_eq!(life.phase(), Phase::Interactive);

        life.tick(Input {
            alt_alone: true,
            elapsed_ms: ALT_HIDE_MS_INTERACTIVE,
            ..quiet(Point { x: 1200, y: 400 })
        });

        assert!(!life.is_drawn());
    }

    #[test]
    fn releasing_alt_brings_the_same_panel_back() {
        let mut life = watching();

        life.tick(Input {
            alt_alone: true,
            elapsed_ms: ALT_HIDE_MS_WATCHING,
            ..quiet(ORIGIN)
        });

        assert!(!life.is_drawn());

        life.tick(quiet(ORIGIN));

        assert!(life.is_drawn());
        assert_eq!(life.phase(), Phase::Watching);
    }

    #[test]
    fn a_hidden_overlay_does_not_close_behind_the_users_back() {
        let mut life = watching();

        life.tick(Input {
            alt_alone: true,
            elapsed_ms: ALT_HIDE_MS_WATCHING,
            ..quiet(ORIGIN)
        });

        life.tick(Input {
            alt_alone: true,
            elapsed_ms: 16,
            ..quiet(Point { x: 2000, y: 900 })
        });

        life.tick(quiet(Point { x: 2000, y: 900 }));

        assert!(life.is_drawn(), "it closed while the user could not see it");
    }

    #[test]
    fn a_hidden_overlay_never_takes_clicks() {
        let mut life = watching();
        life.tick(quiet(Point { x: 1200, y: 400 }));

        life.tick(Input {
            alt_alone: true,
            elapsed_ms: ALT_HIDE_MS_INTERACTIVE,
            ..quiet(Point { x: 1200, y: 400 })
        });

        assert!(!life.takes_input());
    }

    #[test]
    fn alt_with_another_modifier_is_a_different_gesture() {
        let mut life = watching();

        life.tick(Input {
            alt_alone: false,
            elapsed_ms: 1000,
            ..quiet(ORIGIN)
        });

        assert!(life.is_drawn());
    }

    #[test]
    fn a_result_arriving_after_the_user_left_does_not_reopen_it() {
        let mut life = Lifecycle::new(HoldKey::Ctrl);
        life.begin(ORIGIN);

        life.tick(quiet(Point { x: 2000, y: 900 }));

        assert_eq!(life.phase(), Phase::Closed);

        life.ready(PANEL);

        assert_eq!(life.phase(), Phase::Closed);
    }

    #[test]
    fn a_result_arriving_while_the_user_waits_opens_it() {
        let mut life = Lifecycle::new(HoldKey::Ctrl);
        life.begin(ORIGIN);

        life.tick(quiet(Point { x: 505, y: 502 }));
        life.ready(PANEL);

        assert_eq!(life.phase(), Phase::Watching);
    }

    #[test]
    fn a_second_check_reopens_a_closed_panel() {
        let mut life = watching();
        life.dismiss();

        life.begin(Point { x: 800, y: 800 });
        life.ready(PANEL);

        assert_eq!(life.phase(), Phase::Watching);
    }

    #[test]
    fn a_point_on_the_edge_is_outside() {
        assert!(!PANEL.contains(Point { x: PANEL.x, y: 400 }));
        assert!(!PANEL.contains(Point {
            x: PANEL.x + PANEL.width,
            y: 400
        }));
    }

    #[test]
    fn distance_is_the_straight_line() {
        let a = Point { x: 0, y: 0 };
        let b = Point { x: 3, y: 4 };

        assert!((a.distance_to(b) - 5.0).abs() < f64::EPSILON);
        assert!((b.distance_to(a) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn the_close_threshold_matches_the_reference() {
        assert!((CLOSE_THRESHOLD - 2.5 * 15.0).abs() < 1.0);
    }
}
