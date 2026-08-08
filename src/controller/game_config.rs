use std::collections::BTreeMap;

const ACTION_KEYS: &str = "ACTION_KEYS";

const SHOW_ADVANCED: &str = "show_advanced_item_descriptions";

pub const DEFAULT_SHOW_MODS_KEY: &str = "Alt";

const KEY_CODES: &[(u32, &str)] = &[
    (8, "Backspace"),
    (9, "Tab"),
    (13, "Enter"),
    (16, "Shift"),
    (17, "Ctrl"),
    (18, "Alt"),
    (20, "CapsLock"),
    (27, "Escape"),
    (32, "Space"),
    (33, "PageUp"),
    (34, "PageDown"),
    (35, "End"),
    (36, "Home"),
    (37, "ArrowLeft"),
    (38, "ArrowUp"),
    (39, "ArrowRight"),
    (40, "ArrowDown"),
    (45, "Insert"),
    (46, "Delete"),
    (48, "0"),
    (49, "1"),
    (50, "2"),
    (51, "3"),
    (52, "4"),
    (53, "5"),
    (54, "6"),
    (55, "7"),
    (56, "8"),
    (57, "9"),
    (65, "A"),
    (66, "B"),
    (67, "C"),
    (68, "D"),
    (69, "E"),
    (70, "F"),
    (71, "G"),
    (72, "H"),
    (73, "I"),
    (74, "J"),
    (75, "K"),
    (76, "L"),
    (77, "M"),
    (78, "N"),
    (79, "O"),
    (80, "P"),
    (81, "Q"),
    (82, "R"),
    (83, "S"),
    (84, "T"),
    (85, "U"),
    (86, "V"),
    (87, "W"),
    (88, "X"),
    (89, "Y"),
    (90, "Z"),
    (96, "Numpad0"),
    (97, "Numpad1"),
    (98, "Numpad2"),
    (99, "Numpad3"),
    (100, "Numpad4"),
    (101, "Numpad5"),
    (102, "Numpad6"),
    (103, "Numpad7"),
    (104, "Numpad8"),
    (105, "Numpad9"),
    (106, "NumpadMultiply"),
    (107, "NumpadAdd"),
    (109, "NumpadSubtract"),
    (110, "NumpadDecimal"),
    (111, "NumpadDivide"),
    (112, "F1"),
    (113, "F2"),
    (114, "F3"),
    (115, "F4"),
    (116, "F5"),
    (117, "F6"),
    (118, "F7"),
    (119, "F8"),
    (120, "F9"),
    (121, "F10"),
    (122, "F11"),
    (123, "F12"),
    (124, "F13"),
    (125, "F14"),
    (126, "F15"),
    (127, "F16"),
    (128, "F17"),
    (129, "F18"),
    (130, "F19"),
    (131, "F20"),
    (132, "F21"),
    (133, "F22"),
    (134, "F23"),
    (135, "F24"),
    (186, "Semicolon"),
    (187, "Equal"),
    (188, "Comma"),
    (189, "Minus"),
    (190, "Period"),
    (191, "Slash"),
    (192, "Backquote"),
    (219, "BracketLeft"),
    (220, "Backslash"),
    (221, "BracketRight"),
    (222, "Quote"),
];

pub fn key_name(code: u32) -> Option<&'static str> {
    KEY_CODES
        .iter()
        .find(|(known, _)| *known == code)
        .map(|(_, name)| *name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Hotkey {
    pub fn to_text(&self) -> String {
        let mut parts = Vec::new();

        if self.ctrl {
            parts.push("Ctrl");
        }

        if self.shift {
            parts.push("Shift");
        }

        if self.alt {
            parts.push("Alt");
        }

        parts.push(&self.key);

        parts.join(" + ")
    }
}

pub fn parse_ini(contents: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut section = String::new();

    for line in contents.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.trim().to_string();

            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        out.entry(section.clone())
            .or_default()
            .insert(key.trim().to_string(), value.trim().to_string());
    }

    out
}

pub fn parse_config_hotkey(entry: &str) -> Option<Hotkey> {
    let entry = entry.trim();

    if entry.is_empty() {
        return None;
    }

    let mut parts = entry.split_whitespace();

    let code: u32 = parts.next()?.parse().ok()?;

    if code == 0 {
        return None;
    }

    let key = key_name(code)?;

    let modifier = parts.next();

    if parts.next().is_some() {
        return None;
    }

    let (ctrl, shift, alt) = match modifier {
        None => (false, false, false),
        Some("1") => (false, true, false),
        Some("2") => (true, false, false),
        Some("3") => (false, false, true),
        Some(_) => return None,
    };

    Some(Hotkey {
        key: key.to_string(),
        ctrl,
        shift,
        alt,
    })
}

pub fn show_mods_key(config: &BTreeMap<String, BTreeMap<String, String>>) -> String {
    config
        .get(ACTION_KEYS)
        .and_then(|section| section.get(SHOW_ADVANCED))
        .and_then(|entry| parse_config_hotkey(entry))
        .map_or_else(|| DEFAULT_SHOW_MODS_KEY.to_string(), |h| h.to_text())
}

