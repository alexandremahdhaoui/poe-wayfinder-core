//! Our config reader against a real Path of Exile 2 config file.
//!
//! `fixtures/poe2_config.ini` is the key binding sections of an actual
//! `poe2_production_Config.ini`, taken from an installed game. Only the
//! sections that hold key bindings were kept, so nothing personal is in it.
//!
//! # What this caught that the unit tests could not
//!
//! The unit tests were written against config text I invented, and I invented
//! it wrong. I assumed the game stored key names, so `show_advanced_item_
//! descriptions=C 2` parsed and rendered as `Ctrl + C` and every test passed.
//!
//! The game stores key codes. The real line is
//! `show_advanced_item_descriptions=18`, and the reader turned that into the
//! string "18". The overlay would have told the user to hold a key called 18
//! and held nothing while copying, so every item would have come back without
//! its modifier tiers.
//!
//! Synthetic fixtures test the code against my understanding. This tests it
//! against the game.

use poe_wayfinder_core::controller::game_config::{
    key_name, parse_config_hotkey, parse_ini, show_mods_key, show_mods_key_was_read,
};

const REAL: &str = include_str!("fixtures/poe2_config.ini");

fn config() -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>> {
    parse_ini(REAL)
}

#[test]
fn the_real_file_parses_into_sections() {
    let got = config();

    assert!(
        got.contains_key("ACTION_KEYS"),
        "sections: {:?}",
        got.keys()
    );
    assert!(!got["ACTION_KEYS"].is_empty());
}

#[test]
fn the_setting_the_overlay_needs_is_found() {
    // The whole reason the file is read. Not finding it means the overlay
    // falls back and holds a key the user may have rebound.
    assert!(
        show_mods_key_was_read(&config()),
        "the show mods key was not read from a real config"
    );
}

#[test]
fn the_real_show_mods_key_reads_as_a_key_name() {
    // The bug this file exists for. The game stores 18, and rendering that as
    // "18" tells the user to hold a key that does not exist.
    let got = show_mods_key(&config());

    // 18 is Alt. Rendering it as "18" is what this file caught.
    assert_eq!(got, "Alt");
    assert_ne!(got, "18");
}

#[test]
fn every_bound_action_in_the_real_file_reads_as_a_key_name() {
    // A single unreadable binding is a key the overlay would either hold
    // wrongly or report as a number.
    let config = config();
    let mut checked = 0;

    for (name, entry) in &config["ACTION_KEYS"] {
        let Some(hotkey) = parse_config_hotkey(entry) else {
            // Unbound, or a code this build does not know. Both are reported
            // as nothing rather than guessed, which is the safe answer.
            continue;
        };

        // The name must not be the code echoed back. A digit key is honestly
        // named "1", so checking for digits alone rejects a correct answer:
        // code 49 is the number one key and "1" is what it is called. What
        // must never happen is the name equalling the code.
        let code = entry.split_whitespace().next().unwrap_or_default();

        assert_ne!(
            hotkey.key, code,
            "{name}={entry} rendered as its own code rather than a key name"
        );

        checked += 1;
    }

    // The file binds plenty. Zero would mean this test proved nothing.
    assert!(checked > 20, "only {checked} bindings parsed");
}

#[test]
fn an_unbound_action_reports_nothing() {
    // The real file writes 0 for an action with no key. Rendering it would
    // tell the user to hold something the game ignores.
    let config = config();

    let unbound: Vec<&String> = config["ACTION_KEYS"]
        .iter()
        .filter(|(_, v)| v.trim() == "0")
        .map(|(k, _)| k)
        .collect();

    assert!(!unbound.is_empty(), "the real file binds everything");

    for name in unbound {
        assert_eq!(
            parse_config_hotkey(&config["ACTION_KEYS"][name]),
            None,
            "{name} is unbound and still produced a hotkey"
        );
    }
}

#[test]
fn the_modifier_suffix_in_the_real_file_is_understood() {
    // The file writes bindings like "81 2", a key and a modifier. Reading only
    // the first field loses the modifier and the overlay reports a hotkey the
    // user never set.
    let config = config();

    let with_modifier: Vec<(&String, &String)> = config["ACTION_KEYS"]
        .iter()
        .filter(|(_, v)| v.split_whitespace().count() == 2)
        .collect();

    assert!(
        !with_modifier.is_empty(),
        "no binding in the real file carries a modifier"
    );

    for (name, entry) in with_modifier {
        let Some(hotkey) = parse_config_hotkey(entry) else {
            continue;
        };

        assert!(
            hotkey.ctrl || hotkey.shift || hotkey.alt,
            "{name}={entry} lost its modifier"
        );
    }
}

#[test]
fn the_codes_the_real_file_uses_are_all_known() {
    // A code missing from the table is a key the overlay cannot name. Counting
    // them keeps the gap visible instead of silently falling back.
    let config = config();
    let mut unknown = Vec::new();

    for (name, entry) in &config["ACTION_KEYS"] {
        let Some(first) = entry.split_whitespace().next() else {
            continue;
        };

        let Ok(code) = first.parse::<u32>() else {
            continue;
        };

        // Zero is unbound, not a key.
        if code != 0 && key_name(code).is_none() {
            unknown.push((name.clone(), code));
        }
    }

    // Mouse buttons and a few pad codes sit outside the table the reference
    // ships. They are named here rather than asserted to zero, because the
    // overlay only ever reads one setting and that one is known.
    assert!(
        unknown.len() < 20,
        "{} codes in the real file have no name: {unknown:?}",
        unknown.len()
    );
}

#[test]
fn parsing_the_real_file_is_deterministic() {
    assert_eq!(parse_ini(REAL), parse_ini(REAL));
}

#[test]
fn the_real_file_survives_being_truncated_anywhere() {
    // A config file being written while the overlay reads it is a partial
    // file, and a panic there takes the overlay down at startup.
    for end in 0..REAL.len() {
        if !REAL.is_char_boundary(end) {
            continue;
        }

        let _ = parse_ini(&REAL[..end]);
    }
}
