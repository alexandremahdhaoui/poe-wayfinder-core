//! The overlay's own runtime decisions.
//!
//! Ported from `main/src/shortcuts/Shortcuts.ts`,
//! `main/src/shortcuts/HostClipboard.ts` and
//! `main/src/windowing/WidgetAreaTracker.ts` in Awakened PoE Trade.
//!
//! # Why this is in the domain crate
//!
//! None of it touches a window. Whether a point is inside a widget, whether
//! the clipboard holds an item, which keys to hold while copying: all of it is
//! arithmetic and string work that can be tested without a screen, a mouse or
//! a running game.
//!
//! Leaving it in the driver made it untestable, and untestable is how the
//! overlay ended up unable to whisper a seller without anyone noticing.

/// A rectangle in screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Whether a point falls inside a rectangle.
///
/// Ported from `isPointInsideRect`.
///
/// # Why the edges are excluded
///
/// The overlay takes mouse input only while the pointer is over one of its
/// widgets. A point exactly on the border belongs to the game, because the
/// game's own edge is what the user is aiming at when they hit it, and
/// stealing that click makes the overlay feel like it is in the way.
pub fn point_in_rect(x: i32, y: i32, rect: Rect) -> bool {
    x > rect.x && x < rect.x + rect.width && y > rect.y && y < rect.y + rect.height
}

/// The first line every English item begins with.
///
/// This build is English only by policy, so a client in another language is
/// detected and refused rather than half parsed.
pub const ITEM_FIRST_LINE: &str = "Item Class: ";

/// The first lines the other client languages print.
///
/// Only used to tell a user their client is not English. Nothing here is
/// parsed, because a partial parse of a language we cannot read produces a
/// confident wrong price.
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

/// What the clipboard turned out to hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardKind {
    /// An English item. The only thing this build can price.
    Item,
    /// An item from a client in another language, named by its code.
    ForeignItem(&'static str),
    /// Anything else.
    NotAnItem,
}

/// Read what the clipboard holds.
///
/// Ported from `isPoeItem`.
///
/// # Why this is checked before parsing
///
/// The overlay presses Ctrl and C in the game and then reads the clipboard. It
/// cannot know when the game has answered, so it polls until it sees item
/// text. Without this check it would accept whatever the user had copied
/// before and price that instead, which is how a shopping list becomes a
/// price for a Sapphire Ring.
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

/// Which modifier keys to hold while copying an item.
///
/// Ported from `pressKeysToCopyItemText`.
///
/// # Why any are held at all
///
/// The game prints the detailed item text only while its own show mods key is
/// down. Copying without it gives text with no tiers, no ranges and no
/// modifier names, and every price built from it is worse than it looks.
///
/// A key the user is already holding is skipped. Pressing a key that is
/// already down and releasing it afterwards leaves the game seeing a release
/// it never saw pressed, which on a movement key means the character keeps
/// running.
pub fn keys_to_hold_for_copy(show_mods_key: &str, already_held: &[String]) -> Vec<String> {
    show_mods_key
        .split(" + ")
        .map(str::trim)
        .filter(|key| !key.is_empty())
        // C is the copy key itself and is tapped rather than held.
        .filter(|key| *key != "C")
        .filter(|key| !already_held.iter().any(|held| held == key))
        .map(str::to_string)
        .collect()
}

/// Render a key event as a hotkey string.
///
/// Ported from `eventToString`. The order is fixed so two spellings of one
/// combination compare equal.
///
/// A modifier pressed by itself is its own name. Rendering it as `Ctrl + Ctrl`
/// makes a hotkey the user can never trigger.
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

/// The fraction of the window height where the stash panel starts.
///
/// Measured against a 1600 pixel tall client, which is where the reference
/// took them from.
const STASH_TOP: f64 = 154.0 / 1600.0;

/// The fraction of the window height where the stash panel ends.
const STASH_BOTTOM: f64 = 1192.0 / 1600.0;

