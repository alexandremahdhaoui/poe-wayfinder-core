//! Typing into the game's chat box.
//!
//! Ported from `main/src/shortcuts/text-box.ts` in Awakened PoE Trade.
//!
//! # Why an overlay needs this
//!
//! Pricing an item is half the job. The other half is whispering the seller,
//! and a user who has to alt tab and retype the whisper by hand has an overlay
//! that saved them nothing.
//!
//! # Why this is a key sequence and not an API call
//!
//! The game has no way in. The only way to put text in its chat box is to type
//! it, so the overlay puts the text on the clipboard and sends the keystrokes
//! that paste and send it.
//!
//! That makes the exact sequence the whole of the correctness. A missing
//! select all leaves the user's half typed message in front of the whisper and
//! sends both. A missing escape leaves the chat box open swallowing movement
//! keys, which in a game where standing still gets you killed is worse than
//! not whispering at all.

/// One keystroke to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Home,
    Delete,
    Up,
    /// Select all, with the platform's modifier held.
    SelectAll,
    /// Paste, with the platform's modifier held.
    Paste,
    /// Open the game's own search box, with the modifier held.
    Find,
    /// Send with the modifier held, which resumes the last chat channel.
    ResumeChat,
}

/// What the overlay has to do to type a message.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatAction {
    /// The text to put on the clipboard first.
    pub text: String,
    /// The keystrokes to send, in order.
    pub keys: Vec<Key>,
}

/// The marker meaning "the channel I last spoke in".
///
/// A user who wants to reply in whatever channel they were already using
/// writes `@last` rather than naming it, because the channel changes and the
/// hotkey does not.
pub const LAST_CHANNEL: &str = "@last";

/// Channel prefixes the game clears the box for on its own.
///
/// Typing one of these opens a fresh line, so sending a select all afterwards
/// would select nothing and the extra keystroke can land in the game instead.
const AUTO_CLEAR: [char; 6] = ['#', '%', '@', '$', '&', '/'];

/// Work out how to type a message into chat.
///
/// `send` says whether to submit it. A user often wants the whisper staged in
/// the box so they can add a word before sending.
pub fn type_in_chat(text: &str, send: bool) -> ChatAction {
    let mut keys = Vec::new();

    // A message that starts with the marker replies in the last channel used.
    // Resuming it puts the cursor after the channel prefix already.
    let body = if let Some(rest) = text.strip_prefix(LAST_CHANNEL) {
        keys.push(Key::ResumeChat);

        rest.strip_prefix(' ').unwrap_or(rest).to_string()
    } else if let Some(rest) = text.strip_suffix(LAST_CHANNEL) {
        // The marker at the end means the text goes before the channel prefix
        // the game inserts, so the cursor has to go back to the start and the
        // prefix has to come out.
        keys.push(Key::ResumeChat);
        // Twice. Once does not focus the box when the user is on a controller,
        // and the delete then lands in the game.
        keys.push(Key::Home);
        keys.push(Key::Home);
        keys.push(Key::Delete);

        rest.to_string()
    } else {
        keys.push(Key::Enter);

        // The game clears the box itself for a channel prefix. Selecting all
        // afterwards selects nothing and the keystroke can reach the game.
        if !starts_with_channel(text) {
            keys.push(Key::SelectAll);
        }

        text.to_string()
    };

    keys.push(Key::Paste);

    if send {
        keys.push(Key::Enter);

        // Reopen and step back through the history so the box ends holding
        // what the user had before, then close it. Leaving chat open swallows
        // movement keys, and standing still in this game gets you killed.
        keys.push(Key::Enter);
        keys.push(Key::Up);
        keys.push(Key::Up);
        keys.push(Key::Escape);
    }

    ChatAction { text: body, keys }
}

/// Whether the text opens a chat channel by itself.
fn starts_with_channel(text: &str) -> bool {
    text.chars().next().is_some_and(|c| AUTO_CLEAR.contains(&c))
}

