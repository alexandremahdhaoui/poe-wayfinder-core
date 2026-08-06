//! Reading the game's own configuration file.
//!
//! Ported from `main/src/host-files/GameConfig.ts` in Awakened PoE Trade.
//!
//! # Why the overlay reads the game's config
//!
//! The parser needs Advanced Item Description turned on. Without it the game
//! prints no modifier tiers, no ranges and no modifier names, and every price
//! the overlay produces is built from less than the item actually says.
//!
//! The game binds that to a key the user can change. The overlay has to hold
//! that key while copying, so it has to know which key it is. Guessing Alt is
//! right for most people and silently wrong for the rest, and the failure
//! looks like the parser being bad rather than a setting being different.

use std::collections::BTreeMap;

/// The section holding the game's action keys.
const ACTION_KEYS: &str = "ACTION_KEYS";

/// The setting that toggles the detailed item text.
const SHOW_ADVANCED: &str = "show_advanced_item_descriptions";

/// The key the overlay falls back to.
///
/// Alt is the game's own default. A user who never changed it gets the right
/// answer even when the config file cannot be found at all.
pub const DEFAULT_SHOW_MODS_KEY: &str = "Alt";

/// One hotkey read from the game's config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Hotkey {
    /// Render as the overlay writes hotkeys, modifiers first.
    ///
    /// A fixed order so two spellings of one combination compare equal.
    /// `Ctrl + Shift + C` and `Shift + Ctrl + C` are the same key and must not
    /// read as a conflict.
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

