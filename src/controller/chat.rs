#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Home,
    Delete,
    Up,
    SelectAll,
    Paste,
    Find,
    ResumeChat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatAction {
    pub text: String,
    pub keys: Vec<Key>,
}

pub const LAST_CHANNEL: &str = "@last";

const AUTO_CLEAR: [char; 6] = ['#', '%', '@', '$', '&', '/'];

pub fn type_in_chat(text: &str, send: bool) -> ChatAction {
    let mut keys = Vec::new();

    let body = if let Some(rest) = text.strip_prefix(LAST_CHANNEL) {
        keys.push(Key::ResumeChat);

        rest.strip_prefix(' ').unwrap_or(rest).to_string()
    } else if let Some(rest) = text.strip_suffix(LAST_CHANNEL) {
        keys.push(Key::ResumeChat);
        keys.push(Key::Home);
        keys.push(Key::Home);
        keys.push(Key::Delete);

        rest.to_string()
    } else {
        keys.push(Key::Enter);

        if !starts_with_channel(text) {
            keys.push(Key::SelectAll);
        }

        text.to_string()
    };

    keys.push(Key::Paste);

    if send {
        keys.push(Key::Enter);

        keys.push(Key::Enter);
        keys.push(Key::Up);
        keys.push(Key::Up);
        keys.push(Key::Escape);
    }

    ChatAction { text: body, keys }
}

fn starts_with_channel(text: &str) -> bool {
    text.chars().next().is_some_and(|c| AUTO_CLEAR.contains(&c))
}

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
        assert!(type_in_chat("hello", false).keys.contains(&Key::SelectAll));
    }

    #[test]
    fn a_channel_prefix_skips_the_select_all() {
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
        let got = type_in_chat("@Seller hi", false);

        assert_eq!(got.text, "@Seller hi");
    }

    #[test]
    fn the_last_channel_marker_resumes_rather_than_opening() {
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
        assert!(!type_in_chat("@last thanks", false)
            .keys
            .contains(&Key::Home));
    }

    #[test]
    fn every_message_pastes_exactly_once() {
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
        let got = type_in_chat("hello", false);

        assert_eq!(*got.keys.last().expect("keys are sent"), Key::Paste);
    }

    #[test]
    fn a_sent_message_ends_by_closing_the_chat_box() {
        let got = type_in_chat("hello", true);

        assert_eq!(*got.keys.last().expect("keys are sent"), Key::Escape);
    }

    #[test]
    fn a_sent_message_restores_what_the_user_had_typed() {
        let got = type_in_chat("hello", true);

        let ups = got.keys.iter().filter(|k| **k == Key::Up).count();

        assert_eq!(ups, 2);
    }

    #[test]
    fn the_send_always_comes_after_the_paste() {
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
        assert!(!stash_search("Sapphire Ring").keys.contains(&Key::SelectAll));
        assert_eq!(stash_search("x").keys[0], Key::Find);
    }
}