/// Work out how to search the stash for a term.
///
/// Ported from `stashSearch`. Used to find the item the user just priced
/// without them typing its name.
pub fn stash_search(text: &str) -> ChatAction {
    ChatAction {
        text: text.to_string(),
        keys: vec![Key::Find, Key::Paste, Key::Enter],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_message_opens_chat_and_pastes() {
        let got = type_in_chat("hello", false);

        assert_eq!(got.text, "hello");
        assert_eq!(got.keys, [Key::Enter, Key::SelectAll, Key::Paste]);
    }

    #[test]
    fn a_plain_message_clears_whatever_was_in_the_box() {
        // Without the select all the user's half typed message stays in front
        // of the whisper and both get sent.
        assert!(type_in_chat("hello", false).keys.contains(&Key::SelectAll));
    }

    #[test]
    fn a_channel_prefix_skips_the_select_all() {
        // The game clears the box itself, so selecting all selects nothing and
        // the keystroke can reach the game instead.
        for prefix in ['#', '%', '@', '$', '&', '/'] {
            let got = type_in_chat(&format!("{prefix}hello"), false);

            assert!(
                !got.keys.contains(&Key::SelectAll),
                "{prefix} still selected all"
            );
        }
    }

    #[test]
    fn a_whisper_keeps_its_prefix_in_the_text() {
        // The prefix is what makes it a whisper. Stripping it sends the
        // message to whatever channel was open, which is usually global.
        let got = type_in_chat("@Seller hi", false);

        assert_eq!(got.text, "@Seller hi");
    }

    #[test]
    fn the_last_channel_marker_resumes_rather_than_opening() {
        // The channel a user was last in changes and the hotkey does not.
        let got = type_in_chat("@last thanks", false);

        assert_eq!(got.text, "thanks");
        assert_eq!(got.keys[0], Key::ResumeChat);
        assert!(!got.keys.contains(&Key::Enter));
    }

    #[test]
    fn the_marker_at_the_end_puts_the_text_before_the_prefix() {
        let got = type_in_chat("thanks@last", false);

        assert_eq!(got.text, "thanks");
        assert_eq!(got.keys[0], Key::ResumeChat);
    }

    #[test]
    fn the_trailing_marker_presses_home_twice() {
        // Once does not focus the box on a controller, and the delete then
        // lands in the game.
        let got = type_in_chat("thanks@last", false);

        let homes = got.keys.iter().filter(|k| **k == Key::Home).count();

        assert_eq!(homes, 2);
    }

    #[test]
    fn the_trailing_marker_removes_the_inserted_prefix() {
        assert!(type_in_chat("thanks@last", false)
            .keys
            .contains(&Key::Delete));
    }

    #[test]
    fn a_leading_marker_does_not_press_home() {
        // Resuming already leaves the cursor after the prefix.
        assert!(!type_in_chat("@last thanks", false)
            .keys
            .contains(&Key::Home));
    }

    #[test]
    fn every_message_pastes_exactly_once() {
        // Twice sends the text doubled, which in a whisper reads as a mistake
        // the seller sees.
        for text in ["hello", "@Seller hi", "@last thanks", "thanks@last"] {
            for send in [true, false] {
                let pastes = type_in_chat(text, send)
                    .keys
                    .iter()
                    .filter(|k| **k == Key::Paste)
                    .count();

                assert_eq!(pastes, 1, "{text} send={send}");
            }
        }
    }

    #[test]
    fn a_staged_message_is_not_sent() {
        // A user often wants the whisper in the box so they can add a word.
        let got = type_in_chat("hello", false);

        assert_eq!(*got.keys.last().expect("keys are sent"), Key::Paste);
    }

    #[test]
    fn a_sent_message_ends_by_closing_the_chat_box() {
        // Leaving it open swallows movement keys, and standing still in this
        // game gets you killed.
        let got = type_in_chat("hello", true);

        assert_eq!(*got.keys.last().expect("keys are sent"), Key::Escape);
    }

    #[test]
    fn a_sent_message_restores_what_the_user_had_typed() {
        // It steps back through the history so the box ends where it started.
        let got = type_in_chat("hello", true);

        let ups = got.keys.iter().filter(|k| **k == Key::Up).count();

        assert_eq!(ups, 2);
    }

    #[test]
    fn the_send_always_comes_after_the_paste() {
        // The other order sends an empty line and then types into a closed
        // box, so the text reaches the game as movement keys.
        //
        // A plain message opens chat with an Enter of its own, so the check is
        // that a submitting Enter exists after the paste and not that the
        // first Enter does.
        for text in ["hello", "@Seller hi", "@last thanks", "thanks@last"] {
            let keys = type_in_chat(text, true).keys;

            let paste = keys.iter().position(|k| *k == Key::Paste).expect("pastes");

            assert!(
                keys[paste + 1..].contains(&Key::Enter),
                "{text} never submits after pasting"
            );
        }
    }

    #[test]
    fn nothing_is_typed_before_the_chat_box_is_open() {
        // A paste before the box opens goes to the game, where Ctrl and V are
        // bound to skills.
        for text in ["hello", "@Seller hi", "@last thanks", "thanks@last"] {
            let keys = type_in_chat(text, true).keys;

            let opens = keys
                .iter()
                .position(|k| matches!(k, Key::Enter | Key::ResumeChat))
                .expect("the box is opened");

            let paste = keys.iter().position(|k| *k == Key::Paste).expect("pastes");

            assert!(opens < paste, "{text} pasted before opening chat");
        }
    }

    #[test]
    fn an_empty_message_still_produces_a_safe_sequence() {
        // A user can bind a hotkey to an empty string. It must not leave the
        // chat box open.
        let got = type_in_chat("", true);

        assert_eq!(*got.keys.last().expect("keys are sent"), Key::Escape);
    }

    #[test]
    fn the_bare_marker_leaves_no_text() {
        let got = type_in_chat(LAST_CHANNEL, false);

        assert_eq!(got.text, "");
    }

    #[test]
    fn a_stash_search_opens_the_search_box_and_submits() {
        let got = stash_search("Sapphire Ring");

        assert_eq!(got.text, "Sapphire Ring");
        assert_eq!(got.keys, [Key::Find, Key::Paste, Key::Enter]);
    }

    #[test]
    fn a_stash_search_never_opens_chat() {
        // An Enter before the find would type the search term into chat, and
        // a stash search that whispers the term to global is a leak.
        assert!(!stash_search("Sapphire Ring").keys.contains(&Key::SelectAll));
        assert_eq!(stash_search("x").keys[0], Key::Find);
    }
}
