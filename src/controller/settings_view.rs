#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    IncludeOffline,
    FilterItemLevel,
    RestoreClipboard,
    RollTolerance,
    PanelWidth,
}

impl Field {
    pub fn label(self) -> &'static str {
        match self {
            Field::IncludeOffline => "Include offline sellers",
            Field::FilterItemLevel => "Filter on item level",
            Field::RestoreClipboard => "Put the clipboard back",
            Field::RollTolerance => "Roll tolerance",
            Field::PanelWidth => "Panel width",
        }
    }

    pub fn explains(self) -> &'static str {
        match self {
            Field::IncludeOffline => "offline sellers rarely answer, but they set the floor",
            Field::FilterItemLevel => "narrows the search, and hides cheaper near matches",
            Field::RestoreClipboard => "the check has to copy, which destroys what you had",
            Field::RollTolerance => "how far a roll may sit from the item's own",
            Field::PanelWidth => "how wide the price panel is drawn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub low: f64,
    pub high: f64,
}

pub const TOLERANCE: Bounds = Bounds {
    low: 0.0,
    high: 1.0,
};

pub const PANEL_WIDTH: Bounds = Bounds {
    low: 320.0,
    high: 900.0,
};

pub fn bounds_of(field: Field) -> Option<Bounds> {
    match field {
        Field::RollTolerance => Some(TOLERANCE),
        Field::PanelWidth => Some(PANEL_WIDTH),
        _ => None,
    }
}

pub fn clamp(field: Field, value: f64) -> f64 {
    match bounds_of(field) {
        Some(bounds) => value.clamp(bounds.low, bounds.high),
        None => value,
    }
}

pub fn switches() -> Vec<Field> {
    vec![
        Field::IncludeOffline,
        Field::FilterItemLevel,
        Field::RestoreClipboard,
    ]
}

pub fn sliders() -> Vec<Field> {
    vec![Field::RollTolerance, Field::PanelWidth]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_says_what_it_is_and_why_it_matters() {
        for field in switches().into_iter().chain(sliders()) {
            assert!(!field.label().trim().is_empty());
            assert!(!field.explains().trim().is_empty());
        }
    }

    #[test]
    fn a_switch_has_no_bounds_and_a_slider_does() {
        for field in switches() {
            assert!(bounds_of(field).is_none(), "{field:?}");
        }

        for field in sliders() {
            assert!(bounds_of(field).is_some(), "{field:?}");
        }
    }

    #[test]
    fn a_value_outside_its_bounds_is_pulled_back_in() {
        assert_eq!(clamp(Field::RollTolerance, 5.0), 1.0);
        assert_eq!(clamp(Field::RollTolerance, -2.0), 0.0);
        assert_eq!(clamp(Field::PanelWidth, 10.0), PANEL_WIDTH.low);
        assert_eq!(clamp(Field::PanelWidth, 5000.0), PANEL_WIDTH.high);
    }

    #[test]
    fn a_value_inside_its_bounds_is_left_alone() {
        assert_eq!(clamp(Field::RollTolerance, 0.25), 0.25);
        assert_eq!(clamp(Field::PanelWidth, 460.0), 460.0);
    }

    #[test]
    fn a_switch_is_never_clamped() {
        assert_eq!(clamp(Field::IncludeOffline, 9999.0), 9999.0);
    }

    #[test]
    fn no_field_appears_in_both_lists() {
        for field in switches() {
            assert!(!sliders().contains(&field), "{field:?} is in both");
        }
    }

    #[test]
    fn every_slider_has_a_low_below_its_high() {
        for field in sliders() {
            let bounds = bounds_of(field).expect("bounds");

            assert!(bounds.low < bounds.high, "{field:?}");
        }
    }
}
