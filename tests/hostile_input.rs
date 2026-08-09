//! The parser must not panic on any input.
//!
//! Clipboard text is fully attacker controlled. Anyone who can get a string in
//! front of a user can put one on their clipboard, and a panic in an overlay
//! that runs alongside a game is a crash the user blames on the game.
//!
//! This is a smoke fuzz and not a proof. It covers the shapes that break hand
//! written scanners: truncation, unbalanced delimiters, huge numbers, and
//! multi byte characters split at a boundary.

use poe_wayfinder_core::adapter::data_adapter::NO_DATA;
use poe_wayfinder_core::controller::parse::parse_clipboard;
use poe_wayfinder_core::types::GameVersion;

/// Parse must return, never panic. The result itself does not matter.
fn survives(text: &str) {
    for game in [GameVersion::Poe1, GameVersion::Poe2] {
        let _ = parse_clipboard(text, game, &NO_DATA);
    }
}

fn item(tail: &str) -> String {
    format!("Item Class: Rings\nRarity: Rare\nDoom Loop\nSapphire Ring\n--------\n{tail}")
}

#[test]
fn empty_and_whitespace_input_survives() {
    for text in ["", " ", "\n", "\r\n", "\t", "\0", "\n\n\n"] {
        survives(text);
    }
}

#[test]
fn separator_abuse_survives() {
    survives("--------");
    survives("--------\n--------");
    survives("-------");
    survives("---------");
    survives(" -------- ");
    survives(&"--------\n".repeat(500));
}

#[test]
fn a_truncated_item_survives_at_every_length() {
    let full = item("Item Level: 78\n--------\n+25(20-30) to maximum Life\n--------\nCorrupted");

    // Every prefix, byte by byte. A scanner that reads one character past the
    // end fails here and nowhere else.
    for end in 0..=full.len() {
        if full.is_char_boundary(end) {
            survives(&full[..end]);
        }
    }
}

#[test]
fn unbalanced_markup_survives() {
    for tail in [
        "<if:X>{",
        "<<set:",
        "<<<<<<<<",
        "}}}}}}}}",
        "<if:><if:><if:>{",
        "{unclosed metadata",
        "metadata unopened}",
    ] {
        survives(&format!("Item Class: Rings\nRarity: Rare\n{tail}"));
        survives(&item(tail));
    }
}

#[test]
fn hostile_numbers_survive() {
    for tail in [
        "+99999999999999999999999999 to maximum Life",
        "+-+-+-+-12 to maximum Life",
        "+1(2-3(4-5)) to maximum Life",
        "+1( to maximum Life",
        "+1) to maximum Life",
        "Item Level: 99999999999999999999",
        "Item Level: -0",
        "Stack Size: 1/0",
        "Stack Size: /",
        "Quality: +%",
        "Adds 1 to 2 to 3 to 4 to 5 Fire Damage",
        "+1.2.3.4.5% increased",
    ] {
        survives(&item(tail));
    }
}

#[test]
fn multi_byte_characters_survive() {
    // A scanner that indexes by byte rather than by char panics on these.
    for tail in [
        "\u{65e5}\u{672c}\u{8a9e}",
        "+25 to maximum L\u{e4}ife",
        "\u{1f525}\u{1f525}\u{1f525}",
        "Item Level: \u{ff18}\u{ff14}",
        "\u{ff08}reminder\u{ff09}",
        "\u{2014}\u{2014}\u{2014}",
        "\u{e9}+1\u{e9}(\u{e9}2\u{e9}-\u{e9}3\u{e9})\u{e9}",
    ] {
        survives(&item(tail));
    }
}

#[test]
fn a_very_large_input_survives() {
    // No stack recursion and no quadratic blowup that would hang the overlay.
    survives(&item(&"+25 to maximum Life\n".repeat(20_000)));
}

#[test]
fn a_line_of_only_numbers_survives() {
    // The template generator caps at four numbers. A line of a thousand must
    // fall through to the verbatim case rather than build 2^1000 candidates.
    let numbers = (0..1000)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(" ");

    survives(&item(&numbers));
}

#[test]
fn a_cannot_use_line_in_a_hostile_position_survives() {
    // The hoist rewrites the section list. Feeding it shapes it does not
    // expect is the obvious way to break it.
    const LINE: &str = "You cannot use this item. Its stats will be ignored";

    survives(&format!("a\nb\n{LINE}"));
    survives(LINE);
    survives(&format!("a\nb\n{LINE}\n--------"));
    survives(&format!("a\nb\n{LINE}\n--------\n--------"));
}
