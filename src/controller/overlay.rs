#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn point_in_rect(x: i32, y: i32, rect: Rect) -> bool {
    x > rect.x && x < rect.x + rect.width && y > rect.y && y < rect.y + rect.height
}

pub const ITEM_FIRST_LINE: &str = "Item Class: ";

pub const FOREIGN_FIRST_LINES: &[(&str, &str)] = &[
    ("ru", "Класс предмета: "),
    ("ko", "아이템 종류: "),
    ("cmn-Hant", "物品種類: "),
    ("de", "Gegenstandsklasse: "),
    ("fr", "Classe d'objet: "),
    ("es", "Clase de objeto: "),
    ("pt", "Classe do Item: "),
    ("th", "ประเภทไอเทม: "),
    ("ja", "アイテムクラス: "),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardKind {
    Item,
    ForeignItem(&'static str),
    NotAnItem,
}

pub fn clipboard_kind(text: &str) -> ClipboardKind {
    if text.starts_with(ITEM_FIRST_LINE) {
        return ClipboardKind::Item;
    }

    for (lang, first_line) in FOREIGN_FIRST_LINES {
        if text.starts_with(first_line) {
            return ClipboardKind::ForeignItem(lang);
        }
    }

    ClipboardKind::NotAnItem
}

pub fn keys_to_hold_for_copy(show_mods_key: &str, already_held: &[String]) -> Vec<String> {
    show_mods_key
        .split(" + ")
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .filter(|key| *key != "C")
        .filter(|key| !already_held.iter().any(|held| held == key))
        .map(str::to_string)
        .collect()
}

pub fn event_to_hotkey(key: &str, ctrl: bool, shift: bool, alt: bool) -> String {
    if matches!(key, "Ctrl" | "Shift" | "Alt") {
        return key.to_string();
    }

    let mut parts = Vec::new();

    if ctrl {
        parts.push("Ctrl");
    }

    if shift {
        parts.push("Shift");
    }

    if alt {
        parts.push("Alt");
    }

    parts.push(key);

    parts.join(" + ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyStroke {
    pub key: String,
    pub down: bool,
}

pub fn copy_key_sequence() -> Vec<KeyStroke> {
    let press = |key: &str| KeyStroke {
        key: key.to_string(),
        down: true,
    };

    let release = |key: &str| KeyStroke {
        key: key.to_string(),
        down: false,
    };

    vec![press("Ctrl"), press("C"), release("C"), release("Ctrl")]
}

const STASH_TOP: f64 = 154.0 / 1600.0;

const STASH_BOTTOM: f64 = 1192.0 / 1600.0;

pub fn is_stash_area(x: i32, y: i32, window: Rect, sidebar_width: i32) -> bool {
    if x > window.x + sidebar_width {
        return false;
    }

    let height = f64::from(window.height);
    let top = f64::from(window.y) + height * STASH_TOP;
    let bottom = f64::from(window.y) + height * STASH_BOTTOM;

    let y = f64::from(y);

    y > top && y < bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect {
            x: 100,
            y: 100,
            width: 200,
            height: 200,
        }
    }

    #[test]
    fn a_point_inside_is_inside() {
        assert!(point_in_rect(150, 150, rect()));
    }

    #[test]
    fn a_point_outside_is_outside() {
        assert!(!point_in_rect(50, 150, rect()));
        assert!(!point_in_rect(150, 50, rect()));
        assert!(!point_in_rect(350, 150, rect()));
        assert!(!point_in_rect(150, 350, rect()));
    }

    #[test]
    fn a_point_on_the_border_belongs_to_the_game() {
        assert!(!point_in_rect(100, 150, rect()));
        assert!(!point_in_rect(300, 150, rect()));
        assert!(!point_in_rect(150, 100, rect()));
        assert!(!point_in_rect(150, 300, rect()));
    }

    #[test]
    fn a_rectangle_with_no_area_contains_nothing() {
        let empty = Rect {
            x: 100,
            y: 100,
            width: 0,
            height: 0,
        };

        assert!(!point_in_rect(100, 100, empty));
    }

    #[test]
    fn a_negative_coordinate_works() {
        let left = Rect {
            x: -500,
            y: 0,
            width: 200,
            height: 200,
        };

        assert!(point_in_rect(-400, 100, left));
        assert!(!point_in_rect(-600, 100, left));
    }

    #[test]
    fn english_item_text_is_recognised() {
        assert_eq!(
            clipboard_kind("Item Class: Rings\nRarity: Rare\n"),
            ClipboardKind::Item
        );
    }

    #[test]
    fn anything_else_is_not_an_item() {
        assert_eq!(clipboard_kind("milk, eggs"), ClipboardKind::NotAnItem);
        assert_eq!(clipboard_kind(""), ClipboardKind::NotAnItem);
    }

    #[test]
    fn a_foreign_client_is_named_rather_than_parsed() {
        assert_eq!(
            clipboard_kind("Класс предмета: Кольца"),
            ClipboardKind::ForeignItem("ru")
        );
        assert_eq!(
            clipboard_kind("アイテムクラス: 指輪"),
            ClipboardKind::ForeignItem("ja")
        );
    }

    #[test]
    fn every_foreign_first_line_is_distinct() {
        let mut seen: Vec<&str> = FOREIGN_FIRST_LINES.iter().map(|(_, l)| *l).collect();
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();

        assert_eq!(seen.len(), count);
    }

    #[test]
    fn no_foreign_first_line_collides_with_english() {
        for (lang, line) in FOREIGN_FIRST_LINES {
            assert!(!line.starts_with(ITEM_FIRST_LINE), "{lang}");
            assert!(!ITEM_FIRST_LINE.starts_with(line), "{lang}");
        }
    }

    #[test]
    fn text_that_merely_contains_the_marker_is_not_an_item() {
        assert_eq!(
            clipboard_kind("look at this Item Class: Rings"),
            ClipboardKind::NotAnItem
        );
    }

    #[test]
    fn the_show_mods_key_is_held_while_copying() {
        assert_eq!(keys_to_hold_for_copy("Alt", &[]), ["Alt"]);
    }

    #[test]
    fn a_combination_holds_every_part() {
        assert_eq!(keys_to_hold_for_copy("Ctrl + Alt", &[]), ["Ctrl", "Alt"]);
    }

    #[test]
    fn the_copy_key_itself_is_not_held() {
        assert_eq!(keys_to_hold_for_copy("Ctrl + C", &[]), ["Ctrl"]);
    }

    #[test]
    fn a_key_the_user_already_holds_is_skipped() {
        let held = vec!["Ctrl".to_string()];

        assert_eq!(keys_to_hold_for_copy("Ctrl + Alt", &held), ["Alt"]);
    }

    #[test]
    fn holding_every_needed_key_leaves_nothing_to_press() {
        let held = vec!["Ctrl".to_string(), "Alt".to_string()];

        assert!(keys_to_hold_for_copy("Ctrl + Alt", &held).is_empty());
    }

    #[test]
    fn an_empty_show_mods_key_holds_nothing() {
        assert!(keys_to_hold_for_copy("", &[]).is_empty());
        assert!(keys_to_hold_for_copy("   ", &[]).is_empty());
    }

    #[test]
    fn a_plain_key_renders_as_itself() {
        assert_eq!(event_to_hotkey("D", false, false, false), "D");
    }

    #[test]
    fn modifiers_render_in_a_fixed_order() {
        assert_eq!(
            event_to_hotkey("D", true, true, true),
            "Ctrl + Shift + Alt + D"
        );
        assert_eq!(event_to_hotkey("D", true, false, true), "Ctrl + Alt + D");
        assert_eq!(event_to_hotkey("D", false, true, true), "Shift + Alt + D");
    }

    #[test]
    fn a_modifier_pressed_alone_is_its_own_name() {
        for key in ["Ctrl", "Shift", "Alt"] {
            assert_eq!(event_to_hotkey(key, true, true, true), key);
        }
    }

    fn window() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 2560,
            height: 1600,
        }
    }

    #[test]
    fn the_middle_of_the_stash_panel_is_the_stash() {
        assert!(is_stash_area(100, 800, window(), 600));
    }

    #[test]
    fn above_and_below_the_panel_is_not_the_stash() {
        assert!(!is_stash_area(100, 100, window(), 600));
        assert!(!is_stash_area(100, 1500, window(), 600));
    }

    #[test]
    fn past_the_sidebar_is_the_game_world() {
        assert!(!is_stash_area(1000, 800, window(), 600));
    }

    #[test]
    fn the_bounds_scale_with_the_window() {
        let small = Rect {
            x: 0,
            y: 0,
            width: 1280,
            height: 800,
        };

        assert!(is_stash_area(100, 400, small, 300));
        assert!(!is_stash_area(100, 50, small, 300));
    }

    #[test]
    fn an_offset_window_moves_the_panel_with_it() {
        let offset = Rect {
            x: 500,
            y: 300,
            width: 2560,
            height: 1600,
        };

        assert!(is_stash_area(600, 1100, offset, 600));
        assert!(!is_stash_area(600, 350, offset, 600));
    }

    #[test]
    fn the_panel_top_is_above_its_bottom() {
        let mid = f64::from(window().height) * (STASH_TOP + STASH_BOTTOM) / 2.0;

        assert!(is_stash_area(100, mid as i32, window(), 600));
    }

    #[test]
    fn the_copy_sequence_is_control_and_c() {
        let got = copy_key_sequence();

        assert_eq!(got.len(), 4);
        assert_eq!(got[0].key, "Ctrl");
        assert_eq!(got[1].key, "C");
    }

    #[test]
    fn the_letter_is_released_before_the_modifier() {
        let got = copy_key_sequence();

        let c_up = got
            .iter()
            .position(|s| s.key == "C" && !s.down)
            .expect("C is released");
        let ctrl_up = got
            .iter()
            .position(|s| s.key == "Ctrl" && !s.down)
            .expect("Ctrl is released");

        assert!(c_up < ctrl_up);
    }

    #[test]
    fn the_modifier_goes_down_before_the_letter() {
        let got = copy_key_sequence();

        let ctrl_down = got
            .iter()
            .position(|s| s.key == "Ctrl" && s.down)
            .expect("Ctrl is pressed");
        let c_down = got
            .iter()
            .position(|s| s.key == "C" && s.down)
            .expect("C is pressed");

        assert!(ctrl_down < c_down);
    }

    #[test]
    fn every_press_has_a_release() {
        let got = copy_key_sequence();

        for stroke in got.iter().filter(|s| s.down) {
            assert!(
                got.iter().any(|s| s.key == stroke.key && !s.down),
                "{} was pressed and never released",
                stroke.key
            );
        }
    }

    #[test]
    fn no_key_is_released_without_being_pressed() {
        let got = copy_key_sequence();

        for stroke in got.iter().filter(|s| !s.down) {
            assert!(
                got.iter().any(|s| s.key == stroke.key && s.down),
                "{} was released without being pressed",
                stroke.key
            );
        }
    }

    #[test]
    fn the_sequence_nests_rather_than_overlapping() {
        let got = copy_key_sequence();

        let mut held: Vec<&str> = Vec::new();

        for stroke in &got {
            if stroke.down {
                held.push(&stroke.key);
            } else {
                assert_eq!(
                    held.pop(),
                    Some(stroke.key.as_str()),
                    "{} came up out of order",
                    stroke.key
                );
            }
        }

        assert!(held.is_empty(), "the sequence ends with keys still held");
    }

    #[test]
    fn the_show_mods_key_is_not_held_during_a_copy() {
        assert!(!copy_key_sequence().iter().any(|s| s.key == "Alt"));
    }
}
