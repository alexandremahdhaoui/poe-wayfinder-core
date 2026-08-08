pub const SEPARATOR: &str = "--------";

pub fn text_to_sections(text: &str) -> Vec<Vec<String>> {
    let normalised = text.replace("\r\n", "\n");
    let mut lines: Vec<&str> = normalised.split('\n').collect();

    if lines.last() == Some(&"") {
        lines.pop();
    }

    let mut sections: Vec<Vec<String>> = vec![Vec::new()];

    for line in lines {
        if line == SEPARATOR {
            sections.push(Vec::new());
        } else {
            sections
                .last_mut()
                .expect("sections always holds one element")
                .push(line.to_string());
        }
    }

    sections.retain(|s| !s.is_empty());

    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_the_separator() {
        let text = "Item Class: Bows\nRarity: Rare\n--------\nQuality: +20%\n--------\nCorrupted";

        let got = text_to_sections(text);

        assert_eq!(got.len(), 3);
        assert_eq!(got[0], vec!["Item Class: Bows", "Rarity: Rare"]);
        assert_eq!(got[1], vec!["Quality: +20%"]);
        assert_eq!(got[2], vec!["Corrupted"]);
    }

    #[test]
    fn handles_windows_line_endings() {
        let got = text_to_sections("A\r\n--------\r\nB");

        assert_eq!(got, vec![vec!["A"], vec!["B"]]);
    }

    #[test]
    fn drops_only_the_final_trailing_newline() {
        let got = text_to_sections("A\n--------\nB\n");

        assert_eq!(got, vec![vec!["A"], vec!["B"]]);
    }

    #[test]
    fn a_trailing_separator_leaves_no_empty_section() {
        let got = text_to_sections("A\n--------\n");

        assert_eq!(got, vec![vec!["A"]]);
    }

    #[test]
    fn a_section_holding_one_blank_line_survives() {
        let got = text_to_sections("A\n--------\n\n--------\nB");

        assert_eq!(got, vec![vec!["A"], vec![""], vec!["B"]]);
    }

    #[test]
    fn back_to_back_separators_produce_nothing() {
        let got = text_to_sections("A\n--------\n--------\nB");

        assert_eq!(got, vec![vec!["A"], vec!["B"]]);
    }

    #[test]
    fn empty_text_yields_no_sections() {
        assert!(text_to_sections("").is_empty());
    }

    #[test]
    fn a_lone_separator_yields_no_sections() {
        assert!(text_to_sections("--------").is_empty());
    }

    #[test]
    fn a_line_that_only_resembles_the_separator_is_content() {
        let got = text_to_sections("-------\n--------\n---------");

        assert_eq!(got, vec![vec!["-------"], vec!["---------"]]);
    }

    #[test]
    fn indentation_around_a_separator_makes_it_content() {
        let got = text_to_sections("A\n -------- \nB");

        assert_eq!(got, vec![vec!["A", " -------- ", "B"]]);
    }
}
