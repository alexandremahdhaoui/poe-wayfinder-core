//! The item text parser.
//!
//! The algorithm is a section consuming ordered pipeline. See STUDY section 1.
//!
//! 1. Split the clipboard on lines equal to `--------`.
//! 2. Parse section zero as the name plate.
//! 3. Run an ordered list of stages. Each stage consumes at most one section
//!    and each section is consumed at most once.
//!
//! That consumption rule is why the modifier stage appears five times in the
//! pipeline. Each occurrence eats a different modifier section.

/// What a stage did with a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseOutcome {
    /// The section is consumed and removed from the remaining list.
    SectionParsed,
    /// Not this section. Try the stage against the next one.
    SectionSkipped,
    /// This stage does not apply to this item at all. Move to the next stage.
    ParserSkipped,
}

/// Split item text into sections on the game's separator line.
///
/// Empty sections are dropped. A trailing empty line is ignored.
pub fn text_to_sections(text: &str) -> Vec<Vec<String>> {
    let mut sections: Vec<Vec<String>> = vec![Vec::new()];

    for line in text.replace("\r\n", "\n").split('\n') {
        if line == "--------" {
            sections.push(Vec::new());
        } else {
            sections
                .last_mut()
                .expect("sections always has one element")
                .push(line.to_string());
        }
    }

    sections.retain(|s| !s.is_empty() && !(s.len() == 1 && s[0].is_empty()));

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
    fn drops_a_trailing_empty_section() {
        let got = text_to_sections("A\n--------\n");

        assert_eq!(got, vec![vec!["A"]]);
    }
}