/// Parse an ini file into sections of key and value.
///
/// Hand written because the game's file is a plain ini and a dependency for
/// this is not worth it. Unknown lines are skipped rather than failing: the
/// game writes settings this build has never heard of, and refusing the whole
/// file over one would lose the setting we came for.
pub fn parse_ini(contents: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut section = String::new();

    // The game writes a byte order mark. Left in place it makes the first
    // section name unmatchable.
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

/// Turn a config entry into a hotkey.
///
/// Ported from `parseConfigHotkey`. The game stores the key name and a
/// modifier number separated by a space, so `C 2` is Ctrl and C.
///
/// Returns nothing for an entry this build cannot read. Falling back to a
/// guess would hold the wrong key while copying, and the item would come back
/// without its tiers with nothing saying why.
pub fn parse_config_hotkey(entry: &str) -> Option<Hotkey> {
    let entry = entry.trim();

    if entry.is_empty() {
        return None;
    }

    let mut parts = entry.split_whitespace();

    let key = parts.next()?;

    if key.is_empty() {
        return None;
    }

    let modifier = parts.next();

    // An entry with more than a key and one modifier is a shape this build
    // does not know, and reading the first two fields of it would produce a
    // confident wrong answer.
    if parts.next().is_some() {
        return None;
    }

    let (ctrl, shift, alt) = match modifier {
        None => (false, false, false),
        Some("1") => (false, true, false),
        Some("2") => (true, false, false),
        Some("3") => (false, false, true),
        // Any other number is a modifier the game added since this was
        // written.
        Some(_) => return None,
    };

    Some(Hotkey {
        key: key.to_string(),
        ctrl,
        shift,
        alt,
    })
}

/// The key that shows the detailed item text.
///
/// Ported from the `showModsKey` accessor. Falls back to the game's own
/// default when the setting is missing or unreadable, because a user who never
/// changed it still gets the right answer.
pub fn show_mods_key(config: &BTreeMap<String, BTreeMap<String, String>>) -> String {
    config
        .get(ACTION_KEYS)
        .and_then(|section| section.get(SHOW_ADVANCED))
        .and_then(|entry| parse_config_hotkey(entry))
        .map_or_else(|| DEFAULT_SHOW_MODS_KEY.to_string(), |h| h.to_text())
}

/// Whether the setting was actually read rather than assumed.
///
/// The UI needs the difference. "Alt, from your config" and "Alt, because we
/// could not read your config" are the same key and very different amounts of
/// confidence, and the second is worth telling the user about before they
/// report the parser as broken.
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

    // -----------------------------------------------------------------
    // The ini reader
    // -----------------------------------------------------------------

    #[test]
    fn a_section_and_key_are_read() {
        let got = parse_ini("[ACTION_KEYS]\nshow_advanced_item_descriptions=C 2\n");

        assert_eq!(got["ACTION_KEYS"]["show_advanced_item_descriptions"], "C 2");
    }

    #[test]
    fn the_byte_order_mark_does_not_hide_the_first_section() {
        // Left in place it makes the section name unmatchable and the setting
        // reads as missing.
        let got = parse_ini("\u{feff}[ACTION_KEYS]\nkey=C\n");

        assert!(got.contains_key("ACTION_KEYS"));
    }

    #[test]
    fn whitespace_around_names_and_values_is_trimmed() {
        let got = parse_ini("[ ACTION_KEYS ]\n  key  =  C 2  \n");

        assert_eq!(got["ACTION_KEYS"]["key"], "C 2");
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let got = parse_ini("[A]\n; a comment\n\n# another\nkey=1\n");

        assert_eq!(got["A"].len(), 1);
    }

    #[test]
    fn a_line_with_no_equals_is_skipped_rather_than_failing() {
        // The game writes settings this build has never heard of, and refusing
        // the whole file over one loses the setting we came for.
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

    // -----------------------------------------------------------------
    // Hotkeys
    // -----------------------------------------------------------------

    #[test]
    fn a_bare_key_has_no_modifier() {
        let got = parse_config_hotkey("C").expect("a key parses");

        assert_eq!(got.key, "C");
        assert!(!got.ctrl && !got.shift && !got.alt);
    }

    #[test]
    fn each_modifier_number_maps_to_its_own_key() {
        // The game stores them as numbers, and swapping two would hold the
        // wrong key while copying.
        assert!(parse_config_hotkey("C 1").expect("parses").shift);
        assert!(parse_config_hotkey("C 2").expect("parses").ctrl);
        assert!(parse_config_hotkey("C 3").expect("parses").alt);
    }

    #[test]
    fn exactly_one_modifier_is_set() {
        for entry in ["C 1", "C 2", "C 3"] {
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
        // Falling back to a guess would hold the wrong key and the item comes
        // back without its tiers with nothing saying why.
        assert_eq!(parse_config_hotkey("C 4"), None);
        assert_eq!(parse_config_hotkey("C x"), None);
    }

    #[test]
    fn an_entry_with_too_many_fields_reports_nothing() {
        // Reading the first two fields of a shape this build does not know
        // produces a confident wrong answer.
        assert_eq!(parse_config_hotkey("C 2 3"), None);
    }

    #[test]
    fn an_empty_entry_reports_nothing() {
        assert_eq!(parse_config_hotkey(""), None);
        assert_eq!(parse_config_hotkey("   "), None);
    }

    #[test]
    fn a_hotkey_renders_with_its_modifier_first() {
        let got = parse_config_hotkey("C 2").expect("parses");

        assert_eq!(got.to_text(), "Ctrl + C");
    }

    #[test]
    fn a_bare_key_renders_as_itself() {
        assert_eq!(parse_config_hotkey("Alt").expect("parses").to_text(), "Alt");
    }

    #[test]
    fn the_modifier_order_is_fixed() {
        // Two spellings of one combination must compare equal, or they read as
        // a conflict that is not there.
        let a = Hotkey {
            key: "C".into(),
            ctrl: true,
            shift: true,
            alt: true,
        };

        assert_eq!(a.to_text(), "Ctrl + Shift + Alt + C");
    }

    // -----------------------------------------------------------------
    // The setting we came for
    // -----------------------------------------------------------------

    #[test]
    fn the_configured_key_is_used() {
        let got = show_mods_key(&config(
            "[ACTION_KEYS]\nshow_advanced_item_descriptions=C 2\n",
        ));

        assert_eq!(got, "Ctrl + C");
    }

    #[test]
    fn a_missing_setting_falls_back_to_the_game_default() {
        // A user who never changed it still gets the right answer.
        assert_eq!(show_mods_key(&config("[ACTION_KEYS]\n")), "Alt");
        assert_eq!(show_mods_key(&config("")), "Alt");
    }

    #[test]
    fn an_unreadable_setting_falls_back_rather_than_failing() {
        assert_eq!(
            show_mods_key(&config(
                "[ACTION_KEYS]\nshow_advanced_item_descriptions=C 9\n"
            )),
            "Alt"
        );
    }

    #[test]
    fn the_fallback_is_reported_as_a_fallback() {
        // "Alt, from your config" and "Alt, because we could not read your
        // config" are the same key and very different confidence.
        assert!(show_mods_key_was_read(&config(
            "[ACTION_KEYS]\nshow_advanced_item_descriptions=Alt\n"
        )));

        assert!(!show_mods_key_was_read(&config("")));
    }

    #[test]
    fn a_setting_in_another_section_is_not_used() {
        // A key of the same name elsewhere is a different setting, and reading
        // it would hold a key bound to something else entirely.
        let got = config("[OTHER]\nshow_advanced_item_descriptions=C 2\n");

        assert_eq!(show_mods_key(&got), "Alt");
        assert!(!show_mods_key_was_read(&got));
    }

    #[test]
    fn a_real_looking_config_reads_the_right_line() {
        let body = "\u{feff}[SOUND]\nmaster_volume=100\n\n\
                    [ACTION_KEYS]\nmove_item=LeftButton\n\
                    show_advanced_item_descriptions=LeftAlt 2\n\
                    highlight=Z\n";

        assert_eq!(show_mods_key(&config(body)), "Ctrl + LeftAlt");
    }
}
