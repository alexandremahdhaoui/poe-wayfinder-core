#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn right(self) -> i32 {
        self.x + self.width
    }

    pub fn bottom(self) -> i32 {
        self.y + self.height
    }

    pub fn overlap(self, other: Rect) -> i64 {
        let w = (self.right().min(other.right()) - self.x.max(other.x)).max(0);
        let h = (self.bottom().min(other.bottom()) - self.y.max(other.y)).max(0);

        i64::from(w) * i64::from(h)
    }

    pub fn area(self) -> i64 {
        i64::from(self.width.max(0)) * i64::from(self.height.max(0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
    Empty,
    OffScreen,
    MostlyOffScreen,
    BehindGame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measured {
    pub window: Rect,
    pub desktop: Rect,
    pub shown: bool,
    pub above_game: bool,
}

pub const ENOUGH_ON_SCREEN: f64 = 0.9;

pub fn visibility(measured: Measured) -> Visibility {
    if !measured.shown {
        return Visibility::Hidden;
    }

    let area = measured.window.area();

    if area <= 0 {
        return Visibility::Empty;
    }

    let on_screen = measured.window.overlap(measured.desktop);

    if on_screen == 0 {
        return Visibility::OffScreen;
    }

    if (on_screen as f64) < area as f64 * ENOUGH_ON_SCREEN {
        return Visibility::MostlyOffScreen;
    }

    if !measured.above_game {
        return Visibility::BehindGame;
    }

    Visibility::Visible
}

pub fn explain(visibility: Visibility) -> Option<&'static str> {
    match visibility {
        Visibility::Visible => None,

        Visibility::Hidden => Some(
            "the panel window is hidden. Windows does not repaint a hidden window, so the \
             frame loop stops too and the hotkey goes dead.",
        ),

        Visibility::Empty => Some("the panel window has no size, so there is nothing to draw."),

        Visibility::OffScreen => Some(
            "the panel is off the screen entirely. The usual cause is sending physical \
             pixels where logical points are wanted, which puts it out by the display \
             scale factor.",
        ),

        Visibility::MostlyOffScreen => Some(
            "most of the panel is past the edge of the screen. The usual cause is sending \
             physical pixels where logical points are wanted.",
        ),

        Visibility::BehindGame => Some(
            "the panel is on screen but underneath the game. Moving a window on Windows \
             also reorders it, and this one moves every frame to follow the game, so \
             always-on-top has to be re-asserted rather than set once.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESKTOP: Rect = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1600,
    };

    fn measured(window: Rect) -> Measured {
        Measured {
            window,
            desktop: DESKTOP,
            shown: true,
            above_game: true,
        }
    }

    const PANEL: Rect = Rect {
        x: 1936,
        y: 24,
        width: 600,
        height: 900,
    };

    #[test]
    fn a_panel_inside_the_desktop_and_above_the_game_is_visible() {
        assert_eq!(visibility(measured(PANEL)), Visibility::Visible);
    }

    #[test]
    fn a_hidden_window_is_reported_before_anything_else() {
        let m = Measured {
            shown: false,
            above_game: false,
            ..measured(Rect { x: -9999, ..PANEL })
        };

        assert_eq!(visibility(m), Visibility::Hidden);
    }

    #[test]
    fn a_window_with_no_area_is_empty() {
        let m = measured(Rect { width: 0, ..PANEL });

        assert_eq!(visibility(m), Visibility::Empty);
    }

    #[test]
    fn the_scale_factor_bug_reads_as_off_screen() {
        let m = measured(Rect { x: 2904, ..PANEL });

        assert_eq!(visibility(m), Visibility::OffScreen);
    }

    #[test]
    fn a_panel_hanging_mostly_past_the_edge_is_caught() {
        let m = measured(Rect { x: 2500, ..PANEL });

        assert_eq!(visibility(m), Visibility::MostlyOffScreen);
    }

    #[test]
    fn a_panel_hanging_slightly_past_the_edge_is_allowed() {
        let m = measured(Rect {
            x: 2560 - 580,
            ..PANEL
        });

        assert_eq!(visibility(m), Visibility::Visible);
    }

    #[test]
    fn a_panel_under_the_game_is_reported_even_though_it_is_on_screen() {
        let m = Measured {
            above_game: false,
            ..measured(PANEL)
        };

        assert_eq!(visibility(m), Visibility::BehindGame);
    }

    #[test]
    fn a_second_monitor_left_of_the_primary_still_counts_as_on_screen() {
        let m = Measured {
            desktop: Rect {
                x: -1920,
                y: 0,
                width: 4480,
                height: 1600,
            },
            ..measured(Rect {
                x: -1800,
                y: 100,
                width: 600,
                height: 900,
            })
        };

        assert_eq!(visibility(m), Visibility::Visible);
    }

    #[test]
    fn a_working_panel_says_nothing() {
        assert_eq!(explain(Visibility::Visible), None);
    }

    #[test]
    fn every_failure_explains_itself_and_names_a_cause() {
        for visibility in [
            Visibility::Hidden,
            Visibility::Empty,
            Visibility::OffScreen,
            Visibility::MostlyOffScreen,
            Visibility::BehindGame,
        ] {
            let text = explain(visibility).expect("explanation");

            assert!(text.len() > 40, "{visibility:?} says too little: {text}");
        }
    }

    #[test]
    fn two_rectangles_that_do_not_touch_overlap_by_nothing() {
        let a = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let b = Rect {
            x: 200,
            y: 200,
            width: 100,
            height: 100,
        };

        assert_eq!(a.overlap(b), 0);
        assert_eq!(b.overlap(a), 0);
    }

    #[test]
    fn a_rectangle_inside_another_overlaps_by_its_whole_area() {
        let inner = Rect {
            x: 10,
            y: 10,
            width: 50,
            height: 40,
        };

        assert_eq!(inner.overlap(DESKTOP), inner.area());
    }

    #[test]
    fn overlap_is_the_same_whichever_way_round() {
        assert_eq!(PANEL.overlap(DESKTOP), DESKTOP.overlap(PANEL));
    }
}