pub fn show_mods_key_was_read(config: &BTreeMap<String, BTreeMap<String, String>>) -> bool {
    config
        .get(ACTION_KEYS)
        .and_then(|section| section.get(SHOW_ADVANCED))
        .and_then(|entry| parse_config_hotkey(entry))
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(body: &str) -> BTreeMap<String, BTreeMap<String, String>> {
        parse_ini(body)
    }

    #[test]
    fn a_section_and_key_are_read() {
        let got = parse_ini("[ACTION_KEYS]\nshow_advanced_item_descriptions=67 2\n");

        assert_eq!(
            got["ACTION_KEYS"]["show_advanced_item_descriptions"],
            "67 2"
        );
    }

    #[test]
    fn the_byte_order_mark_does_not_hide_the_first_section() {
        let got = parse_ini("\u{feff}[ACTION_KEYS]\nkey=67\n");

        assert!(got.contains_key("ACTION_KEYS"));
    }

    #[test]
    fn whitespace_around_names_and_values_is_trimmed() {
        let got = parse_ini("[ ACTION_KEYS ]\n  key  =  67 2  \n");

        assert_eq!(got["ACTION_KEYS"]["key"], "67 2");
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let got = parse_ini("[A]\n; a comment\n\n# another\nkey=1\n");

        assert_eq!(got["A"].len(), 1);
    }

    #[test]
    fn a_line_with_no_equals_is_skipped_rather_than_failing() {
        let got = parse_ini("[A]\ngarbage\nkey=1\n");

        assert_eq!(got["A"]["key"], "1");
    }

    #[test]
    fn a_value_containing_an_equals_keeps_it() {
        let got = parse_ini("[A]\nkey=a=b\n");

        assert_eq!(got["A"]["key"], "a=b");
    }

    #[test]
    fn an_empty_file_reads_as_empty() {
        assert!(parse_ini("").is_empty());
    }

    #[test]
    fn a_key_before_any_section_does_not_panic() {
        let got = parse_ini("key=1\n[A]\nother=2\n");

        assert_eq!(got[""]["key"], "1");
    }

    #[test]
    fn a_bare_key_has_no_modifier() {
        let got = parse_config_hotkey("67").expect("a key parses");

        assert_eq!(got.key, "C");
        assert!(!got.ctrl && !got.shift && !got.alt);
    }

    #[test]
    fn each_modifier_number_maps_to_its_own_key() {
        assert!(parse_config_hotkey("67 1").expect("parses").shift);
        assert!(parse_config_hotkey("67 2").expect("parses").ctrl);
        assert!(parse_config_hotkey("67 3").expect("parses").alt);
    }

    #[test]
    fn exactly_one_modifier_is_set() {
        for entry in ["67 1", "67 2", "67 3"] {
            let got = parse_config_hotkey(entry).expect("parses");

            let count = [got.ctrl, got.shift, got.alt]
                .iter()
                .filter(|set| **set)
                .count();

            assert_eq!(count, 1, "{entry}");
        }
    }

    #[test]
    fn a_modifier_number_this_build_does_not_know_reports_nothing() {
        assert_eq!(parse_config_hotkey("67 4"), None);
        assert_eq!(parse_config_hotkey("67 x"), None);
    }

    #[test]
    fn an_entry_with_too_many_fields_reports_nothing() {
        assert_eq!(parse_config_hotkey("67 2 3"), None);
    }

    #[test]
    fn an_empty_entry_reports_nothing() {
        assert_eq!(parse_config_hotkey(""), None);
        assert_eq!(parse_config_hotkey("   "), None);
    }

    #[test]
    fn a_hotkey_renders_with_its_modifier_first() {
        let got = parse_config_hotkey("67 2").expect("parses");

        assert_eq!(got.to_text(), "Ctrl + C");
    }

    #[test]
    fn a_bare_key_renders_as_itself() {
        assert_eq!(parse_config_hotkey("18").expect("parses").to_text(), "Alt");
    }

    #[test]
    fn the_modifier_order_is_fixed() {
        let a = Hotkey {
            key: "C".into(),
            ctrl: true,
            shift: true,
            alt: true,
        };

        assert_eq!(a.to_text(), "Ctrl + Shift + Alt + C");
    }

    #[test]
    fn the_configured_key_is_used() {
        let got = show_mods_key(&config(
            "[ACTION_KEYS]\nshow_advanced_item_descriptions=67 2\n",
        ));

        assert_eq!(got, "Ctrl + C");
    }

    #[test]
    fn a_missing_setting_falls_back_to_the_game_default() {
        assert_eq!(show_mods_key(&config("[ACTION_KEYS]\n")), "Alt");
        assert_eq!(show_mods_key(&config("")), "Alt");
    }

    #[test]
    fn an_unreadable_setting_falls_back_rather_than_failing() {
        assert_eq!(
            show_mods_key(&config(
                "[ACTION_KEYS]\nshow_advanced_item_descriptions=67 9\n"
            )),
            "Alt"
        );
    }

    #[test]
    fn the_fallback_is_reported_as_a_fallback() {
        assert!(show_mods_key_was_read(&config(
            "[ACTION_KEYS]\nshow_advanced_item_descriptions=18\n"
        )));

        assert!(!show_mods_key_was_read(&config("")));
    }

    #[test]
    fn a_setting_in_another_section_is_not_used() {
        let got = config("[OTHER]\nshow_advanced_item_descriptions=67 2\n");

        assert_eq!(show_mods_key(&got), "Alt");
        assert!(!show_mods_key_was_read(&got));
    }

    #[test]
    fn a_real_looking_config_reads_the_right_line() {
        let body = "\u{feff}[SOUND]\nmaster_volume=100\n\n\
                    [ACTION_KEYS]\nmove_up=0\n\
                    show_advanced_item_descriptions=67 2\n\
                    highlight=90\n";

        assert_eq!(show_mods_key(&config(body)), "Ctrl + C");
    }
}
