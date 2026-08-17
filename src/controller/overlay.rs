#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
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

pub fn copy_key_sequence(also_hold: &[String]) -> Vec<KeyStroke> {
    let press = |key: &str| KeyStroke {
        key: key.to_string(),
        down: true,
    };

    let release = |key: &str| KeyStroke {
        key: key.to_string(),
        down: false,
    };

    let mut out = vec![press("Ctrl")];

    for key in also_hold {
        out.push(press(key));
    }

    out.push(press("C"));
    out.push(release("C"));

    for key in also_hold.iter().rev() {
        out.push(release(key));
    }

    out.push(release("Ctrl"));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn the_copy_sequence_is_control_and_c() {
        let got = copy_key_sequence(&nothing_extra());

        assert_eq!(got.len(), 4);
        assert_eq!(got[0].key, "Ctrl");
        assert_eq!(got[1].key, "C");
    }

    #[test]
    fn the_letter_is_released_before_the_modifier() {
        let got = copy_key_sequence(&holding_alt());

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
        let got = copy_key_sequence(&holding_alt());

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
        let got = copy_key_sequence(&holding_alt());

        for stroke in got.iter().filter(|s| s.down) {
            assert!(
                got.iter().any(|s| s.key == stroke.key && !s.down),
                "{} was pressed and never released",
                stroke.key
            );
        }
    }

    fn nothing_extra() -> Vec<String> {
        Vec::new()
    }

    fn holding_alt() -> Vec<String> {
        vec!["Alt".to_string()]
    }

    #[test]
    fn no_key_is_released_without_being_pressed() {
        let got = copy_key_sequence(&holding_alt());

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
        let got = copy_key_sequence(&holding_alt());

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
    fn the_show_mods_key_is_held_during_a_copy_or_the_roll_ranges_are_lost() {
        let got = copy_key_sequence(&holding_alt());

        assert!(got.iter().any(|s| s.key == "Alt" && s.down));
    }

    #[test]
    fn the_show_mods_key_is_held_before_the_copy_and_let_go_after_it() {
        let got = copy_key_sequence(&holding_alt());
        let at = |key: &str, down: bool| {
            got.iter()
                .position(|s| s.key == key && s.down == down)
                .expect("the stroke")
        };

        assert!(at("Alt", true) < at("C", true), "held too late to matter");
        assert!(at("C", false) < at("Alt", false), "let go too early");
    }

    #[test]
    fn a_game_that_needs_no_extra_key_still_gets_a_plain_copy() {
        let got = copy_key_sequence(&nothing_extra());
        let keys: Vec<&str> = got.iter().map(|s| s.key.as_str()).collect();

        assert_eq!(keys, ["Ctrl", "C", "C", "Ctrl"]);
    }
}
