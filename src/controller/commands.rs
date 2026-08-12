#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub hotkey: String,
    pub text: String,
    pub send: bool,
}

pub const DEFAULTS: &str =
    "F5=/hideout;F9=/exit;=@last ty;=/invite @last;=/tradewith @last;=/hideout @last";

pub fn parse(declared: &str) -> Vec<Command> {
    declared.split(';').filter_map(one).collect()
}

fn one(entry: &str) -> Option<Command> {
    let entry = entry.trim();

    if entry.is_empty() {
        return None;
    }

    let (hotkey, text) = entry.split_once('=')?;

    let hotkey = hotkey.trim();
    let text = text.trim();

    if hotkey.is_empty() || text.is_empty() {
        return None;
    }

    let (text, send) = match text.strip_suffix("\\n") {
        Some(without) => (without, false),
        None => (text, true),
    };

    Some(Command {
        hotkey: hotkey.to_string(),
        text: text.to_string(),
        send,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_binds_a_key_to_a_line_of_chat() {
        let got = parse("F5=/hideout");

        assert_eq!(
            got,
            vec![Command {
                hotkey: "F5".to_string(),
                text: "/hideout".to_string(),
                send: true,
            }]
        );
    }

    #[test]
    fn several_commands_are_separated_by_semicolons() {
        assert_eq!(parse("F5=/hideout;F9=/exit").len(), 2);
    }

    #[test]
    fn a_command_with_no_key_is_skipped_rather_than_bound_to_nothing() {
        let got = parse("=@last ty;F5=/hideout");

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].hotkey, "F5");
    }

    #[test]
    fn a_command_with_no_text_is_skipped() {
        assert!(parse("F5=").is_empty());
        assert!(parse("F5").is_empty());
    }

    #[test]
    fn trailing_backslash_n_means_type_it_without_sending() {
        let got = parse("F5=/hideout\\n");

        assert_eq!(got[0].text, "/hideout");
        assert!(!got[0].send, "the line is typed and left for the user");
    }

    #[test]
    fn whitespace_around_an_entry_does_not_break_it() {
        let got = parse("  F5 = /hideout  ;  F9 = /exit ");

        assert_eq!(got.len(), 2);
        assert_eq!(got[0].hotkey, "F5");
        assert_eq!(got[1].text, "/exit");
    }

    #[test]
    fn an_empty_declaration_binds_nothing() {
        assert!(parse("").is_empty());
        assert!(parse(";;;").is_empty());
    }

    #[test]
    fn the_text_may_hold_an_equals_sign_of_its_own() {
        let got = parse("F5=/trade a=b");

        assert_eq!(got[0].text, "/trade a=b");
    }

    #[test]
    fn the_defaults_are_the_ones_upstream_ships() {
        let got = parse(DEFAULTS);

        assert_eq!(got.len(), 2, "only the two with a key are bound");
        assert_eq!(got[0].hotkey, "F5");
        assert_eq!(got[0].text, "/hideout");
        assert_eq!(got[1].hotkey, "F9");
        assert_eq!(got[1].text, "/exit");
    }

    #[test]
    fn every_default_entry_is_well_formed_even_when_it_has_no_key() {
        for entry in DEFAULTS.split(';') {
            assert!(entry.contains('='), "{entry:?} has no key separator");
        }
    }
}
