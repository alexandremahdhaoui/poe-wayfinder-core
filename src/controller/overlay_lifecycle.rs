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
    locked: bool,
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
            locked: false,
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
        self.locked = false;
    }

    pub fn begin_locked(&mut self, pointer: Point) {
        self.begin(pointer);
        self.locked = true;
    }

    pub fn is_locked(self) -> bool {
        self.locked
    }

    pub fn toggle_locked(&mut self) -> bool {
        if !self.phase.is_on_screen() {
            return false;
        }

        match self.locked {
            true => {
                self.dismiss();

                false
            }
            false => {
                self.locked = true;
                self.phase = Phase::Interactive;
                self.alt_hidden = false;
                self.alt_held_ms = 0;

                true
            }
        }
    }

    pub fn ready(&mut self, panel: Rect) {
        self.panel = panel;

        if self.phase == Phase::Loading {
            self.phase = match self.locked {
                true => Phase::Interactive,
                false => Phase::Watching,
            };
        }
    }

    pub fn dismiss(&mut self) {
        self.phase = Phase::Closed;
        self.alt_hidden = false;
        self.alt_held_ms = 0;
        self.locked = false;
    }

    pub fn origin(&self) -> Point {
        self.origin
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
            if self.locked {
                return;
            }

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

        if self.locked {
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

    #[test]
    fn a_rectangle_with_no_area_contains_nothing() {
        let empty = Rect {
            x: 100,
            y: 100,
            width: 0,
            height: 0,
        };

        assert!(!empty.contains(Point { x: 100, y: 100 }));
    }

    #[test]
    fn a_negative_coordinate_works() {
        let left = Rect {
            x: -500,
            y: 0,
            width: 200,
            height: 200,
        };

        assert!(left.contains(Point { x: -400, y: 100 }));
        assert!(!left.contains(Point { x: -600, y: 100 }));
    }

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

    fn locked() -> Lifecycle {
        let mut life = Lifecycle::new(HoldKey::Ctrl);
        life.begin_locked(ORIGIN);
        life.ready(PANEL);

        life
    }

    const FAR: Point = Point { x: 40, y: 40 };

    #[test]
    fn the_overlay_key_grabs_a_panel_that_is_only_being_watched() {
        let mut life = watching();

        assert!(life.toggle_locked(), "the panel was grabbed");
        assert!(life.is_locked());
        assert!(life.takes_input(), "grabbing it is what makes it clickable");
    }

    #[test]
    fn the_overlay_key_releases_a_panel_it_already_grabbed() {
        let mut life = locked();

        assert!(!life.toggle_locked(), "the panel was released");
        assert_eq!(life.phase(), Phase::Closed);
    }

    #[test]
    fn the_overlay_key_does_nothing_when_no_panel_is_up() {
        let mut life = Lifecycle::new(HoldKey::Ctrl);

        assert!(!life.toggle_locked());
        assert_eq!(life.phase(), Phase::Closed);
    }

    #[test]
    fn grabbing_a_panel_that_alt_had_hidden_brings_it_back() {
        let mut life = watching();

        life.tick(Input {
            alt_alone: true,
            elapsed_ms: ALT_HIDE_MS_WATCHING + 10,
            ..quiet(ORIGIN)
        });

        assert!(!life.is_drawn(), "alt hides it");

        life.toggle_locked();

        assert!(life.is_drawn(), "the overlay key brings it back to be used");
    }

    #[test]
    fn a_locked_panel_is_interactive_the_moment_it_is_ready() {
        let life = locked();

        assert!(life.is_locked());
        assert_eq!(life.phase(), Phase::Interactive);
        assert!(life.takes_input(), "a locked panel is there to be used");
    }

    #[test]
    fn a_locked_panel_ignores_the_pointer_wandering_off() {
        let mut life = locked();

        life.tick(quiet(FAR));
        life.tick(quiet(FAR));

        assert_eq!(life.phase(), Phase::Interactive, "locked means it stays");
    }

    #[test]
    fn an_unlocked_panel_still_closes_when_the_pointer_wanders_off() {
        let mut life = watching();

        life.tick(quiet(FAR));

        assert_eq!(life.phase(), Phase::Closed);
    }

    #[test]
    fn a_locked_panel_still_closes_on_a_click_outside_it() {
        let mut life = locked();

        life.tick(Input {
            clicked: true,
            ..quiet(FAR)
        });

        assert_eq!(life.phase(), Phase::Closed);
    }

    #[test]
    fn dismissing_a_locked_panel_clears_the_lock() {
        let mut life = locked();

        life.dismiss();

        assert!(!life.is_locked());
        assert_eq!(life.phase(), Phase::Closed);
    }

    #[test]
    fn a_locked_panel_survives_the_drift_that_would_close_it_while_loading() {
        let mut life = Lifecycle::new(HoldKey::Ctrl);

        life.begin_locked(ORIGIN);
        life.tick(quiet(FAR));

        assert_ne!(
            life.phase(),
            Phase::Closed,
            "moving to the panel must not close it before it is even drawn"
        );
    }

    #[test]
    fn the_next_plain_check_is_not_still_locked() {
        let mut life = locked();

        life.begin(ORIGIN);
        life.ready(PANEL);

        assert!(!life.is_locked());
        assert_eq!(life.phase(), Phase::Watching);
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
    #[test]
    fn the_origin_is_where_the_check_began() {
        let mut life = Lifecycle::new(HoldKey::Ctrl);
        life.begin(Point { x: 700, y: 500 });

        assert_eq!(life.origin(), Point { x: 700, y: 500 });
    }

    #[test]
    fn a_pointer_that_never_leaves_the_origin_keeps_the_panel_open() {
        let mut life = Lifecycle::new(HoldKey::Ctrl);
        life.begin(Point { x: 700, y: 500 });
        life.ready(Rect {
            x: 720,
            y: 520,
            width: 400,
            height: 300,
        });

        for _ in 0..50 {
            life.tick(Input {
                pointer: life.origin(),
                hold_down: false,
                alt_alone: false,
                clicked: false,
                elapsed_ms: 100,
            });
        }

        assert_ne!(life.phase(), Phase::Closed);
    }
}