/// Whether the pointer is over the stash panel.
///
/// Ported from `isStashArea`. Used by the scroll to change tabs feature, which
/// must not fire while the user is scrolling anything else.
///
/// The bounds are fractions of the window rather than pixels, because the
/// panel scales with the client and a pixel offset is right at one resolution
/// only.
pub fn is_stash_area(x: i32, y: i32, window: Rect, sidebar_width: i32) -> bool {
    // Past the sidebar is the game world, not the stash.
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

    // -----------------------------------------------------------------
    // Hit testing
    // -----------------------------------------------------------------

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
        // The game's own edge is what the user is aiming at when they hit it,
        // and stealing that click makes the overlay feel like it is in the way.
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
        // A second monitor left of the primary one has negative coordinates,
        // and an overlay that ignored them would never take a click there.
        let left = Rect {
            x: -500,
            y: 0,
            width: 200,
            height: 200,
        };

        assert!(point_in_rect(-400, 100, left));
        assert!(!point_in_rect(-600, 100, left));
    }

    // -----------------------------------------------------------------
    // What the clipboard holds
    // -----------------------------------------------------------------

    #[test]
    fn english_item_text_is_recognised() {
        assert_eq!(
            clipboard_kind("Item Class: Rings\nRarity: Rare\n"),
            ClipboardKind::Item
        );
    }

    #[test]
    fn anything_else_is_not_an_item() {
        // Without this the overlay prices whatever the user copied before,
        // which is how a shopping list becomes a price for a ring.
        assert_eq!(clipboard_kind("milk, eggs"), ClipboardKind::NotAnItem);
        assert_eq!(clipboard_kind(""), ClipboardKind::NotAnItem);
    }

    #[test]
    fn a_foreign_client_is_named_rather_than_parsed() {
        // A partial parse of a language we cannot read produces a confident
        // wrong price.
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
        // Two languages sharing a first line would report whichever came
        // first, and tell the user to change a setting they already have.
        let mut seen: Vec<&str> = FOREIGN_FIRST_LINES.iter().map(|(_, l)| *l).collect();
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();

        assert_eq!(seen.len(), count);
    }

    #[test]
    fn no_foreign_first_line_collides_with_english() {
        // A collision would send a foreign item into the English parser.
        for (lang, line) in FOREIGN_FIRST_LINES {
            assert!(!line.starts_with(ITEM_FIRST_LINE), "{lang}");
            assert!(!ITEM_FIRST_LINE.starts_with(line), "{lang}");
        }
    }

    #[test]
    fn text_that_merely_contains_the_marker_is_not_an_item() {
        // The game always puts it first. Accepting it anywhere lets a chat log
        // quoting an item look like the item.
        assert_eq!(
            clipboard_kind("look at this Item Class: Rings"),
            ClipboardKind::NotAnItem
        );
    }

    // -----------------------------------------------------------------
    // Copying
    // -----------------------------------------------------------------

    #[test]
    fn the_show_mods_key_is_held_while_copying() {
        // Copying without it gives text with no tiers and no ranges, and every
        // price built from it is worse than it looks.
        assert_eq!(keys_to_hold_for_copy("Alt", &[]), ["Alt"]);
    }

    #[test]
    fn a_combination_holds_every_part() {
        assert_eq!(keys_to_hold_for_copy("Ctrl + Alt", &[]), ["Ctrl", "Alt"]);
    }

    #[test]
    fn the_copy_key_itself_is_not_held() {
        // It is tapped, and holding it repeats the copy.
        assert_eq!(keys_to_hold_for_copy("Ctrl + C", &[]), ["Ctrl"]);
    }

    #[test]
    fn a_key_the_user_already_holds_is_skipped() {
        // Releasing it afterwards leaves the game seeing a release it never
        // saw pressed, and on a movement key the character keeps running.
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
        // Better than pressing an empty key name, which reaches the game as
        // nothing useful.
        assert!(keys_to_hold_for_copy("", &[]).is_empty());
        assert!(keys_to_hold_for_copy("   ", &[]).is_empty());
    }

    // -----------------------------------------------------------------
    // Hotkey rendering
    // -----------------------------------------------------------------

    #[test]
    fn a_plain_key_renders_as_itself() {
        assert_eq!(event_to_hotkey("D", false, false, false), "D");
    }

    #[test]
    fn modifiers_render_in_a_fixed_order() {
        // Two spellings of one combination must compare equal.
        assert_eq!(
            event_to_hotkey("D", true, true, true),
            "Ctrl + Shift + Alt + D"
        );
        assert_eq!(event_to_hotkey("D", true, false, true), "Ctrl + Alt + D");
        assert_eq!(event_to_hotkey("D", false, true, true), "Shift + Alt + D");
    }

    #[test]
    fn a_modifier_pressed_alone_is_its_own_name() {
        // Rendering it as "Ctrl + Ctrl" makes a hotkey the user can never
        // trigger.
        for key in ["Ctrl", "Shift", "Alt"] {
            assert_eq!(event_to_hotkey(key, true, true, true), key);
        }
    }

    // -----------------------------------------------------------------
    // The stash panel
    // -----------------------------------------------------------------

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
        // Scrolling there changes tabs while the user is looking at the map.
        assert!(!is_stash_area(1000, 800, window(), 600));
    }

    #[test]
    fn the_bounds_scale_with_the_window() {
        // The panel scales with the client, so a pixel offset is right at one
        // resolution only.
        let small = Rect {
            x: 0,
            y: 0,
            width: 1280,
            height: 800,
        };

        // Half the height, so the same fraction is half the pixels.
        assert!(is_stash_area(100, 400, small, 300));
        assert!(!is_stash_area(100, 50, small, 300));
    }

    #[test]
    fn an_offset_window_moves_the_panel_with_it() {
        // A windowed client is not at the origin, and reading in screen
        // coordinates without the offset puts the panel off the window.
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
        // Reversed, the span is empty and every point reads as outside. Tested
        // through the function rather than on the constants, so the check is
        // of the behaviour and not of two literals.
        let mid = f64::from(window().height) * (STASH_TOP + STASH_BOTTOM) / 2.0;

        assert!(is_stash_area(100, mid as i32, window(), 600));
    }
}
